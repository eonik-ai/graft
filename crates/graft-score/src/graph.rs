// SPDX-License-Identifier: Apache-2.0

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::material::require_material;
use crate::scion::{validate_ease, Keyframe};
use crate::time::{FrameRange, TimedRange};
use crate::Error;

/// ffmpeg `xfade` transition names plus `cut`. `fade` is xfade fade.
pub const XFADE_TRANSITIONS: &[&str] = &[
    "fade",
    "fadefast",
    "fadeslow",
    "fadeblack",
    "fadewhite",
    "fadegrays",
    "dissolve",
    "pixelize",
    "wipeleft",
    "wiperight",
    "wipeup",
    "wipedown",
    "wipetl",
    "wipetr",
    "wipebl",
    "wipebr",
    "slideleft",
    "slideright",
    "slideup",
    "slidedown",
    "smoothleft",
    "smoothright",
    "smoothup",
    "smoothdown",
    "circlecrop",
    "rectcrop",
    "circleopen",
    "circleclose",
    "vertopen",
    "vertclose",
    "horzopen",
    "horzclose",
    "radial",
    "distance",
    "diagtl",
    "diagtr",
    "diagbl",
    "diagbr",
    "hlslice",
    "hrslice",
    "vuslice",
    "vdslice",
    "hlwind",
    "hrwind",
    "vuwind",
    "vdwind",
    "hblur",
    "squeezeh",
    "squeezev",
    "zoomin",
    "coverleft",
    "coverright",
    "coverup",
    "coverdown",
    "revealleft",
    "revealright",
    "revealup",
    "revealdown",
    "custom",
];

pub const NODE_OPS: &[&str] = &[
    "source",
    "audio_source",
    "gap",
    "concat",
    "xfade",
    "acrossfade",
    "overlay",
    "blend",
    "lut3d",
    "lut1d",
    "eq",
    "hue",
    "curves",
    "colorbalance",
    "colortemperature",
    "vibrance",
    "unsharp",
    "vignette",
    "noise",
    "gblur",
    "setrange",
    "colorspace",
    "tonemap",
    "scale",
    "crop",
    "pad",
    "rotate",
    "flip",
    "zoompan",
    "reverse",
    "freeze",
    "fade",
    "afade",
    "amix",
    "volume",
    "loudnorm",
    "sidechain",
    "captions",
    "chromakey",
    "colorkey",
    "lumakey",
    "chromahold",
    "alphamerge",
    "drawbox",
    "drawgrid",
    "colorchannelmixer",
    "setsar",
    "setdar",
    "perspective",
    "shear",
    "scroll",
    "premultiply",
    "unpremultiply",
    "streamselect",
    "maskedmerge",
    "hstack",
    "vstack",
    "xstack",
    "mix",
    "lenscorrection",
    "yadif",
    "bwdif",
    "tile",
    "haldclut",
    "colorcorrect",
    "cdl",
    "lgg",
    "geq",
    "selectivecolor",
    "exposure",
    "colorhold",
    "colorize",
    "deband",
    "switch",
    "acompressor",
    "alimiter",
    "equalizer",
    "highpass",
    "lowpass",
    "bass",
    "treble",
    "adelay",
    "aecho",
    "areverse",
    "pan",
    "nested",
    "group",
];

pub fn is_xfade_name(name: &str) -> bool {
    XFADE_TRANSITIONS.contains(&name)
}

/// A name ffmpeg could have. The binary, not this list, decides at compile.
pub fn is_filter_name(op: &str) -> bool {
    let mut chars = op.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() => {}
        _ => return false,
    }
    op.len() <= 64 && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

pub fn op_allowed(op: &str) -> bool {
    NODE_OPS.contains(&op) || is_filter_name(op)
}

pub fn op_is_audio(op: &str) -> bool {
    matches!(
        op,
        "audio_source"
            | "volume"
            | "loudnorm"
            | "afade"
            | "acrossfade"
            | "amix"
            | "sidechain"
            | "acompressor"
            | "alimiter"
            | "equalizer"
            | "highpass"
            | "lowpass"
            | "bass"
            | "treble"
            | "adelay"
            | "aecho"
            | "areverse"
            | "pan"
    )
}

/// Side pads (`source`, `nested`, `group`) keep their own id. They do not steal `#picture`.
pub fn op_writes_picture(op: &str) -> bool {
    !op_is_audio(op) && op != "captions" && op != "source" && op != "nested" && op != "group"
}

