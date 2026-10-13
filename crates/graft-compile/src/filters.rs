// SPDX-License-Identifier: Apache-2.0

//! The lowerer is this ffmpeg binary, not a hand-maintained op list.
//!
//! Structural ops stay dedicated. Every other name is a filter this process
//! can run. Options come from `ffmpeg -h filter=`. A missing filter fails
//! closed as `toolchain`.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::process::Command;
use std::sync::{Mutex, OnceLock};

use graft_score::{sample_keys, GraphNode, Keyframe, NODE_OPS};
use serde_json::Value;

use crate::encode::EncodeError;
use crate::ffmpeg::ffmpeg_bin;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilterClass {
    Video { source: bool },
    Audio { source: bool },
    Missing,
}

#[derive(Clone, Debug)]
struct Opt {
    name: String,
    kind: String,
    runtime: bool,
}

impl Opt {
    fn expression(&self) -> bool {
        self.kind == "string" && self.runtime
    }

    fn integer(&self) -> bool {
        matches!(self.kind.as_str(), "int" | "int64" | "uint64" | "boolean")
    }
}

fn catalog() -> Result<&'static HashMap<String, FilterClass>, EncodeError> {
    static CACHE: OnceLock<Result<HashMap<String, FilterClass>, String>> = OnceLock::new();
    match CACHE.get_or_init(load_catalog) {
        Ok(rows) => Ok(rows),
        Err(err) => Err(EncodeError::Invalid(err.clone())),
    }
}

fn load_catalog() -> Result<HashMap<String, FilterClass>, String> {
    let bin = ffmpeg_bin();
    let out = Command::new(&bin)
        .args(["-hide_banner", "-filters"])
        .output()
        .map_err(|e| format!("{}: {e}", bin.display()))?;
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    if text.trim().is_empty() {
        return Err(format!("{} produced no filter list", bin.display()));
    }
    Ok(parse_catalog(&text))
}

fn parse_catalog(text: &str) -> HashMap<String, FilterClass> {
    let mut rows = HashMap::new();
    for line in text.lines() {
        let mut parts = line.split_whitespace();
        let Some(flags) = parts.next() else { continue };
        let Some(name) = parts.next() else { continue };
        let Some(io) = parts.next() else { continue };
        if !flags.chars().all(|c| matches!(c, 'T' | 'S' | 'C' | '.')) {
            continue;
        }
        if !name.chars().next().is_some_and(|c| c.is_ascii_lowercase()) {
            continue;
        }
        if let Some(class) = class_from_io(io) {
            rows.insert(name.to_string(), class);
        }
    }
    rows
}

fn class_from_io(io: &str) -> Option<FilterClass> {
    let (left, right) = io.split_once("->")?;
    let source = left.starts_with('|');
    if right.contains('A') && !right.contains('V') {
        Some(FilterClass::Audio { source })
    } else if right.contains('V') || left.contains('V') {
        Some(FilterClass::Video { source })
    } else if right.contains('A') || left.contains('A') {
        Some(FilterClass::Audio { source })
    } else {
        None
    }
}

pub fn classify(op: &str) -> Result<FilterClass, EncodeError> {
    if graft_score::op_is_audio(op) {
        return Ok(FilterClass::Audio {
            source: op == "audio_source",
        });
    }
    if matches!(op, "source" | "gap") {
        return Ok(FilterClass::Video { source: true });
    }
    if NODE_OPS.contains(&op) {
        return Ok(FilterClass::Video { source: false });
    }
    Ok(*catalog()?.get(op).unwrap_or(&FilterClass::Missing))
}

pub fn routes_audio(op: &str) -> Result<bool, EncodeError> {
    Ok(matches!(classify(op)?, FilterClass::Audio { .. }))
}

