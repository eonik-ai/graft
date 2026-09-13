// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use crate::canonical::require_graft_version;
use crate::ident::{require_slot_id, validate_span};
use crate::score::Score;
use crate::{Error, GRAFT_SCHEMA};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimeMapEntry {
    pub span: [f64; 2],
    pub slot: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimeMap {
    pub graft: String,
    pub scion: String,
    pub dest_id: String,
    pub entries: Vec<TimeMapEntry>,
}

impl TimeMap {
    pub fn validate(&self) -> Result<(), Error> {
        require_graft_version(&self.graft)?;
        if self.scion.is_empty() || self.dest_id.is_empty() {
            return Err(Error::invalid("time-map scion and dest_id are required"));
        }
        if self.entries.is_empty() {
            return Err(Error::invalid("time-map entries must be non-empty"));
        }
        for entry in &self.entries {
            require_slot_id(&entry.slot)?;
            validate_span(entry.span, &format!("time-map {}", entry.slot))?;
        }
        Ok(())
    }

    /// Identity map: dest clock == score clock (no retime).
    pub fn from_score(score: &Score, scion_id: &str, dest_id: &str) -> Self {
        let mut slots = score.slots.clone();
        slots.sort_by(|a, b| {
            a.start()
                .partial_cmp(&b.start())
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.id.cmp(&b.id))
        });
        Self {
            graft: GRAFT_SCHEMA.into(),
            scion: scion_id.into(),
            dest_id: dest_id.into(),
            entries: slots
                .into_iter()
                .map(|s| TimeMapEntry {
                    span: s.span,
                    slot: s.id,
                })
                .collect(),
        }
    }
}
