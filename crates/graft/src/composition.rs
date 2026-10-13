// SPDX-License-Identifier: Apache-2.0

//! Porcelain authorship for a composition.
//!
//! The IR is still `scion.graph` + `score.joins` + `dest.working`.
//! This module reads the take directory and writes those documents.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use graft_cas::{Fs, Kind, Store};
use std::collections::BTreeMap;

use graft_score::{
    canonical_transition, find_node_mut, load_scion, load_score, op_allowed, parse_wh,
    resolve_scion, save_json, CompositionGraph, FrameRate, GraphMode, GraphNode, Join, Keyframe,
    Marker, Motion, Role, Scion, TimedRange, WorkingSpace,
};

use crate::paths;

const LOOK_FILES: &[&str] = &["look.cube", "grade.cube", "look.3dl", "grade.3dl"];
const LEGAL_FILES: &[&str] = &["legal.cube", "legal.3dl"];

pub fn author(
    dir: &Path,
    from: &Path,
    scion_id: &str,
    scions: &BTreeMap<String, Scion>,
) -> Result<()> {
    let spec = load_take_composition(from)?;
    apply_joins(dir, from, spec.joins.as_ref())?;
    apply_markers(dir, spec.markers.as_ref())?;
    let id = scion_id;
    let resolved = resolve_scion(scions, id)?;
    let path = paths::scion(dir, id);
    let mut scion = load_scion(&path)?;
    if let Some((w, h)) = spec.dest {
        scion.dest.width = w;
        scion.dest.height = h;
    }
    if let Some(ref working) = spec.working {
        scion.dest.working = Some(working.clone());
    }
    let mut graph = scion
        .graph
        .clone()
        .or(resolved.graph)
        .unwrap_or(CompositionGraph {
            mode: GraphMode::Append,
            nodes: Vec::new(),
        });
    let rate = scion.dest.rate;
    let score = load_score(&paths::score(dir))?;
    let default_out = score
        .clock
        .rate
        .seconds_from_frames(score.clock.duration_frames as i64)
        .max(1.0 / 30.0);
    for draft in spec.nodes {
        upsert_draft(dir, from, &mut graph, draft, rate, default_out)?;
    }
    for (id, enable) in spec.spans {
        let Some(node) = graph.nodes.iter_mut().find(|node| node.id == id) else {
            bail!("span {id} has no graph node");
        };
        node.enable = Some(enable);
    }
    if let Some(file) = first_existing(from, LOOK_FILES) {
        if !graph.nodes.iter().any(|node| node.id == "look") {
            upsert_lut(&mut graph, "look", put_lut_material(dir, &file)?);
        }
    }
    if let Some(file) = first_existing(from, LEGAL_FILES) {
        if !graph.nodes.iter().any(|node| node.id == "legal") {
            upsert_lut(&mut graph, "legal", put_lut_material(dir, &file)?);
        }
    }
    if graph.nodes.is_empty() && spec.dest.is_none() && spec.working.is_none() {
        return Ok(());
    }
    if !graph.nodes.is_empty() {
        graph.validate()?;
        scion.graph = Some(graph);
    }
    scion.validate()?;
    save_json(&path, &scion)?;
    eprintln!("wrote {}", path.display());
    Ok(())
}

pub fn author_nest(
    dir: &Path,
    from: &Path,
    nest_id: &str,
    scions: &BTreeMap<String, Scion>,
) -> Result<()> {
    let spec = load_take_composition(from)?;
    let nest = spec
        .nests
        .get(nest_id)
        .ok_or_else(|| anyhow::anyhow!("composition.json has no nest {nest_id}"))?;
    let path = paths::scion(dir, nest_id);
    let mut scion = load_scion(&path)?;
    let resolved = resolve_scion(scions, nest_id)?;
    let mut graph = scion
        .graph
        .clone()
        .or(resolved.graph)
        .unwrap_or(CompositionGraph {
            mode: GraphMode::Append,
            nodes: Vec::new(),
        });
    let rate = scion.dest.rate;
    let score = load_score(&paths::nest_score(dir, nest_id))?;
    let default_out = score
        .clock
        .rate
        .seconds_from_frames(score.clock.duration_frames as i64)
        .max(1.0 / 30.0);
    for draft in nest.nodes.clone() {
        upsert_draft(dir, from, &mut graph, draft, rate, default_out)?;
    }
    for (id, enable) in &nest.spans {
        let Some(node) = graph.nodes.iter_mut().find(|node| node.id == *id) else {
            bail!("span {id} has no graph node");
        };
        node.enable = Some(enable.clone());
    }
    if graph.nodes.is_empty() {
        return Ok(());
    }
    graph.validate()?;
    scion.graph = Some(graph);
    scion.validate()?;
    save_json(&path, &scion)?;
    eprintln!("wrote {}", path.display());
    Ok(())
}

