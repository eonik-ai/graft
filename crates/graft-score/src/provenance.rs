// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use crate::canonical::require_graft_version;
use crate::{Error, FrameRange, Result};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildRecord {
    pub graft: String,
    pub id: String,
    pub scion: String,
    pub scion_hash: String,
    pub dest_id: String,
    pub time_map: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
}

impl BuildRecord {
    pub fn validate(&self) -> Result<()> {
        require_graft_version(&self.graft)?;
        if self.id.is_empty()
            || self.scion.is_empty()
            || self.scion_hash.is_empty()
            || self.dest_id.is_empty()
            || self.time_map.is_empty()
        {
            return Err(Error::invalid("build identity fields must be non-empty"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FeedbackResolution {
    pub slots: Vec<String>,
    pub kerfs: Vec<Vec<String>>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Feedback {
    pub graft: String,
    pub id: String,
    pub build: String,
    pub kind: String,
    pub range: FrameRange,
    #[serde(default)]
    pub raw: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved: Option<FeedbackResolution>,
}

impl Feedback {
    pub fn validate(&self) -> Result<()> {
        require_graft_version(&self.graft)?;
        if self.id.is_empty() || self.build.is_empty() || self.kind.is_empty() {
            return Err(Error::invalid(
                "feedback id, build, and kind must be non-empty",
            ));
        }
        self.range.validate("feedback.range")
    }
}
