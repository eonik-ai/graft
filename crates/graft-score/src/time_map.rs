// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use crate::canonical::require_graft_version;
use crate::ident::require_slot_id;
use crate::score::Score;
use crate::{Binding, Error, FrameRange, FrameRate, TimedRange, GRAFT_SCHEMA};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimeMapEntry {
    pub dest: FrameRange,
    pub source: TimedRange,
    pub slot: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimeMap {
    pub graft: String,
    pub build: String,
    pub scion: String,
    pub scion_hash: String,
    pub dest_id: String,
    pub rate: FrameRate,
    pub entries: Vec<TimeMapEntry>,
}

impl TimeMap {
    pub fn validate(&self) -> Result<(), Error> {
        require_graft_version(&self.graft)?;
        if self.build.is_empty()
            || self.scion.is_empty()
            || self.scion_hash.is_empty()
            || self.dest_id.is_empty()
        {
            return Err(Error::invalid(
                "time-map build, scion, scion_hash, and dest_id are required",
            ));
        }
        self.rate.validate("time-map.rate")?;
        if self.entries.is_empty() {
            return Err(Error::invalid("time-map entries must be non-empty"));
        }
        for entry in &self.entries {
            require_slot_id(&entry.slot)?;
            entry
                .dest
                .validate(&format!("time-map {}.dest", entry.slot))?;
            entry
                .source
                .validate(&format!("time-map {}.source", entry.slot))?;
        }
        Ok(())
    }

    /// Build map: dest ranges address semantic slots; source ranges preserve
    /// the exact selected material time. Retime may make these rates differ.
    pub fn from_bindings(
        score: &Score,
        scion_id: &str,
        scion_hash: &str,
        dest_id: &str,
        rate: FrameRate,
        bindings: &std::collections::BTreeMap<String, Binding>,
    ) -> Self {
        Self {
            graft: GRAFT_SCHEMA.into(),
            build: scion_hash.into(),
            scion: scion_id.into(),
            scion_hash: scion_hash.into(),
            dest_id: dest_id.into(),
            rate,
            entries: score
                .spine()
                .into_iter()
                .filter_map(|slot| {
                    bindings.get(&slot.id).map(|binding| TimeMapEntry {
                        dest: slot.range,
                        source: binding.source,
                        slot: slot.id.clone(),
                    })
                })
                .collect(),
        }
    }
}