#[derive(Clone, Debug)]
pub struct SlotDraft {
    pub id: String,
    pub role: Role,
    pub file: PathBuf,
    pub in_s: f64,
    pub out_s: Option<f64>,
    pub optional: bool,
    pub dest_start_s: Option<f64>,
    pub dest_duration_s: Option<f64>,
    pub speed: Option<f64>,
    pub audio_in_s: Option<f64>,
    pub audio_out_s: Option<f64>,
    pub audio_place_s: Option<f64>,
    pub audio_place_duration_s: Option<f64>,
    pub nested: Option<String>,
    pub x: Option<String>,
    pub y: Option<String>,
    pub opacity: Option<f64>,
    pub motion: Option<Motion>,
    pub remap: Vec<Keyframe>,
}

#[derive(Clone, Debug)]
pub struct NestDraft {
    pub slots: Vec<SlotDraft>,
    nodes: Vec<NodeDraft>,
    spans: Vec<(String, String)>,
}

struct TakeComposition {
    dest: Option<(u32, u32)>,
    working: Option<WorkingSpace>,
    joins: Option<Vec<serde_json::Value>>,
    markers: Option<Vec<serde_json::Value>>,
    spans: Vec<(String, String)>,
    nodes: Vec<NodeDraft>,
    slots: Vec<SlotDraft>,
    nests: BTreeMap<String, NestDraft>,
}

/// Authored dest pixels from `composition.json`. `None` means probe the first clip.
pub fn dest_wh(from: &Path) -> Result<Option<(u32, u32)>> {
    Ok(load_take_composition(from)?.dest)
}

/// Authored slot bindings from `composition.json`. `None` means the
/// porcelain should fall back to named `hook.mov` files (the crutch).
pub fn slot_drafts(from: &Path) -> Result<Option<Vec<SlotDraft>>> {
    let spec = load_take_composition(from)?;
    if spec.slots.is_empty() {
        Ok(None)
    } else {
        Ok(Some(spec.slots))
    }
}

pub fn nest_drafts(from: &Path) -> Result<BTreeMap<String, NestDraft>> {
    Ok(load_take_composition(from)?.nests)
}

pub fn nested_placeholder_material() -> String {
    graft_score::material_id(b"graft:nested-placeholder")
}

pub fn nest_spine_duration_s(slots: &[SlotDraft]) -> f64 {
    let sum: f64 = slots
        .iter()
        .filter(|slot| slot.role.is_spine())
        .map(|slot| {
            slot.out_s
                .unwrap_or(slot.in_s + slot.dest_duration_s.unwrap_or(1.0))
                - slot.in_s
        })
        .sum();
    sum.max(1.0 / 30.0)
}

#[derive(Clone, Debug)]
struct NodeDraft {
    id: String,
    op: String,
    inputs: Vec<String>,
    file: Option<PathBuf>,
    enable: Option<String>,
    role: Option<String>,
    params: Option<serde_json::Value>,
    keys: Option<BTreeMap<String, Vec<Keyframe>>>,
    nodes: Vec<NodeDraft>,
    in_s: Option<f64>,
    out_s: Option<f64>,
}

