// SPDX-License-Identifier: Apache-2.0

use crate::http::Http;
use crate::id::{ActionKey, BlobId, CacheEntry};
use crate::kind::Kind;
use crate::object::Object;
use crate::s3::S3;
use crate::{Error, Result, Store};

/// Filesystem, HTTP, or S3 remote. Same blob/action layout. No remote compile.
pub enum Remote {
    Fs(Object),
    Http(Http),
    S3(S3),
}

impl Remote {
    pub fn open(spec: &str) -> Result<Self> {
        if spec.starts_with("s3://") {
            Ok(Self::S3(S3::open(spec)?))
        } else if spec.starts_with("http://") || spec.starts_with("https://") {
            Ok(Self::Http(Http::open(spec)?))
        } else if spec.is_empty() {
            Err(Error::invalid("remote is empty"))
        } else {
            Ok(Self::Fs(Object::open(spec)?))
        }
    }

    pub fn missing_blobs(&self, wanted: &[(Kind, BlobId)]) -> Vec<(Kind, BlobId)> {
        match self {
            Self::Fs(store) => store.missing_blobs(wanted),
            Self::Http(store) => store.missing_blobs(wanted),
            Self::S3(store) => store.missing_blobs(wanted),
        }
    }

    pub fn list_blobs(&self) -> Result<Vec<(Kind, BlobId)>> {
        match self {
            Self::Fs(store) => store.list_blobs(),
            Self::Http(store) => store.list_blobs(),
            Self::S3(store) => store.list_blobs(),
        }
    }

    pub fn list_actions(&self) -> Result<Vec<(ActionKey, CacheEntry)>> {
        match self {
            Self::Fs(store) => store.list_actions(),
            Self::Http(store) => store.list_actions(),
            Self::S3(store) => store.list_actions(),
        }
    }

    pub fn push_from(&self, local: &dyn Store, blobs: &[(Kind, BlobId)]) -> Result<Vec<String>> {
        match self {
            Self::Fs(store) => store.push_from(local, blobs),
            Self::Http(store) => store.push_from(local, blobs),
            Self::S3(store) => store.push_from(local, blobs),
        }
    }

    pub fn pull_into(&self, local: &dyn Store, blobs: &[(Kind, BlobId)]) -> Result<Vec<String>> {
        match self {
            Self::Fs(store) => store.pull_into(local, blobs),
            Self::Http(store) => store.pull_into(local, blobs),
            Self::S3(store) => store.pull_into(local, blobs),
        }
    }
}

impl Store for Remote {
    fn put_blob(&self, kind: Kind, bytes: &[u8]) -> Result<BlobId> {
        match self {
            Self::Fs(store) => store.put_blob(kind, bytes),
            Self::Http(store) => store.put_blob(kind, bytes),
            Self::S3(store) => store.put_blob(kind, bytes),
        }
    }

    fn get_blob(&self, kind: Kind, id: &BlobId) -> Result<Vec<u8>> {
        match self {
            Self::Fs(store) => store.get_blob(kind, id),
            Self::Http(store) => store.get_blob(kind, id),
            Self::S3(store) => store.get_blob(kind, id),
        }
    }

    fn contains_blob(&self, kind: Kind, id: &BlobId) -> bool {
        match self {
            Self::Fs(store) => store.contains_blob(kind, id),
            Self::Http(store) => store.contains_blob(kind, id),
            Self::S3(store) => store.contains_blob(kind, id),
        }
    }

    fn get_action(&self, key: &ActionKey) -> Result<Option<CacheEntry>> {
        match self {
            Self::Fs(store) => store.get_action(key),
            Self::Http(store) => store.get_action(key),
            Self::S3(store) => store.get_action(key),
        }
    }

    fn put_action(&self, key: &ActionKey, entry: CacheEntry) -> Result<()> {
        match self {
            Self::Fs(store) => store.put_action(key, entry),
            Self::Http(store) => store.put_action(key, entry),
            Self::S3(store) => store.put_action(key, entry),
        }
    }
}
