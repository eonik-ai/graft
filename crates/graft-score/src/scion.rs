// SPDX-License-Identifier: Apache-2.0

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::canonical::require_graft_version;
use crate::graph::{CompositionGraph, WorkingSpace};
use crate::ident::{require_slot_id, require_uuid};
use crate::material::require_material;
use crate::role::Layer;
use crate::score::Score;
use crate::time::{FrameRange, FrameRate, TimedRange};
use crate::Error;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RateControl {
    pub mode: RateControlMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crf: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bitrate: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RateControlMode {
    Crf,
    Bitrate,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Encoder {
    #[serde(rename = "impl")]
    pub impl_name: String,
    pub version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rate_control: Option<RateControl>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preset: Option<String>,
    pub keyint: u32,
    pub sc_threshold: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub toolchain: Option<Value>,
}

impl Encoder {
    /// Fingerprint only. Does not invoke this encoder.
    pub fn default_x264() -> Self {
        Self {
            impl_name: "x264".into(),
            version: "r3222".into(),
            profile: Some("high".into()),
            level: Some("4.1".into()),
            rate_control: Some(RateControl {
                mode: RateControlMode::Crf,
                crf: Some(18.0),
                bitrate: None,
            }),
            preset: Some("medium".into()),
            keyint: 30,
            sc_threshold: 0,
            toolchain: Some(serde_json::json!({
                "contract": "ffmpeg-libx264-closed-gop-v1",
                "threads": 1,
                "open_gop": false,
                "stitchable": true
            })),
        }
    }

    /// In-tree intra / image-seq grain. Not a Long-GOP fingerprint.
    pub fn graft_intra() -> Self {
        Self {
            impl_name: "graft-intra".into(),
            version: "1".into(),
            profile: Some("intra".into()),
            level: None,
            rate_control: None,
            preset: None,
            keyint: 1,
            sc_threshold: 0,
            toolchain: Some(serde_json::json!({
                "contract": "graft-intra-v1"
            })),
        }
    }

    pub fn named(name: &str) -> Result<Self, Error> {
        match name {
            "x264" | "libx264" => Ok(Self::default_x264()),
            "graft-intra" | "intra" => Ok(Self::graft_intra()),
            other => Err(Error::invalid(format!(
                "unknown encoder {other:?} (x264 or graft-intra)"
            ))),
        }
    }

    pub fn validate(&self) -> Result<(), Error> {
        if self.impl_name.is_empty() || self.version.is_empty() {
            return Err(Error::invalid("encoder impl and version are required"));
        }
        if self.keyint < 1 {
            return Err(Error::invalid("encoder.keyint must be >= 1"));
        }
        if self.sc_threshold != 0 {
            return Err(Error::invalid("encoder.sc_threshold must be 0"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dest {
    pub id: String,
    pub width: u32,
    pub height: u32,
    pub rate: FrameRate,
    pub pix_fmt: String,
    pub color: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub working: Option<WorkingSpace>,
    pub encoder: Encoder,
}

impl Dest {
    pub fn validate(&self) -> Result<(), Error> {
        if self.id.is_empty() {
            return Err(Error::invalid("dest.id is empty"));
        }
        if self.id.contains(':') {
            return Err(Error::invalid(
                "dest id is 9x16, not 9:16 (ratio is not a slot)",
            ));
        }
        if self.width < 1 || self.height < 1 {
            return Err(Error::invalid("dest width/height must be >= 1"));
        }
        self.rate.validate("dest.rate")?;
        if self.pix_fmt.is_empty() || self.color.is_empty() {
            return Err(Error::invalid("dest pix_fmt and color are required"));
        }
        if let Some(working) = &self.working {
            if working.pix_fmt.is_empty() || working.color.is_empty() {
                return Err(Error::invalid(
                    "dest.working pix_fmt and color are required",
                ));
            }
        }
        self.encoder.validate()
    }

    pub fn working_space(&self) -> WorkingSpace {
        self.working
            .clone()
            .unwrap_or_else(|| WorkingSpace::default_for_color(&self.color))
    }

    pub fn working_pix_fmt(&self) -> String {
        self.working_space().pix_fmt
    }

    pub fn working_color(&self) -> String {
        self.working_space().color
    }

    pub fn working_primaries(&self) -> String {
        self.working
            .as_ref()
            .and_then(|w| w.primaries.clone())
            .unwrap_or_else(|| self.working_color())
    }

    pub fn working_trc(&self) -> String {
        self.working
            .as_ref()
            .and_then(|w| w.trc.clone())
            .unwrap_or_else(|| self.working_color())
    }

    /// Working-pad fingerprint. Does not include the delivery encoder.
    pub fn working_fingerprint_value(&self) -> Value {
        let mut value = serde_json::json!({
            "width": self.width,
            "height": self.height,
            "rate": self.rate,
            "working_pix_fmt": self.working_pix_fmt(),
            "working_color": self.working_color(),
            "working_range": "tv",
            "working_codec": "prores_ks",
        });
        if let Some(obj) = value.as_object_mut() {
            if let Some(primaries) = self.working.as_ref().and_then(|w| w.primaries.as_ref()) {
                obj.insert("working_primaries".into(), Value::String(primaries.clone()));
            }
            if let Some(trc) = self.working.as_ref().and_then(|w| w.trc.as_ref()) {
                obj.insert("working_trc".into(), Value::String(trc.clone()));
            }
        }
        value
    }

    pub fn fingerprint_value(&self) -> Value {
        serde_json::json!({
            "width": self.width,
            "height": self.height,
            "rate": self.rate,
            "pix_fmt": self.pix_fmt,
            "color": self.color,
        })
    }

    pub fn encoder_value(&self) -> Value {
        serde_json::to_value(&self.encoder).expect("encoder JSON")
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub material: String,
    pub source: TimedRange,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audio: Option<AudioBinding>,
    /// Inner scion compiled to a working-depth pad (not delivery).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nested: Option<String>,
    /// Overlay mix geometry. Not in the working-source key.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opacity: Option<f64>,
    /// Dest-time overlay path. Mix geometry, not the working-source key.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub motion: Option<Motion>,
    /// Dest-time speed keys (`v` is speed). Working-source key.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub remap: Vec<Keyframe>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Keyframe {
    pub t: f64,
    pub v: f64,
    /// How `v` is approached from the previous key. `hold` stays put until this key.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ease: Option<String>,
}

impl Keyframe {
    pub fn ease_name(&self) -> &str {
        match self.ease.as_deref() {
            None | Some("linear") => "linear",
            Some(other) => other,
        }
    }
}

pub fn validate_ease(ease: Option<&str>) -> Result<(), Error> {
    match ease {
        None | Some("linear" | "hold" | "ease_in" | "ease_out" | "ease_in_out") => Ok(()),
        Some(other) => Err(Error::invalid(format!(
            r#"{{"error":"bad_ease","ease":"{other}"}}"#
        ))),
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Motion {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub x: Vec<Keyframe>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub y: Vec<Keyframe>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub opacity: Vec<Keyframe>,
}

/// Piecewise-linear ffmpeg expression on dest `t`.
pub fn piecewise_linear(keys: &[Keyframe]) -> String {
    piecewise_eased(keys, "t")
}

/// Eased ffmpeg expression. `ease` on the later key is how `v` is approached.
/// Linear matches the historical `if(lt(t,…),v0+(v1-v0)*(t-t0)/dt,…)` form.
pub fn piecewise_eased(keys: &[Keyframe], time: &str) -> String {
    let mut keys: Vec<&Keyframe> = keys
        .iter()
        .filter(|k| k.t.is_finite() && k.v.is_finite())
        .collect();
    keys.sort_by(|a, b| a.t.partial_cmp(&b.t).unwrap_or(std::cmp::Ordering::Equal));
    match keys.as_slice() {
        [] => "0".into(),
        [one] => format!("{:.6}", one.v),
        rest => {
            let mut expr = format!("{:.6}", rest[rest.len() - 1].v);
            for window in rest.windows(2).rev() {
                let t0 = window[0].t;
                let t1 = window[1].t;
                let v0 = window[0].v;
                let v1 = window[1].v;
                let dt = (t1 - t0).abs().max(1e-6);
                let shaped = match window[1].ease_name() {
                    "hold" => format!("{v0:.6}"),
                    "ease_in" => {
                        format!("{v0:.6}+({v1:.6}-{v0:.6})*pow(({time}-{t0:.6})/{dt:.6},2)")
                    }
                    "ease_out" => {
                        format!("{v0:.6}+({v1:.6}-{v0:.6})*(1-pow(1-(({time}-{t0:.6})/{dt:.6}),2))")
                    }
                    "ease_in_out" => {
                        let u = format!("(({time}-{t0:.6})/{dt:.6})");
                        format!("{v0:.6}+({v1:.6}-{v0:.6})*({u}*{u}*(3-2*{u}))")
                    }
                    _ => format!("{v0:.6}+({v1:.6}-{v0:.6})*({time}-{t0:.6})/{dt:.6}"),
                };
                expr = format!("if(lt({time},{t1:.6}),{shaped},{expr})");
            }
            expr
        }
    }
}

/// Numeric value of an eased curve at dest time `t`.
pub fn sample_keys(keys: &[Keyframe], t: f64) -> f64 {
    let mut ordered: Vec<&Keyframe> = keys
        .iter()
        .filter(|k| k.t.is_finite() && k.v.is_finite())
        .collect();
    ordered.sort_by(|a, b| a.t.partial_cmp(&b.t).unwrap_or(std::cmp::Ordering::Equal));
    if ordered.is_empty() {
        return 0.0;
    }
    if t <= ordered[0].t {
        return ordered[0].v;
    }
    if t >= ordered[ordered.len() - 1].t {
        return ordered[ordered.len() - 1].v;
    }
    for window in ordered.windows(2) {
        if t >= window[0].t && t < window[1].t {
            let dt = (window[1].t - window[0].t).abs().max(1e-6);
            let u = ((t - window[0].t) / dt).clamp(0.0, 1.0);
            let shaped = match window[1].ease_name() {
                "hold" => 0.0,
                "ease_in" => u * u,
                "ease_out" => 1.0 - (1.0 - u).powi(2),
                "ease_in_out" => u * u * (3.0 - 2.0 * u),
                _ => u,
            };
            return window[0].v + (window[1].v - window[0].v) * shaped;
        }
    }
    ordered[ordered.len() - 1].v
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RemapSpan {
    pub src_in: f64,
    pub src_out: f64,
    pub speed: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioBinding {
    pub material: String,
    pub source: TimedRange,
    /// Dest-clock placement when picture and sound cut at different times (J/L).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub place: Option<FrameRange>,
}

impl Binding {
    pub fn in_s(&self) -> f64 {
        self.source.start_seconds()
    }

    pub fn out_s(&self) -> f64 {
        self.source.end_seconds()
    }

    pub fn speed(&self) -> f64 {
        self.params
            .as_ref()
            .and_then(|value| value.get("speed"))
            .and_then(|value| value.as_f64())
            .unwrap_or(1.0)
    }

    pub fn nested_id(&self) -> Option<&str> {
        self.nested.as_deref().filter(|id| !id.is_empty())
    }

    pub fn overlay_x(&self) -> String {
        if let Some(motion) = &self.motion {
            if !motion.x.is_empty() {
                return piecewise_linear(&motion.x);
            }
        }
        self.x.clone().unwrap_or_else(|| "0".into())
    }

    pub fn overlay_y(&self) -> String {
        if let Some(motion) = &self.motion {
            if !motion.y.is_empty() {
                return piecewise_linear(&motion.y);
            }
        }
        self.y.clone().unwrap_or_else(|| "0".into())
    }

    pub fn overlay_opacity_expr(&self) -> String {
        if let Some(motion) = &self.motion {
            if !motion.opacity.is_empty() {
                return piecewise_linear(&motion.opacity);
            }
        }
        format!("{:.6}", self.overlay_opacity())
    }

    pub fn overlay_opacity(&self) -> f64 {
        if let Some(motion) = &self.motion {
            if let Some(first) = motion.opacity.first() {
                return first.v;
            }
        }
        self.opacity.filter(|v| v.is_finite()).unwrap_or(1.0)
    }

    pub fn remap_keys(&self) -> &[Keyframe] {
        &self.remap
    }

    pub fn remap_is_curve(&self) -> bool {
        self.remap
            .iter()
            .any(|k| k.t.is_finite() && k.v.is_finite() && (k.v - self.speed()).abs() > 1e-9)
            || self.remap.len() > 1
    }

    /// Stepwise speed at dest time `t`. Keys hold until the next key.
    pub fn speed_at(&self, t: f64) -> f64 {
        let mut speed = self.speed();
        let mut keys: Vec<&Keyframe> = self.remap.iter().collect();
        keys.sort_by(|a, b| a.t.partial_cmp(&b.t).unwrap_or(std::cmp::Ordering::Equal));
        for key in keys {
            if key.t.is_finite() && key.v.is_finite() && key.v > 0.0 && key.t <= t + 1e-9 {
                speed = key.v;
            }
        }
        speed
    }

    /// Source spans that fill `dest_s`. A curve starts at `binding.source`.
    pub fn remap_spans(&self, dest_s: f64) -> Vec<RemapSpan> {
        let dest_s = if dest_s.is_finite() {
            dest_s.max(0.0)
        } else {
            0.0
        };
        if !self.remap_is_curve() || dest_s <= 1e-9 {
            return vec![RemapSpan {
                src_in: self.in_s(),
                src_out: self.out_s(),
                speed: self.speed(),
            }];
        }
        let mut marks = vec![0.0_f64];
        for key in &self.remap {
            if key.t > 1e-9 && key.t < dest_s - 1e-9 {
                marks.push(key.t);
            }
        }
        marks.push(dest_s);
        marks.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        marks.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
        let mut cursor = self.in_s();
        let mut spans = Vec::new();
        for window in marks.windows(2) {
            let dt = window[1] - window[0];
            if dt <= 1e-9 {
                continue;
            }
            let speed = self.speed_at(window[0]);
            let src_len = speed * dt;
            spans.push(RemapSpan {
                src_in: cursor,
                src_out: cursor + src_len,
                speed,
            });
            cursor += src_len;
        }
        spans
    }

    pub fn remap_source_seconds(&self, dest_s: f64) -> f64 {
        self.remap_spans(dest_s)
            .iter()
            .map(|span| span.src_out - span.src_in)
            .sum()
    }

    pub fn validate(&self, slot_id: &str) -> Result<(), Error> {
        require_material(&self.material)?;
        self.source.validate(&format!("binding {slot_id}.source"))?;
        if let Some(nested) = &self.nested {
            if nested.is_empty() {
                return Err(Error::invalid(format!("binding {slot_id}.nested is empty")));
            }
        }
        if let Some(x) = &self.x {
            if x.is_empty() {
                return Err(Error::invalid(format!("binding {slot_id}.x is empty")));
            }
        }
        if let Some(y) = &self.y {
            if y.is_empty() {
                return Err(Error::invalid(format!("binding {slot_id}.y is empty")));
            }
        }
        if let Some(opacity) = self.opacity {
            if !opacity.is_finite() || !(0.0..=1.0).contains(&opacity) {
                return Err(Error::invalid(format!(
                    r#"{{"error":"bad_opacity","slot":"{slot_id}"}}"#
                )));
            }
        }
        if let Some(motion) = &self.motion {
            for (axis, keys) in [
                ("x", motion.x.as_slice()),
                ("y", motion.y.as_slice()),
                ("opacity", motion.opacity.as_slice()),
            ] {
                for key in keys {
                    if !key.t.is_finite() || key.t < 0.0 || !key.v.is_finite() {
                        return Err(Error::invalid(format!(
                            r#"{{"error":"bad_motion","slot":"{slot_id}","axis":"{axis}"}}"#
                        )));
                    }
                    if axis == "opacity" && !(0.0..=1.0).contains(&key.v) {
                        return Err(Error::invalid(format!(
                            r#"{{"error":"bad_opacity","slot":"{slot_id}"}}"#
                        )));
                    }
                    validate_ease(key.ease.as_deref()).map_err(|_| {
                        Error::invalid(format!(
                            r#"{{"error":"bad_ease","slot":"{slot_id}","axis":"{axis}"}}"#
                        ))
                    })?;
                }
            }
        }
        for key in &self.remap {
            if !key.t.is_finite() || key.t < 0.0 || !key.v.is_finite() || key.v <= 0.0 {
                return Err(Error::invalid(format!(
                    r#"{{"error":"bad_remap","slot":"{slot_id}"}}"#
                )));
            }
            if key
                .ease
                .as_deref()
                .is_some_and(|ease| ease != "hold" && ease != "linear")
            {
                return Err(Error::invalid(format!(
                    r#"{{"error":"bad_remap","slot":"{slot_id}"}}"#
                )));
            }
        }
        if let Some(audio) = &self.audio {
            require_material(&audio.material)?;
            audio
                .source
                .validate(&format!("binding {slot_id}.audio.source"))?;
            if let Some(place) = audio.place {
                place.validate(&format!("binding {slot_id}.audio.place"))?;
            }
        }
        if let Some(serde_json::Value::Object(map)) = &self.params {
            for key in map.keys() {
                if key != "speed" {
                    return Err(Error::invalid(format!(
                        r#"{{"error":"unknown_param","slot":"{slot_id}","param":"{key}"}}"#
                    )));
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BindingLayer {
    pub name: Layer,
    #[serde(default)]
    pub bindings: BTreeMap<String, Binding>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChangeRequest {
    pub feedback: String,
    pub slots: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scion {
    pub graft: String,
    pub id: String,
    pub concept: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub change_request: Option<ChangeRequest>,
    pub dest: Dest,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub graph: Option<CompositionGraph>,
    pub layers: Vec<BindingLayer>,
}

impl Scion {
    pub fn layer(&self, name: Layer) -> Option<&BindingLayer> {
        self.layers.iter().find(|layer| layer.name == name)
    }

    pub fn layer_mut(&mut self, name: Layer) -> Option<&mut BindingLayer> {
        self.layers.iter_mut().find(|layer| layer.name == name)
    }

    pub fn ensure_layer(&mut self, name: Layer) -> &mut BindingLayer {
        if self.layer(name).is_none() {
            self.layers.push(BindingLayer {
                name,
                bindings: BTreeMap::new(),
            });
        }
        self.layer_mut(name).expect("layer was inserted")
    }

    pub fn validate(&self) -> Result<(), Error> {
        require_graft_version(&self.graft)?;
        if self.id.is_empty() {
            return Err(Error::invalid("scion.id is empty"));
        }
        if self.id.contains("9:16") && !self.id.contains("9x16") {
            return Err(Error::invalid("dest axis is 9x16, not a slot named 9:16"));
        }
        require_uuid(&self.concept, "concept")?;
        self.dest.validate()?;
        if let Some(graph) = &self.graph {
            graph.validate()?;
        }
        if self.layers.is_empty() && self.parent.is_none() {
            return Err(Error::invalid("root scion must have at least one layer"));
        }
        if let Some(request) = &self.change_request {
            if request.feedback.is_empty() || request.slots.is_empty() {
                return Err(Error::invalid(
                    "change_request feedback and slots must be non-empty",
                ));
            }
            for slot in &request.slots {
                require_slot_id(slot)?;
            }
        }
        let mut seen = std::collections::BTreeSet::new();
        for layer in &self.layers {
            if !seen.insert(layer.name) {
                return Err(Error::invalid(format!(
                    "duplicate scion layer {}",
                    layer.name
                )));
            }
            for (slot_id, binding) in &layer.bindings {
                require_slot_id(slot_id)?;
                binding.validate(slot_id)?;
            }
        }
        Ok(())
    }

    pub fn validate_against_score(&self, score: &Score) -> Result<(), Error> {
        self.validate()?;
        if self.concept != score.concept {
            return Err(Error::invalid("scion.concept != score.concept"));
        }
        let slot_ids: Vec<&str> = score.slots.iter().map(|s| s.id.as_str()).collect();
        for layer in &self.layers {
            if !score.layers.contains(&layer.name) {
                return Err(Error::invalid(format!(
                    "scion layer {} is not declared by score.layers",
                    layer.name
                )));
            }
            for (key, binding) in &layer.bindings {
                if !slot_ids.iter().any(|id| *id == key) {
                    return Err(Error::invalid(format!("scion binds unknown slot {key}")));
                }
                let slot = score.slot(key).expect("slot id checked");
                binding_duration_compatible(slot, binding, &self.dest.rate)?;
            }
        }
        Ok(())
    }
}

fn binding_duration_compatible(
    slot: &crate::score::Slot,
    binding: &Binding,
    dest_rate: &FrameRate,
) -> Result<(), Error> {
    let speed = binding.speed();
    if speed <= 0.0 || !speed.is_finite() {
        return Err(Error::invalid(format!(
            "binding {} speed must be finite and > 0",
            slot.id
        )));
    }
    let source_s = binding
        .source
        .rate
        .seconds_from_frames(binding.source.range.duration as i64);
    let dest_s = dest_rate.seconds_from_frames(slot.range.duration as i64);
    if binding.remap_is_curve() {
        let consumed = binding.remap_source_seconds(dest_s);
        let tolerance = dest_rate.seconds_from_frames(1);
        if (consumed - source_s).abs() > tolerance {
            return Err(Error::invalid(format!(
                "binding {} remap consumes {consumed:.3}s over dest {dest_s:.3}s but source is {source_s:.3}s",
                slot.id
            )));
        }
        return Ok(());
    }
    let expected = source_s / speed;
    let tolerance = dest_rate.seconds_from_frames(1);
    if (expected - dest_s).abs() > tolerance {
        return Err(Error::invalid(format!(
            "binding {} source duration {:.3}s at speed {speed} does not match slot {:.3}s",
            slot.id, source_s, dest_s
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dest_rejects_colon_ratio() {
        let mut dest = Dest {
            id: "9:16".into(),
            width: 1080,
            height: 1920,
            rate: FrameRate::new(30, 1),
            pix_fmt: "yuv420p".into(),
            color: "bt709".into(),
            working: None,
            encoder: Encoder::default_x264(),
        };
        assert!(dest.validate().is_err());
        dest.id = "9x16".into();
        dest.validate().unwrap();
    }

    #[test]
    fn binding_rejects_unknown_params() {
        let binding = Binding {
            material: format!("blake3:{:064x}", 1),
            source: TimedRange {
                rate: FrameRate::new(30, 1),
                range: crate::FrameRange::new(0, 30),
            },
            params: Some(serde_json::json!({ "crop": "center" })),
            audio: None,
            nested: None,
            x: None,
            y: None,
            opacity: None,
            motion: None,
            remap: Vec::new(),
        };
        let err = binding.validate("hook").unwrap_err().to_string();
        assert!(err.contains("unknown_param"));
        assert!(err.contains("crop"));
    }

    #[test]
    fn stepwise_remap_consumes_source_across_dest() {
        let mut binding = Binding {
            material: format!("blake3:{:064x}", 1),
            source: TimedRange {
                rate: FrameRate::new(30, 1),
                range: crate::FrameRange::new(0, 90),
            },
            params: None,
            audio: None,
            nested: None,
            x: None,
            y: None,
            opacity: None,
            motion: None,
            remap: vec![
                Keyframe {
                    t: 0.0,
                    v: 2.0,
                    ease: None,
                },
                Keyframe {
                    t: 1.0,
                    v: 1.0,
                    ease: None,
                },
            ],
        };
        let spans = binding.remap_spans(2.0);
        assert_eq!(spans.len(), 2);
        assert!((spans[0].src_in - 0.0).abs() < 1e-9);
        assert!((spans[0].src_out - 2.0).abs() < 1e-9);
        assert!((spans[0].speed - 2.0).abs() < 1e-9);
        assert!((spans[1].src_in - 2.0).abs() < 1e-9);
        assert!((spans[1].src_out - 3.0).abs() < 1e-9);
        assert!((binding.remap_source_seconds(2.0) - 3.0).abs() < 1e-9);
        binding.remap.clear();
        assert!(!binding.remap_is_curve());
    }
}