fn load_take_composition(from: &Path) -> Result<TakeComposition> {
    let path = ["composition.json", "graph.json"]
        .into_iter()
        .map(|name| from.join(name))
        .find(|path| path.is_file());
    let Some(path) = path else {
        return Ok(TakeComposition {
            dest: None,
            working: None,
            joins: None,
            markers: None,
            spans: Vec::new(),
            nodes: Vec::new(),
            slots: Vec::new(),
            nests: BTreeMap::new(),
        });
    };
    let value: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?,
    )
    .with_context(|| format!("parse {}", path.display()))?;
    let dest =
        parse_dest_spec(value.get("dest")).with_context(|| format!("{} dest", path.display()))?;
    let working = value
        .get("working")
        .map(|v| serde_json::from_value::<WorkingSpace>(v.clone()))
        .transpose()
        .with_context(|| format!("{} working", path.display()))?;
    let joins = value.get("joins").and_then(|v| v.as_array()).cloned();
    let markers = value.get("markers").and_then(|v| v.as_array()).cloned();
    let mut spans = Vec::new();
    if let Some(arr) = value.get("spans").and_then(|v| v.as_array()) {
        for item in arr {
            let id = item
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("{} span needs id", path.display()))?;
            let enable = item
                .get("enable")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("span {id} needs enable"))?;
            spans.push((id.to_string(), enable.to_string()));
        }
    }
    let mut nodes = parse_nodes(from, &path, value.get("nodes"))?;
    parse_adjustments(
        from,
        &path,
        value.get("adjustments"),
        &mut nodes,
        &mut spans,
    )?;
    let mut nests = BTreeMap::new();
    if let Some(obj) = value.get("nests").and_then(|v| v.as_object()) {
        for (id, item) in obj {
            if id.is_empty() {
                bail!("{} nest id is empty", path.display());
            }
            let mut nest_nodes = parse_nodes(from, &path, item.get("nodes"))?;
            let mut nest_spans = Vec::new();
            if let Some(arr) = item.get("spans").and_then(|v| v.as_array()) {
                for span in arr {
                    let span_id = span.get("id").and_then(|v| v.as_str()).ok_or_else(|| {
                        anyhow::anyhow!("{} nest {id} span needs id", path.display())
                    })?;
                    let enable = span
                        .get("enable")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| anyhow::anyhow!("span {span_id} needs enable"))?;
                    nest_spans.push((span_id.to_string(), enable.to_string()));
                }
            }
            let mut nest_slots = Vec::new();
            if let Some(arr) = item.get("slots").and_then(|v| v.as_array()) {
                for slot in arr {
                    nest_slots.push(parse_slot_draft(from, &path, slot, &BTreeMap::new())?);
                }
            }
            parse_adjustments(
                from,
                &path,
                item.get("adjustments"),
                &mut nest_nodes,
                &mut nest_spans,
            )?;
            if let Some(switch) = item.get("switch") {
                author_switch(from, &path, switch, &mut nest_slots, &mut nest_nodes)?;
            }
            if nest_slots.is_empty() {
                bail!("{} nest {id} needs slots", path.display());
            }
            nests.insert(
                id.clone(),
                NestDraft {
                    slots: nest_slots,
                    nodes: nest_nodes,
                    spans: nest_spans,
                },
            );
        }
    }
    let mut slots = Vec::new();
    if let Some(arr) = value.get("slots").and_then(|v| v.as_array()) {
        for item in arr {
            slots.push(parse_slot_draft(from, &path, item, &nests)?);
        }
    }
    Ok(TakeComposition {
        dest,
        working,
        joins,
        markers,
        spans,
        nodes,
        slots,
        nests,
    })
}

fn parse_nodes(
    from: &Path,
    doc: &Path,
    value: Option<&serde_json::Value>,
) -> Result<Vec<NodeDraft>> {
    let mut nodes = Vec::new();
    let Some(arr) = value.and_then(|v| v.as_array()) else {
        return Ok(nodes);
    };
    for item in arr {
        let id = item
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("{} node needs id", doc.display()))?;
        let op = item
            .get("op")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("graph {id} needs op"))?;
        if !op_allowed(op) {
            bail!("unknown_op {op} in {}", doc.display());
        }
        let file = item
            .get("file")
            .and_then(|v| v.as_str())
            .map(|rel| resolve_take_file(from, rel));
        let inputs = item
            .get("inputs")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        let children = if op == "group" {
            parse_nodes(from, doc, item.get("nodes"))?
        } else if item.get("nodes").is_some() {
            bail!("graph {id} op {op} cannot nest nodes");
        } else {
            Vec::new()
        };
        if op == "group" && children.is_empty() {
            bail!("graph {id} op group needs nodes");
        }
        nodes.push(NodeDraft {
            id: id.into(),
            op: op.into(),
            inputs,
            file,
            enable: item
                .get("enable")
                .and_then(|v| v.as_str())
                .map(str::to_string),
            role: item
                .get("role")
                .and_then(|v| v.as_str())
                .map(str::to_string),
            params: item.get("params").cloned(),
            keys: parse_key_map(item.get("keys"))?,
            nodes: children,
            in_s: item
                .get("in")
                .or_else(|| item.get("in_s"))
                .and_then(|v| v.as_f64()),
            out_s: item
                .get("out")
                .or_else(|| item.get("out_s"))
                .and_then(|v| v.as_f64()),
        });
    }
    Ok(nodes)
}

