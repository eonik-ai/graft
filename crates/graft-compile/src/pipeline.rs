// SPDX-License-Identifier: Apache-2.0

use graft_cas::{BlobId, CacheEntry, Kind, Store};
use graft_score::{Scion, Score, TimeMap};

use crate::concat::{ConcatBackend, ConcatError, ConcatPartBytes, ConcatRequest};
use crate::encode::{
    AudioEncodeRequest, AudioMixPart, EncodeBackend, EncodeError, KerfEncodeRequest,
    SlotEncodeRequest,
};
use crate::graph::{lower, ActionGraph, ConcatPart, KerfAction, LowerError};
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
    pub captions: Vec<(String, BlobId)>,
}

fn apply_fade_trims(kerfs: &[KerfAction], parts: &[ConcatPart], bytes: &mut [ConcatPartBytes]) {
    for (i, part) in parts.iter().enumerate() {
        let ConcatPart::Kerf { left, right } = part else {
            continue;
        };
        let Some(kerf) = kerfs.iter().find(|k| k.left == *left && k.right == *right) else {
            continue;
        };
        if !kerf.transition.eq_ignore_ascii_case("fade") || kerf.duration_frames == 0 {
            continue;
        }
        if i > 0 {
            bytes[i - 1].trim_tail_frames = kerf.duration_frames;
        }
        if i + 1 < bytes.len() {
            bytes[i + 1].trim_head_frames = kerf.duration_frames;
        }
    }
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
) -> Result<(BlobId, Vec<(String, BlobId)>), CompileError> {
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

    let mut overlay_audio_blobs: Vec<BlobId> = Vec::new();
    for scheduled in &sched.overlay_audio {
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
                    serde_json::json!({"slot": scheduled.action.slot.id}),
                    serde_json::json!({"action": "audio_encode"}),
                )?
            }
        };
        overlay_audio_blobs.push(blob);
    }

    let mut caption_blobs: Vec<(String, BlobId)> = Vec::new();
    for scheduled in &sched.captions {
        let blob = match &scheduled.cache {
            CacheStatus::Hit { blob } => blob.clone(),
            CacheStatus::Miss => {
                let material = store.get_blob(Kind::Material, &scheduled.action.material)?;
                let bytes = encode.encode_captions(&material)?;
                put_action(
                    store,
                    &scheduled.action.key,
                    Kind::Captions,
                    &bytes,
                    serde_json::json!({"slot": scheduled.action.slot.id}),
                    serde_json::json!({"action": "captions"}),
                )?
            }
        };
        caption_blobs.push((scheduled.action.slot.id.clone(), blob));
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
        CacheStatus::Hit { blob } => Ok((blob.clone(), caption_blobs)),
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
                    duration_s: 0.0,
                    trim_head_frames: 0,
                    trim_tail_frames: 0,
                });
            }
            apply_fade_trims(&graph.kerfs, &graph.concat.parts, &mut parts);
            let audio_by_slot: std::collections::BTreeMap<String, graft_cas::BlobId> = graph
                .audio
                .iter()
                .zip(audio_blobs.iter())
                .map(|(action, blob)| (action.slot.id.clone(), blob.clone()))
                .collect();
            let audio_parts = if graph.audio.is_empty() {
                Vec::new()
            } else {
                graph
                    .slots
                    .iter()
                    .map(|slot| {
                        let duration_s = graph
                            .dest
                            .rate
                            .seconds_from_frames(slot.slot.range.duration as i64);
                        if let Some(blob) = audio_by_slot.get(&slot.slot.id) {
                            store
                                .get_blob(Kind::AudioEncode, blob)
                                .map(|bytes| ConcatPartBytes {
                                    empty: bytes.is_empty(),
                                    bytes,
                                    duration_s,
                                    trim_head_frames: 0,
                                    trim_tail_frames: 0,
                                })
                        } else {
                            Ok(ConcatPartBytes {
                                empty: true,
                                bytes: Vec::new(),
                                duration_s,
                                trim_head_frames: 0,
                                trim_tail_frames: 0,
                            })
                        }
                    })
                    .collect::<Result<Vec<_>, _>>()?
            };
            let picture_only = graph.audio_mix.is_some() || graph.overlay_mix.is_some();
            let mut dest_bytes = concat.concat(&ConcatRequest {
                action: &graph.concat,
                dest: &graph.dest,
                parts: &parts,
                audio_parts: if picture_only { &[] } else { &audio_parts },
            })?;
            if let Some(overlay) = &graph.overlay_mix {
                let brand = store.get_blob(Kind::Material, &overlay.brand.material)?;
                dest_bytes = encode.overlay_brand(&graph.dest, &dest_bytes, &brand)?;
                put_action(
                    store,
                    &overlay.key,
                    Kind::OverlayMix,
                    &dest_bytes,
                    serde_json::json!({"slot": overlay.brand.slot.id}),
                    serde_json::json!({"action": "overlay_mix"}),
                )?;
            }
            if let Some(mix) = &graph.audio_mix {
                let mut owned = Vec::new();
                for blob in audio_blobs.iter().chain(overlay_audio_blobs.iter()) {
                    owned.push(store.get_blob(Kind::AudioEncode, blob)?);
                }
                let refs = graph
                    .audio
                    .iter()
                    .chain(graph.overlay_audio.iter())
                    .zip(owned.iter())
                    .map(|(action, bytes)| AudioMixPart {
                        bytes,
                        start_s: graph.dest.rate.seconds_from_frames(action.slot.range.start),
                        duration_s: graph
                            .dest
                            .rate
                            .seconds_from_frames(action.slot.range.duration as i64),
                    })
                    .collect::<Vec<_>>();
                let mixed = encode.mix_audio(&graph.dest, &refs)?;
                put_action(
                    store,
                    &mix.key,
                    Kind::AudioMix,
                    &mixed,
                    serde_json::json!({"parts": refs.len()}),
                    serde_json::json!({"action": "audio_mix"}),
                )?;
                dest_bytes = mux_mixed_audio(concat, graph, dest_bytes, mixed)?;
            } else if picture_only && !audio_parts.is_empty() {
                dest_bytes = concat.concat(&ConcatRequest {
                    action: &graph.concat,
                    dest: &graph.dest,
                    parts: &[ConcatPartBytes {
                        empty: dest_bytes.is_empty(),
                        bytes: dest_bytes,
                        duration_s: 0.0,
                        trim_head_frames: 0,
                        trim_tail_frames: 0,
                    }],
                    audio_parts: &audio_parts,
                })?;
            }
            let blob = put_action(
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
            )?;
            Ok((blob, caption_blobs))
        }
    }
}

