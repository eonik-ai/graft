// SPDX-License-Identifier: Apache-2.0

use std::path::{Path, PathBuf};

pub const SCORE_FILE: &str = "score.json";
pub const SCIONS_DIR: &str = "scions";
pub const FEEDBACK_DIR: &str = "feedback";

pub fn score(dir: &Path) -> PathBuf {
    dir.join(SCORE_FILE)
}

pub fn scions(dir: &Path) -> PathBuf {
    dir.join(SCIONS_DIR)
}

pub fn scion(dir: &Path, id: &str) -> PathBuf {
    scions(dir).join(format!("{id}.json"))
}

pub fn feedback_dir(dir: &Path) -> PathBuf {
    dir.join(FEEDBACK_DIR)
}

pub fn feedback(dir: &Path, id: &str) -> PathBuf {
    feedback_dir(dir).join(format!("{id}.json"))
}

/// Device store: blobs, action cache, build artifacts.
pub fn graft_dir(dir: &Path) -> PathBuf {
    dir.join(".graft")
}

pub fn head(dir: &Path) -> PathBuf {
    graft_dir(dir).join("HEAD")
}

pub fn builds(dir: &Path) -> PathBuf {
    graft_dir(dir).join("builds")
}

pub fn build(dir: &Path, id: &str) -> PathBuf {
    builds(dir).join(id)
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
