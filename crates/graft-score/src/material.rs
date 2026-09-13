// SPDX-License-Identifier: Apache-2.0

use crate::ident::looks_like_hex64;
use crate::Error;

pub fn material_id(bytes: &[u8]) -> String {
    format!("blake3:{}", blake3::hash(bytes).to_hex())
}

pub fn is_material_id(s: &str) -> bool {
    s.strip_prefix("blake3:").is_some_and(looks_like_hex64)
}

pub fn require_material(s: &str) -> Result<(), Error> {
    if is_material_id(s) {
        Ok(())
    } else {
        Err(Error::invalid(format!(
            "material must be blake3:<64 hex> (got {s:?})"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn material_id_is_blake3_hex() {
        let id = material_id(b"hook");
        assert!(is_material_id(&id));
    }
}
