// SPDX-License-Identifier: Apache-2.0

use graft_score::{Dest, Slot};

use crate::grain::Grain;
use crate::graph::{KerfAction, SlotEncodeAction};

/// A backend that produces `slot_encode` / kerf **bytes**. The pipeline
/// puts them in CAS and records the action cache. Implement this against
/// [`SlotEncodeAction`] / [`KerfAction`], not against two mp4s.
pub trait EncodeBackend {
    fn enabled(&self) -> bool {
        false
    }

    fn grain(&self, dest: &Dest) -> Grain {
        Grain::from_dest(dest)
    }

    fn encode_slot(&self, req: &SlotEncodeRequest<'_>) -> Result<Vec<u8>, EncodeError>;

    fn encode_kerf(&self, req: &KerfEncodeRequest<'_>) -> Result<Vec<u8>, EncodeError>;
}

pub struct SlotEncodeRequest<'a> {
    pub action: &'a SlotEncodeAction,
    pub slot: &'a Slot,
    pub dest: &'a Dest,
    pub material: &'a [u8],
}

pub struct KerfEncodeRequest<'a> {
    pub action: &'a KerfAction,
    pub dest: &'a Dest,
    pub left: &'a [u8],
    pub right: &'a [u8],
}

#[derive(Debug, thiserror::Error)]
pub enum EncodeError {
    #[error("no encode backend for this dest")]
    Unimplemented,
    #[error("{0}")]
    Invalid(String),
}

/// Honest placeholder for dests that are not `graft-intra`.
pub struct Unimplemented;

impl EncodeBackend for Unimplemented {
    fn encode_slot(&self, _req: &SlotEncodeRequest<'_>) -> Result<Vec<u8>, EncodeError> {
        Err(EncodeError::Unimplemented)
    }

    fn encode_kerf(&self, _req: &KerfEncodeRequest<'_>) -> Result<Vec<u8>, EncodeError> {
        Err(EncodeError::Unimplemented)
    }
}
