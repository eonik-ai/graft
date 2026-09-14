// SPDX-License-Identifier: Apache-2.0

//! Two stores: namespaced content-addressed blobs, and an action cache.
//! `ActionKey` (recipe + dest + encoder) is not `BlobId` (bytes).

use std::path::{Path, PathBuf};

mod fs;
mod http;
mod id;
mod kind;
mod memory;
mod object;
mod remote;
mod s3;

pub use fs::Fs;
pub use http::Http;
pub use id::{ActionKey, ActionResult, BlobId, CacheEntry};
pub use kind::Kind;
pub use memory::Memory;
pub use object::Object;
pub use remote::Remote;
pub use s3::S3;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Invalid(String),
    #[error("missing {0}")]
    Missing(String),
    #[error("{path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

impl Error {
    pub(crate) fn invalid(msg: impl Into<String>) -> Self {
        Self::Invalid(msg.into())
    }

    pub(crate) fn io(path: impl AsRef<Path>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.as_ref().to_path_buf(),
            source,
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// Device or server store. Blobs are namespaced by [`Kind`].
/// The action cache maps [`ActionKey`] → [`CacheEntry`].
pub trait Store {
    fn put_blob(&self, kind: Kind, bytes: &[u8]) -> Result<BlobId>;
    fn get_blob(&self, kind: Kind, id: &BlobId) -> Result<Vec<u8>>;
    fn contains_blob(&self, kind: Kind, id: &BlobId) -> bool;
    fn get_action(&self, key: &ActionKey) -> Result<Option<CacheEntry>>;
    fn put_action(&self, key: &ActionKey, entry: CacheEntry) -> Result<()>;
}
