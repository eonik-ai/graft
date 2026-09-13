// SPDX-License-Identifier: Apache-2.0

use serde_json::{Map, Number, Value};

use crate::Error;

/// UTF-8 JSON, sorted keys, no insignificant whitespace.
/// Times and other floats use millisecond scale (3 decimal places).
pub fn canonical_json(value: &Value) -> String {
    let mut out = String::new();
    write_canonical(value, &mut out);
    out
}

pub fn blake3_canonical(value: &Value) -> String {
    let bytes = canonical_json(value);
    blake3::hash(bytes.as_bytes()).to_hex().to_string()
}

fn write_canonical(value: &Value, out: &mut String) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(true) => out.push_str("true"),
        Value::Bool(false) => out.push_str("false"),
        Value::Number(n) => out.push_str(&canonical_number(n)),
        Value::String(s) => {
            out.push_str(&serde_json::to_string(s).expect("string JSON"));
        }
        Value::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_canonical(item, out);
            }
            out.push(']');
        }
        Value::Object(map) => write_object(map, out),
    }
}

fn write_object(map: &Map<String, Value>, out: &mut String) {
    let mut keys: Vec<&String> = map.keys().collect();
    keys.sort();
    out.push('{');
    for (i, key) in keys.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&serde_json::to_string(*key).expect("key JSON"));
        out.push(':');
        write_canonical(&map[*key], out);
    }
    out.push('}');
}

fn canonical_number(n: &Number) -> String {
    if let Some(i) = n.as_i64() {
        return i.to_string();
    }
    if let Some(u) = n.as_u64() {
        return u.to_string();
    }
    let f = n.as_f64().unwrap_or(0.0);
    if !f.is_finite() {
        return "0.000".to_string();
    }
    format!("{f:.3}")
}

pub fn require_graft_version(graft: &str) -> Result<(), Error> {
    if graft == crate::GRAFT_SCHEMA {
        Ok(())
    } else {
        Err(Error::invalid(format!(
            "graft format id {graft:?} != {:?}",
            crate::GRAFT_SCHEMA
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn sorts_keys_and_fixes_seconds() {
        let v = json!({"b": 1, "a": 0.4});
        assert_eq!(canonical_json(&v), r#"{"a":0.400,"b":1}"#);
    }

    #[test]
    fn hash_is_stable() {
        let v = json!({"role": "hook", "in_s": 0.4});
        let once = blake3_canonical(&v);
        let twice = blake3_canonical(&v);
        assert_eq!(once, twice);
        assert_eq!(once.len(), 64);
    }
}