pub fn nested_scion_id(node: &GraphNode) -> Option<&str> {
    if node.op != "nested" {
        return None;
    }
    node.params
        .as_ref()
        .and_then(|value| value.get("scion"))
        .and_then(|value| value.as_str())
        .filter(|id| !id.is_empty())
}

pub fn join_is_overlap(transition: &str) -> bool {
    matches!(canonical_transition(transition).as_deref(), Ok(name) if name != "cut")
}

/// Host aliases (`dipToBlack`, `wipeLeft`, …) → ffmpeg `xfade` name or `cut`.
pub fn canonical_transition(transition: &str) -> Result<String, Error> {
    let raw = transition.trim();
    let lower = raw.to_ascii_lowercase().replace('_', "");
    if lower.starts_with("whip") || lower == "flash" {
        return Err(Error::invalid(format!(
            r#"{{"error":"guest_transition","transition":"{transition}"}}"#
        )));
    }
    let mapped = match lower.as_str() {
        "cut" => "cut",
        "fade" => "fade",
        "dissolve" => "dissolve",
        "diptoblack" => "fadeblack",
        "diptowhite" => "fadewhite",
        "wipeleft" => "wipeleft",
        "wiperight" => "wiperight",
        "wipeup" => "wipeup",
        "wipedown" => "wipedown",
        "slideleft" => "slideleft",
        "slideright" => "slideright",
        "slideup" => "slideup",
        "slidedown" => "slidedown",
        "zoomin" => "zoomin",
        "blur" => "hblur",
        other => other,
    };
    if mapped == "cut" || is_xfade_name(mapped) {
        Ok(mapped.into())
    } else {
        Err(Error::invalid(format!(
            r#"{{"error":"unknown_transition","transition":"{transition}"}}"#
        )))
    }
}

pub fn xfade_filter_name(transition: &str) -> Result<String, Error> {
    let name = canonical_transition(transition)?;
    if name == "cut" {
        return Err(Error::invalid("cut is not an xfade"));
    }
    Ok(name)
}

pub fn validate_join_transition(transition: &str) -> Result<(), Error> {
    canonical_transition(transition).map(|_| ())
}

/// Host `LayerBlendMode` names → ffmpeg `blend=all_mode=`.
/// HSL channel blends have no ffmpeg name; they fail closed (guest).
pub fn canonical_blend(mode: &str) -> Result<String, Error> {
    let raw = mode.trim();
    let lower = raw.to_ascii_lowercase().replace('_', "");
    let mapped = match lower.as_str() {
        "hardlight" => "hardlight",
        "softlight" => "softlight",
        "colordodge" => "dodge",
        "colorburn" => "burn",
        "hue" | "saturation" | "color" | "luminosity" => {
            return Err(Error::invalid(format!(
                r#"{{"error":"guest_blend","mode":"{mode}"}}"#
            )))
        }
        other => other,
    };
    if BLEND_MODES.contains(&mapped) {
        Ok(mapped.into())
    } else {
        Err(Error::invalid(format!(
            r#"{{"error":"unknown_blend","mode":"{mode}"}}"#
        )))
    }
}

