// SPDX-License-Identifier: Apache-2.0

use crate::Error;

pub const SPINE_ROLES: &[Role] = &[Role::Hook, Role::Body, Role::Proof, Role::Cta];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Hook,
    Body,
    Proof,
    Cta,
    Vo,
    Captions,
    Bed,
    Brand,
}

impl Role {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Hook => "hook",
            Self::Body => "body",
            Self::Proof => "proof",
            Self::Cta => "cta",
            Self::Vo => "vo",
            Self::Captions => "captions",
            Self::Bed => "bed",
            Self::Brand => "brand",
        }
    }

    pub fn parse(name: &str) -> Result<Self, Error> {
        match name {
            "hook" => Ok(Self::Hook),
            "body" => Ok(Self::Body),
            "proof" => Ok(Self::Proof),
            "cta" => Ok(Self::Cta),
            "vo" => Ok(Self::Vo),
            "captions" => Ok(Self::Captions),
            "bed" => Ok(Self::Bed),
            "brand" => Ok(Self::Brand),
            other => Err(Error::invalid(format!("unknown role {other:?}"))),
        }
    }

    pub fn is_spine(self) -> bool {
        SPINE_ROLES.contains(&self)
    }
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Layer {
    Base,
    Copy,
    Grade,
    Legal,
}
