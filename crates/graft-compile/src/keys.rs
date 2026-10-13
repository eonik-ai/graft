// SPDX-License-Identifier: Apache-2.0

use graft_cas::ActionKey;
use graft_score::{AudioBinding, Binding, Dest, Marker, Scion, Slot};
use serde_json::{json, Value};

pub fn slot_encode_key(slot: &Slot, binding: &Binding, dest: &Dest) -> ActionKey {
    let params = binding
        .params
        .clone()
        .unwrap_or(Value::Object(serde_json::Map::new()));
    let value = json!({
        "action": "working_source",
        "slot": slot.id,
        "role": slot.role.as_str(),
        "material": binding.material,
        "source": binding.source,
        "audio": binding.audio,
        "params": params,
        "remap": binding.remap,
        "working": dest.working_fingerprint_value(),
    });
    ActionKey::from_canonical(&value)
}

pub fn audio_encode_key(slot: &Slot, binding: &AudioBinding, speed: f64, dest: &Dest) -> ActionKey {
    ActionKey::from_canonical(&json!({
        "action": "audio_encode",
        "slot": slot.id,
        "material": binding.material,
        "source": binding.source,
        "speed": speed,
        "codec": "pcm_s24le",
        "sample_rate": 48000,
        "channels": 2,
        "working": dest.working_fingerprint_value()
    }))
}

pub fn kerf_key(
    left: &ActionKey,
    right: &ActionKey,
    transition: &str,
    duration_frames: u64,
    dest: &Dest,
) -> ActionKey {
    let value = json!({
        "left": left.hex(),
        "right": right.hex(),
        "transition": transition,
        "duration_frames": duration_frames,
        "working": dest.working_fingerprint_value(),
    });
    ActionKey::from_canonical(&value)
}

pub fn spine_picture_key(
    slot_keys: &[ActionKey],
    joins: &[(String, u64)],
    dest: &Dest,
) -> ActionKey {
    let slots: Vec<&str> = slot_keys.iter().map(ActionKey::hex).collect();
    let join_vals: Vec<Value> = joins
        .iter()
        .map(|(transition, frames)| json!({"transition": transition, "duration_frames": frames}))
        .collect();
    ActionKey::from_canonical(&json!({
        "action": "working_spine",
        "slots": slots,
        "joins": join_vals,
        "working": dest.working_fingerprint_value(),
    }))
}

pub fn node_key(op: &str, id: &str, inputs: &[ActionKey], extra: &Value, dest: &Dest) -> ActionKey {
    let ins: Vec<&str> = inputs.iter().map(ActionKey::hex).collect();
    ActionKey::from_canonical(&json!({
        "action": "working_node",
        "op": op,
        "id": id,
        "inputs": ins,
        "extra": extra,
        "working": dest.working_fingerprint_value(),
    }))
}

pub fn deliver_key(
    picture: &ActionKey,
    audio: Option<&ActionKey>,
    captions: &[ActionKey],
    dest: &Dest,
    markers: &[Marker],
) -> ActionKey {
    let caps: Vec<&str> = captions.iter().map(ActionKey::hex).collect();
    let mut value = json!({
        "action": "deliver",
        "picture": picture.hex(),
        "audio": audio.map(ActionKey::hex),
        "captions": caps,
        "dest": dest.fingerprint_value(),
        "encoder": dest.encoder_value(),
    });
    if !markers.is_empty() {
        value["markers"] = json!(markers
            .iter()
            .map(|m| json!({"id": m.id, "start": m.range.start, "duration": m.range.duration}))
            .collect::<Vec<_>>());
    }
    ActionKey::from_canonical(&value)
}

pub fn captions_key(slot: &Slot, binding: &Binding, dest: &Dest) -> ActionKey {
    ActionKey::from_canonical(&json!({
        "action": "captions",
        "slot": slot.id,
        "material": binding.material,
        "source": binding.source,
        "dest": dest.fingerprint_value(),
    }))
}

pub fn audio_mix_key(parts: &[(ActionKey, Option<(i64, u64)>)], dest: &Dest) -> ActionKey {
    let audio: Vec<Value> = parts
        .iter()
        .map(|(key, place)| json!({"key": key.hex(), "place": place}))
        .collect();
    ActionKey::from_canonical(&json!({
        "action": "audio_mix",
        "audio_encode": audio,
        "working": dest.working_fingerprint_value(),
        "sample_rate": 48000,
        "channels": 2,
    }))
}

pub fn overlay_mix_key(picture: &ActionKey, pads: &[Value], dest: &Dest) -> ActionKey {
    ActionKey::from_canonical(&json!({
        "action": "overlay_mix",
        "picture": picture.hex(),
        "pads": pads,
        "working": dest.working_fingerprint_value(),
    }))
}

/// Inner working `#picture` used as a nested pad. Not delivery.
pub fn working_picture_key(scion_id: &str, picture: &ActionKey) -> ActionKey {
    ActionKey::from_canonical(&json!({
        "action": "working_picture",
        "scion": scion_id,
        "picture": picture.hex(),
    }))
}

pub fn working_audio_key(scion_id: &str, picture: &ActionKey) -> ActionKey {
    ActionKey::from_canonical(&json!({
        "action": "working_audio",
        "scion": scion_id,
        "picture": picture.hex(),
    }))
}

pub fn scion_hash(
    scion: &Scion,
    slot_keys: &[ActionKey],
    audio_keys: &[ActionKey],
    kerf_keys: &[ActionKey],
    extra: &Value,
) -> ActionKey {
    let slots: Vec<&str> = slot_keys.iter().map(ActionKey::hex).collect();
    let kerfs: Vec<&str> = kerf_keys.iter().map(ActionKey::hex).collect();
    let audio: Vec<&str> = audio_keys.iter().map(ActionKey::hex).collect();
    let mut value = json!({
        "slot_encode": slots,
        "audio_encode": audio,
        "kerf": kerfs,
        "dest": scion.dest.fingerprint_value(),
    });
    if let (Some(obj), Some(extra)) = (value.as_object_mut(), extra.as_object()) {
        for (key, item) in extra {
            obj.insert(key.clone(), item.clone());
        }
    }
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
