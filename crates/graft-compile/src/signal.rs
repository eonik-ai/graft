// SPDX-License-Identifier: Apache-2.0

use graft_score::{FrameRange, Role, Score, TimeMap};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Signal {
    pub kind: String,
    pub range: FrameRange,
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

pub fn dirty_from_signal(
    time_map: &TimeMap,
    score: &Score,
    kind: &str,
    t0_s: f64,
    t1_s: f64,
    dest_id: Option<String>,
) -> DirtySet {
    let range =
        FrameRange::from_seconds(time_map.rate, t0_s, t1_s).expect("validated signal seconds");
    dirty_from_signal_range(time_map, score, kind, range, dest_id)
}

pub fn declared_signal_range(score: &Score, kind: &str) -> Option<FrameRange> {
    score
        .slots
        .iter()
        .filter_map(|slot| slot.window.as_ref())
        .find(|window| window.kind == kind)
        .map(|window| window.range)
}

pub fn dirty_from_signal_range(
    time_map: &TimeMap,
    score: &Score,
    kind: &str,
    requested: FrameRange,
    dest_id: Option<String>,
) -> DirtySet {
    let range = declared_signal_range(score, kind).unwrap_or(requested);
    let mut overlaps = std::collections::BTreeMap::<String, u64>::new();
    let mut dirty = std::collections::BTreeSet::<String>::new();
    for entry in &time_map.entries {
        let overlap = entry.dest.overlap_frames(range);
        if overlap > 0 {
            dirty.insert(entry.slot.clone());
            overlaps.insert(entry.slot.clone(), overlap);
        }
    }

    let mut warnings = Vec::new();
    if kind == "hook_rate" {
        for hook in score.slots.iter().filter(|slot| slot.role == Role::Hook) {
            dirty.insert(hook.id.clone());
        }
        for slot_id in dirty.iter().cloned().collect::<Vec<_>>() {
            let Some(slot) = score.slot(&slot_id) else {
                continue;
            };
            if slot.role == Role::Hook {
                continue;
            }
            let overlap = overlaps.get(&slot_id).copied().unwrap_or(0);
            if overlap < score.spill_threshold_frames {
                dirty.remove(&slot_id);
            } else if overlap > 0 {
                warnings.push(format!(
                    "{slot_id} overlaps hook_rate window by {:.3}s \
                     (>= spill_threshold_frames {}); pad hook to the declared window \
                     or accept a dirty body",
                    time_map.rate.seconds_from_frames(overlap as i64),
                    score.spill_threshold_frames
                ));
            }
        }
    }

    let spine = score.spine();
    let kerfs = spine
        .windows(2)
        .filter(|pair| dirty.contains(&pair[0].id) || dirty.contains(&pair[1].id))
        .map(|pair| vec![pair[0].id.clone(), pair[1].id.clone()])
        .collect();
    let clean = spine
        .iter()
        .filter(|slot| !dirty.contains(&slot.id))
        .map(|slot| slot.id.clone())
        .collect();
    let mut slots: Vec<String> = spine
        .iter()
        .filter(|slot| dirty.contains(&slot.id))
        .map(|slot| slot.id.clone())
        .collect();
    let mut extra: Vec<String> = dirty
        .iter()
        .filter(|id| !slots.contains(id))
        .cloned()
        .collect();
    extra.sort();
    slots.extend(extra);
    DirtySet {
        graft: graft_score::GRAFT_SCHEMA.into(),
        signal: Signal {
            kind: kind.into(),
            range,
            dest_id,
        },
        slots,
        kerfs,
        clean,
        warnings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use graft_score::{load_score, load_time_map};

    fn example_dir() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/hook-v3-body-v1-9x16")
    }

    #[test]
    fn declared_hook_window_does_not_dirty_body() {
        let dir = example_dir();
        let score = load_score(&dir.join("score.json")).unwrap();
        let time_map = load_time_map(&dir.join("time-map.json")).unwrap();
        let got = dirty_from_signal_range(
            &time_map,
            &score,
            "hook_rate",
            FrameRange::new(0, 1),
            Some("9x16".into()),
        );
        assert_eq!(got.signal.range, FrameRange::new(0, 90));
        assert_eq!(got.slots, vec!["hook"]);
        assert_eq!(got.kerfs, vec![vec!["hook", "body"]]);
        assert_eq!(got.clean, vec!["body", "cta"]);
    }

    #[test]
    fn body_signal_keeps_hook_clean() {
        let dir = example_dir();
        let score = load_score(&dir.join("score.json")).unwrap();
        let time_map = load_time_map(&dir.join("time-map.json")).unwrap();
        let got = dirty_from_signal_range(
            &time_map,
            &score,
            "hold",
            FrameRange::new(240, 120),
            Some("9x16".into()),
        );
        assert_eq!(got.slots, vec!["body"]);
        assert_eq!(got.clean, vec!["hook", "cta"]);
    }
}
