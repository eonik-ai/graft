// SPDX-License-Identifier: Apache-2.0

use graft_score::Dest;

use crate::graph::ConcatAction;

/// Join slot_encodes + kerfs into dest bytes. Concat output is never essence.
pub trait ConcatBackend {
    fn enabled(&self) -> bool {
        false
    }

    fn concat(&self, req: &ConcatRequest<'_>) -> Result<Vec<u8>, ConcatError>;
}

pub struct ConcatPartBytes {
    pub empty: bool,
    pub bytes: Vec<u8>,
    pub duration_s: f64,
    pub trim_head_frames: u64,
    pub trim_tail_frames: u64,
}

pub struct ConcatRequest<'a> {
    pub action: &'a ConcatAction,
    pub dest: &'a Dest,
    pub parts: &'a [ConcatPartBytes],
    pub audio_parts: &'a [ConcatPartBytes],
}

#[derive(Debug, thiserror::Error)]
pub enum ConcatError {
    #[error("no concat backend for this dest")]
    Unimplemented,
    #[error("{0}")]
    Invalid(String),
}

pub struct Unimplemented;

impl ConcatBackend for Unimplemented {
    fn concat(&self, _req: &ConcatRequest<'_>) -> Result<Vec<u8>, ConcatError> {
        Err(ConcatError::Unimplemented)
    }
}
