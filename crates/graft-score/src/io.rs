// SPDX-License-Identifier: Apache-2.0

use std::fs;
use std::path::Path;

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::{Error, Result, Scion, Score, TimeMap};

pub fn load_json<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let text = fs::read_to_string(path).map_err(|e| Error::io(path, e))?;
    serde_json::from_str(&text).map_err(|e| Error::json(path, e))
}

pub fn save_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let mut text = serde_json::to_string_pretty(value).map_err(|e| Error::json(path, e))?;
    if !text.ends_with('\n') {
        text.push('\n');
    }
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
        }
    }
    fs::write(path, text).map_err(|e| Error::io(path, e))
}

pub fn load_score(path: &Path) -> Result<Score> {
    let score: Score = load_json(path)?;
    score.validate()?;
    Ok(score)
}

pub fn load_scion(path: &Path) -> Result<Scion> {
    let scion: Scion = load_json(path)?;
    scion.validate()?;
    Ok(scion)
}

pub fn load_time_map(path: &Path) -> Result<TimeMap> {
    let time_map: TimeMap = load_json(path)?;
    time_map.validate()?;
    Ok(time_map)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn example_dir() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/hook-v3-body-v1-9x16")
    }

    #[test]
    fn loads_worked_example() {
        let dir = example_dir();
        let score = load_score(&dir.join("score.json")).unwrap();
        let scion = load_scion(&dir.join("scion.json")).unwrap();
        let time_map = load_time_map(&dir.join("time-map.json")).unwrap();
        scion.validate_against_score(&score).unwrap();
        assert_eq!(time_map.dest_id, "9x16");
        assert_eq!(score.slots.len(), 3);
        assert_eq!(scion.dest.id, "9x16");
    }
}
