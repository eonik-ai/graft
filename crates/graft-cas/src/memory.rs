// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use std::sync::Mutex;

use crate::id::{ActionKey, BlobId, CacheEntry};
use crate::kind::Kind;
use crate::{Error, Result, Store};

/// Ephemeral store. For tests and in-process device use.
#[derive(Debug, Default)]
pub struct Memory {
    blobs: Mutex<HashMap<(Kind, String), Vec<u8>>>,
    actions: Mutex<HashMap<String, CacheEntry>>,
}

impl Memory {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Store for Memory {
    fn put_blob(&self, kind: Kind, bytes: &[u8]) -> Result<BlobId> {
        let id = BlobId::of(bytes);
        self.blobs
            .lock()
            .map_err(|_| Error::invalid("cas lock poisoned"))?
            .insert((kind, id.as_str().to_string()), bytes.to_vec());
        Ok(id)
    }

    fn get_blob(&self, kind: Kind, id: &BlobId) -> Result<Vec<u8>> {
        self.blobs
            .lock()
            .map_err(|_| Error::invalid("cas lock poisoned"))?
            .get(&(kind, id.as_str().to_string()))
            .cloned()
            .ok_or_else(|| Error::Missing(format!("{kind}/{id}")))
    }

    fn contains_blob(&self, kind: Kind, id: &BlobId) -> bool {
        self.blobs
            .lock()
            .map(|m| m.contains_key(&(kind, id.as_str().to_string())))
            .unwrap_or(false)
    }

    fn get_action(&self, key: &ActionKey) -> Result<Option<CacheEntry>> {
        Ok(self
            .actions
            .lock()
            .map_err(|_| Error::invalid("cas lock poisoned"))?
            .get(key.hex())
            .cloned())
    }

    fn put_action(&self, key: &ActionKey, entry: CacheEntry) -> Result<()> {
        self.actions
            .lock()
            .map_err(|_| Error::invalid("cas lock poisoned"))?
            .insert(key.hex().to_string(), entry);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn namespaces_keep_same_bytes_apart() {
        let cas = Memory::new();
        let material = cas.put_blob(Kind::Material, b"same").unwrap();
        let encode = cas.put_blob(Kind::SlotEncode, b"same").unwrap();
        assert_eq!(material, encode);
        assert!(cas.contains_blob(Kind::Material, &material));
        assert!(cas.contains_blob(Kind::SlotEncode, &encode));
        assert!(!cas.contains_blob(Kind::Kerf, &material));
    }

    #[test]
    fn action_cache_is_not_the_blob_id() {
        let cas = Memory::new();
        let blob = cas.put_blob(Kind::SlotEncode, b"body-v1").unwrap();
        let key = ActionKey::from_canonical(&serde_json::json!({"role":"body"}));
        assert_ne!(key.hex(), blob.hex());
        cas.put_action(&key, CacheEntry::new(Kind::SlotEncode, blob.clone(), 7))
            .unwrap();
        let hit = cas.get_action(&key).unwrap().unwrap();
        assert_eq!(hit.blob, blob);
        let other = ActionKey::from_canonical(&serde_json::json!({"role":"hook"}));
        assert!(cas.get_action(&other).unwrap().is_none());
    }
}
