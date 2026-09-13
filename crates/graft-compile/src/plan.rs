// SPDX-License-Identifier: Apache-2.0

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use graft_cas::ActionKey;
use graft_score::{save_json, Scion, Score, TimeMap, GRAFT_SCHEMA};
use serde::Serialize;

use crate::graph::ActionGraph;
use crate::keys::slot_encode_key;
use crate::schedule::Schedule;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SlotPlan {
    pub id: String,
    pub cache: &'static str,
    pub slot_encode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blob: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct KerfPlan {
    pub join: Vec<String>,
    pub cache: &'static str,
    pub kerf: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blob: Option<String>,
}

/// Debug only. Not the dirty oracle.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PrevOverlay {
    pub note: &'static str,
    pub slots: BTreeMap<String, &'static str>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CompilePlan {
    pub graft: String,
    pub scion: String,
    pub dest_id: String,
    pub scion_hash: String,
    /// True when an encode backend ran (or would run) for this dest.
    pub encode: bool,
    pub grain: &'static str,
    pub slots: Vec<SlotPlan>,
    pub audio: Vec<SlotPlan>,
    pub kerfs: Vec<KerfPlan>,
    pub concat: Vec<String>,
    pub concat_cache: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prev_overlay: Option<PrevOverlay>,
}

pub fn plan_from_schedule(graph: &ActionGraph, schedule: &Schedule, encode: bool) -> CompilePlan {
    let slots: Vec<SlotPlan> = schedule
        .slots
        .iter()
        .map(|s| SlotPlan {
            id: s.action.slot.id.clone(),
            cache: s.cache.as_str(),
            slot_encode: s.action.key.hex().to_string(),
            blob: s.cache.blob().map(|b| b.to_string()),
        })
        .collect();
    let audio: Vec<SlotPlan> = schedule
        .audio
        .iter()
        .map(|s| SlotPlan {
            id: s.action.slot.id.clone(),
            cache: s.cache.as_str(),
            slot_encode: s.action.key.hex().to_string(),
            blob: s.cache.blob().map(|b| b.to_string()),
        })
        .collect();
    let kerfs: Vec<KerfPlan> = schedule
        .kerfs
        .iter()
        .map(|k| KerfPlan {
            join: vec![k.action.left.clone(), k.action.right.clone()],
            cache: k.cache.as_str(),
            kerf: k.action.key.hex().to_string(),
            blob: k.cache.blob().map(|b| b.to_string()),
        })
        .collect();
    let concat: Vec<String> = graph
        .concat
        .parts
        .iter()
        .map(|p| match p {
            crate::graph::ConcatPart::Slot(id) => format!("slot:{id}"),
            crate::graph::ConcatPart::Kerf { left, right } => format!("kerf:{left}-{right}"),
        })
        .collect();
    CompilePlan {
        graft: GRAFT_SCHEMA.into(),
        scion: graph.scion_id.clone(),
        dest_id: graph.dest.id.clone(),
        scion_hash: graph.concat.key.hex().to_string(),
        encode,
        grain: graph.grain.as_str(),
        slots,
        audio,
        kerfs,
        concat,
        concat_cache: schedule.concat.cache.as_str(),
        prev_overlay: None,
    }
}

/// JSON scion diff. Debug overlay; the action cache is still the oracle.
pub fn prev_overlay(score: &Score, scion: &Scion, prev: &Scion) -> PrevOverlay {
    let mut slots = BTreeMap::new();
    let current = graft_score::effective_bindings(score, scion).unwrap_or_default();
    let previous = graft_score::effective_bindings(score, prev).unwrap_or_default();
    for slot in score.spine() {
        let Some(binding) = current.get(&slot.id) else {
            continue;
        };
        let key = slot_encode_key(slot, binding, &scion.dest);
        let status = match previous.get(&slot.id) {
            Some(prev_b) if slot_encode_key(slot, prev_b, &prev.dest).hex() == key.hex() => "hit",
            _ => "miss",
        };
        slots.insert(slot.id.clone(), status);
    }
    PrevOverlay {
        note: "debug overlay; dirty oracle is the action cache",
        slots,
    }
}

pub fn time_map_artifact_dir(store_root: &Path, scion_hash: &ActionKey) -> PathBuf {
    store_root.join("builds").join(scion_hash.hex())
}

pub fn write_time_map_artifact(
    store_root: &Path,
    scion_hash: &ActionKey,
    time_map: &TimeMap,
) -> graft_score::Result<PathBuf> {
    let path = time_map_artifact_dir(store_root, scion_hash).join("time-map.json");
    save_json(&path, time_map)?;
    Ok(path)
}
