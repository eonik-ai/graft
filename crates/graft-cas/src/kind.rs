// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use crate::{Error, Result};

/// Namespace for a blob. Concat is a linker product, never essence.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Material,
    SlotEncode,
    AudioEncode,
    Kerf,
    Concat,
    Captions,
    AudioMix,
    OverlayMix,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Material => "material",
            Self::SlotEncode => "slot_encode",
            Self::AudioEncode => "audio_encode",
            Self::Kerf => "kerf",
            Self::Concat => "concat",
            Self::Captions => "captions",
            Self::AudioMix => "audio_mix",
            Self::OverlayMix => "overlay_mix",
        }
    }

    pub fn parse(name: &str) -> Result<Self> {
        match name {
            "material" => Ok(Self::Material),
            "slot_encode" => Ok(Self::SlotEncode),
            "audio_encode" => Ok(Self::AudioEncode),
            "kerf" => Ok(Self::Kerf),
            "concat" => Ok(Self::Concat),
            "captions" => Ok(Self::Captions),
            "audio_mix" => Ok(Self::AudioMix),
            "overlay_mix" => Ok(Self::OverlayMix),
            other => Err(Error::invalid(format!("unknown blob kind {other:?}"))),
        }
    }
}

impl std::fmt::Display for Kind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