const BLEND_MODES: &[&str] = &[
    "normal",
    "addition",
    "addition128",
    "grainmerge",
    "and",
    "average",
    "burn",
    "darken",
    "difference",
    "difference128",
    "grainextract",
    "divide",
    "dodge",
    "exclusion",
    "extremity",
    "freeze",
    "glow",
    "hardlight",
    "hardmix",
    "heat",
    "lighten",
    "linearlight",
    "multiply",
    "multiply128",
    "negation",
    "or",
    "overlay",
    "phoenix",
    "pinlight",
    "reflect",
    "screen",
    "softlight",
    "subtract",
    "vividlight",
    "xor",
    "softdifference",
    "geometric",
    "harmonic",
    "bleach",
    "stain",
    "interpolate",
    "hardoverlay",
];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GraphMode {
    #[default]
    Append,
    Replace,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkingSpace {
    pub pix_fmt: String,
    pub color: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub primaries: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trc: Option<String>,
}

impl WorkingSpace {
    pub fn default_for_color(color: &str) -> Self {
        Self {
            pix_fmt: "yuv422p10le".into(),
            color: color.to_string(),
            primaries: None,
            trc: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GraphNode {
    pub id: String,
    pub op: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inputs: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub material: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<TimedRange>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub place: Option<FrameRange>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enable: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
    /// Per-option dest-time keys. Expression options lower to eased `t`.
    /// Numeric options lower to `enable=` segments.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keys: Option<BTreeMap<String, Vec<Keyframe>>>,
    /// Stored adjustment group. Compile expands children and inherits `enable`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub nodes: Vec<GraphNode>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompositionGraph {
    #[serde(default)]
    pub mode: GraphMode,
    pub nodes: Vec<GraphNode>,
}

impl GraphNode {
    pub fn validate(&self) -> Result<(), Error> {
        if self.id.is_empty() {
            return Err(Error::invalid("graph node id is empty"));
        }
        if !op_allowed(&self.op) {
            return Err(Error::invalid(format!(
                r#"{{"error":"unknown_op","op":"{}","id":"{}"}}"#,
                self.op, self.id
            )));
        }
        if let Some(material) = &self.material {
            require_material(material)?;
        }
        if let Some(source) = &self.source {
            source.validate(&format!("graph {}.source", self.id))?;
        }
        if let Some(place) = &self.place {
            place.validate(&format!("graph {}.place", self.id))?;
        }
        if let Some(role) = &self.role {
            if role.is_empty() {
                return Err(Error::invalid(format!("graph {}.role is empty", self.id)));
            }
        }
        match self.op.as_str() {
            "lut3d" | "lut1d" | "source" | "audio_source" if self.material.is_none() => {
                return Err(Error::invalid(format!(
                    "graph {} op {} needs material",
                    self.id, self.op
                )));
            }
            "nested" if nested_scion_id(self).is_none() => {
                return Err(Error::invalid(format!(
                    "graph {} op nested needs params.scion",
                    self.id
                )));
            }
            "xfade" | "acrossfade" | "overlay" | "blend" | "sidechain" | "concat" | "amix"
            | "alphamerge" | "hstack" | "vstack" | "xstack" | "mix"
                if self.inputs.len() < 2 =>
            {
                return Err(Error::invalid(format!(
                    "graph {} op {} needs two inputs",
                    self.id, self.op
                )));
            }
            "maskedmerge" if self.inputs.len() < 3 => {
                return Err(Error::invalid(format!(
                    "graph {} op maskedmerge needs three inputs",
                    self.id
                )));
            }
            "switch" if self.inputs.len() < 2 => {
                return Err(Error::invalid(format!(
                    "graph {} op switch needs two inputs",
                    self.id
                )));
            }
            _ => {}
        }
        if self.op == "group" && self.nodes.is_empty() {
            return Err(Error::invalid(format!(
                "graph {} op group needs nodes",
                self.id
            )));
        }
        if let Some(keys) = &self.keys {
            for (name, row) in keys {
                for key in row {
                    if !key.t.is_finite() || key.t < 0.0 || !key.v.is_finite() {
                        return Err(Error::invalid(format!(
                            r#"{{"error":"bad_keys","id":"{}","param":"{name}"}}"#,
                            self.id
                        )));
                    }
                    validate_ease(key.ease.as_deref())?;
                }
            }
        }
        for child in &self.nodes {
            child.validate()?;
        }
        if self.op == "switch" {
            let cuts = self
                .params
                .as_ref()
                .and_then(|value| value.get("cuts"))
                .and_then(|value| value.as_array());
            let Some(cuts) = cuts else {
                return Err(Error::invalid(format!(
                    "graph {} op switch needs params.cuts",
                    self.id
                )));
            };
            if cuts.is_empty() {
                return Err(Error::invalid(format!(
                    "graph {} op switch needs params.cuts",
                    self.id
                )));
            }
            for cut in cuts {
                let map = cut.get("map").and_then(|value| value.as_u64()).unwrap_or(0) as usize;
                if map >= self.inputs.len() {
                    return Err(Error::invalid(format!(
                        r#"{{"error":"bad_switch","id":"{}"}}"#,
                        self.id
                    )));
                }
            }
        }
        Ok(())
    }
}

impl CompositionGraph {
    pub fn validate(&self) -> Result<(), Error> {
        let mut seen = BTreeSet::new();
        collect_ids(&self.nodes, &mut seen)?;
        for node in &self.nodes {
            node.validate()?;
        }
        Ok(())
    }
}

fn collect_ids<'a>(nodes: &'a [GraphNode], seen: &mut BTreeSet<&'a str>) -> Result<(), Error> {
    for node in nodes {
        if !seen.insert(node.id.as_str()) {
            return Err(Error::invalid(format!("duplicate graph node {}", node.id)));
        }
        collect_ids(&node.nodes, seen)?;
    }
    Ok(())
}

pub fn expand_groups(nodes: &[GraphNode]) -> Vec<GraphNode> {
    fn walk(nodes: &[GraphNode], inherited: Option<&str>, out: &mut Vec<GraphNode>) {
        for node in nodes {
            if node.op == "group" {
                let enable = node.enable.as_deref().or(inherited);
                walk(&node.nodes, enable, out);
            } else {
                let mut flat = node.clone();
                if flat.enable.is_none() {
                    flat.enable = inherited.map(str::to_string);
                }
                flat.nodes.clear();
                out.push(flat);
            }
        }
    }
    let mut out = Vec::new();
    walk(nodes, None, &mut out);
    out
}

pub fn find_node_mut<'a>(nodes: &'a mut [GraphNode], id: &str) -> Option<&'a mut GraphNode> {
    for node in nodes {
        if node.id == id {
            return Some(node);
        }
        if let Some(found) = find_node_mut(&mut node.nodes, id) {
            return Some(found);
        }
    }
    None
}

pub fn graph_has_node(nodes: &[GraphNode], id: &str) -> bool {
    nodes
        .iter()
        .any(|node| node.id == id || graph_has_node(&node.nodes, id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fade_is_xfade() {
        assert!(join_is_overlap("fade"));
        assert!(join_is_overlap("wipeleft"));
        assert!(!join_is_overlap("cut"));
        assert_eq!(xfade_filter_name("fade").unwrap(), "fade");
        assert_eq!(canonical_transition("dipToBlack").unwrap(), "fadeblack");
        assert_eq!(canonical_transition("wipeLeft").unwrap(), "wipeleft");
        assert!(validate_join_transition("zoomin").is_ok());
        assert!(validate_join_transition("nope").is_err());
        assert!(canonical_transition("whipLeft").is_err());
        assert!(canonical_transition("flash").is_err());
        assert_eq!(canonical_blend("hardLight").unwrap(), "hardlight");
        assert_eq!(canonical_blend("colorDodge").unwrap(), "dodge");
        assert!(canonical_blend("hue").is_err());
        let nested = GraphNode {
            id: "plate".into(),
            op: "nested".into(),
            inputs: vec![],
            material: None,
            source: None,
            place: None,
            enable: None,
            role: None,
            params: Some(serde_json::json!({"scion": "plate"})),
            keys: None,
            nodes: Vec::new(),
        };
        nested.validate().unwrap();
        assert!(!op_writes_picture("nested"));
        assert_eq!(nested_scion_id(&nested), Some("plate"));
        for op in ["colorcorrect", "cdl", "lgg", "bwdif", "tile", "haldclut"] {
            GraphNode {
                id: op.into(),
                op: op.into(),
                inputs: vec!["#picture".into()],
                material: None,
                source: None,
                place: None,
                enable: None,
                role: None,
                params: None,
                keys: None,
                nodes: Vec::new(),
            }
            .validate()
            .unwrap();
        }
    }

    #[test]
    fn unknown_op_fails() {
        let node = GraphNode {
            id: "x".into(),
            op: "Look!".into(),
            inputs: vec![],
            material: None,
            source: None,
            place: None,
            enable: None,
            role: None,
            params: None,
            keys: None,
            nodes: Vec::new(),
        };
        assert!(node
            .validate()
            .unwrap_err()
            .to_string()
            .contains("unknown_op"));
    }

    #[test]
    fn filter_shaped_name_is_a_candidate() {
        let node = GraphNode {
            id: "denoise".into(),
            op: "hqdn3d".into(),
            inputs: vec!["#picture".into()],
            material: None,
            source: None,
            place: None,
            enable: None,
            role: None,
            params: None,
            keys: None,
            nodes: Vec::new(),
        };
        node.validate().unwrap();
        assert!(is_filter_name("hqdn3d"));
        assert!(!is_filter_name("cinematic grade"));
    }
}
