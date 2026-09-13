// SPDX-License-Identifier: Apache-2.0

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use crate::id::{ActionKey, BlobId, CacheEntry};
use crate::kind::Kind;
use crate::{Error, Result, Store};

const CHUNK: usize = 1024 * 1024;

/// Object-store layout that mirrors the local CAS. Device and server share
/// this transport; they do not grow a second IR.
pub struct Object {
    root: PathBuf,
}

impl Object {
    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        fs::create_dir_all(root.join("blobs")).map_err(|e| Error::io(root.join("blobs"), e))?;
        fs::create_dir_all(root.join("actions")).map_err(|e| Error::io(root.join("actions"), e))?;
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
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

    pub fn missing_blobs(&self, wanted: &[(Kind, BlobId)]) -> Vec<(Kind, BlobId)> {
        wanted
            .iter()
            .filter(|(kind, id)| !self.contains_blob(*kind, id))
            .cloned()
            .collect()
    }

    pub fn list_blobs(&self) -> Result<Vec<(Kind, BlobId)>> {
        let mut out = Vec::new();
        let blobs = self.root.join("blobs");
        if !blobs.is_dir() {
            return Ok(out);
        }
        for kind_ent in fs::read_dir(&blobs).map_err(|e| Error::io(&blobs, e))? {
            let kind_ent = kind_ent.map_err(|e| Error::io(&blobs, e))?;
            let kind = match Kind::parse(&kind_ent.file_name().to_string_lossy()) {
                Ok(kind) => kind,
                Err(_) => continue,
            };
            let kind_dir = kind_ent.path();
            for shard in fs::read_dir(&kind_dir).map_err(|e| Error::io(&kind_dir, e))? {
                let shard = shard.map_err(|e| Error::io(&kind_dir, e))?;
                if !shard.path().is_dir() {
                    continue;
                }
                let prefix = shard.file_name().to_string_lossy().into_owned();
                for file in fs::read_dir(shard.path()).map_err(|e| Error::io(shard.path(), e))? {
                    let file = file.map_err(|e| Error::io(shard.path(), e))?;
                    if file.path().extension().is_some() {
                        continue;
                    }
                    let hex = format!("{prefix}{}", file.file_name().to_string_lossy());
                    if let Ok(id) = BlobId::parse(&format!("blake3:{hex}")) {
                        out.push((kind, id));
                    }
                }
            }
        }
        Ok(out)
    }

    pub fn list_actions(&self) -> Result<Vec<(ActionKey, CacheEntry)>> {
        let mut out = Vec::new();
        let actions = self.root.join("actions");
        if !actions.is_dir() {
            return Ok(out);
        }
        for shard in fs::read_dir(&actions).map_err(|e| Error::io(&actions, e))? {
            let shard = shard.map_err(|e| Error::io(&actions, e))?;
            if !shard.path().is_dir() {
                continue;
            }
            let prefix = shard.file_name().to_string_lossy().into_owned();
            for file in fs::read_dir(shard.path()).map_err(|e| Error::io(shard.path(), e))? {
                let file = file.map_err(|e| Error::io(shard.path(), e))?;
                if file.path().extension().is_some() {
                    continue;
                }
                let hex = format!("{prefix}{}", file.file_name().to_string_lossy());
                if let Ok(key) = ActionKey::parse(&hex) {
                    if let Some(entry) = self.get_action(&key)? {
                        out.push((key, entry));
                    }
                }
            }
        }
        Ok(out)
    }

    pub fn push_from(&self, local: &dyn Store, blobs: &[(Kind, BlobId)]) -> Result<Vec<String>> {
        let mut copied = Vec::new();
        for (kind, id) in blobs {
            if self.contains_blob(*kind, id) {
                continue;
            }
            let bytes = local.get_blob(*kind, id)?;
            resumable_write(&self.blob_path(*kind, id), &bytes)?;
            copied.push(format!("{kind}/{id}"));
        }
        Ok(copied)
    }

    pub fn pull_into(&self, local: &dyn Store, blobs: &[(Kind, BlobId)]) -> Result<Vec<String>> {
        let mut copied = Vec::new();
        for (kind, id) in blobs {
            if local.contains_blob(*kind, id) {
                continue;
            }
            let bytes = self.get_blob(*kind, id)?;
            let got = local.put_blob(*kind, &bytes)?;
            if &got != id {
                return Err(Error::invalid(format!(
                    "object store blob {id} hashed as {got}"
                )));
            }
            copied.push(format!("{kind}/{id}"));
        }
        Ok(copied)
    }
}

impl Store for Object {
    fn put_blob(&self, kind: Kind, bytes: &[u8]) -> Result<BlobId> {
        let id = BlobId::of(bytes);
        resumable_write(&self.blob_path(kind, &id), bytes)?;
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
        let bytes = serde_json::to_vec(&entry).map_err(|e| Error::invalid(e.to_string()))?;
        resumable_write(&self.action_path(key), &bytes)
    }
}

fn resumable_write(path: &Path, bytes: &[u8]) -> Result<()> {
    if path.exists() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
    }
    let part = path.with_extension("part");
    let mut start = 0u64;
    if part.exists() {
        start = part.metadata().map_err(|e| Error::io(&part, e))?.len();
    }
    if start > bytes.len() as u64 {
        fs::remove_file(&part).map_err(|e| Error::io(&part, e))?;
        start = 0;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&part)
        .map_err(|e| Error::io(&part, e))?;
    if start > 0 {
        file.seek(SeekFrom::Start(start))
            .map_err(|e| Error::io(&part, e))?;
    }
    let mut offset = start as usize;
    while offset < bytes.len() {
        let end = (offset + CHUNK).min(bytes.len());
        file.write_all(&bytes[offset..end])
            .map_err(|e| Error::io(&part, e))?;
        offset = end;
    }
    file.flush().map_err(|e| Error::io(&part, e))?;
    drop(file);
    fs::rename(&part, path).map_err(|e| Error::io(path, e))?;
    let _ = File::open(path).and_then(|mut f| {
        let mut buf = [0u8; 1];
        f.read(&mut buf)
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Memory;

    #[test]
    fn missing_then_push_then_pull() {
        let dir = std::env::temp_dir().join(format!("graft-object-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let remote = Object::open(dir.join("remote")).unwrap();
        let local = Memory::new();
        let id = local.put_blob(Kind::Material, b"hook-v3").unwrap();
        assert_eq!(
            remote.missing_blobs(&[(Kind::Material, id.clone())]),
            vec![(Kind::Material, id.clone())]
        );
        remote
            .push_from(&local, &[(Kind::Material, id.clone())])
            .unwrap();
        assert!(remote
            .missing_blobs(&[(Kind::Material, id.clone())])
            .is_empty());
        let other = Memory::new();
        remote
            .pull_into(&other, &[(Kind::Material, id.clone())])
            .unwrap();
        assert_eq!(other.get_blob(Kind::Material, &id).unwrap(), b"hook-v3");
        let _ = fs::remove_dir_all(&dir);
    }
}