pub fn is_lavfi_source(op: &str) -> Result<bool, EncodeError> {
    if matches!(op, "source" | "audio_source" | "gap" | "nested" | "group") {
        return Ok(false);
    }
    Ok(matches!(
        classify(op)?,
        FilterClass::Video { source: true } | FilterClass::Audio { source: true }
    ))
}

pub fn is_structural(op: &str) -> bool {
    matches!(
        op,
        "source"
            | "audio_source"
            | "gap"
            | "nested"
            | "group"
            | "captions"
            | "sidechain"
            | "concat"
            | "xfade"
            | "acrossfade"
            | "overlay"
            | "blend"
            | "amix"
            | "lut3d"
            | "lut1d"
            | "haldclut"
            | "flip"
            | "freeze"
            | "zoompan"
            | "switch"
            | "geq"
            | "cdl"
            | "lgg"
            | "alphamerge"
            | "maskedmerge"
            | "hstack"
            | "vstack"
            | "xstack"
            | "mix"
            | "colorspace"
    )
}

pub fn should_generic(op: &str, node: &GraphNode) -> bool {
    if is_structural(op) {
        return false;
    }
    if !NODE_OPS.contains(&op) {
        return true;
    }
    node.keys
        .as_ref()
        .is_some_and(|keys| keys.values().any(|row| !row.is_empty()))
}

pub fn ensure_op(op: &str) -> Result<(), EncodeError> {
    if is_structural(op) || NODE_OPS.contains(&op) {
        return Ok(());
    }
    if !graft_score::is_filter_name(op) {
        return Err(EncodeError::Invalid(format!(
            r#"{{"error":"unknown_op","op":"{op}"}}"#
        )));
    }
    match classify(op)? {
        FilterClass::Missing => Err(EncodeError::Invalid(format!(
            r#"{{"error":"toolchain","op":"{op}"}}"#
        ))),
        _ => Ok(()),
    }
}

fn options(name: &str) -> Result<Vec<Opt>, EncodeError> {
    type OptionCache = HashMap<String, Result<Vec<Opt>, String>>;
    static CACHE: OnceLock<Mutex<OptionCache>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut guard = cache.lock().unwrap_or_else(|err| err.into_inner());
    if let Some(hit) = guard.get(name) {
        return match hit {
            Ok(opts) => Ok(opts.clone()),
            Err(err) => Err(EncodeError::Invalid(err.clone())),
        };
    }
    let probed = probe_options(name);
    let out = match &probed {
        Ok(opts) => Ok(opts.clone()),
        Err(err) => Err(EncodeError::Invalid(err.clone())),
    };
    guard.insert(name.to_string(), probed);
    out
}

fn probe_options(name: &str) -> Result<Vec<Opt>, String> {
    let bin = ffmpeg_bin();
    let out = Command::new(&bin)
        .args(["-hide_banner", "-h", &format!("filter={name}")])
        .output()
        .map_err(|e| format!("{}: {e}", bin.display()))?;
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    if text.contains("Unknown filter") || text.contains("No such filter") {
        return Err(format!(r#"{{"error":"toolchain","op":"{name}"}}"#));
    }
    Ok(parse_options(&text))
}

fn parse_options(text: &str) -> Vec<Opt> {
    let mut out = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        let Some(start) = trimmed.find('<') else {
            continue;
        };
        let Some(end) = trimmed.find('>') else {
            continue;
        };
        if end <= start {
            continue;
        }
        let kind = &trimmed[start + 1..end];
        if kind.is_empty()
            || !kind
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        {
            continue;
        }
        let name = trimmed[..start].trim();
        if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            continue;
        }
        let flags = trimmed[end + 1..].split_whitespace().next().unwrap_or("");
        out.push(Opt {
            name: name.to_string(),
            kind: kind.to_string(),
            runtime: flags.contains('T'),
        });
    }
    out
}

