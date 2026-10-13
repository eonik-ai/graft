// SPDX-License-Identifier: Apache-2.0

use std::collections::{BTreeMap, BTreeSet};

use graft_cas::{BlobId, CacheEntry, Kind, Store};
use graft_score::{
    nested_scion_id, op_writes_picture, scion_has_nested, AudioBinding, FrameRange, Scion, Score,
    TimeMap, TimedRange,
};

use crate::concat::{ConcatBackend, ConcatError, ConcatPartBytes, ConcatRequest};
use crate::encode::{
    AudioEncodeRequest, AudioMixPart, EncodeBackend, EncodeError, KerfEncodeRequest,
    SlotEncodeRequest,
};
use crate::graph::{lower, ActionGraph, ConcatPart, KerfAction, LowerError};
use crate::keys::{working_audio_key, working_picture_key};
use crate::plan::{plan_from_schedule, CompilePlan};
use crate::schedule::{schedule, CacheStatus, Schedule};
use crate::working;

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
        && graph.overlay_mix.as_ref().is_none_or(|mix| {
            store.contains_blob(Kind::Material, &mix.brand.material)
                && mix
                    .rest
                    .iter()
                    .all(|pad| store.contains_blob(Kind::Material, &pad.material))
        })
        && graph.nodes.iter().all(|node| match &node.node.material {
            Some(material) => match BlobId::parse(material) {
                Ok(id) => store.contains_blob(Kind::Material, &id),
                Err(_) => false,
            },
            None => true,
        })
}

