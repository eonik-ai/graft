// SPDX-License-Identifier: Apache-2.0

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::canonical::require_graft_version;
use crate::ident::{require_slot_id, require_uuid};
use crate::role::{Layer, Role};
use crate::time::{FrameRange, FrameRate};
use crate::Error;

pub const DEFAULT_SPILL_FRAMES: u64 = 11;

fn default_layers() -> Vec<Layer> {
    vec![Layer::Base]
}

fn is_false(v: &bool) -> bool {
    !*v
}

fn default_spill() -> u64 {
    DEFAULT_SPILL_FRAMES
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Clock {
    pub rate: FrameRate,
    pub duration_frames: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Window {
    pub kind: String,
    pub range: FrameRange,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Slot {
    pub id: String,
    pub role: Role,
    pub range: FrameRange,
    #[serde(default, skip_serializing_if = "is_false")]
    pub optional: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window: Option<Window>,
}

impl Slot {
    pub fn start(&self) -> i64 {
        self.range.start
    }

    pub fn end(&self) -> i64 {
        self.range.end()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Join {
    pub left: String,
    pub right: String,
    pub transition: String,
    pub duration_frames: u64,
}

impl Join {
    pub fn cut(left: impl Into<String>, right: impl Into<String>) -> Self {
        Self {
            left: left.into(),
            right: right.into(),
            transition: "cut".into(),
            duration_frames: 0,
        }
    }

    pub fn is_fade(&self) -> bool {
        self.transition == "fade"
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
    pub spill_threshold_frames: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub joins: Vec<Join>,
}

impl Score {
    pub fn validate(&self) -> Result<(), Error> {
        require_graft_version(&self.graft)?;
        require_uuid(&self.concept, "concept")?;
        self.clock.rate.validate("clock.rate")?;
        if self.clock.duration_frames == 0 {
            return Err(Error::invalid("clock.duration_frames must be > 0"));
        }
        if self.slots.is_empty() {
            return Err(Error::invalid("slots must be non-empty"));
        }
        let mut layers = BTreeMap::new();
        for layer in &self.layers {
            if layers.insert(*layer, ()).is_some() {
                return Err(Error::invalid(format!("duplicate score layer {layer}")));
            }
        }
        let mut seen = BTreeMap::new();
        let mut spine = 0usize;
        for slot in &self.slots {
            require_slot_id(&slot.id)?;
            if seen.insert(&slot.id, ()).is_some() {
                return Err(Error::invalid(format!("duplicate slot id {}", slot.id)));
            }
            if slot.role.is_spine() {
                spine += 1;
            }
            slot.range.validate(&format!("slot {}.range", slot.id))?;
            if slot.range.end() > self.clock.duration_frames as i64 {
                return Err(Error::invalid(format!(
                    "slot {} ends after score duration",
                    slot.id
                )));
            }
            if let Some(window) = &slot.window {
                if window.kind.is_empty() {
                    return Err(Error::invalid(format!(
                        "slot {} window.kind is empty",
                        slot.id
                    )));
                }
                window
                    .range
                    .validate(&format!("slot {} window.range", slot.id))?;
            }
        }
        if spine == 0 {
            return Err(Error::invalid(
                "score must declare at least one spine slot (hook, body, proof, or cta)",
            ));
        }
        self.validate_joins()?;
        Ok(())
    }

    fn validate_joins(&self) -> Result<(), Error> {
        let spine: Vec<&str> = self
            .spine()
            .into_iter()
            .map(|slot| slot.id.as_str())
            .collect();
        let mut seen = BTreeMap::new();
        for join in &self.joins {
            require_slot_id(&join.left)?;
            require_slot_id(&join.right)?;
            if seen
                .insert((join.left.as_str(), join.right.as_str()), ())
                .is_some()
            {
                return Err(Error::invalid(format!(
                    "duplicate join {}→{}",
                    join.left, join.right
                )));
            }
            match join.transition.as_str() {
                "cut" => {}
                "fade" => {
                    if join.duration_frames == 0 {
                        return Err(Error::invalid(format!(
                            r#"{{"error":"invalid_fade","left":"{}","right":"{}","reason":"duration_frames must be > 0"}}"#,
                            join.left, join.right
                        )));
                    }
                }
                other => {
                    return Err(Error::invalid(format!(
                        r#"{{"error":"unknown_transition","transition":"{other}","left":"{}","right":"{}"}}"#,
                        join.left, join.right
                    )));
                }
            }
            let Some(left_i) = spine.iter().position(|id| *id == join.left) else {
                return Err(Error::invalid(format!(
                    "join left {} is not a spine slot",
                    join.left
                )));
            };
            if spine.get(left_i + 1).copied() != Some(join.right.as_str()) {
                return Err(Error::invalid(format!(
                    "join {}→{} is not an adjacent spine pair",
                    join.left, join.right
                )));
            }
            if join.is_fade() {
                let left = self.slot(&join.left).expect("join left checked");
                let right = self.slot(&join.right).expect("join right checked");
                if join.duration_frames >= left.range.duration
                    || join.duration_frames >= right.range.duration
                {
                    return Err(Error::invalid(format!(
                        r#"{{"error":"invalid_fade","left":"{}","right":"{}","reason":"duration_frames must be shorter than both slots"}}"#,
                        join.left, join.right
                    )));
                }
            }
        }
        Ok(())
    }

    pub fn join_between(&self, left: &str, right: &str) -> Join {
        self.joins
            .iter()
            .find(|join| join.left == left && join.right == right)
            .cloned()
            .unwrap_or_else(|| Join::cut(left, right))
    }

    pub fn slot(&self, id: &str) -> Option<&Slot> {
        self.slots.iter().find(|s| s.id == id)
    }

    pub fn slot_mut(&mut self, id: &str) -> Option<&mut Slot> {
        self.slots.iter_mut().find(|s| s.id == id)
    }

    pub fn spine(&self) -> Vec<&Slot> {
        let mut slots: Vec<&Slot> = self.slots.iter().filter(|s| s.role.is_spine()).collect();
        slots.sort_by(|a, b| a.start().cmp(&b.start()).then_with(|| a.id.cmp(&b.id)));
        slots
    }

    pub fn recompute_duration(&mut self) {
        let max_end = self.slots.iter().map(Slot::end).max().unwrap_or(0);
        if max_end > 0 {
            self.clock.duration_frames = max_end as u64;
        }
    }
}
