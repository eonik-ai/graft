// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use crate::kind::Kind;
use crate::{Error, Result};

fn looks_like_hex64(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit())
}

/// BLAKE3 of **bytes**. Displayed as `blake3:<hex>`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BlobId(String);

impl BlobId {
    pub fn of(bytes: &[u8]) -> Self {
        Self(graft_score::material_id(bytes))
    }

    pub fn parse(s: &str) -> Result<Self> {
        if !graft_score::is_material_id(s) {
            return Err(Error::invalid(format!(
                "blob id must be blake3:<64 hex> (got {s:?})"
            )));
        }
        Ok(Self(s.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn hex(&self) -> &str {
        &self.0["blake3:".len()..]
    }
}

impl std::fmt::Display for BlobId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// BLAKE3 of canonical action JSON (`slot_encode`, `kerf`, `scion_hash`).
/// Hex only — not a content hash of encoded bytes.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ActionKey(String);

impl ActionKey {
    pub fn from_canonical(value: &serde_json::Value) -> Self {
        Self(graft_score::blake3_canonical(value))
    }

    pub fn parse(s: &str) -> Result<Self> {
        let hex = s.strip_prefix("blake3:").unwrap_or(s);
        if !looks_like_hex64(hex) {
            return Err(Error::invalid(format!(
                "action key must be 64 hex (got {s:?})"
            )));
        }
        Ok(Self(hex.to_string()))
    }

    pub fn hex(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ActionKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Action cache value: which namespaced blob this action produced.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CacheEntry {
    pub kind: Kind,
    pub blob: BlobId,
}