fn parse_adjustments(
    from: &Path,
    doc: &Path,
    value: Option<&serde_json::Value>,
    nodes: &mut Vec<NodeDraft>,
    spans: &mut Vec<(String, String)>,
) -> Result<()> {
    let Some(arr) = value.and_then(|v| v.as_array()) else {
        return Ok(());
    };
    for (index, item) in arr.iter().enumerate() {
        let enable = item
            .get("enable")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("{} adjustment needs enable", doc.display()))?;
        let group = item
            .get("nodes")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow::anyhow!("{} adjustment needs nodes", doc.display()))?;
        let mut children = Vec::new();
        for node in group {
            if let Some(id) = node.as_str() {
                if id.is_empty() {
                    bail!("{} adjustment node id is empty", doc.display());
                }
                spans.push((id.to_string(), enable.to_string()));
                continue;
            }
            let parsed = parse_nodes(from, doc, Some(&serde_json::json!([node])))?;
            if parsed.len() != 1 {
                bail!("{} adjustment node is invalid", doc.display());
            }
            children.extend(parsed);
        }
        if children.is_empty() {
            continue;
        }
        let id = item
            .get("id")
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .unwrap_or_else(|| format!("adj{index}"));
        nodes.push(NodeDraft {
            id,
            op: "group".into(),
            inputs: Vec::new(),
            file: None,
            enable: Some(enable.to_string()),
            role: item
                .get("role")
                .and_then(|v| v.as_str())
                .map(str::to_string),
            params: None,
            keys: None,
            nodes: children,
            in_s: None,
            out_s: None,
        });
    }
    Ok(())
}

fn json_expr(item: &serde_json::Value, key: &str) -> Option<String> {
    item.get(key).and_then(|v| match v {
        serde_json::Value::String(s) if !s.is_empty() => Some(s.clone()),
        serde_json::Value::Number(n) => Some(n.to_string()),
        _ => None,
    })
}

fn parse_dest_spec(value: Option<&serde_json::Value>) -> Result<Option<(u32, u32)>> {
    match value {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::String(spec)) => {
            parse_wh(spec).map(Some).map_err(|e| anyhow::anyhow!("{e}"))
        }
        Some(obj) => {
            let w = obj
                .get("w")
                .or_else(|| obj.get("width"))
                .and_then(|v| v.as_u64())
                .ok_or_else(|| anyhow::anyhow!("dest needs w/width"))?;
            let h = obj
                .get("h")
                .or_else(|| obj.get("height"))
                .and_then(|v| v.as_u64())
                .ok_or_else(|| anyhow::anyhow!("dest needs h/height"))?;
            Ok(Some((
                u32::try_from(w).context("dest width")?,
                u32::try_from(h).context("dest height")?,
            )))
        }
    }
}

fn parse_slot_draft(
    from: &Path,
    doc: &Path,
    item: &serde_json::Value,
    nests: &BTreeMap<String, NestDraft>,
) -> Result<SlotDraft> {
    let id = item
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("{} slot needs id", doc.display()))?;
    let role_name = item.get("role").and_then(|v| v.as_str()).unwrap_or(id);
    let role = Role::parse(role_name)?;
    let nested = item
        .get("scion")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    if let Some(ref nest_id) = nested {
        let nest = nests.get(nest_id).ok_or_else(|| {
            anyhow::anyhow!("slot {id} scion {nest_id} is not in composition.json nests")
        })?;
        let dur = nest_spine_duration_s(&nest.slots);
        return Ok(SlotDraft {
            id: id.into(),
            role,
            file: PathBuf::new(),
            in_s: 0.0,
            out_s: Some(dur),
            optional: item
                .get("optional")
                .and_then(|v| v.as_bool())
                .unwrap_or(!role.is_spine()),
            dest_start_s: None,
            dest_duration_s: Some(dur),
            speed: item.get("speed").and_then(|v| v.as_f64()),
            audio_in_s: None,
            audio_out_s: None,
            audio_place_s: None,
            audio_place_duration_s: None,
            nested,
            x: json_expr(item, "x"),
            y: json_expr(item, "y"),
            opacity: item.get("opacity").and_then(|v| v.as_f64()),
            motion: parse_motion(item)?,
            remap: parse_remap(item)?,
        });
    }
    let file = item
        .get("file")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("slot {id} needs file or scion"))?;
    let in_s = item
        .get("in")
        .or_else(|| item.get("in_s"))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let out_s = item
        .get("out")
        .or_else(|| item.get("out_s"))
        .and_then(|v| v.as_f64());
    let (dest_start_s, dest_duration_s) = match item.get("place") {
        Some(serde_json::Value::Number(n)) => (n.as_f64(), None),
        Some(obj) => (
            obj.get("start").and_then(|v| v.as_f64()),
            obj.get("duration").and_then(|v| v.as_f64()),
        ),
        None => (None, None),
    };
    let (audio_in_s, audio_out_s, audio_place_s, audio_place_duration_s) = match item.get("audio") {
        Some(audio) => {
            let (place_s, place_dur) = match audio.get("place") {
                Some(serde_json::Value::Number(n)) => (n.as_f64(), None),
                Some(obj) => (
                    obj.get("start").and_then(|v| v.as_f64()),
                    obj.get("duration").and_then(|v| v.as_f64()),
                ),
                None => (None, None),
            };
            (
                audio
                    .get("in")
                    .or_else(|| audio.get("in_s"))
                    .and_then(|v| v.as_f64()),
                audio
                    .get("out")
                    .or_else(|| audio.get("out_s"))
                    .and_then(|v| v.as_f64()),
                place_s,
                place_dur,
            )
        }
        None => (None, None, None, None),
    };
    Ok(SlotDraft {
        id: id.into(),
        role,
        file: resolve_take_file(from, file),
        in_s,
        out_s,
        optional: item
            .get("optional")
            .and_then(|v| v.as_bool())
            .unwrap_or(!role.is_spine()),
        dest_start_s,
        dest_duration_s,
        speed: item.get("speed").and_then(|v| v.as_f64()),
        audio_in_s,
        audio_out_s,
        audio_place_s,
        audio_place_duration_s,
        nested: None,
        x: json_expr(item, "x"),
        y: json_expr(item, "y"),
        opacity: item.get("opacity").and_then(|v| v.as_f64()),
        motion: parse_motion(item)?,
        remap: parse_remap(item)?,
    })
}

