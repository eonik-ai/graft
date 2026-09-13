// SPDX-License-Identifier: Apache-2.0

use graft_cas::{BlobId, CacheEntry, Kind, Store};
use graft_score::{Scion, Score, TimeMap};

use crate::concat::{ConcatBackend, ConcatError, ConcatPartBytes, ConcatRequest};
use crate::encode::{
    AudioEncodeRequest, EncodeBackend, EncodeError, KerfEncodeRequest, SlotEncodeRequest,
};
use crate::graph::{lower, ActionGraph, ConcatPart, LowerError};
use crate::plan::{plan_from_schedule, CompilePlan};
use crate::schedule::{schedule, CacheStatus, Schedule};

#[derive(Debug, thiserror::Error)]
pub enum CompileError {
    #[error(transparent)]
    Encode(#[from] EncodeError),
    #[error(transparent)]
    Concat(#[from] ConcatError),
    #[error(transparent)]
    Cas(#[from] graft_cas::Error),
    #[error("{0}")]
    Invalid(String),
}

impl From<LowerError> for CompileError {
    fn from(e: LowerError) -> Self {
        Self::Invalid(e.to_string())
    }
}

pub struct CompileOutcome {
    pub plan: CompilePlan,
    pub time_map: TimeMap,
    pub graph: ActionGraph,
    pub encoded: bool,
    pub concat: Option<BlobId>,
}

fn put_action(
    store: &dyn Store,
    key: &graft_cas::ActionKey,
    kind: Kind,
    bytes: &[u8],
    metadata: serde_json::Value,
    provenance: serde_json::Value,
) -> Result<BlobId, CompileError> {
    let blob = store.put_blob(kind, bytes)?;
    let mut result = CacheEntry::new(kind, blob.clone(), bytes.len() as u64);
    result.metadata = metadata;
    result.provenance = provenance;
    store.put_action(key, result)?;
    Ok(blob)
}

fn execute(
    graph: &ActionGraph,
    sched: &Schedule,
    store: &dyn Store,
    encode: &dyn EncodeBackend,
    concat: &dyn ConcatBackend,
) -> Result<BlobId, CompileError> {
    let mut slot_blobs: Vec<BlobId> = Vec::new();
    for scheduled in &sched.slots {
        let blob = match &scheduled.cache {
            CacheStatus::Hit { blob } => blob.clone(),
            CacheStatus::Miss => {
                let material = store.get_blob(Kind::Material, &scheduled.action.material)?;
                let bytes = encode.encode_slot(&SlotEncodeRequest {
                    action: &scheduled.action,
                    slot: &scheduled.action.slot,
                    dest: &graph.dest,
                    material: &material,
                })?;
                put_action(
                    store,
                    &scheduled.action.key,
                    Kind::SlotEncode,
                    &bytes,
                    serde_json::json!({
                        "slot": scheduled.action.slot.id,
                        "dest": graph.dest,
                        "source": scheduled.action.binding.source,
                        "duration_frames": scheduled.action.slot.range.duration
                    }),
                    serde_json::json!({
                        "action": "slot_encode",
                        "material": scheduled.action.material,
                        "encoder": graph.dest.encoder
                    }),
                )?
            }
        };
        slot_blobs.push(blob);
    }

    let mut kerf_blobs: Vec<BlobId> = Vec::new();
    for scheduled in &sched.kerfs {
        let blob = match &scheduled.cache {
            CacheStatus::Hit { blob } => blob.clone(),
            CacheStatus::Miss => {
                let bytes = if scheduled.action.noop {
                    Vec::new()
                } else {
                    let left_i = graph
                        .slots
                        .iter()
                        .position(|s| s.slot.id == scheduled.action.left)
                        .ok_or_else(|| CompileError::Invalid("kerf left missing".into()))?;
                    let right_i = graph
                        .slots
                        .iter()
                        .position(|s| s.slot.id == scheduled.action.right)
                        .ok_or_else(|| CompileError::Invalid("kerf right missing".into()))?;
                    let left = store.get_blob(Kind::SlotEncode, &slot_blobs[left_i])?;
                    let right = store.get_blob(Kind::SlotEncode, &slot_blobs[right_i])?;
                    encode.encode_kerf(&KerfEncodeRequest {
                        action: &scheduled.action,
                        dest: &graph.dest,
                        left: &left,
                        right: &right,
                    })?
                };
                put_action(
                    store,
                    &scheduled.action.key,
                    Kind::Kerf,
                    &bytes,
                    serde_json::json!({
                        "left": scheduled.action.left,
                        "right": scheduled.action.right,
                        "noop": scheduled.action.noop
                    }),
                    serde_json::json!({"action": "kerf"}),
                )?
            }
        };
        kerf_blobs.push(blob);
    }

    let mut audio_blobs: Vec<BlobId> = Vec::new();
    for scheduled in &sched.audio {
        let blob = match &scheduled.cache {
            CacheStatus::Hit { blob } => blob.clone(),
            CacheStatus::Miss => {
                let material = store.get_blob(Kind::Material, &scheduled.action.material)?;
                let bytes = encode.encode_audio(&AudioEncodeRequest {
                    action: &scheduled.action,
                    dest: &graph.dest,
                    material: &material,
                })?;
                put_action(
                    store,
                    &scheduled.action.key,
                    Kind::AudioEncode,
                    &bytes,
                    serde_json::json!({
                        "slot": scheduled.action.slot.id,
                        "source": scheduled.action.binding.source,
                        "codec": "aac",
                        "sample_rate": 48000,
                        "channels": 2
                    }),
                    serde_json::json!({
                        "action": "audio_encode",
                        "material": scheduled.action.material
                    }),
                )?
            }
        };
        audio_blobs.push(blob);
    }

    match &sched.concat.cache {
        CacheStatus::Hit { blob } => Ok(blob.clone()),
        CacheStatus::Miss => {
            let mut parts = Vec::new();
            let mut slot_i = 0;
            let mut kerf_i = 0;
            for part in &graph.concat.parts {
                let (kind, blob) = match part {
                    ConcatPart::Slot(_) => {
                        let b = &slot_blobs[slot_i];
                        slot_i += 1;
                        (Kind::SlotEncode, b)
                    }
                    ConcatPart::Kerf { .. } => {
                        let b = &kerf_blobs[kerf_i];
                        kerf_i += 1;
                        (Kind::Kerf, b)
                    }
                };
                let bytes = store.get_blob(kind, blob)?;
                parts.push(ConcatPartBytes {
                    empty: kind == Kind::Kerf && bytes.is_empty(),
                    bytes,
                });
            }
            let dest_bytes = concat.concat(&ConcatRequest {
                action: &graph.concat,
                dest: &graph.dest,
                parts: &parts,
                audio_parts: &audio_blobs
                    .iter()
                    .map(|blob| {
                        store
                            .get_blob(Kind::AudioEncode, blob)
                            .map(|bytes| ConcatPartBytes {
                                empty: false,
                                bytes,
                            })
                    })
                    .collect::<Result<Vec<_>, _>>()?,
            })?;
            put_action(
                store,
                &graph.concat.key,
                Kind::Concat,
                &dest_bytes,
                serde_json::json!({
                    "scion": graph.scion_id,
                    "dest": graph.dest,
                    "parts": graph.concat.parts.len()
                }),
                serde_json::json!({"action": "link_composition"}),
            )
        }
    }
}

pub fn materials_present(graph: &ActionGraph, store: &dyn Store) -> bool {
    graph
        .slots
        .iter()
        .all(|s| store.contains_blob(Kind::Material, &s.material))
        && graph
            .audio
            .iter()
            .all(|audio| store.contains_blob(Kind::Material, &audio.material))
}

/// Lower → schedule from the action cache → encode misses when a backend is on.
/// `prev` scion is not an argument; it is not the dirty oracle.
/// Missing essence → plan only (worked example has placeholder hashes).
pub fn compile(
    score: &Score,
    scion: &Scion,
    store: &dyn Store,
    encode: &dyn EncodeBackend,
    concat: &dyn ConcatBackend,
) -> Result<CompileOutcome, CompileError> {
    let graph = lower(score, scion)?;
    let sched = schedule(&graph, store)?;
    let ready = encode.enabled() && materials_present(&graph, store);
    if ready && !concat.enabled() {
        return Err(CompileError::Invalid(
            "encode backend is enabled but concat is not".into(),
        ));
    }
    let plan = plan_from_schedule(&graph, &sched, ready);
    let concat_blob = if ready {
        Some(execute(&graph, &sched, store, encode, concat)?)
    } else {
        None
    };
    Ok(CompileOutcome {
        time_map: graph.time_map.clone(),
        graph,
        plan,
        encoded: ready,
        concat: concat_blob,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::concat::Unimplemented as ConcatUnimplemented;
    use crate::encode::Unimplemented as EncodeUnimplemented;
    use graft_cas::Memory;
    use graft_score::{load_scion, load_score};

    #[test]
    fn unimplemented_backend_returns_plan_from_empty_cache() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/hook-v3-body-v1-9x16");
        let score = load_score(&dir.join("score.json")).unwrap();
        let scion = load_scion(&dir.join("scion.json")).unwrap();
        let store = Memory::new();
        let out = compile(
            &score,
            &scion,
            &store,
            &EncodeUnimplemented,
            &ConcatUnimplemented,
        )
        .unwrap();
        assert!(!out.encoded);
        assert!(!out.plan.encode);
        assert_eq!(out.plan.dest_id, "9x16");
        assert!(out.plan.slots.iter().all(|s| s.cache == "miss"));
    }
}
