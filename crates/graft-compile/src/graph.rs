// SPDX-License-Identifier: Apache-2.0

use graft_cas::{ActionKey, BlobId};
use graft_score::{
    op_writes_picture, AudioBinding, Binding, Dest, GraphMode, GraphNode, Role, Scion, Score, Slot,
    TimeMap,
};

use crate::flatten::flatten;
use crate::grain::Grain;
use crate::keys::{
    audio_encode_key, audio_mix_key, captions_key, deliver_key, kerf_key, node_key,
    overlay_mix_key, slot_encode_key, spine_picture_key,
};

#[derive(Clone, Debug)]
pub struct SlotEncodeAction {
    pub slot: Slot,
    pub binding: Binding,
    pub key: ActionKey,
    pub material: BlobId,
}

#[derive(Clone, Debug)]
pub struct AudioEncodeAction {
    pub slot: Slot,
    pub binding: AudioBinding,
    pub key: ActionKey,
    pub material: BlobId,
    pub speed: f64,
}

#[derive(Clone, Debug)]
pub struct KerfAction {
    pub left: String,
    pub right: String,
    pub key: ActionKey,
    pub left_key: ActionKey,
    pub right_key: ActionKey,
    pub transition: String,
    pub duration_frames: u64,
    pub grain: Grain,
    pub noop: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConcatPart {
    Slot(String),
    Kerf { left: String, right: String },
}

#[derive(Clone, Debug)]
pub struct ConcatAction {
    pub key: ActionKey,
    pub parts: Vec<ConcatPart>,
}

#[derive(Clone, Debug)]
pub struct CaptionsAction {
    pub slot: Slot,
    pub binding: Binding,
    pub key: ActionKey,
    pub material: BlobId,
}

#[derive(Clone, Debug)]
pub struct OverlayMixAction {
    pub key: ActionKey,
    pub brand: SlotEncodeAction,
    pub rest: Vec<SlotEncodeAction>,
}

#[derive(Clone, Debug)]
pub struct AudioMixAction {
    pub key: ActionKey,
}

#[derive(Clone, Debug)]
pub struct NodeAction {
    pub node: GraphNode,
    pub key: ActionKey,
}

/// Compile IR. Lowered from a flattened scion. Not a second recipe language.
#[derive(Clone, Debug)]
pub struct ActionGraph {
    pub dest: Dest,
    pub scion_id: String,
    pub grain: Grain,
    pub slots: Vec<SlotEncodeAction>,
    pub audio: Vec<AudioEncodeAction>,
    pub overlay_audio: Vec<AudioEncodeAction>,
    pub captions: Vec<CaptionsAction>,
    pub kerfs: Vec<KerfAction>,
    pub nodes: Vec<NodeAction>,
    pub spine_key: ActionKey,
    pub concat: ConcatAction,
    pub audio_mix: Option<AudioMixAction>,
    pub overlay_mix: Option<OverlayMixAction>,
    pub replace: bool,
    pub time_map: TimeMap,
    pub picture_key: ActionKey,
    pub audio_key: Option<ActionKey>,
}

#[derive(Debug, thiserror::Error)]
pub enum LowerError {
    #[error("{0}")]
    Invalid(String),
}

pub fn lower(score: &Score, scion: &Scion) -> Result<ActionGraph, LowerError> {
    scion
        .validate_against_score(score)
        .map_err(|e| LowerError::Invalid(e.to_string()))?;
    let flat = flatten(score, scion).map_err(|e| LowerError::Invalid(e.to_string()))?;
    let grain = Grain::from_dest(&scion.dest);
    let spine = score.spine();
    let mut slots = Vec::new();
    let mut audio = Vec::new();
    for slot in &spine {
        let Some(binding) = flat.bindings.get(&slot.id) else {
            continue;
        };
        let material =
            BlobId::parse(&binding.material).map_err(|e| LowerError::Invalid(e.to_string()))?;
        slots.push(SlotEncodeAction {
            slot: (*slot).clone(),
            binding: binding.clone(),
            key: slot_encode_key(slot, binding, &scion.dest),
            material,
        });
        if let Some(audio_binding) = &binding.audio {
            let audio_material = BlobId::parse(&audio_binding.material)
                .map_err(|e| LowerError::Invalid(e.to_string()))?;
            audio.push(AudioEncodeAction {
                slot: (*slot).clone(),
                binding: audio_binding.clone(),
                key: audio_encode_key(slot, audio_binding, binding.speed(), &scion.dest),
                material: audio_material,
                speed: binding.speed(),
            });
        }
    }

    let mut kerfs = Vec::new();
    for pair in slots.windows(2) {
        let left = &pair[0];
        let right = &pair[1];
        let join = score.join_between(&left.slot.id, &right.slot.id);
        if join.transition != "cut" && graft_score::xfade_filter_name(&join.transition).is_err() {
            return Err(LowerError::Invalid(format!(
                r#"{{"error":"unknown_transition","transition":"{}"}}"#,
                join.transition
            )));
        }
        let fade = join.is_fade();
        let key = kerf_key(
            &left.key,
            &right.key,
            &join.transition,
            join.duration_frames,
            &scion.dest,
        );
        kerfs.push(KerfAction {
            left: left.slot.id.clone(),
            right: right.slot.id.clone(),
            key,
            left_key: left.key.clone(),
            right_key: right.key.clone(),
            transition: join.transition,
            duration_frames: join.duration_frames,
            grain,
            noop: grain.kerf_is_noop() && !fade,
        });
    }

    let mut parts = Vec::new();
    for (i, slot) in slots.iter().enumerate() {
        parts.push(ConcatPart::Slot(slot.slot.id.clone()));
        if i < kerfs.len() {
            let k = &kerfs[i];
            parts.push(ConcatPart::Kerf {
                left: k.left.clone(),
                right: k.right.clone(),
            });
        }
    }

    let mut overlay_audio = Vec::new();
    let mut captions = Vec::new();
    let mut picture_overlays = Vec::new();
    for slot in &score.slots {
        if slot.role.is_spine() {
            continue;
        }
        let Some(binding) = flat.bindings.get(&slot.id) else {
            continue;
        };
        let material =
            BlobId::parse(&binding.material).map_err(|e| LowerError::Invalid(e.to_string()))?;
        match slot.role {
            Role::Vo | Role::Bed | Role::Sfx => {
                let audio_binding = binding.audio.clone().unwrap_or(AudioBinding {
                    material: binding.material.clone(),
                    source: binding.source,
                    place: None,
                });
                let audio_material = BlobId::parse(&audio_binding.material)
                    .map_err(|e| LowerError::Invalid(e.to_string()))?;
                overlay_audio.push(AudioEncodeAction {
                    slot: (*slot).clone(),
                    binding: audio_binding.clone(),
                    key: audio_encode_key(slot, &audio_binding, binding.speed(), &scion.dest),
                    material: audio_material,
                    speed: binding.speed(),
                });
            }
            Role::Captions => {
                captions.push(CaptionsAction {
                    slot: (*slot).clone(),
                    binding: binding.clone(),
                    key: captions_key(slot, binding, &scion.dest),
                    material,
                });
            }
            Role::Brand | Role::Overlay => {
                picture_overlays.push(SlotEncodeAction {
                    slot: (*slot).clone(),
                    binding: binding.clone(),
                    key: slot_encode_key(slot, binding, &scion.dest),
                    material,
                });
            }
            _ => {}
        }
    }

    let mut mix_audio_parts: Vec<(ActionKey, Option<(i64, u64)>)> = audio
        .iter()
        .map(|action| (action.key.clone(), audio_place(&action.binding)))
        .collect();
    mix_audio_parts.extend(
        overlay_audio
            .iter()
            .map(|action| (action.key.clone(), audio_place(&action.binding))),
    );
    let audio_mix = if overlay_audio.is_empty() && !audio.iter().any(audio_is_split) {
        None
    } else {
        Some(AudioMixAction {
            key: audio_mix_key(&mix_audio_parts, &scion.dest),
        })
    };
    let deliver_audio_key = if mix_audio_parts.is_empty() {
        None
    } else {
        Some(audio_mix_key(&mix_audio_parts, &scion.dest))
    };

    let slot_keys: Vec<ActionKey> = slots.iter().map(|s| s.key.clone()).collect();
    let joins: Vec<(String, u64)> = kerfs
        .iter()
        .map(|k| (k.transition.clone(), k.duration_frames))
        .collect();
    let spine_key = spine_picture_key(&slot_keys, &joins, &scion.dest);
    let overlay_mix = picture_overlays.split_first().map(|(brand, rest)| {
        let pads: Vec<serde_json::Value> = std::iter::once(brand)
            .chain(rest.iter())
            .map(|pad| {
                serde_json::json!({
                    "key": pad.key.hex(),
                    "x": pad.binding.overlay_x(),
                    "y": pad.binding.overlay_y(),
                    "opacity": pad.binding.overlay_opacity_expr(),
                })
            })
            .collect();
        OverlayMixAction {
            key: overlay_mix_key(&spine_key, &pads, &scion.dest),
            brand: brand.clone(),
            rest: rest.to_vec(),
        }
    });

    let replace = scion
        .graph
        .as_ref()
        .map(|g| g.mode == GraphMode::Replace)
        .unwrap_or(false);
    let mut pad_keys: std::collections::BTreeMap<String, ActionKey> =
        std::collections::BTreeMap::new();
    if !replace {
        for slot in &slots {
            pad_keys.insert(slot.slot.id.clone(), slot.key.clone());
        }
        let mut picture_key = slots
            .first()
            .map(|s| s.key.clone())
            .unwrap_or(spine_key.clone());
        if !kerfs.is_empty() {
            picture_key = spine_key.clone();
        }
        if let Some(overlay) = &overlay_mix {
            pad_keys.insert(overlay.brand.slot.id.clone(), overlay.brand.key.clone());
            for pad in &overlay.rest {
                pad_keys.insert(pad.slot.id.clone(), pad.key.clone());
            }
            picture_key = overlay.key.clone();
        }
        for ov in &overlay_audio {
            pad_keys.insert(ov.slot.id.clone(), ov.key.clone());
        }
        pad_keys.insert("#picture".into(), picture_key);
        if let Some(key) = &deliver_audio_key {
            pad_keys.insert("#audio".into(), key.clone());
        }
    }

    let mut nodes = Vec::new();
    if let Some(g) = &scion.graph {
        for node in graft_score::expand_groups(&g.nodes) {
            crate::filters::ensure_op(&node.op).map_err(|e| LowerError::Invalid(e.to_string()))?;
            let inputs: Vec<ActionKey> = node
                .inputs
                .iter()
                .filter_map(|name| pad_keys.get(name).cloned())
                .collect();
            let mut extra = serde_json::json!({
                "material": node.material,
                "source": node.source,
                "place": node.place,
                "enable": node.enable,
                "role": node.role,
                "params": node.params,
            });
            if node.keys.is_some() {
                extra["keys"] = serde_json::to_value(&node.keys).unwrap_or(serde_json::Value::Null);
            }
            let key = node_key(&node.op, &node.id, &inputs, &extra, &scion.dest);
            pad_keys.insert(node.id.clone(), key.clone());
            if crate::filters::routes_audio(&node.op)
                .map_err(|e| LowerError::Invalid(e.to_string()))?
            {
                pad_keys.insert("#audio".into(), key.clone());
            } else if op_writes_picture(&node.op) {
                pad_keys.insert("#picture".into(), key.clone());
            }
            nodes.push(NodeAction { node, key });
        }
    }

    let picture_key = pad_keys
        .get("#picture")
        .cloned()
        .or_else(|| slots.first().map(|s| s.key.clone()))
        .unwrap_or(spine_key.clone());
    let audio_key = pad_keys.get("#audio").cloned();
    let caption_keys: Vec<ActionKey> = captions.iter().map(|c| c.key.clone()).collect();
    let concat_key = deliver_key(
        &picture_key,
        audio_key.as_ref(),
        &caption_keys,
        &scion.dest,
        &score.markers,
    );
    let time_map = TimeMap::from_bindings(
        score,
        &scion.id,
        concat_key.hex(),
        &scion.dest.id,
        scion.dest.rate,
        &flat.bindings,
    );

    Ok(ActionGraph {
        dest: scion.dest.clone(),
        scion_id: scion.id.clone(),
        grain,
        slots,
        audio,
        overlay_audio,
        captions,
        kerfs,
        nodes,
        spine_key,
        concat: ConcatAction {
            key: concat_key,
            parts,
        },
        audio_mix,
        overlay_mix,
        replace,
        time_map,
        picture_key,
        audio_key,
    })
}

fn audio_place(binding: &AudioBinding) -> Option<(i64, u64)> {
    binding.place.map(|place| (place.start, place.duration))
}

pub(crate) fn audio_is_split(action: &AudioEncodeAction) -> bool {
    if action.binding.place.is_some() {
        return true;
    }
    let pic = action.slot.range.duration as f64 / action.binding.source.rate.as_f64().max(1.0);
    let aud = action.binding.source.end_seconds() - action.binding.source.start_seconds();
    (pic - aud).abs() > 0.5 / action.binding.source.rate.as_f64().max(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use graft_score::{load_scion, load_score};

    #[test]
    fn example_has_three_slots_two_kerfs() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/hook-v3-body-v1-9x16");
        let score = load_score(&dir.join("score.json")).unwrap();
        let scion = load_scion(&dir.join("scion.json")).unwrap();
        let g = lower(&score, &scion).unwrap();
        assert_eq!(g.slots.len(), 3);
        assert_eq!(g.kerfs.len(), 2);
        assert_eq!(g.time_map.scion, scion.id);
        assert_eq!(g.time_map.dest_id, "9x16");
        assert!(!g.kerfs[0].noop); // x264 fingerprint → GOP
    }
}