fn lookup<'a>(opts: &'a [Opt], op: &str, key: &str) -> Option<&'a Opt> {
    if let Some(opt) = opts.iter().find(|opt| opt.name == key) {
        return Some(opt);
    }
    let alias = match (op, key) {
        ("unsharp", "lx") => "luma_msize_x",
        ("unsharp", "la") => "luma_amount",
        ("fade" | "afade", "type") => "t",
        ("fade" | "afade", "start") => "st",
        ("fade" | "afade", "duration") => "d",
        ("rotate", "angle") => "a",
        _ => return None,
    };
    opts.iter().find(|opt| opt.name == alias)
}

pub(crate) fn quote_filter_arg(expr: &str) -> String {
    if expr
        .chars()
        .any(|c| matches!(c, ',' | '(' | ' ' | ':' | '\''))
    {
        format!("'{}'", expr.replace('\'', "'\\''"))
    } else {
        expr.to_string()
    }
}

fn json_param(value: &Value) -> Result<String, EncodeError> {
    match value {
        Value::String(text) => Ok(text.clone()),
        Value::Number(number) => Ok(number.to_string()),
        Value::Bool(flag) => Ok(flag.to_string()),
        _ => Err(EncodeError::Invalid(
            "filter params are strings, numbers, or bools".into(),
        )),
    }
}

fn builtin_defaults(op: &str) -> Vec<(&'static str, String)> {
    match op {
        "eq" => vec![
            ("brightness", "0".into()),
            ("contrast", "1".into()),
            ("saturation", "1".into()),
            ("gamma", "1".into()),
        ],
        "hue" => vec![("h", "0".into()), ("s", "1".into())],
        "loudnorm" => vec![
            ("I", "-14".into()),
            ("LRA", "7".into()),
            ("TP", "-1.5".into()),
        ],
        "volume" => vec![("volume", "1".into())],
        "deband" => vec![
            ("1thr", "0.02".into()),
            ("2thr", "0.02".into()),
            ("3thr", "0.02".into()),
            ("range", "16".into()),
            ("blur", "1".into()),
        ],
        "exposure" => vec![("exposure", "0".into()), ("black", "0".into())],
        "gblur" => vec![("sigma", "0.6".into())],
        "yadif" => vec![("mode", "0".into()), ("parity", "-1".into())],
        "bwdif" => vec![
            ("mode", "1".into()),
            ("parity", "-1".into()),
            ("deint", "all".into()),
        ],
        _ => Vec::new(),
    }
}

enum AssignmentValue {
    Const(String),
    Expr(String),
    Curve(Vec<Keyframe>),
}

