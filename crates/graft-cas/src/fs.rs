// SPDX-License-Identifier: Apache-2.0

use std::fs;
use std::path::{Path, PathBuf};

use crate::id::{ActionKey, BlobId, CacheEntry};
use crate::kind::Kind;
use crate::{Error, Result, Store};

/// Device-local store under `{root}/blobs/<kind>/<aa>/<hex>` and
/// `{root}/actions/<aa>/<hex>`.
pub struct Fs {
    root: PathBuf,
}

impl Fs {
    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        fs::create_dir_all(root.join("blobs")).map_err(|e| Error::io(root.join("blobs"), e))?;
        fs::create_dir_all(root.join("actions")).map_err(|e| Error::io(root.join("actions"), e))?;
        Ok(Self { root })
    }

    fn blob_path(&self, kind: Kind, id: &BlobId) -> PathBuf {
        let hex = id.hex();
        self.root
            .join("blobs")
            .join(kind.as_str())
            .join(&hex[..2])
            .join(&hex[2..])
    }

    fn action_path(&self, key: &ActionKey) -> PathBuf {
        let hex = key.hex();
        self.root.join("actions").join(&hex[..2]).join(&hex[2..])
    }

    fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
        }
        if path.exists() {
            return Ok(());
        }
        let tmp = path.with_extension("tmp");
        fs::write(&tmp, bytes).map_err(|e| Error::io(&tmp, e))?;
        fs::rename(&tmp, path).map_err(|e| Error::io(path, e))?;
        Ok(())
    }
}

impl Store for Fs {
    fn put_blob(&self, kind: Kind, bytes: &[u8]) -> Result<BlobId> {
        let id = BlobId::of(bytes);
        let path = self.blob_path(kind, &id);
        Self::write_atomic(&path, bytes)?;
        Ok(id)
    }

    fn get_blob(&self, kind: Kind, id: &BlobId) -> Result<Vec<u8>> {
        let path = self.blob_path(kind, id);
        fs::read(&path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                Error::Missing(format!("{kind}/{id}"))
            } else {
                Error::io(&path, e)
            }
        })
    }

    fn contains_blob(&self, kind: Kind, id: &BlobId) -> bool {
        self.blob_path(kind, id).is_file()
    }

    fn get_action(&self, key: &ActionKey) -> Result<Option<CacheEntry>> {
        let path = self.action_path(key);
        match fs::read(&path) {
            Ok(bytes) => {
                let entry: CacheEntry = serde_json::from_slice(&bytes)
                    .map_err(|e| Error::invalid(format!("{}: {e}", path.display())))?;
                Ok(Some(entry))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(Error::io(&path, e)),
        }
    }

    fn put_action(&self, key: &ActionKey, entry: CacheEntry) -> Result<()> {
        let path = self.action_path(key);
        let bytes = serde_json::to_vec(&entry).map_err(|e| Error::invalid(e.to_string()))?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
        }
        let tmp = path.with_extension("tmp");
        fs::write(&tmp, bytes).map_err(|e| Error::io(&tmp, e))?;
        fs::rename(&tmp, &path).map_err(|e| Error::io(&path, e))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_blob_and_action() {
        let dir = std::env::temp_dir().join(format!("graft-cas-fs-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let cas = Fs::open(&dir).unwrap();
        let a = cas.put_blob(Kind::Material, b"body-v1").unwrap();
        let b = cas.put_blob(Kind::Material, b"body-v1").unwrap();
        assert_eq!(a, b);
        assert_eq!(cas.get_blob(Kind::Material, &a).unwrap(), b"body-v1");
        let key = ActionKey::from_canonical(&serde_json::json!({"k":1}));
        cas.put_action(
            &key,
            CacheEntry {
                kind: Kind::SlotEncode,
                blob: a.clone(),
            },
        )
        .unwrap();
        assert_eq!(cas.get_action(&key).unwrap().unwrap().blob, a);
        assert!(cas
            .get_action(&ActionKey::from_canonical(&serde_json::json!({"k":2})))
            .unwrap()
            .is_none());
        let _ = fs::remove_dir_all(&dir);
    }
}