fn parse_keyframes(value: Option<&serde_json::Value>) -> Result<Vec<Keyframe>> {
    let Some(arr) = value.and_then(|v| v.as_array()) else {
        return Ok(Vec::new());
    };
    let mut keys = Vec::new();
    for item in arr {
        let t = item
            .get("t")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| anyhow::anyhow!("keyframe needs t"))?;
        let v = item
            .get("v")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| anyhow::anyhow!("keyframe needs v"))?;
        keys.push(Keyframe {
            t,
            v,
            ease: item
                .get("ease")
                .and_then(|v| v.as_str())
                .map(str::to_string),
        });
    }
    Ok(keys)
}

fn parse_key_map(
    value: Option<&serde_json::Value>,
) -> Result<Option<BTreeMap<String, Vec<Keyframe>>>> {
    let Some(obj) = value.and_then(|v| v.as_object()) else {
        return Ok(None);
    };
    let mut map = BTreeMap::new();
    for (name, keys) in obj {
        let parsed = parse_keyframes(Some(keys))?;
        if parsed.is_empty() {
            bail!("keys.{name} needs at least one keyframe");
        }
        map.insert(name.clone(), parsed);
    }
    if map.is_empty() {
        Ok(None)
    } else {
        Ok(Some(map))
    }
}

fn parse_motion(item: &serde_json::Value) -> Result<Option<Motion>> {
    let Some(motion) = item.get("motion") else {
        return Ok(None);
    };
    let parsed = Motion {
        x: parse_keyframes(motion.get("x"))?,
        y: parse_keyframes(motion.get("y"))?,
        opacity: parse_keyframes(motion.get("opacity"))?,
    };
    if parsed.x.is_empty() && parsed.y.is_empty() && parsed.opacity.is_empty() {
        Ok(None)
    } else {
        Ok(Some(parsed))
    }
}

fn parse_remap(item: &serde_json::Value) -> Result<Vec<Keyframe>> {
    parse_keyframes(item.get("remap"))
}

