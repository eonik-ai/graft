// SPDX-License-Identifier: Apache-2.0

use graft_cas::{ActionKey, BlobId, Kind, Store};

use crate::graph::{ActionGraph, ConcatAction, KerfAction, SlotEncodeAction};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CacheStatus {
    Hit { blob: BlobId },
    Miss,
}

impl CacheStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Hit { .. } => "hit",
            Self::Miss => "miss",
        }
    }

    pub fn blob(&self) -> Option<&BlobId> {
        match self {
            Self::Hit { blob } => Some(blob),
            Self::Miss => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ScheduledSlot {
    pub action: SlotEncodeAction,
    pub cache: CacheStatus,
}

#[derive(Clone, Debug)]
pub struct ScheduledKerf {
    pub action: KerfAction,
    pub cache: CacheStatus,
}

#[derive(Clone, Debug)]
pub struct ScheduledConcat {
    pub action: ConcatAction,
    pub cache: CacheStatus,
}

/// Hits and misses from the action cache. Not a scion JSON diff.
#[derive(Clone, Debug)]
pub struct Schedule {
    pub slots: Vec<ScheduledSlot>,
    pub kerfs: Vec<ScheduledKerf>,
    pub concat: ScheduledConcat,
}

fn lookup(store: &dyn Store, key: &ActionKey, kind: Kind) -> Result<CacheStatus, graft_cas::Error> {
    match store.get_action(key)? {
        Some(entry) if entry.kind == kind && store.contains_blob(kind, &entry.blob) => {
            Ok(CacheStatus::Hit { blob: entry.blob })
        }
        _ => Ok(CacheStatus::Miss),
    }
}

pub fn schedule(graph: &ActionGraph, store: &dyn Store) -> Result<Schedule, graft_cas::Error> {
    let mut slots = Vec::new();
    for action in &graph.slots {
        slots.push(ScheduledSlot {
            action: action.clone(),
            cache: lookup(store, &action.key, Kind::SlotEncode)?,
        });
    }
    let mut kerfs = Vec::new();
    for action in &graph.kerfs {
        kerfs.push(ScheduledKerf {
            action: action.clone(),
            cache: lookup(store, &action.key, Kind::Kerf)?,
        });
    }
    let concat = ScheduledConcat {
        action: graph.concat.clone(),
        cache: lookup(store, &graph.concat.key, Kind::Concat)?,
    };
    Ok(Schedule {
        slots,
        kerfs,
        concat,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::lower;
    use graft_cas::{CacheEntry, Memory};
    use graft_score::{load_scion, load_score};

    #[test]
    fn hook_material_change_misses_without_prev_scion() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/hook-v3-body-v1-9x16");
        let score = load_score(&dir.join("score.json")).unwrap();
        let current = load_scion(&dir.join("scion.json")).unwrap();
        let mut prev = current.clone();
        prev.bindings.get_mut("hook").unwrap().material =
            "blake3:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd".into();

        let store = Memory::new();
        let old = lower(&score, &prev).unwrap();
        // Prior compile of hook_v2+body_v1 left body/cta (and body→cta kerf) in cache.
        for slot in &old.slots {
            if slot.slot.id == "hook" {
                continue;
            }
            let blob = store
                .put_blob(Kind::SlotEncode, slot.slot.id.as_bytes())
                .unwrap();
            store
                .put_action(
                    &slot.key,
                    CacheEntry {
                        kind: Kind::SlotEncode,
                        blob,
                    },
                )
                .unwrap();
        }
        let body_cta = old
            .kerfs
            .iter()
            .find(|k| k.left == "body" && k.right == "cta")
            .unwrap();
        let kerf_blob = store.put_blob(Kind::Kerf, b"body-cta").unwrap();
        store
            .put_action(
                &body_cta.key,
                CacheEntry {
                    kind: Kind::Kerf,
                    blob: kerf_blob,
                },
            )
            .unwrap();

        let graph = lower(&score, &current).unwrap();
        let sched = schedule(&graph, &store).unwrap();
        let hook = sched
            .slots
            .iter()
            .find(|s| s.action.slot.id == "hook")
            .unwrap();
        let body = sched
            .slots
            .iter()
            .find(|s| s.action.slot.id == "body")
            .unwrap();
        let cta = sched
            .slots
            .iter()
            .find(|s| s.action.slot.id == "cta")
            .unwrap();
        assert_eq!(hook.cache.as_str(), "miss");
        assert_eq!(body.cache.as_str(), "hit");
        assert_eq!(cta.cache.as_str(), "hit");
        let hb = sched
            .kerfs
            .iter()
            .find(|k| k.action.left == "hook" && k.action.right == "body")
            .unwrap();
        let bc = sched
            .kerfs
            .iter()
            .find(|k| k.action.left == "body" && k.action.right == "cta")
            .unwrap();
        assert_eq!(hb.cache.as_str(), "miss");
        assert_eq!(bc.cache.as_str(), "hit");
        // Body ActionKey is unchanged across the hook swap.
        let old_body = old.slots.iter().find(|s| s.slot.id == "body").unwrap();
        let new_body = graph.slots.iter().find(|s| s.slot.id == "body").unwrap();
        assert_eq!(old_body.key, new_body.key);
    }
}