pub fn render(fps: f64, width: u32, height: u32, node: &GraphNode) -> Result<String, EncodeError> {
    if classify(&node.op)? == FilterClass::Missing {
        return Err(EncodeError::Invalid(format!(
            r#"{{"error":"toolchain","op":"{}"}}"#,
            node.op
        )));
    }
    let opts = options(&node.op)?;
    let mut assigned: BTreeMap<String, AssignmentValue> = BTreeMap::new();
    if NODE_OPS.contains(&node.op.as_str()) {
        for (key, value) in builtin_defaults(&node.op) {
            if let Some(opt) = lookup(&opts, &node.op, key) {
                assigned.insert(opt.name.clone(), AssignmentValue::Const(value));
            }
        }
    }
    inject_geometry(&node.op, &opts, width, height, &mut assigned);
    if let Some(Value::Object(map)) = &node.params {
        for (key, value) in map {
            let Some(opt) = lookup(&opts, &node.op, key) else {
                return Err(EncodeError::Invalid(format!(
                    r#"{{"error":"unknown_param","op":"{}","param":"{key}"}}"#,
                    node.op
                )));
            };
            assigned.insert(opt.name.clone(), AssignmentValue::Const(json_param(value)?));
        }
    }
    if let Some(keys) = &node.keys {
        for (key, row) in keys {
            if row.is_empty() {
                continue;
            }
            let Some(opt) = lookup(&opts, &node.op, key) else {
                return Err(EncodeError::Invalid(format!(
                    r#"{{"error":"unknown_param","op":"{}","param":"{key}"}}"#,
                    node.op
                )));
            };
            if opt.expression() {
                assigned.insert(
                    opt.name.clone(),
                    AssignmentValue::Expr(graft_score::piecewise_eased(row, "t")),
                );
            } else {
                assigned.insert(opt.name.clone(), AssignmentValue::Curve(row.clone()));
            }
        }
    }
    if assigned
        .values()
        .any(|value| matches!(value, AssignmentValue::Expr(_)))
        && opts.iter().any(|opt| opt.name == "eval")
    {
        assigned
            .entry("eval".into())
            .or_insert(AssignmentValue::Const("frame".into()));
    }
    if matches!(
        classify(&node.op)?,
        FilterClass::Video { source: true } | FilterClass::Audio { source: true }
    ) && !matches!(node.op.as_str(), "source" | "audio_source" | "gap")
    {
        inject_source_defaults(&opts, fps, width, height, &mut assigned);
    }
    let curves: Vec<(String, Vec<Keyframe>)> = assigned
        .iter()
        .filter_map(|(name, value)| match value {
            AssignmentValue::Curve(keys) => Some((name.clone(), keys.clone())),
            _ => None,
        })
        .collect();
    if curves.is_empty() {
        return Ok(format_instance(
            &node.op,
            &flatten_const(&assigned),
            node.enable.as_deref(),
        ));
    }
    let marks = curve_marks(&curves, fps);
    let mut chain = Vec::new();
    for window in marks.windows(2) {
        let mut flat = flatten_const(&assigned);
        for (name, keys) in &curves {
            let opt = opts.iter().find(|opt| opt.name == *name);
            flat.insert(
                name.clone(),
                format_sample(opt, sample_keys(keys, window[0])),
            );
        }
        let segment = format!("gte(t,{:.6})*lt(t,{:.6})", window[0], window[1]);
        chain.push(format_instance(
            &node.op,
            &flat,
            and_enable(node.enable.as_deref(), Some(segment)).as_deref(),
        ));
    }
    let last = *marks.last().unwrap_or(&0.0);
    let mut flat = flatten_const(&assigned);
    for (name, keys) in &curves {
        let opt = opts.iter().find(|opt| opt.name == *name);
        flat.insert(name.clone(), format_sample(opt, sample_keys(keys, last)));
    }
    chain.push(format_instance(
        &node.op,
        &flat,
        and_enable(node.enable.as_deref(), Some(format!("gte(t,{last:.6})"))).as_deref(),
    ));
    Ok(chain.join(","))
}

fn flatten_const(assigned: &BTreeMap<String, AssignmentValue>) -> BTreeMap<String, String> {
    assigned
        .iter()
        .filter_map(|(name, value)| match value {
            AssignmentValue::Const(text) | AssignmentValue::Expr(text) => {
                Some((name.clone(), text.clone()))
            }
            AssignmentValue::Curve(_) => None,
        })
        .collect()
}

fn format_sample(opt: Option<&Opt>, value: f64) -> String {
    if opt.is_some_and(|opt| opt.integer()) {
        format!("{}", value.round() as i64)
    } else {
        format!("{value:.6}")
    }
}

fn curve_marks(curves: &[(String, Vec<Keyframe>)], fps: f64) -> Vec<f64> {
    let mut marks = Vec::new();
    let mut curved = false;
    for (_, keys) in curves {
        let mut ordered: Vec<&Keyframe> = keys
            .iter()
            .filter(|key| key.t.is_finite() && key.v.is_finite())
            .collect();
        ordered.sort_by(|a, b| a.t.partial_cmp(&b.t).unwrap_or(std::cmp::Ordering::Equal));
        if ordered.is_empty() {
            continue;
        }
        marks.push(ordered[0].t);
        for pair in ordered.windows(2) {
            if pair[1].ease_name() != "hold" {
                curved = true;
            }
            marks.push(pair[1].t);
        }
    }
    marks.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    marks.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
    if marks.len() < 2 || !curved {
        if marks.is_empty() {
            marks.push(0.0);
        }
        return marks;
    }
    let start = marks[0];
    let end = *marks.last().unwrap();
    let step = (1.0 / fps.max(1.0)).max(1e-4);
    let mut sampled = vec![start];
    let mut t = start + step;
    while t < end - 1e-6 {
        sampled.push(t);
        t += step;
    }
    sampled.push(end);
    sampled
}