fn author_switch(
    from: &Path,
    doc: &Path,
    switch: &serde_json::Value,
    slots: &mut Vec<SlotDraft>,
    nodes: &mut Vec<NodeDraft>,
) -> Result<()> {
    let cuts = switch
        .get("cuts")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow::anyhow!("{} switch needs cuts", doc.display()))?;
    if cuts.len() < 2 {
        bail!("{} switch needs two cuts", doc.display());
    }
    let node_id = switch
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("switch")
        .to_string();
    let mut inputs = Vec::new();
    let mut params_cuts = Vec::new();
    for (index, cut) in cuts.iter().enumerate() {
        let t = cut.get("t").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let cut_id = cut
            .get("id")
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .unwrap_or_else(|| {
                if index == 0 {
                    "hook".into()
                } else {
                    format!("angle{index}")
                }
            });
        if index == 0 {
            if !slots.iter().any(|slot| slot.id == cut_id) {
                let mut authored = cut.clone();
                if let Some(obj) = authored.as_object_mut() {
                    obj.insert("id".into(), serde_json::json!(cut_id));
                    obj.entry("role").or_insert(serde_json::json!("hook"));
                }
                slots.push(parse_slot_draft(from, doc, &authored, &BTreeMap::new())?);
            }
        } else if !nodes.iter().any(|node| node.id == cut_id) {
            let file = cut
                .get("file")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("switch cut {cut_id} needs file"))?;
            nodes.push(NodeDraft {
                id: cut_id.clone(),
                op: "source".into(),
                inputs: Vec::new(),
                file: Some(resolve_take_file(from, file)),
                enable: None,
                role: None,
                params: None,
                keys: None,
                nodes: Vec::new(),
                in_s: cut
                    .get("in")
                    .or_else(|| cut.get("in_s"))
                    .and_then(|v| v.as_f64()),
                out_s: cut
                    .get("out")
                    .or_else(|| cut.get("out_s"))
                    .and_then(|v| v.as_f64()),
            });
        }
        inputs.push(cut_id);
        params_cuts.push(serde_json::json!({ "t": t, "map": index }));
    }
    nodes.push(NodeDraft {
        id: node_id,
        op: "switch".into(),
        inputs,
        file: None,
        enable: None,
        role: None,
        params: Some(serde_json::json!({ "cuts": params_cuts })),
        keys: None,
        nodes: Vec::new(),
        in_s: None,
        out_s: None,
    });
    Ok(())
}

fn resolve_take_file(from: &Path, spec: &str) -> PathBuf {
    let path = Path::new(spec);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        from.join(path)
    }
}

fn apply_joins(dir: &Path, from: &Path, extra: Option<&Vec<serde_json::Value>>) -> Result<()> {
    let mut items = Vec::new();
    if let Some(extra) = extra {
        items.extend(extra.iter().cloned());
    }
    let path = from.join("joins.json");
    if path.is_file() {
        let value: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?,
        )
        .with_context(|| format!("parse {}", path.display()))?;
        if let Some(arr) = value.as_array() {
            items.extend(arr.iter().cloned());
        } else if let Some(arr) = value.get("joins").and_then(|v| v.as_array()) {
            items.extend(arr.iter().cloned());
        } else {
            bail!(
                "{} must be an array or {{\"joins\": [...]}}",
                path.display()
            );
        }
    }
    if items.is_empty() {
        return Ok(());
    }
    let score_path = paths::score(dir);
    let mut score = load_score(&score_path)?;
    for item in items {
        let left = item
            .get("left")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("join needs left"))?;
        let right = item
            .get("right")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("join needs right"))?;
        let transition = canonical_transition(
            item.get("transition")
                .and_then(|v| v.as_str())
                .unwrap_or("fade"),
        )
        .map_err(|e| anyhow::anyhow!("{e}"))?;
        let duration_frames =
            if let Some(frames) = item.get("duration_frames").and_then(|v| v.as_u64()) {
                frames
            } else if let Some(seconds) = item.get("duration_seconds").and_then(|v| v.as_f64()) {
                score.clock.rate.frames_from_seconds(seconds)?.max(1) as u64
            } else {
                score.clock.rate.frames_from_seconds(0.2)?.max(1) as u64
            };
        let join = Join {
            left: left.into(),
            right: right.into(),
            transition,
            duration_frames,
        };
        if let Some(existing) = score
            .joins
            .iter_mut()
            .find(|row| row.left == join.left && row.right == join.right)
        {
            *existing = join;
        } else {
            score.joins.push(join);
        }
    }
    score.validate()?;
    save_json(&score_path, &score)?;
    eprintln!("wrote {}", score_path.display());
    Ok(())
}

fn apply_markers(dir: &Path, extra: Option<&Vec<serde_json::Value>>) -> Result<()> {
    let Some(items) = extra else {
        return Ok(());
    };
    if items.is_empty() {
        return Ok(());
    }
    let score_path = paths::score(dir);
    let mut score = load_score(&score_path)?;
    for item in items {
        let marker: Marker = if item.get("range").is_some() {
            serde_json::from_value(item.clone())
                .with_context(|| format!("marker {}", item.get("id").unwrap_or(item)))?
        } else {
            let id = item
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("marker needs id"))?;
            let start = item
                .get("start")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| anyhow::anyhow!("marker {id} needs start"))?;
            let duration = item
                .get("duration")
                .and_then(|v| v.as_u64())
                .ok_or_else(|| anyhow::anyhow!("marker {id} needs duration"))?;
            Marker {
                id: id.into(),
                range: graft_score::FrameRange::new(start, duration),
            }
        };
        if let Some(existing) = score.markers.iter_mut().find(|row| row.id == marker.id) {
            *existing = marker;
        } else {
            score.markers.push(marker);
        }
    }
    score.validate()?;
    save_json(&score_path, &score)?;
    eprintln!("wrote {}", score_path.display());
    Ok(())
}

