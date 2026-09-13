// SPDX-License-Identifier: Apache-2.0

use graft_cas::{ActionKey, BlobId};
use graft_score::{Binding, Dest, Scion, Score, Slot, TimeMap};

use crate::flatten::flatten;
use crate::grain::Grain;
use crate::keys::{kerf_key, scion_hash, slot_encode_key};

#[derive(Clone, Debug)]
pub struct SlotEncodeAction {
    pub slot: Slot,
    pub binding: Binding,
    pub key: ActionKey,
    pub material: BlobId,
}

#[derive(Clone, Debug)]
pub struct KerfAction {
    pub left: String,
    pub right: String,
    pub key: ActionKey,
    pub left_key: ActionKey,
    pub right_key: ActionKey,
    pub transition: String,
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

/// Compile IR. Lowered from a flattened scion. Not a second recipe language.
#[derive(Clone, Debug)]
pub struct ActionGraph {
    pub dest: Dest,
    pub scion_id: String,
    pub grain: Grain,
    pub slots: Vec<SlotEncodeAction>,
    pub kerfs: Vec<KerfAction>,
    pub concat: ConcatAction,
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
    let flat = flatten(score, scion);
    let grain = Grain::from_dest(&scion.dest);
    let spine = score.spine();
    let mut slots = Vec::new();
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
    }

    let mut kerfs = Vec::new();
    for pair in slots.windows(2) {
        let left = &pair[0];
        let right = &pair[1];
        let key = kerf_key(&left.key, &right.key, "cut", &scion.dest);
        kerfs.push(KerfAction {
            left: left.slot.id.clone(),
            right: right.slot.id.clone(),
            key,
            left_key: left.key.clone(),
            right_key: right.key.clone(),
            transition: "cut".into(),
            grain,
            noop: grain.kerf_is_noop(),
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

    let slot_keys: Vec<ActionKey> = slots.iter().map(|s| s.key.clone()).collect();
    let kerf_keys: Vec<ActionKey> = kerfs.iter().map(|k| k.key.clone()).collect();
    let concat_key = scion_hash(scion, &slot_keys, &kerf_keys);
    // Dest clock; keyed on disk by concat.key (scion_hash), not mixed into schema.
    let time_map = TimeMap::from_score(score, &scion.id, &scion.dest.id);

    Ok(ActionGraph {
        dest: scion.dest.clone(),
        scion_id: scion.id.clone(),
        grain,
        slots,
        kerfs,
        concat: ConcatAction {
            key: concat_key,
            parts,
        },
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
