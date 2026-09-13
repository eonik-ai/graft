// SPDX-License-Identifier: Apache-2.0

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::canonical::require_graft_version;
use crate::ident::{require_slot_id, require_uuid, validate_span};
use crate::role::{Layer, Role};
use crate::Error;

pub const DEFAULT_SPILL_S: f64 = 0.35;

fn default_layers() -> Vec<Layer> {
    vec![Layer::Base]
}

fn is_false(v: &bool) -> bool {
    !*v
}

fn default_spill() -> f64 {
    DEFAULT_SPILL_S
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Clock {
    pub fps: f64,
    pub duration_s: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Window {
    pub kind: String,
    pub span: [f64; 2],
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Slot {
    pub id: String,
    pub role: Role,
    pub span: [f64; 2],
    #[serde(default, skip_serializing_if = "is_false")]
    pub optional: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window: Option<Window>,
}

impl Slot {
    pub fn start(&self) -> f64 {
        self.span[0]
    }

    pub fn end(&self) -> f64 {
        self.span[1]
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Score {
    pub graft: String,
    pub concept: String,
    pub clock: Clock,
    pub slots: Vec<Slot>,
    #[serde(default = "default_layers")]
    pub layers: Vec<Layer>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dest_default: Option<String>,
    #[serde(default = "default_spill")]
    pub spill_threshold_s: f64,
}

impl Score {
    pub fn validate(&self) -> Result<(), Error> {
        require_graft_version(&self.graft)?;
        require_uuid(&self.concept, "concept")?;
        if self.clock.fps <= 0.0 || !self.clock.fps.is_finite() {
            return Err(Error::invalid("clock.fps must be > 0"));
        }
        if self.clock.duration_s <= 0.0 || !self.clock.duration_s.is_finite() {
            return Err(Error::invalid("clock.duration_s must be > 0"));
        }
        if self.slots.is_empty() {
            return Err(Error::invalid("slots must be non-empty"));
        }
        if self.spill_threshold_s < 0.0 {
            return Err(Error::invalid("spill_threshold_s must be >= 0"));
        }
        let mut seen = BTreeMap::new();
        for slot in &self.slots {
            require_slot_id(&slot.id)?;
            if seen.insert(&slot.id, ()).is_some() {
                return Err(Error::invalid(format!("duplicate slot id {}", slot.id)));
            }
            validate_span(slot.span, &format!("slot {}", slot.id))?;
            if let Some(window) = &slot.window {
                if window.kind.is_empty() {
                    return Err(Error::invalid(format!(
                        "slot {} window.kind is empty",
                        slot.id
                    )));
                }
                validate_span(window.span, &format!("slot {} window", slot.id))?;
            }
        }
        Ok(())
    }

    pub fn slot(&self, id: &str) -> Option<&Slot> {
        self.slots.iter().find(|s| s.id == id)
    }

    pub fn slot_mut(&mut self, id: &str) -> Option<&mut Slot> {
        self.slots.iter_mut().find(|s| s.id == id)
    }

    pub fn spine(&self) -> Vec<&Slot> {
        let mut slots: Vec<&Slot> = self.slots.iter().filter(|s| s.role.is_spine()).collect();
        slots.sort_by(|a, b| {
            a.start()
                .partial_cmp(&b.start())
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.id.cmp(&b.id))
        });
        slots
    }

    pub fn recompute_duration(&mut self) {
        let max_end = self.slots.iter().map(Slot::end).fold(0.0_f64, f64::max);
        if max_end > 0.0 {
            self.clock.duration_s = max_end;
        }
    }
}