fn execute_graph(
    graph: &ActionGraph,
    sched: &Schedule,
    store: &dyn Store,
    encode: &dyn EncodeBackend,
    score: &Score,
    scion: &Scion,
) -> Result<(BlobId, Vec<(String, BlobId)>), CompileError> {
    if let CacheStatus::Hit { blob } = &sched.concat.cache {
        if cached_working_picture(store, graph).is_some() {
            let captions = sched
                .captions
                .iter()
                .map(|c| {
                    let blob = match &c.cache {
                        CacheStatus::Hit { blob } => blob.clone(),
                        CacheStatus::Miss => {
                            let material = store.get_blob(Kind::Material, &c.action.material)?;
                            put_action(
                                store,
                                &c.action.key,
                                Kind::Captions,
                                &encode.encode_captions(&material)?,
                                serde_json::json!({"slot": c.action.slot.id}),
                                serde_json::json!({"action": "captions"}),
                            )?
                        }
                    };
                    Ok((c.action.slot.id.clone(), blob))
                })
                .collect::<Result<Vec<_>, CompileError>>()?;
            return Ok((blob.clone(), captions));
        }
    }

    let mut slot_bytes: Vec<Vec<u8>> = Vec::new();
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
                    serde_json::json!({"slot": scheduled.action.slot.id, "working": true}),
                    serde_json::json!({"action": "working_source"}),
                )?
            }
        };
        slot_bytes.push(store.get_blob(Kind::SlotEncode, &blob)?);
    }

    let mut audio_by_slot: std::collections::BTreeMap<String, Vec<u8>> =
        std::collections::BTreeMap::new();
    for scheduled in sched.audio.iter().chain(sched.overlay_audio.iter()) {
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
                    serde_json::json!({"action": "working_audio"}),
                )?
            }
        };
        audio_by_slot.insert(
            scheduled.action.slot.id.clone(),
            store.get_blob(Kind::AudioEncode, &blob)?,
        );
    }

    let mut pads: std::collections::BTreeMap<String, Vec<u8>> = std::collections::BTreeMap::new();
    for (slot, bytes) in graph.slots.iter().zip(slot_bytes.iter()) {
        pads.insert(slot.slot.id.clone(), bytes.clone());
    }

    let mut picture = Vec::new();
    let mut audio = None;
    let mut premix_sidechain = std::collections::BTreeSet::new();
    if !graph.replace {
        picture = match store.get_action(&graph.spine_key)? {
            Some(entry)
                if entry.kind == Kind::Node && store.contains_blob(Kind::Node, &entry.blob) =>
            {
                store.get_blob(Kind::Node, &entry.blob)?
            }
            _ => {
                let mut folded = slot_bytes
                    .first()
                    .cloned()
                    .ok_or_else(|| CompileError::Invalid("no spine slots".into()))?;
                for (i, kerf) in graph.kerfs.iter().enumerate() {
                    let next = slot_bytes
                        .get(i + 1)
                        .ok_or_else(|| CompileError::Invalid("kerf missing right slot".into()))?;
                    if kerf.transition == "cut" {
                        folded = working::working_concat_video(&graph.dest, &[&folded, next])?;
                    } else {
                        let left_s = working::probe_duration_s(&folded);
                        let overlap = kerf.duration_frames as f64 / graph.dest.rate.as_f64();
                        let offset = (left_s - overlap).max(0.0);
                        folded = working::working_xfade(
                            &graph.dest,
                            &folded,
                            next,
                            &kerf.transition,
                            overlap,
                            offset,
                        )?;
                    }
                    put_action(
                        store,
                        &kerf.key,
                        Kind::Kerf,
                        b"join",
                        serde_json::json!({"left": kerf.left, "right": kerf.right, "transition": kerf.transition}),
                        serde_json::json!({"action": "join"}),
                    )?;
                }
                put_action(
                    store,
                    &graph.spine_key,
                    Kind::Node,
                    &folded,
                    serde_json::json!({"scion": graph.scion_id}),
                    serde_json::json!({"action": "working_spine"}),
                )?;
                folded
            }
        };

        if let Some(overlay) = &graph.overlay_mix {
            picture = match store.get_action(&overlay.key)? {
                Some(entry)
                    if entry.kind == Kind::OverlayMix
                        && store.contains_blob(Kind::OverlayMix, &entry.blob) =>
                {
                    store.get_blob(Kind::OverlayMix, &entry.blob)?
                }
                _ => {
                    let mut out = picture.clone();
                    for pad in std::iter::once(&overlay.brand).chain(overlay.rest.iter()) {
                        let material = store.get_blob(Kind::Material, &pad.material)?;
                        let scaled = working::encode_working_video(
                            &graph.dest,
                            &material,
                            pad.binding.in_s(),
                            pad.binding.out_s(),
                            pad.binding.speed(),
                        )?;
                        let placed = score.compiled_range(pad.slot.range);
                        let t0 = graph.dest.rate.seconds_from_frames(placed.start);
                        let t1 = graph.dest.rate.seconds_from_frames(placed.end());
                        let node = graft_score::GraphNode {
                            id: pad.slot.id.clone(),
                            op: "overlay".into(),
                            inputs: vec!["#picture".into(), pad.slot.id.clone()],
                            material: None,
                            source: None,
                            place: None,
                            enable: Some(format!("between(t,{t0:.6},{t1:.6})")),
                            role: Some(pad.slot.role.as_str().into()),
                            params: Some(serde_json::json!({
                                "x": pad.binding.overlay_x(),
                                "y": pad.binding.overlay_y(),
                                "opacity": pad.binding.overlay_opacity_expr(),
                            })),

                            keys: None,
                            nodes: Vec::new(),
                        };
                        out =
                            working::apply_video_node(&graph.dest, &node, &[&out, &scaled], None)?;
                    }
                    put_action(
                        store,
                        &overlay.key,
                        Kind::OverlayMix,
                        &out,
                        serde_json::json!({
                            "slots": std::iter::once(&overlay.brand.slot.id)
                                .chain(overlay.rest.iter().map(|p| &p.slot.id))
                                .collect::<Vec<_>>()
                        }),
                        serde_json::json!({"action": "overlay_mix"}),
                    )?;
                    out
                }
            };
        }

        let any_spine_audio = graph
            .slots
            .iter()
            .any(|s| audio_by_slot.contains_key(&s.slot.id));
        let split_jl = graph.audio.iter().any(crate::graph::audio_is_split);
        let mut spine_audio: Option<Vec<u8>> = None;
        if !split_jl {
            for (i, slot) in graph.slots.iter().enumerate() {
                if !any_spine_audio && graph.overlay_audio.is_empty() {
                    break;
                }
                let bytes = match audio_by_slot.get(&slot.slot.id) {
                    Some(b) => b.clone(),
                    None => {
                        let dur = graph
                            .dest
                            .rate
                            .seconds_from_frames(slot.slot.range.duration as i64);
                        working::encode_silence(dur)?
                    }
                };
                spine_audio = Some(match spine_audio {
                    None => bytes,
                    Some(left) => {
                        let kerf = &graph.kerfs[i - 1];
                        if kerf.transition == "cut" {
                            working::working_concat_audio(&[&left, &bytes])?
                        } else {
                            let overlap = kerf.duration_frames as f64 / graph.dest.rate.as_f64();
                            working::working_acrossfade(&left, &bytes, overlap)?
                        }
                    }
                });
            }
        }
        if let Some(g) = scion.graph.as_ref() {
            for node in &graft_score::expand_groups(&g.nodes) {
                if node.op != "sidechain" || node.inputs.len() < 2 {
                    continue;
                }
                let Some(bed) = audio_by_slot.get(&node.inputs[0]).cloned() else {
                    continue;
                };
                let Some(key) = audio_by_slot.get(&node.inputs[1]).cloned() else {
                    continue;
                };
                let ducked = working::apply_audio_node(&graph.dest, node, &[&bed, &key], None)?;
                if let Some(action) = graph.nodes.iter().find(|n| n.node.id == node.id) {
                    put_action(
                        store,
                        &action.key,
                        Kind::Node,
                        &ducked,
                        serde_json::json!({"id": node.id, "op": node.op, "premix": true}),
                        serde_json::json!({"action": "working_node"}),
                    )?;
                }
                audio_by_slot.insert(node.inputs[0].clone(), ducked);
                premix_sidechain.insert(node.id.clone());
            }
        }

        let mut mix_parts: Vec<(Vec<u8>, f64, f64)> = Vec::new();
        if split_jl {
            for action in &graph.audio {
                if let Some(bytes) = audio_by_slot.get(&action.slot.id) {
                    let range = action.binding.place.unwrap_or_else(|| {
                        let aud = action.binding.source.end_seconds()
                            - action.binding.source.start_seconds();
                        let duration = graph
                            .dest
                            .rate
                            .frames_from_seconds(aud)
                            .unwrap_or(action.slot.range.duration as i64)
                            .max(1) as u64;
                        graft_score::FrameRange::new(action.slot.range.start, duration)
                    });
                    let placed = score.compiled_range(range);
                    mix_parts.push((
                        bytes.clone(),
                        graph.dest.rate.seconds_from_frames(placed.start),
                        graph.dest.rate.seconds_from_frames(placed.duration as i64),
                    ));
                }
            }
        } else if let Some(s) = spine_audio {
            let dur = working::probe_duration_s(&s);
            mix_parts.push((s, 0.0, dur));
        }
        for ov in &graph.overlay_audio {
            if let Some(bytes) = audio_by_slot.get(&ov.slot.id) {
                let placed = score.compiled_range(ov.slot.range);
                mix_parts.push((
                    bytes.clone(),
                    graph.dest.rate.seconds_from_frames(placed.start),
                    graph.dest.rate.seconds_from_frames(placed.duration as i64),
                ));
            }
        }
        audio = if mix_parts.is_empty() {
            None
        } else {
            let mixed = working::mix_pcm(&mix_parts)?;
            if let Some(mix) = &graph.audio_mix {
                put_action(
                    store,
                    &mix.key,
                    Kind::AudioMix,
                    &mixed,
                    serde_json::json!({"parts": mix_parts.len()}),
                    serde_json::json!({"action": "audio_mix"}),
                )?;
            }
            Some(mixed)
        };
        pads.insert("#picture".into(), picture.clone());
        if let Some(a) = &audio {
            pads.insert("#audio".into(), a.clone());
        }
    }

    for action in &graph.nodes {
        let node = &action.node;
        if node.op == "captions" || premix_sidechain.contains(&node.id) {
            continue;
        }
        if let Some(entry) = store.get_action(&action.key)? {
            if entry.kind == Kind::Node && store.contains_blob(Kind::Node, &entry.blob) {
                let out = store.get_blob(Kind::Node, &entry.blob)?;
                pads.insert(node.id.clone(), out.clone());
                if crate::filters::routes_audio(&node.op)?
                    || (node.op == "gap" && working::gap_is_audio(node))
                {
                    audio = Some(out.clone());
                    pads.insert("#audio".into(), out);
                } else if op_writes_picture(&node.op) {
                    picture = out.clone();
                    pads.insert("#picture".into(), out);
                }
                continue;
            }
        }
        let extra = match &node.material {
            Some(id) => Some(store.get_blob(Kind::Material, &graft_cas::BlobId::parse(id)?)?),
            None => None,
        };
        let extra_ref = extra.as_deref();
        let input_bytes: Vec<Vec<u8>> = if matches!(
            node.op.as_str(),
            "audio_source" | "source" | "gap" | "nested"
        ) || crate::filters::is_lavfi_source(&node.op)?
        {
            Vec::new()
        } else {
            node.inputs
                .iter()
                .map(|name| {
                    pads.get(name).cloned().ok_or_else(|| {
                        CompileError::Invalid(format!("graph {} missing input {name}", node.id))
                    })
                })
                .collect::<Result<Vec<_>, _>>()?
        };
        let refs: Vec<&[u8]> = input_bytes.iter().map(|b| b.as_slice()).collect();
        let out = match node.op.as_str() {
            "source" | "nested" => {
                let mat = extra.ok_or_else(|| {
                    CompileError::Invalid(format!("graph {} {} needs material", node.id, node.op))
                })?;
                let src = node.source.as_ref().ok_or_else(|| {
                    CompileError::Invalid(format!(
                        "graph {} {} needs source range",
                        node.id, node.op
                    ))
                })?;
                working::encode_working_video(
                    &graph.dest,
                    &mat,
                    src.start_seconds(),
                    src.end_seconds(),
                    1.0,
                )?
            }
            "audio_source" => {
                let mat = extra.ok_or_else(|| {
                    CompileError::Invalid(format!("graph {} audio_source needs material", node.id))
                })?;
                let src = node.source.as_ref().ok_or_else(|| {
                    CompileError::Invalid(format!(
                        "graph {} audio_source needs source range",
                        node.id
                    ))
                })?;
                working::encode_working_audio(&mat, src.start_seconds(), src.end_seconds(), 1.0)?
            }
            "gap" => {
                let dur = working::node_duration_s(&graph.dest, node);
                if working::gap_is_audio(node) {
                    working::encode_silence(dur)?
                } else {
                    working::encode_gap_video(&graph.dest, dur)?
                }
            }
            "lut3d" | "lut1d" | "eq" | "hue" | "curves" | "colorbalance" | "colortemperature"
            | "vibrance" | "unsharp" | "vignette" | "noise" | "gblur" | "setrange"
            | "colorspace" | "tonemap" | "scale" | "crop" | "pad" | "rotate" | "flip"
            | "zoompan" | "reverse" | "freeze" | "fade" | "overlay" | "blend" | "concat"
            | "xfade" | "chromakey" | "colorkey" | "lumakey" | "chromahold" | "alphamerge"
            | "drawbox" | "drawgrid" | "colorchannelmixer" | "setsar" | "setdar"
            | "perspective" | "shear" | "scroll" | "premultiply" | "unpremultiply"
            | "streamselect" | "maskedmerge" | "hstack" | "vstack" | "xstack" | "mix"
            | "lenscorrection" | "yadif" | "bwdif" | "tile" | "haldclut" | "colorcorrect"
            | "cdl" | "lgg" | "switch" | "geq" | "selectivecolor" | "exposure" | "colorhold"
            | "colorize" | "deband" => {
                if node.op == "concat" {
                    working::working_concat_video(&graph.dest, &refs)?
                } else if node.op == "xfade" {
                    let transition = node
                        .params
                        .as_ref()
                        .and_then(|v| v.get("transition"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("fade");
                    let duration = node
                        .params
                        .as_ref()
                        .and_then(|v| v.get("duration"))
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.2);
                    let offset = node
                        .params
                        .as_ref()
                        .and_then(|v| v.get("offset"))
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0);
                    working::working_xfade(
                        &graph.dest,
                        refs[0],
                        refs[1],
                        transition,
                        duration,
                        offset,
                    )?
                } else {
                    working::apply_video_node(&graph.dest, node, &refs, extra_ref)?
                }
            }
            "volume" | "loudnorm" | "afade" | "acrossfade" | "amix" | "sidechain"
            | "acompressor" | "alimiter" | "equalizer" | "highpass" | "lowpass" | "bass"
            | "treble" | "adelay" | "aecho" | "areverse" | "pan" => {
                working::apply_audio_node(&graph.dest, node, &refs, extra_ref)?
            }
            other => match crate::filters::classify(other)? {
                crate::filters::FilterClass::Missing => {
                    return Err(CompileError::Invalid(format!(
                        r#"{{"error":"toolchain","op":"{other}"}}"#
                    )))
                }
                crate::filters::FilterClass::Audio { source: true } => {
                    working::lavfi_audio(&graph.dest, node)?
                }
                crate::filters::FilterClass::Audio { source: false } => {
                    working::apply_audio_node(&graph.dest, node, &refs, extra_ref)?
                }
                crate::filters::FilterClass::Video { source: true } => {
                    working::lavfi_video(&graph.dest, node)?
                }
                crate::filters::FilterClass::Video { source: false } => {
                    working::apply_video_node(&graph.dest, node, &refs, extra_ref)?
                }
            },
        };
        put_action(
            store,
            &action.key,
            Kind::Node,
            &out,
            serde_json::json!({"id": node.id, "op": node.op}),
            serde_json::json!({"action": "working_node"}),
        )?;
        pads.insert(node.id.clone(), out.clone());
        if crate::filters::routes_audio(&node.op)?
            || (node.op == "gap" && working::gap_is_audio(node))
        {
            audio = Some(out.clone());
            pads.insert("#audio".into(), out);
        } else if op_writes_picture(&node.op) {
            picture = out.clone();
            pads.insert("#picture".into(), out);
        }
    }

    stash_working_pads(store, graph, &picture, audio.as_deref())?;

    let mut captions = Vec::new();
    let mut caption_bytes = None;
    for scheduled in &sched.captions {
        let blob = match &scheduled.cache {
            CacheStatus::Hit { blob } => blob.clone(),
            CacheStatus::Miss => {
                let material = store.get_blob(Kind::Material, &scheduled.action.material)?;
                put_action(
                    store,
                    &scheduled.action.key,
                    Kind::Captions,
                    &encode.encode_captions(&material)?,
                    serde_json::json!({"slot": scheduled.action.slot.id}),
                    serde_json::json!({"action": "captions"}),
                )?
            }
        };
        if caption_bytes.is_none() {
            caption_bytes = Some(store.get_blob(Kind::Captions, &blob)?);
        }
        captions.push((scheduled.action.slot.id.clone(), blob));
    }

    let dest_bytes = working::deliver(
        &graph.dest,
        &picture,
        audio.as_deref(),
        caption_bytes.as_deref(),
        &score.markers,
    )?;
    let blob = put_action(
        store,
        &graph.concat.key,
        Kind::Concat,
        &dest_bytes,
        serde_json::json!({
            "scion": graph.scion_id,
            "dest": graph.dest,
            "contract": "ffmpeg-working-graph-v1"
        }),
        serde_json::json!({"action": "deliver"}),
    )?;
    Ok((blob, captions))
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
    compile_in(score, scion, store, encode, concat, &NestFamily::default())
}

