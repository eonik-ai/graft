// SPDX-License-Identifier: Apache-2.0

//! Machine-local map from a material hash to the file that was ingested.
//! The recipe stores the hash. This file is how a guest timeline opens it.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::paths;

pub fn index_path(dir: &Path) -> PathBuf {
    paths::graft_dir(dir).join("media.json")
}

pub fn load(dir: &Path) -> Result<BTreeMap<String, String>> {
    let path = index_path(dir);
    if !path.is_file() {
        return Ok(BTreeMap::new());
    }
    let text =
        std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    serde_json::from_str(&text).with_context(|| format!("parse {}", path.display()))
}

pub fn remember(dir: &Path, material: &str, file: &Path) -> Result<()> {
    let mut map = load(dir)?;
    let abs = if file.is_absolute() {
        file.to_path_buf()
    } else {
        dir.join(file)
    };
    let stored = abs.canonicalize().unwrap_or(abs);
    map.insert(material.to_string(), stored.to_string_lossy().into_owned());
    let path = index_path(dir);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    let mut text = serde_json::to_string_pretty(&map)?;
    if !text.ends_with('\n') {
        text.push('\n');
    }
    std::fs::write(&path, text).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}
