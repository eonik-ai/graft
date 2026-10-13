// SPDX-License-Identifier: Apache-2.0

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::canonical::require_graft_version;
use crate::graph::{join_is_overlap, validate_join_transition};
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
        join_is_overlap(&self.transition)
    }

    pub fn is_cut(&self) -> bool {
        matches!(
            crate::graph::canonical_transition(&self.transition).as_deref(),
            Ok("cut")
        ) || self.transition == "cut"
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Marker {
    pub id: String,
    pub range: FrameRange,
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub markers: Vec<Marker>,
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
        for marker in &self.markers {
            if marker.id.is_empty() {
                return Err(Error::invalid("marker id is empty"));
            }
            marker
                .range
                .validate(&format!("marker {}.range", marker.id))?;
        }
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
            if validate_join_transition(&join.transition).is_err() {
                return Err(Error::invalid(format!(
                    r#"{{"error":"unknown_transition","transition":"{}","left":"{}","right":"{}"}}"#,
                    join.transition, join.left, join.right
                )));
            }
            if !join.is_cut() && join.duration_frames == 0 {
                return Err(Error::invalid(format!(
                    r#"{{"error":"invalid_fade","left":"{}","right":"{}","reason":"duration_frames must be > 0"}}"#,
                    join.left, join.right
                )));
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

    /// Overlap frames of xfade joins whose score-clock join point is at or
    /// before `score_frame`. Cut joins contribute 0.
    pub fn overlap_before(&self, score_frame: i64) -> i64 {
        let spine = self.spine();
        let mut n = 0i64;
        for pair in spine.windows(2) {
            let join = self.join_between(&pair[0].id, &pair[1].id);
            if join.is_fade() && pair[1].range.start <= score_frame {
                n += join.duration_frames as i64;
            }
        }
        n
    }

    pub fn compiled_frame(&self, score_frame: i64) -> i64 {
        (score_frame - self.overlap_before(score_frame)).max(0)
    }

    /// Dest range after each overlap shortens the film. Spine slots keep
    /// their duration; later starts shift left. Overlay ranges remap both
    /// ends so a full-clock span still covers the compiled dest.
    pub fn compiled_range(&self, range: FrameRange) -> FrameRange {
        let start = self.compiled_frame(range.start);
        let last = (range.end() - 1).max(range.start);
        let end = self.compiled_frame(last) + 1;
        FrameRange::new(start, (end - start).max(1) as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GRAFT_SCHEMA;

    fn score_with_fade() -> Score {
        Score {
            graft: GRAFT_SCHEMA.into(),
            concept: "11111111-1111-4111-8111-111111111111".into(),
            clock: Clock {
                rate: FrameRate::new(10, 1),
                duration_frames: 40,
            },
            slots: vec![
                Slot {
                    id: "hook".into(),
                    role: Role::Hook,
                    range: FrameRange::new(0, 10),
                    optional: false,
                    window: None,
                },
                Slot {
                    id: "body".into(),
                    role: Role::Body,
                    range: FrameRange::new(10, 20),
                    optional: false,
                    window: None,
                },
                Slot {
                    id: "cta".into(),
                    role: Role::Cta,
                    range: FrameRange::new(30, 10),
                    optional: false,
                    window: None,
                },
            ],
            layers: vec![Layer::Base],
            dest_default: None,
            spill_threshold_frames: DEFAULT_SPILL_FRAMES,
            joins: vec![Join {
                left: "hook".into(),
                right: "body".into(),
                transition: "fade".into(),
                duration_frames: 2,
            }],
            markers: Vec::new(),
        }
    }

    #[test]
    fn compiled_range_shortens_after_xfade() {
        let score = score_with_fade();
        assert_eq!(
            score.compiled_range(FrameRange::new(0, 10)),
            FrameRange::new(0, 10)
        );
        assert_eq!(
            score.compiled_range(FrameRange::new(10, 20)),
            FrameRange::new(8, 20)
        );
        assert_eq!(
            score.compiled_range(FrameRange::new(30, 10)),
            FrameRange::new(28, 10)
        );
        assert_eq!(
            score.compiled_range(FrameRange::new(0, 40)),
            FrameRange::new(0, 38)
        );
    }

    #[test]
    fn compiled_range_is_identity_for_cuts() {
        let mut score = score_with_fade();
        score.joins.clear();
        assert_eq!(
            score.compiled_range(FrameRange::new(10, 20)),
            FrameRange::new(10, 20)
        );
        assert_eq!(
            score.compiled_range(FrameRange::new(0, 40)),
            FrameRange::new(0, 40)
        );
    }
}