fn mux_mixed_audio(
    concat: &dyn ConcatBackend,
    graph: &ActionGraph,
    picture: Vec<u8>,
    audio: Vec<u8>,
) -> Result<Vec<u8>, CompileError> {
    let duration_s = graph.dest.rate.seconds_from_frames(
        graph
            .slots
            .iter()
            .map(|slot| slot.slot.range.duration as i64)
            .sum(),
    );
    Ok(concat.concat(&ConcatRequest {
        action: &graph.concat,
        dest: &graph.dest,
        parts: &[ConcatPartBytes {
            empty: picture.is_empty(),
            bytes: picture,
            duration_s,
            trim_head_frames: 0,
            trim_tail_frames: 0,
        }],
        audio_parts: &[ConcatPartBytes {
            empty: audio.is_empty(),
            bytes: audio,
            duration_s,
            trim_head_frames: 0,
            trim_tail_frames: 0,
        }],
    })?)
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
        && graph
            .overlay_audio
            .iter()
            .all(|audio| store.contains_blob(Kind::Material, &audio.material))
        && graph
            .captions
            .iter()
            .all(|cap| store.contains_blob(Kind::Material, &cap.material))
        && graph
            .overlay_mix
            .as_ref()
            .map(|mix| store.contains_blob(Kind::Material, &mix.brand.material))
            .unwrap_or(true)
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
    let (concat_blob, captions) = if ready {
        let (blob, captions) = execute(&graph, &sched, store, encode, concat)?;
        (Some(blob), captions)
    } else {
        (None, Vec::new())
    };
    Ok(CompileOutcome {
        time_map: graph.time_map.clone(),
        graph,
        plan,
        encoded: ready,
        concat: concat_blob,
        captions,
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