fn upsert_draft(
    dir: &Path,
    from: &Path,
    graph: &mut CompositionGraph,
    draft: NodeDraft,
    rate: FrameRate,
    default_out: f64,
) -> Result<()> {
    if let Some(existing) = find_node_mut(&mut graph.nodes, &draft.id) {
        let prepared = prepare_draft(dir, from, &draft, rate, default_out)?;
        existing.op = draft.op.clone();
        if !draft.inputs.is_empty() {
            existing.inputs = draft.inputs.clone();
        }
        if prepared.material.is_some() {
            existing.material = prepared.material;
        }
        if prepared.source.is_some() {
            existing.source = prepared.source;
        }
        existing.enable = draft.enable.clone();
        existing.role = draft.role.clone().or(existing.role.clone());
        if prepared.params.is_some() {
            existing.params = prepared.params;
        }
        if draft.keys.is_some() {
            existing.keys = draft.keys.clone();
        }
        if !draft.nodes.is_empty() {
            existing.nodes = materialize_nodes(dir, from, draft.nodes, rate, default_out)?;
        }
        return Ok(());
    }
    let node = materialize_node(dir, from, draft, rate, default_out)?;
    graph.nodes.push(node);
    Ok(())
}

struct PreparedDraft {
    material: Option<String>,
    source: Option<TimedRange>,
    params: Option<serde_json::Value>,
}

fn prepare_draft(
    dir: &Path,
    from: &Path,
    draft: &NodeDraft,
    rate: FrameRate,
    default_out: f64,
) -> Result<PreparedDraft> {
    let material = match &draft.file {
        Some(file) => Some(put_node_material(dir, from, file)?),
        None => None,
    };
    let source = if matches!(draft.op.as_str(), "source" | "audio_source" | "nested") {
        let start = draft.in_s.unwrap_or(0.0);
        let end = draft.out_s.unwrap_or(default_out);
        Some(TimedRange::from_seconds(rate, start, end)?)
    } else {
        None
    };
    let params = match draft.params.clone() {
        Some(serde_json::Value::Object(mut map)) if draft.op == "blend" => {
            if let Some(mode) = map.get("mode").and_then(|v| v.as_str()) {
                let canon =
                    graft_score::canonical_blend(mode).map_err(|e| anyhow::anyhow!("{e}"))?;
                map.insert("mode".into(), serde_json::Value::String(canon));
            }
            Some(serde_json::Value::Object(map))
        }
        other => other,
    };
    Ok(PreparedDraft {
        material,
        source,
        params,
    })
}

fn materialize_nodes(
    dir: &Path,
    from: &Path,
    drafts: Vec<NodeDraft>,
    rate: FrameRate,
    default_out: f64,
) -> Result<Vec<GraphNode>> {
    let mut scratch = CompositionGraph {
        mode: GraphMode::Append,
        nodes: Vec::new(),
    };
    for draft in drafts {
        upsert_draft(dir, from, &mut scratch, draft, rate, default_out)?;
    }
    Ok(scratch.nodes)
}

fn materialize_node(
    dir: &Path,
    from: &Path,
    draft: NodeDraft,
    rate: FrameRate,
    default_out: f64,
) -> Result<GraphNode> {
    let prepared = prepare_draft(dir, from, &draft, rate, default_out)?;
    let structural = matches!(
        draft.op.as_str(),
        "source" | "audio_source" | "gap" | "nested" | "group"
    ) || graft_compile::is_lavfi_source(&draft.op).unwrap_or(false);
    let inputs = if !draft.inputs.is_empty() {
        draft.inputs
    } else if structural {
        Vec::new()
    } else {
        vec![default_input_for(&draft.op)]
    };
    let nodes = if draft.nodes.is_empty() {
        Vec::new()
    } else {
        materialize_nodes(dir, from, draft.nodes, rate, default_out)?
    };
    Ok(GraphNode {
        id: draft.id,
        op: draft.op,
        inputs,
        material: prepared.material,
        source: prepared.source,
        place: None,
        enable: draft.enable,
        role: draft.role,
        params: prepared.params,
        keys: draft.keys,
        nodes,
    })
}