/// Inner composition used as a working-depth pad. Own score/concept.
#[derive(Clone, Debug)]
pub struct Nest {
    pub score: Score,
    pub scion: Scion,
}

#[derive(Clone, Debug, Default)]
pub struct NestFamily {
    pub nests: BTreeMap<String, Nest>,
}

/// Compile a scion, resolving `binding.nested` / graph `nested` against `family`.
pub fn compile_in(
    score: &Score,
    scion: &Scion,
    store: &dyn Store,
    encode: &dyn EncodeBackend,
    concat: &dyn ConcatBackend,
    family: &NestFamily,
) -> Result<CompileOutcome, CompileError> {
    let mut visiting = BTreeSet::new();
    compile_visit(score, scion, store, encode, concat, family, &mut visiting)
}

fn compile_visit(
    score: &Score,
    scion: &Scion,
    store: &dyn Store,
    encode: &dyn EncodeBackend,
    concat: &dyn ConcatBackend,
    family: &NestFamily,
    visiting: &mut BTreeSet<String>,
) -> Result<CompileOutcome, CompileError> {
    if !visiting.insert(scion.id.clone()) {
        return Err(CompileError::Invalid(format!(
            r#"{{"error":"nested_cycle","scion":"{}"}}"#,
            scion.id
        )));
    }
    let result = compile_resolved(score, scion, store, encode, concat, family, visiting);
    visiting.remove(&scion.id);
    result
}

