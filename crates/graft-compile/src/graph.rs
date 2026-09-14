// SPDX-License-Identifier: Apache-2.0

use graft_cas::{ActionKey, BlobId};
use graft_score::{AudioBinding, Binding, Dest, Role, Scion, Score, Slot, TimeMap};

use crate::flatten::flatten;
use crate::grain::Grain;
use crate::keys::{
    audio_encode_key, audio_mix_key, captions_key, kerf_key, overlay_mix_key, scion_hash,
    slot_encode_key,
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
}

#[derive(Clone, Debug)]
pub struct AudioMixAction {
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
    pub concat: ConcatAction,
    pub audio_mix: Option<AudioMixAction>,
    pub overlay_mix: Option<OverlayMixAction>,
    pub time_map: TimeMap,
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
        if join.transition != "cut" && join.transition != "fade" {
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
    let mut brand: Option<SlotEncodeAction> = None;
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
            Role::Vo | Role::Bed => {
                let audio_binding = binding.audio.clone().unwrap_or(AudioBinding {
                    material: binding.material.clone(),
                    source: binding.source,
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
            Role::Brand => {
                brand = Some(SlotEncodeAction {
                    slot: (*slot).clone(),
                    binding: binding.clone(),
                    key: slot_encode_key(slot, binding, &scion.dest),
                    material,
                });
            }
            _ => {}
        }
    }

    let mut mix_audio_keys: Vec<ActionKey> =
        audio.iter().map(|action| action.key.clone()).collect();
    mix_audio_keys.extend(overlay_audio.iter().map(|action| action.key.clone()));
    let audio_mix = if overlay_audio.is_empty() {
        None
    } else {
        Some(AudioMixAction {
            key: audio_mix_key(&mix_audio_keys, &scion.dest),
        })
    };

    let slot_keys: Vec<ActionKey> = slots.iter().map(|s| s.key.clone()).collect();
    let audio_keys: Vec<ActionKey> = mix_audio_keys.clone();
    let kerf_keys: Vec<ActionKey> = kerfs.iter().map(|k| k.key.clone()).collect();
    let caption_keys: Vec<String> = captions.iter().map(|c| c.key.hex().to_string()).collect();
    let extra = serde_json::json!({
        "captions": caption_keys,
        "overlay_audio": overlay_audio.iter().map(|a| a.key.hex().to_string()).collect::<Vec<_>>(),
        "audio_mix": audio_mix.as_ref().map(|m| m.key.hex().to_string()),
        "brand": brand.as_ref().map(|b| b.key.hex().to_string()),
    });
    let concat_key = scion_hash(scion, &slot_keys, &audio_keys, &kerf_keys, &extra);
    let overlay_mix = brand.map(|brand| OverlayMixAction {
        key: overlay_mix_key(&concat_key, &brand.key, &scion.dest),
        brand,
    });
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
        concat: ConcatAction {
            key: concat_key,
            parts,
        },
        audio_mix,
        overlay_mix,
        time_map,
    })
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