fn and_enable(outer: Option<&str>, inner: Option<String>) -> Option<String> {
    match (outer, inner) {
        (None, None) => None,
        (Some(outer), None) => Some(outer.to_string()),
        (None, Some(inner)) => Some(inner),
        (Some(outer), Some(inner)) => Some(format!("({outer})*({inner})")),
    }
}

fn format_instance(name: &str, params: &BTreeMap<String, String>, enable: Option<&str>) -> String {
    let mut parts: Vec<String> = params
        .iter()
        .map(|(key, value)| format!("{key}={}", quote_filter_arg(value)))
        .collect();
    if let Some(enable) = enable {
        parts.push(format!("enable='{}'", enable.replace('\'', "'\\''")));
    }
    if parts.is_empty() {
        name.to_string()
    } else {
        format!("{name}={}", parts.join(":"))
    }
}

fn inject_geometry(
    op: &str,
    opts: &[Opt],
    width: u32,
    height: u32,
    assigned: &mut BTreeMap<String, AssignmentValue>,
) {
    let set = |assigned: &mut BTreeMap<String, AssignmentValue>, key: &str, value: String| {
        if opts.iter().any(|opt| opt.name == key) {
            assigned
                .entry(key.to_string())
                .or_insert(AssignmentValue::Const(value));
        }
    };
    match op {
        "scale" => {
            set(assigned, "w", width.to_string());
            set(assigned, "h", height.to_string());
            set(assigned, "flags", "bicubic".into());
        }
        "crop" | "pad" => {
            set(assigned, "w", width.to_string());
            set(assigned, "h", height.to_string());
            set(assigned, "x", "0".into());
            set(assigned, "y", "0".into());
        }
        _ => {}
    }
}

fn inject_source_defaults(
    opts: &[Opt],
    fps: f64,
    width: u32,
    height: u32,
    assigned: &mut BTreeMap<String, AssignmentValue>,
) {
    let mut set_alias = |names: &[&str], value: String| {
        if names.iter().any(|name| assigned.contains_key(*name)) {
            return;
        }
        if let Some(name) = names
            .iter()
            .find(|name| opts.iter().any(|opt| opt.name == **name))
        {
            assigned.insert((*name).to_string(), AssignmentValue::Const(value));
        }
    };
    set_alias(&["size", "s"], format!("{width}x{height}"));
    set_alias(&["rate", "r"], format!("{fps}"));
    set_alias(&["duration", "d"], "1".into());
}

