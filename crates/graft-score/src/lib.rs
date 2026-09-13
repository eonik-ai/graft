// SPDX-License-Identifier: Apache-2.0

//! Parse, validate, and hash the graft IR. The schema in `/schema` is
//! normative; this crate must not invent fields.

mod canonical;
mod error;
mod ident;
mod io;
mod material;
mod provenance;
mod role;
mod scion;
mod score;
mod span;
mod time;
mod time_map;
mod workspace;

pub use canonical::{blake3_canonical, canonical_json};
pub use error::{Error, Result};
pub use ident::require_slot_id;
pub use io::{load_json, load_scion, load_score, load_time_map, save_json};
pub use material::{is_material_id, material_id};
pub use provenance::{BuildRecord, Feedback, FeedbackResolution};
pub use role::{Layer, Role, SPINE_ROLES};
pub use scion::{
    AudioBinding, Binding, BindingLayer, ChangeRequest, Dest, Encoder, RateControl,
    RateControlMode, Scion,
};
pub use score::{Clock, Score, Slot, Window, DEFAULT_SPILL_FRAMES};
pub use span::{parse_range, parse_wh, Range};
pub use time::{FrameRange, FrameRate, TimedRange};
pub use time_map::{TimeMap, TimeMapEntry};
pub use workspace::{
    effective_bindings, merge_scions, resolve_scion, semantic_diff, validate_family, MergeConflict,
    MergeOutcome, SemanticChange,
};

/// Format id written on every document (`graft` field). Not a product release.
pub const GRAFT_SCHEMA: &str = "0.2.0";
