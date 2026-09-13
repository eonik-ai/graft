// SPDX-License-Identifier: Apache-2.0

use graft_score::{Role, Score, TimeMap};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Signal {
    pub kind: String,
    pub t0_s: f64,
    pub t1_s: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dest_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DirtySet {
    pub graft: String,
    pub signal: Signal,
    pub slots: Vec<String>,
    pub kerfs: Vec<Vec<String>>,
    pub clean: Vec<String>,
    pub warnings: Vec<String>,
}

pub fn overlap_s(a0: f64, a1: f64, b0: f64, b1: f64) -> f64 {
    let lo = a0.max(b0);
    let hi = a1.min(b1);
    (hi - lo).max(0.0)
}

/// Signal → dirty set. Must match `ref/graft_ref/signal.py`.
pub fn dirty_from_signal(
    time_map: &TimeMap,
    score: &Score,
    kind: &str,
    t0_s: f64,
    t1_s: f64,
    dest_id: Option<String>,
) -> DirtySet {
    let spill_threshold_s = score.spill_threshold_s;
    let mut overlaps = std::collections::BTreeMap::<String, f64>::new();
    let mut dirty = std::collections::BTreeSet::<String>::new();

    for entry in &time_map.entries {
        let ov = overlap_s(entry.span[0], entry.span[1], t0_s, t1_s);
        if ov > 0.0 {
            dirty.insert(entry.slot.clone());
            overlaps.insert(entry.slot.clone(), ov);
        }
    }

    let mut warnings = Vec::new();
    if kind == "hook_rate" {
        for slot in &score.slots {
            if slot.role == Role::Hook {
                dirty.insert(slot.id.clone());
            }
        }
        let to_check: Vec<String> = dirty.iter().cloned().collect();
        for slot_id in to_check {
            let Some(slot) = score.slot(&slot_id) else {
                continue;
            };
            if slot.role == Role::Hook {
                continue;
            }
            let ov = overlaps.get(&slot_id).copied().unwrap_or(0.0);
            if ov < spill_threshold_s {
                dirty.remove(&slot_id);
            } else if ov > 0.0 {
                warnings.push(format!(
                    "{slot_id} overlaps hook_rate window by {ov:.3}s \
                     (>= spill_threshold_s {spill_threshold_s}); \
                     pad hook to the declared window or accept a dirty body"
                ));
            }
        }
    }

    let spine = score.spine();
    let mut kerfs = Vec::new();
    for pair in spine.windows(2) {
        let left = pair[0];
        let right = pair[1];
        if dirty.contains(&left.id) || dirty.contains(&right.id) {
            kerfs.push(vec![left.id.clone(), right.id.clone()]);
        }
    }

    let clean: Vec<String> = spine
        .iter()
        .filter(|s| !dirty.contains(&s.id))
        .map(|s| s.id.clone())
        .collect();
    let mut ordered_dirty: Vec<String> = spine
        .iter()
        .filter(|s| dirty.contains(&s.id))
        .map(|s| s.id.clone())
        .collect();
    let mut extra: Vec<String> = dirty
        .iter()
        .filter(|id| !ordered_dirty.contains(id))
        .cloned()
        .collect();
    extra.sort();
    ordered_dirty.extend(extra);

    DirtySet {
        graft: graft_score::GRAFT_SCHEMA.into(),
        signal: Signal {
            kind: kind.into(),
            t0_s,
            t1_s,
            dest_id,
        },
        slots: ordered_dirty,
        kerfs,
        clean,
        warnings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use graft_score::{
        load_score, load_time_map, Clock, Layer, Slot, DEFAULT_SPILL_S, GRAFT_SCHEMA,
    };

    fn example_dir() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/hook-v3-body-v1-9x16")
    }

    fn two_slot_score(hook_end: f64) -> Score {
        Score {
            graft: GRAFT_SCHEMA.into(),
            concept: "6f8c2e14-9a71-4d3b-b0e6-1c4d8a7f92ab".into(),
            clock: Clock {
                fps: 30.0,
                duration_s: 20.0,
            },
            slots: vec![
                Slot {
                    id: "hook".into(),
                    role: Role::Hook,
                    span: [0.0, hook_end],
                    optional: false,
                    window: None,
                },
                Slot {
                    id: "body".into(),
                    role: Role::Body,
                    span: [hook_end, 20.0],
                    optional: false,
                    window: None,
                },
            ],
            layers: vec![Layer::Base],
            dest_default: None,
            spill_threshold_s: DEFAULT_SPILL_S,
        }
    }

    fn identity_map(score: &Score) -> TimeMap {
        TimeMap::from_score(score, "test", "9x16")
    }

    #[test]
    fn hook_rate_does_not_dirty_body() {
        let dir = example_dir();
        let score = load_score(&dir.join("score.json")).unwrap();
        let time_map = load_time_map(&dir.join("time-map.json")).unwrap();
        let expected: serde_json::Value = graft_score::load_json(&dir.join("dirty.json")).unwrap();
        let got = dirty_from_signal(
            &time_map,
            &score,
            "hook_rate",
            0.0,
            3.0,
            Some("9x16".into()),
        );
        assert_eq!(got.slots, vec!["hook".to_string()]);
        assert_eq!(
            got.kerfs,
            vec![vec!["hook".to_string(), "body".to_string()]]
        );
        assert_eq!(got.clean, vec!["body".to_string(), "cta".to_string()]);
        assert!(got.warnings.is_empty());
        assert_eq!(serde_json::to_value(&got.slots).unwrap(), expected["slots"]);
        assert_eq!(serde_json::to_value(&got.kerfs).unwrap(), expected["kerfs"]);
        assert_eq!(serde_json::to_value(&got.clean).unwrap(), expected["clean"]);
        assert!(!got.slots.iter().any(|s| s == "body"));
    }

    #[test]
    fn tiny_bleed_does_not_dirty_body() {
        let score = two_slot_score(2.8);
        let time_map = identity_map(&score);
        let got = dirty_from_signal(&time_map, &score, "hook_rate", 0.0, 3.0, None);
        assert_eq!(got.slots, vec!["hook".to_string()]);
        assert_eq!(
            got.kerfs,
            vec![vec!["hook".to_string(), "body".to_string()]]
        );
        assert!(got.warnings.is_empty());
    }

    #[test]
    fn large_bleed_dirties_body_and_warns() {
        let score = two_slot_score(2.4);
        let time_map = identity_map(&score);
        let got = dirty_from_signal(&time_map, &score, "hook_rate", 0.0, 3.0, None);
        assert_eq!(got.slots, vec!["hook".to_string(), "body".to_string()]);
        assert_eq!(
            got.kerfs,
            vec![vec!["hook".to_string(), "body".to_string()]]
        );
        assert_eq!(got.warnings.len(), 1);
    }

    #[test]
    fn too_slow_on_body_leaves_hook_clean() {
        let score = Score {
            graft: GRAFT_SCHEMA.into(),
            concept: "6f8c2e14-9a71-4d3b-b0e6-1c4d8a7f92ab".into(),
            clock: Clock {
                fps: 30.0,
                duration_s: 23.0,
            },
            slots: vec![
                Slot {
                    id: "hook".into(),
                    role: Role::Hook,
                    span: [0.0, 3.0],
                    optional: false,
                    window: None,
                },
                Slot {
                    id: "body".into(),
                    role: Role::Body,
                    span: [3.0, 20.0],
                    optional: false,
                    window: None,
                },
                Slot {
                    id: "cta".into(),
                    role: Role::Cta,
                    span: [20.0, 23.0],
                    optional: false,
                    window: None,
                },
            ],
            layers: vec![Layer::Base],
            dest_default: None,
            spill_threshold_s: DEFAULT_SPILL_S,
        };
        let time_map = identity_map(&score);
        let got = dirty_from_signal(&time_map, &score, "hold", 8.0, 12.0, None);
        assert_eq!(got.slots, vec!["body".to_string()]);
        assert_eq!(
            got.kerfs,
            vec![
                vec!["hook".to_string(), "body".to_string()],
                vec!["body".to_string(), "cta".to_string()]
            ]
        );
        assert_eq!(got.clean, vec!["hook".to_string(), "cta".to_string()]);
    }
}
