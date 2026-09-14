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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mixes: Vec<String>,
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
    let mut mixes = Vec::new();
    for id in &slots {
        if let Some(slot) = score.slot(id) {
            let mix = match slot.role {
                Role::Vo | Role::Bed => Some("audio_mix"),
                Role::Brand => Some("overlay_mix"),
                _ => None,
            };
            if let Some(name) = mix {
                if !mixes.iter().any(|m| m == name) {
                    mixes.push(name.into());
                }
            }
        }
    }
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
        mixes,
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

    #[test]
    fn vo_window_dirties_vo_and_audio_mix_not_body() {
        let dir = example_dir();
        let mut score = load_score(&dir.join("score.json")).unwrap();
        score.slots.push(graft_score::Slot {
            id: "vo".into(),
            role: Role::Vo,
            range: FrameRange::new(0, 90),
            optional: false,
            window: Some(graft_score::Window {
                kind: "vo_hold".into(),
                range: FrameRange::new(0, 90),
            }),
        });
        let mut time_map = load_time_map(&dir.join("time-map.json")).unwrap();
        time_map.entries.push(graft_score::TimeMapEntry {
            dest: FrameRange::new(0, 90),
            source: graft_score::TimedRange {
                rate: time_map.rate,
                range: FrameRange::new(0, 90),
            },
            slot: "vo".into(),
        });
        let got = dirty_from_signal_range(
            &time_map,
            &score,
            "vo_hold",
            FrameRange::new(0, 30),
            Some("9x16".into()),
        );
        assert!(got.slots.contains(&"vo".to_string()));
        assert_eq!(got.mixes, vec!["audio_mix"]);
        assert!(!got.slots.contains(&"body".to_string()));
        assert!(got.clean.contains(&"body".to_string()));
    }

    fn dub_example_dir() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/dub-en-9x16")
    }

    #[test]
    fn dub_vo_hold_dirties_vo_and_audio_mix_not_body() {
        let dir = dub_example_dir();
        let score = load_score(&dir.join("score.json")).unwrap();
        let time_map = load_time_map(&dir.join("time-map.json")).unwrap();
        let got = dirty_from_signal_range(
            &time_map,
            &score,
            "vo_hold",
            FrameRange::new(0, 30),
            Some("9x16".into()),
        );
        assert_eq!(got.signal.range, FrameRange::new(0, 90));
        assert!(got.slots.contains(&"vo".to_string()));
        assert_eq!(got.mixes, vec!["audio_mix"]);
        assert!(!got.slots.contains(&"body".to_string()));
        assert!(got.clean.contains(&"body".to_string()));
        assert_eq!(
            got.slots,
            vec!["hook".to_string(), "captions".to_string(), "vo".to_string()]
        );
    }

    #[test]
    fn explicit_dest_range_3_to_5s_names_overlapping_body_and_vo() {
        let dir = dub_example_dir();
        let score = load_score(&dir.join("score.json")).unwrap();
        let time_map = load_time_map(&dir.join("time-map.json")).unwrap();
        let got = dirty_from_signal_range(
            &time_map,
            &score,
            "note",
            FrameRange::new(90, 60),
            Some("9x16".into()),
        );
        assert_eq!(got.signal.range, FrameRange::new(90, 60));
        assert!(got.slots.contains(&"body".to_string()));
        assert!(got.slots.contains(&"vo".to_string()));
        assert_eq!(got.mixes, vec!["audio_mix"]);
        assert_eq!(got.clean, vec!["hook".to_string(), "cta".to_string()]);
    }

    #[test]
    fn hold_on_body_uses_declared_window_and_does_not_invent_speed() {
        let dir = dub_example_dir();
        let score = load_score(&dir.join("score.json")).unwrap();
        let time_map = load_time_map(&dir.join("time-map.json")).unwrap();
        let got = dirty_from_signal_range(
            &time_map,
            &score,
            "hold",
            FrameRange::new(90, 30),
            Some("9x16".into()),
        );
        assert_eq!(got.signal.range, FrameRange::new(90, 120));
        assert!(got.slots.contains(&"body".to_string()));
        let encoded = serde_json::to_string(&got).unwrap();
        assert!(!encoded.contains("speed"));
        assert_eq!(got.mixes, vec!["audio_mix"]);
    }

    fn load_dirty(name: &str) -> DirtySet {
        let path = dub_example_dir().join(name);
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    fn assert_dirty_matches(got: &DirtySet, expected: &DirtySet) {
        assert_eq!(got.signal.kind, expected.signal.kind);
        assert_eq!(got.signal.range, expected.signal.range);
        assert_eq!(got.slots, expected.slots);
        assert_eq!(got.kerfs, expected.kerfs);
        assert_eq!(got.clean, expected.clean);
        assert_eq!(got.mixes, expected.mixes);
    }

    #[test]
    fn dub_note_permutation_fixtures() {
        let dir = dub_example_dir();
        let score = load_score(&dir.join("score.json")).unwrap();
        let time_map = load_time_map(&dir.join("time-map.json")).unwrap();
        let cases = [
            ("note", FrameRange::new(0, 90), "dirty-note-0-3.json"),
            ("note", FrameRange::new(90, 60), "dirty-note.json"),
            ("note", FrameRange::new(150, 30), "dirty-note-5-6.json"),
            ("note", FrameRange::new(210, 90), "dirty-note-7-10.json"),
            (
                "note",
                FrameRange::new(75, 30),
                "dirty-note-join-hook-body.json",
            ),
            (
                "note",
                FrameRange::new(207, 6),
                "dirty-note-join-body-cta.json",
            ),
            ("note", FrameRange::new(0, 300), "dirty-note-full.json"),
            ("note", FrameRange::new(360, 30), "dirty-note-past.json"),
            ("note", FrameRange::new(89, 1), "dirty-note-frame-89.json"),
            ("note", FrameRange::new(90, 1), "dirty-note-frame-90.json"),
        ];
        for (kind, requested, file) in cases {
            let expected = load_dirty(file);
            let got =
                dirty_from_signal_range(&time_map, &score, kind, requested, Some("9x16".into()));
            assert_eq!(got.signal.range, requested, "{file}");
            assert_dirty_matches(&got, &expected);
            assert!(!got.slots.iter().any(|s| s == "bed"), "{file}");
            assert!(!serde_json::to_string(&got).unwrap().contains("speed"));
        }
    }

    #[test]
    fn dub_declared_windows_ignore_every_requested_range() {
        let dir = dub_example_dir();
        let score = load_score(&dir.join("score.json")).unwrap();
        let time_map = load_time_map(&dir.join("time-map.json")).unwrap();
        let vo_hold = load_dirty("dirty.json");
        let hold = load_dirty("dirty-hold.json");
        let hook_rate = load_dirty("dirty-hook-rate.json");
        for start in 0..300 {
            let requested = FrameRange::new(start, 1);
            let got_vo = dirty_from_signal_range(
                &time_map,
                &score,
                "vo_hold",
                requested,
                Some("9x16".into()),
            );
            assert_dirty_matches(&got_vo, &vo_hold);
            assert!(!got_vo.slots.contains(&"body".to_string()));
            let got_hold =
                dirty_from_signal_range(&time_map, &score, "hold", requested, Some("9x16".into()));
            assert_dirty_matches(&got_hold, &hold);
            assert!(got_hold.slots.contains(&"body".to_string()));
            let got_hook = dirty_from_signal_range(
                &time_map,
                &score,
                "hook_rate",
                requested,
                Some("9x16".into()),
            );
            assert_eq!(got_hook.slots, hook_rate.slots);
            assert_eq!(got_hook.kerfs, hook_rate.kerfs);
            assert_eq!(got_hook.clean, hook_rate.clean);
            assert_eq!(got_hook.mixes, hook_rate.mixes);
            assert_eq!(got_hook.signal.range, hook_rate.signal.range);
            assert!(!got_hook.slots.contains(&"body".to_string()));
        }
    }

    #[test]
    fn dub_note_every_dest_frame_matches_time_map_overlap() {
        let dir = dub_example_dir();
        let score = load_score(&dir.join("score.json")).unwrap();
        let time_map = load_time_map(&dir.join("time-map.json")).unwrap();
        for start in 0..300 {
            let requested = FrameRange::new(start, 1);
            let got =
                dirty_from_signal_range(&time_map, &score, "note", requested, Some("9x16".into()));
            let overlap: std::collections::BTreeSet<String> = time_map
                .entries
                .iter()
                .filter(|entry| entry.dest.overlap_frames(requested) > 0)
                .map(|entry| entry.slot.clone())
                .collect();
            assert_eq!(
                got.slots
                    .iter()
                    .cloned()
                    .collect::<std::collections::BTreeSet<_>>(),
                overlap,
                "frame {start}"
            );
            assert_eq!(got.signal.range, requested);
            assert!(!got.slots.iter().any(|s| s == "bed"));
            if overlap.contains("vo") {
                assert_eq!(got.mixes, vec!["audio_mix".to_string()]);
            } else {
                assert!(got.mixes.is_empty());
            }
        }
    }

    #[test]
    fn picture_time_map_without_vo_never_names_vo() {
        let dir = dub_example_dir();
        let score = load_score(&dir.join("score.json")).unwrap();
        let mut time_map = load_time_map(&dir.join("time-map.json")).unwrap();
        time_map.entries.retain(|entry| entry.slot != "vo");
        let vo_hold = dirty_from_signal_range(
            &time_map,
            &score,
            "vo_hold",
            FrameRange::new(0, 1),
            Some("9x16".into()),
        );
        assert_eq!(
            vo_hold.slots,
            vec!["hook".to_string(), "captions".to_string()]
        );
        assert!(vo_hold.mixes.is_empty());
        assert!(!vo_hold.slots.contains(&"vo".to_string()));
        assert!(!vo_hold.slots.contains(&"body".to_string()));
        let note = dirty_from_signal_range(
            &time_map,
            &score,
            "note",
            FrameRange::new(90, 60),
            Some("9x16".into()),
        );
        assert_eq!(note.slots, vec!["body".to_string(), "captions".to_string()]);
        assert!(note.mixes.is_empty());
        let hold = dirty_from_signal_range(
            &time_map,
            &score,
            "hold",
            FrameRange::new(0, 1),
            Some("9x16".into()),
        );
        assert_eq!(hold.slots, vec!["body".to_string(), "captions".to_string()]);
        assert!(hold.slots.contains(&"body".to_string()));
        for start in 0..300 {
            let requested = FrameRange::new(start, 1);
            let note =
                dirty_from_signal_range(&time_map, &score, "note", requested, Some("9x16".into()));
            assert!(!note.slots.contains(&"vo".to_string()), "note {start}");
            assert!(!note.slots.contains(&"bed".to_string()), "note {start}");
            assert!(note.mixes.is_empty(), "note {start}");
            let got_vo = dirty_from_signal_range(
                &time_map,
                &score,
                "vo_hold",
                requested,
                Some("9x16".into()),
            );
            assert_eq!(got_vo.slots, vo_hold.slots);
            assert!(got_vo.mixes.is_empty());
            let got_hold =
                dirty_from_signal_range(&time_map, &score, "hold", requested, Some("9x16".into()));
            assert_eq!(got_hold.slots, hold.slots);
            let got_hook = dirty_from_signal_range(
                &time_map,
                &score,
                "hook_rate",
                requested,
                Some("9x16".into()),
            );
            assert_eq!(
                got_hook.slots,
                vec!["hook".to_string(), "captions".to_string()]
            );
            assert!(!got_hook.slots.contains(&"body".to_string()));
        }
    }

    #[test]
    fn join_frames_are_exclusive_at_90_and_210() {
        let dir = dub_example_dir();
        let score = load_score(&dir.join("score.json")).unwrap();
        let time_map = load_time_map(&dir.join("time-map.json")).unwrap();
        let at_89 = dirty_from_signal_range(
            &time_map,
            &score,
            "note",
            FrameRange::new(89, 1),
            Some("9x16".into()),
        );
        let at_90 = dirty_from_signal_range(
            &time_map,
            &score,
            "note",
            FrameRange::new(90, 1),
            Some("9x16".into()),
        );
        let at_209 = dirty_from_signal_range(
            &time_map,
            &score,
            "note",
            FrameRange::new(209, 1),
            Some("9x16".into()),
        );
        let at_210 = dirty_from_signal_range(
            &time_map,
            &score,
            "note",
            FrameRange::new(210, 1),
            Some("9x16".into()),
        );
        assert!(at_89.slots.contains(&"hook".to_string()));
        assert!(!at_89.slots.contains(&"body".to_string()));
        assert!(at_90.slots.contains(&"body".to_string()));
        assert!(!at_90.slots.contains(&"hook".to_string()));
        assert!(at_209.slots.contains(&"body".to_string()));
        assert!(!at_209.slots.contains(&"cta".to_string()));
        assert!(at_210.slots.contains(&"cta".to_string()));
        assert!(!at_210.slots.contains(&"body".to_string()));
    }
}