pub fn append_declared(
    node: &GraphNode,
    filter: &str,
    consumed: &BTreeSet<String>,
) -> Result<String, EncodeError> {
    if is_structural(&node.op)
        || filter.contains('[')
        || filter.contains(';')
        || filter.contains(',')
    {
        return Ok(filter.to_string());
    }
    let Some(Value::Object(map)) = &node.params else {
        return Ok(filter.to_string());
    };
    let pending: Vec<(&String, &Value)> = map
        .iter()
        .filter(|(key, _)| !consumed.contains(*key))
        .collect();
    if pending.is_empty() {
        return Ok(filter.to_string());
    }
    let opts = options(&node.op)?;
    let mut extra = Vec::new();
    for (key, value) in pending {
        let Some(opt) = lookup(&opts, &node.op, key) else {
            return Err(EncodeError::Invalid(format!(
                r#"{{"error":"unknown_param","op":"{}","param":"{key}"}}"#,
                node.op
            )));
        };
        if filter.contains(&format!("{}=", opt.name)) {
            continue;
        }
        extra.push(format!(
            "{}={}",
            opt.name,
            quote_filter_arg(&json_param(value)?)
        ));
    }
    if extra.is_empty() {
        return Ok(filter.to_string());
    }
    if let Some((head, tail)) = filter.split_once('=') {
        Ok(format!("{head}={tail}:{}", extra.join(":")))
    } else {
        Ok(format!("{filter}={}", extra.join(":")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use graft_score::GraphNode;
    use std::process::Command;

    fn node(op: &str, params: Value, keys: Option<BTreeMap<String, Vec<Keyframe>>>) -> GraphNode {
        GraphNode {
            id: op.into(),
            op: op.into(),
            inputs: vec!["#picture".into()],
            material: None,
            source: None,
            place: None,
            enable: None,
            role: None,
            params: Some(params),
            keys,
            nodes: Vec::new(),
        }
    }

    fn ffmpeg_ok() -> bool {
        Command::new(ffmpeg_bin())
            .arg("-version")
            .output()
            .map(|out| out.status.success())
            .unwrap_or(false)
    }

    #[test]
    fn hqdn3d_passes_declared_strength() {
        if !ffmpeg_ok() {
            return;
        }
        let rendered = render(
            30.0,
            96,
            96,
            &node("hqdn3d", serde_json::json!({"luma_spatial": 4}), None),
        )
        .unwrap();
        assert!(rendered.contains("luma_spatial=4"), "{rendered}");
    }

    #[test]
    fn unknown_param_fails_closed() {
        if !ffmpeg_ok() {
            return;
        }
        let err = render(
            30.0,
            96,
            96,
            &node("hqdn3d", serde_json::json!({"nope": 1}), None),
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("unknown_param"), "{err}");
    }

    #[test]
    fn drawtext_is_toolchain() {
        if !ffmpeg_ok() {
            return;
        }
        let err = render(30.0, 96, 96, &node("drawtext", serde_json::json!({}), None))
            .unwrap_err()
            .to_string();
        assert!(err.contains("toolchain"), "{err}");
        assert!(err.contains("drawtext"), "{err}");
    }

    #[test]
    fn hold_numeric_keys_segment_enable() {
        if !ffmpeg_ok() {
            return;
        }
        let mut keys = BTreeMap::new();
        keys.insert(
            "luma_spatial".into(),
            vec![
                Keyframe {
                    t: 0.0,
                    v: 1.0,
                    ease: Some("hold".into()),
                },
                Keyframe {
                    t: 1.0,
                    v: 4.0,
                    ease: Some("hold".into()),
                },
            ],
        );
        let rendered = render(
            30.0,
            96,
            96,
            &node("hqdn3d", serde_json::json!({}), Some(keys)),
        )
        .unwrap();
        assert!(rendered.contains("luma_spatial=1.000000"), "{rendered}");
        assert!(rendered.contains("luma_spatial=4.000000"), "{rendered}");
        assert!(rendered.contains("lt(t,1.000000)"), "{rendered}");
    }

    #[test]
    fn eq_ease_is_an_expression() {
        if !ffmpeg_ok() {
            return;
        }
        let mut keys = BTreeMap::new();
        keys.insert(
            "gamma".into(),
            vec![
                Keyframe {
                    t: 0.0,
                    v: 1.0,
                    ease: None,
                },
                Keyframe {
                    t: 1.0,
                    v: 1.4,
                    ease: Some("ease_in_out".into()),
                },
            ],
        );
        let rendered =
            render(30.0, 96, 96, &node("eq", serde_json::json!({}), Some(keys))).unwrap();
        assert!(rendered.contains("eval=frame"), "{rendered}");
        assert!(rendered.contains("3-2*"), "{rendered}");
    }
}
