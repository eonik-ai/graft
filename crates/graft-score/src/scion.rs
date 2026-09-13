// SPDX-License-Identifier: Apache-2.0

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::canonical::require_graft_version;
use crate::ident::{require_slot_id, require_uuid};
use crate::material::require_material;
use crate::score::Score;
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
    pub fps: f64,
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
        if self.fps <= 0.0 || !self.fps.is_finite() {
            return Err(Error::invalid("dest.fps must be > 0"));
        }
        if self.pix_fmt.is_empty() || self.color.is_empty() {
            return Err(Error::invalid("dest pix_fmt and color are required"));
        }
        self.encoder.validate()
    }

    pub fn fingerprint_value(&self) -> Value {
        serde_json::json!({
            "width": self.width,
            "height": self.height,
            "fps": self.fps,
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
    pub in_s: f64,
    pub out_s: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl Binding {
    pub fn validate(&self, slot_id: &str) -> Result<(), Error> {
        require_material(&self.material)?;
        if self.in_s < 0.0 || self.out_s < 0.0 {
            return Err(Error::invalid(format!(
                "binding {slot_id}: in_s/out_s must be >= 0"
            )));
        }
        if self.out_s <= self.in_s {
            return Err(Error::invalid(format!(
                "binding {slot_id}: out_s must be greater than in_s"
            )));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scion {
    pub graft: String,
    pub id: String,
    pub concept: String,
    pub dest: Dest,
    pub bindings: BTreeMap<String, Binding>,
}

impl Scion {
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
        for (slot_id, binding) in &self.bindings {
            require_slot_id(slot_id)?;
            binding.validate(slot_id)?;
        }
        Ok(())
    }

    pub fn validate_against_score(&self, score: &Score) -> Result<(), Error> {
        self.validate()?;
        if self.concept != score.concept {
            return Err(Error::invalid("scion.concept != score.concept"));
        }
        let slot_ids: Vec<&str> = score.slots.iter().map(|s| s.id.as_str()).collect();
        for key in self.bindings.keys() {
            if !slot_ids.iter().any(|id| *id == key) {
                return Err(Error::invalid(format!("scion binds unknown slot {key}")));
            }
        }
        for slot in &score.slots {
            if !slot.optional && !self.bindings.contains_key(&slot.id) {
                return Err(Error::invalid(format!(
                    "required slot {} is unbound",
                    slot.id
                )));
            }
        }
        Ok(())
    }
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
            fps: 30.0,
            pix_fmt: "yuv420p".into(),
            color: "bt709".into(),
            encoder: Encoder::default_x264(),
        };
        assert!(dest.validate().is_err());
        dest.id = "9x16".into();
        dest.validate().unwrap();
    }
}