fn compile_resolved(
    score: &Score,
    scion: &Scion,
    store: &dyn Store,
    encode: &dyn EncodeBackend,
    concat: &dyn ConcatBackend,
    family: &NestFamily,
    visiting: &mut BTreeSet<String>,
) -> Result<CompileOutcome, CompileError> {
    if !encode.working_graph() && scion_has_nested(scion) {
        return Err(CompileError::Invalid(
            "nested scion pads need the ffmpeg working path (not graft-intra)".into(),
        ));
    }
    let mut scion = scion.clone();
    resolve_nested_pads(&mut scion, store, encode, concat, family, visiting)?;
    let graph = lower(score, &scion)?;
    let sched = schedule(&graph, store)?;
    let graph_ready = scion
        .graph
        .as_ref()
        .map(|g| {
            g.nodes
                .iter()
                .filter_map(|n| n.material.as_ref())
                .all(|m| match BlobId::parse(m) {
                    Ok(id) => store.contains_blob(Kind::Material, &id),
                    Err(_) => false,
                })
        })
        .unwrap_or(true);
    if !encode.working_graph() {
        if scion.graph.as_ref().is_some_and(|g| !g.nodes.is_empty()) {
            return Err(CompileError::Invalid(
                "scion.graph needs the ffmpeg working path (not graft-intra)".into(),
            ));
        }
        if graph
            .overlay_mix
            .as_ref()
            .is_some_and(|mix| !mix.rest.is_empty())
        {
            return Err(CompileError::Invalid(
                "role overlay pads need the ffmpeg working path (not graft-intra)".into(),
            ));
        }
    }
    let ready = encode.enabled() && materials_present(&graph, store) && graph_ready;
    if ready && !concat.enabled() {
        return Err(CompileError::Invalid(
            "encode backend is enabled but concat is not".into(),
        ));
    }
    let plan = plan_from_schedule(&graph, &sched, ready);
    let (concat_blob, captions) = if ready {
        let (blob, captions) = if encode.working_graph() {
            execute_graph(&graph, &sched, store, encode, score, &scion)?
        } else {
            execute(&graph, &sched, store, encode, concat)?
        };
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

struct NestedPad {
    material: String,
    audio: Option<String>,
    source: TimedRange,
}

fn collect_nested_ids(scion: &Scion) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    for layer in &scion.layers {
        for binding in layer.bindings.values() {
            if let Some(id) = binding.nested_id() {
                ids.insert(id.to_string());
            }
        }
    }
    if let Some(graph) = &scion.graph {
        for node in &graph.nodes {
            if let Some(id) = nested_scion_id(node) {
                ids.insert(id.to_string());
            }
        }
    }
    ids
}

fn resolve_nested_pads(
    scion: &mut Scion,
    store: &dyn Store,
    encode: &dyn EncodeBackend,
    concat: &dyn ConcatBackend,
    family: &NestFamily,
    visiting: &mut BTreeSet<String>,
) -> Result<(), CompileError> {
    let ids = collect_nested_ids(scion);
    if ids.is_empty() {
        return Ok(());
    }
    let mut pads = BTreeMap::new();
    for id in ids {
        pads.insert(
            id.clone(),
            ensure_nested_pad(&id, store, encode, concat, family, visiting)?,
        );
    }
    apply_nested_pads(scion, &pads)?;
    Ok(())
}

fn ensure_nested_pad(
    id: &str,
    store: &dyn Store,
    encode: &dyn EncodeBackend,
    concat: &dyn ConcatBackend,
    family: &NestFamily,
    visiting: &mut BTreeSet<String>,
) -> Result<NestedPad, CompileError> {
    let nest = family.nests.get(id).ok_or_else(|| {
        CompileError::Invalid(format!(r#"{{"error":"unknown_nest","scion":"{id}"}}"#))
    })?;
    let outcome = compile_visit(
        &nest.score,
        &nest.scion,
        store,
        encode,
        concat,
        family,
        visiting,
    )?;
    if !outcome.encoded {
        return Err(CompileError::Invalid(format!(
            "nested {id} compiled a plan only — bind takes and install ffmpeg"
        )));
    }
    let picture = cached_working_picture(store, &outcome.graph)
        .ok_or_else(|| CompileError::Invalid(format!("nested {id} has no working picture")))?;
    let material = store.put_blob(Kind::Material, &picture)?.to_string();
    let audio = cached_working_audio(store, &outcome.graph)
        .map(|bytes| {
            store
                .put_blob(Kind::Material, &bytes)
                .map(|id| id.to_string())
        })
        .transpose()?;
    let duration = nest.score.clock.duration_frames.max(1);
    let source = TimedRange {
        rate: nest.scion.dest.rate,
        range: FrameRange::new(0, duration),
    };
    Ok(NestedPad {
        material,
        audio,
        source,
    })
}

fn apply_nested_pads(
    scion: &mut Scion,
    pads: &BTreeMap<String, NestedPad>,
) -> Result<(), CompileError> {
    for layer in &mut scion.layers {
        for binding in layer.bindings.values_mut() {
            let Some(id) = binding.nested_id().map(str::to_string) else {
                continue;
            };
            let pad = pads.get(&id).ok_or_else(|| {
                CompileError::Invalid(format!(r#"{{"error":"unknown_nest","scion":"{id}"}}"#))
            })?;
            binding.material = pad.material.clone();
            if let Some(audio) = &pad.audio {
                let source = binding
                    .audio
                    .as_ref()
                    .map(|a| a.source)
                    .unwrap_or(binding.source);
                let place = binding.audio.as_ref().and_then(|a| a.place);
                binding.audio = Some(AudioBinding {
                    material: audio.clone(),
                    source,
                    place,
                });
            }
        }
    }
    if let Some(graph) = &mut scion.graph {
        fill_nested_graph(&mut graph.nodes, pads)?;
    }
    Ok(())
}

fn fill_nested_graph(
    nodes: &mut [graft_score::GraphNode],
    pads: &BTreeMap<String, NestedPad>,
) -> Result<(), CompileError> {
    for node in nodes {
        if node.op == "group" {
            fill_nested_graph(&mut node.nodes, pads)?;
            continue;
        }
        let Some(id) = nested_scion_id(node).map(str::to_string) else {
            continue;
        };
        let pad = pads.get(&id).ok_or_else(|| {
            CompileError::Invalid(format!(r#"{{"error":"unknown_nest","scion":"{id}"}}"#))
        })?;
        node.material = Some(pad.material.clone());
        if node.source.is_none() {
            node.source = Some(pad.source);
        }
    }
    Ok(())
}

fn cached_working_picture(store: &dyn Store, graph: &ActionGraph) -> Option<Vec<u8>> {
    cached_action_bytes(
        store,
        &working_picture_key(&graph.scion_id, &graph.picture_key),
    )
}

fn cached_working_audio(store: &dyn Store, graph: &ActionGraph) -> Option<Vec<u8>> {
    cached_action_bytes(
        store,
        &working_audio_key(&graph.scion_id, &graph.picture_key),
    )
}

fn cached_action_bytes(store: &dyn Store, key: &graft_cas::ActionKey) -> Option<Vec<u8>> {
    let entry = store.get_action(key).ok().flatten()?;
    if store.contains_blob(entry.kind, &entry.blob) {
        store.get_blob(entry.kind, &entry.blob).ok()
    } else {
        None
    }
}

fn stash_working_pads(
    store: &dyn Store,
    graph: &ActionGraph,
    picture: &[u8],
    audio: Option<&[u8]>,
) -> Result<(), CompileError> {
    put_action(
        store,
        &working_picture_key(&graph.scion_id, &graph.picture_key),
        Kind::Node,
        picture,
        serde_json::json!({"scion": graph.scion_id}),
        serde_json::json!({"action": "working_picture"}),
    )?;
    if let Some(audio) = audio {
        put_action(
            store,
            &working_audio_key(&graph.scion_id, &graph.picture_key),
            Kind::Node,
            audio,
            serde_json::json!({"scion": graph.scion_id}),
            serde_json::json!({"action": "working_audio"}),
        )?;
    }
    Ok(())
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
