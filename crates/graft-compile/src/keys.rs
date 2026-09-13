// SPDX-License-Identifier: Apache-2.0

use graft_cas::ActionKey;
use graft_score::{AudioBinding, Binding, Dest, Scion, Slot};
use serde_json::{json, Value};

pub fn slot_encode_key(slot: &Slot, binding: &Binding, dest: &Dest) -> ActionKey {
    let params = binding
        .params
        .clone()
        .unwrap_or(Value::Object(serde_json::Map::new()));
    let value = json!({
        "slot": slot.id,
        "role": slot.role.as_str(),
        "material": binding.material,
        "source": binding.source,
        "audio": binding.audio,
        "params": params,
        "dest": dest.fingerprint_value(),
        "encoder": dest.encoder_value(),
    });
    ActionKey::from_canonical(&value)
}

pub fn audio_encode_key(slot: &Slot, binding: &AudioBinding, dest: &Dest) -> ActionKey {
    ActionKey::from_canonical(&json!({
        "action": "audio_encode",
        "slot": slot.id,
        "material": binding.material,
        "source": binding.source,
        "codec": "aac",
        "sample_rate": 48000,
        "channels": 2,
        "toolchain": dest.encoder.toolchain
    }))
}

pub fn kerf_key(left: &ActionKey, right: &ActionKey, transition: &str, dest: &Dest) -> ActionKey {
    let value = json!({
        "left": left.hex(),
        "right": right.hex(),
        "transition": transition,
        "dest": dest.fingerprint_value(),
        "encoder": dest.encoder_value(),
    });
    ActionKey::from_canonical(&value)
}

pub fn scion_hash(
    scion: &Scion,
    slot_keys: &[ActionKey],
    audio_keys: &[ActionKey],
    kerf_keys: &[ActionKey],
) -> ActionKey {
    let slots: Vec<&str> = slot_keys.iter().map(ActionKey::hex).collect();
    let kerfs: Vec<&str> = kerf_keys.iter().map(ActionKey::hex).collect();
    let audio: Vec<&str> = audio_keys.iter().map(ActionKey::hex).collect();
    let value = json!({
        "slot_encode": slots,
        "audio_encode": audio,
        "kerf": kerfs,
        "dest": scion.dest.fingerprint_value(),
    });
    ActionKey::from_canonical(&value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use graft_score::{load_scion, load_score};

    #[test]
    fn hook_and_body_keys_differ() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/hook-v3-body-v1-9x16");
        let score = load_score(&dir.join("score.json")).unwrap();
        let scion = load_scion(&dir.join("scion.json")).unwrap();
        let bindings = graft_score::effective_bindings(&score, &scion).unwrap();
        let hook = score.slot("hook").unwrap();
        let body = score.slot("body").unwrap();
        let hk = slot_encode_key(hook, &bindings["hook"], &scion.dest);
        let bk = slot_encode_key(body, &bindings["body"], &scion.dest);
        assert_ne!(hk, bk);
        assert_eq!(hk.hex().len(), 64);
    }
}
