// SPDX-License-Identifier: Apache-2.0

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::canonical::require_graft_version;
use crate::ident::{require_slot_id, require_uuid};
use crate::material::require_material;
use crate::role::Layer;
use crate::score::Score;
use crate::time::{FrameRate, TimedRange};
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
        self.encoder.validate()
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
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioBinding {
    pub material: String,
    pub source: TimedRange,
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

    pub fn validate(&self, slot_id: &str) -> Result<(), Error> {
        require_material(&self.material)?;
        self.source.validate(&format!("binding {slot_id}.source"))?;
        if let Some(audio) = &self.audio {
            require_material(&audio.material)?;
            audio
                .source
                .validate(&format!("binding {slot_id}.audio.source"))?;
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
        };
        let err = binding.validate("hook").unwrap_err().to_string();
        assert!(err.contains("unknown_param"));
        assert!(err.contains("crop"));
    }
}
