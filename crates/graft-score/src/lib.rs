// SPDX-License-Identifier: Apache-2.0

//! Parse, validate, and hash the graft IR. The schema in `/schema` is
//! normative; this crate must not invent fields.

mod canonical;
mod error;
mod ident;
mod io;
mod material;
mod role;
mod scion;
mod score;
mod span;
mod time_map;

pub use canonical::{blake3_canonical, canonical_json};
pub use error::{Error, Result};
pub use ident::require_slot_id;
pub use io::{load_json, load_scion, load_score, load_time_map, save_json};
pub use material::{is_material_id, material_id};
pub use role::{Layer, Role, SPINE_ROLES};
pub use scion::{Binding, Dest, Encoder, RateControl, RateControlMode, Scion};
pub use score::{Clock, Score, Slot, Window, DEFAULT_SPILL_S};
pub use span::{parse_range, parse_wh, Range};
pub use time_map::{TimeMap, TimeMapEntry};

/// Format id written on every document (`graft` field). Not a product release.
pub const GRAFT_SCHEMA: &str = "0.1.0";