fn default_input_for(op: &str) -> String {
    if graft_compile::routes_audio(op).unwrap_or(false) {
        "#audio".into()
    } else {
        "#picture".into()
    }
}

fn upsert_lut(graph: &mut CompositionGraph, id: &str, material: String) {
    let op = if id == "legal" { "lut1d" } else { "lut3d" };
    let inputs = vec!["#picture".to_string()];
    if let Some(existing) = graph.nodes.iter_mut().find(|node| node.id == id) {
        existing.op = op.into();
        existing.material = Some(material);
        if existing.inputs.is_empty() {
            existing.inputs = inputs;
        }
        return;
    }
    graph.nodes.push(GraphNode {
        id: id.into(),
        op: op.into(),
        inputs,
        material: Some(material),
        source: None,
        place: None,
        enable: None,
        role: Some(id.into()),
        params: None,
        keys: None,
        nodes: Vec::new(),
    });
}

fn put_node_material(dir: &Path, from: &Path, file: &Path) -> Result<String> {
    let path = if file.is_file() {
        file.to_path_buf()
    } else {
        resolve_take_file(from, &file.to_string_lossy())
    };
    if !path.is_file() {
        bail!("composition file {} is missing", path.display());
    }
    let bytes = std::fs::read(&path)?;
    let store = Fs::open(paths::graft_dir(dir))?;
    Ok(store.put_blob(Kind::Material, &bytes)?.to_string())
}

pub fn put_lut_material(dir: &Path, path: &Path) -> Result<String> {
    if !path.is_file() {
        bail!("LUT must be an existing file");
    }
    let bytes = std::fs::read(path)?;
    if !looks_like_lut(&bytes) {
        bail!("{} is not a LUT (.cube / .3dl)", path.display());
    }
    let store = Fs::open(paths::graft_dir(dir))?;
    Ok(store.put_blob(Kind::Material, &bytes)?.to_string())
}

pub fn looks_like_lut(bytes: &[u8]) -> bool {
    let text = String::from_utf8_lossy(bytes);
    let trimmed = text.trim_start();
    trimmed.starts_with("LUT_")
        || trimmed.starts_with("TITLE")
        || trimmed.starts_with('#')
        || text.contains("LUT_3D_SIZE")
        || text.contains("LUT_1D_SIZE")
}

pub fn list_lut_files(from: &Path) -> Result<Vec<PathBuf>> {
    if !from.is_dir() {
        bail!("{} is not a LUT directory", from.display());
    }
    let mut files = Vec::new();
    for entry in std::fs::read_dir(from)? {
        let path = entry?.path();
        let ext = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_ascii_lowercase());
        if path.is_file()
            && matches!(ext.as_deref(), Some("cube" | "3dl"))
            && looks_like_lut(&std::fs::read(&path)?)
        {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

fn first_existing(from: &Path, names: &[&str]) -> Option<PathBuf> {
    names
        .iter()
        .map(|name| from.join(name))
        .find(|path| path.is_file())
}

pub fn apply_node_file(
    dir: &Path,
    graph: &mut CompositionGraph,
    node_id: &str,
    file: &Path,
) -> Result<()> {
    let bytes = std::fs::read(file).with_context(|| format!("read {}", file.display()))?;
    if looks_like_lut(&bytes) || looks_like_params(&bytes) {
        if looks_like_params(&bytes) {
            let params: serde_json::Value = serde_json::from_slice(&bytes)?;
            if let Some(node) = find_node_mut(&mut graph.nodes, node_id) {
                node.params = Some(params);
                return Ok(());
            }
            bail!("graph has no node {node_id}");
        }
        let material = put_lut_material(dir, file)?;
        if let Some(node) = find_node_mut(&mut graph.nodes, node_id) {
            node.material = Some(material);
            return Ok(());
        }
        upsert_lut(graph, node_id, material);
        return Ok(());
    }
    let material = put_node_material(dir, Path::new("."), file)?;
    if let Some(node) = find_node_mut(&mut graph.nodes, node_id) {
        node.material = Some(material);
        return Ok(());
    }
    bail!("graph has no node {node_id}");
}

fn looks_like_params(bytes: &[u8]) -> bool {
    serde_json::from_slice::<serde_json::Value>(bytes)
        .ok()
        .is_some_and(|v| v.is_object() && v.get("id").is_none())
}
