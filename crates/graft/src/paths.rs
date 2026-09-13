// SPDX-License-Identifier: Apache-2.0

use std::path::{Path, PathBuf};

pub const SCORE_FILE: &str = "score.json";
pub const SCION_FILE: &str = "scion.json";
pub const TIME_MAP_FILE: &str = "time-map.json";

pub fn score(dir: &Path) -> PathBuf {
    dir.join(SCORE_FILE)
}

pub fn scion(dir: &Path) -> PathBuf {
    dir.join(SCION_FILE)
}

pub fn time_map(dir: &Path) -> PathBuf {
    dir.join(TIME_MAP_FILE)
}

/// Device store: blobs, action cache, build artifacts.
pub fn graft_dir(dir: &Path) -> PathBuf {
    dir.join(".graft")
}

pub fn resolve_in_dir(dir: &Path, p: PathBuf) -> PathBuf {
    if p.is_absolute() {
        p
    } else {
        dir.join(p)
    }
}

pub fn print_json(value: &impl serde::Serialize) -> anyhow::Result<()> {
    let mut text = serde_json::to_string_pretty(value)?;
    if !text.ends_with('\n') {
        text.push('\n');
    }
    print!("{text}");
    Ok(())
}
