// SPDX-License-Identifier: Apache-2.0

//! Lossy OTIO guest adapter. Depends on graft-score only.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use graft_score::{
    effective_bindings, AudioBinding, Binding, BindingLayer, Error, FrameRange, FrameRate, Layer,
    Result, Scion, Score, TimedRange, GRAFT_SCHEMA,
};

pub fn export_otio(score: &Score, scion: &Scion, out: &Path) -> Result<serde_json::Value> {
    let bindings = effective_bindings(score, scion)?;
    let clips: Vec<serde_json::Value> = score
        .spine()
        .into_iter()
        .filter_map(|slot| {
            bindings.get(&slot.id).map(|binding| {
                serde_json::json!({
                    "OTIO_SCHEMA": "Clip.2",
                    "name": slot.id,
                    "source_range": otio_range(binding.source),
                    "metadata": {
                        "graft": {
                            "slot": slot.id,
                            "role": slot.role.as_str(),
                            "audio": binding.audio
                        }
                    },
                    "media_reference": {
                        "OTIO_SCHEMA": "ExternalReference.1",
                        "target_url": format!("graft://{}", binding.material),
                        "available_range": serde_json::Value::Null,
                        "metadata": {}
                    },
                    "effects": [],
                    "markers": [{
                        "OTIO_SCHEMA": "Marker.2",
                        "name": format!("graft:{}", slot.id),
                        "marked_range": otio_frame_range(slot.range, score.clock.rate),
                        "color": "GREEN",
                        "metadata": {"graft": {"slot": slot.id}}
                    }]
                })
            })
        })
        .collect();
    let loss = loss_report(
        &scion.id,
        [
            "cuts",
            "media references",
            "slot markers",
            "rational timing",
            "dest metadata",
        ],
        ["scion inheritance", "layer strength"],
        [
            "effects",
            "grades",
            "generators",
            "transitions other than cuts",
        ],
        &[],
    );
    let document = serde_json::json!({
        "OTIO_SCHEMA": "Timeline.1",
        "name": scion.id,
        "global_start_time": serde_json::Value::Null,
        "metadata": {
            "graft": {
                "format": GRAFT_SCHEMA,
                "concept": scion.concept,
                "scion": scion.id,
                "dest": scion.dest,
                "loss": loss
            }
        },
        "tracks": {
            "OTIO_SCHEMA": "Stack.1",
            "name": "tracks",
            "source_range": serde_json::Value::Null,
            "metadata": {},
            "effects": [],
            "markers": [],
            "children": [{
                "OTIO_SCHEMA": "Track.1",
                "name": "picture",
                "kind": "Video",
                "source_range": serde_json::Value::Null,
                "metadata": {},
                "effects": [],
                "markers": [],
                "children": clips
            }]
        }
    });
    graft_score::save_json(out, &document)?;
    let loss_path = loss_path(out);
    graft_score::save_json(&loss_path, &loss)?;
    Ok(serde_json::json!({
        "wrote": out,
        "loss_report": loss_path,
        "loss": loss
    }))
}

pub fn import_otio(score: &Score, file: &Path, id: &str) -> Result<(Scion, serde_json::Value)> {
    let doc: serde_json::Value = graft_score::load_json(file)?;
    if doc.get("OTIO_SCHEMA").and_then(|v| v.as_str()) != Some("Timeline.1") {
        return Err(Error::invalid(format!(
            "{} is not an OTIO Timeline.1 document",
            file.display()
        )));
    }
    let graft = doc
        .pointer("/metadata/graft")
        .and_then(|v| v.as_object())
        .ok_or_else(|| {
            Error::invalid("OTIO metadata.graft is required for deterministic import")
        })?;
    let concept = graft
        .get("concept")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::invalid("OTIO metadata.graft.concept is required"))?
        .to_string();
    if concept != score.concept {
        return Err(Error::invalid("OTIO concept does not match score concept"));
    }
    let dest: graft_score::Dest = serde_json::from_value(
        graft
            .get("dest")
            .cloned()
            .ok_or_else(|| Error::invalid("OTIO metadata.graft.dest is required"))?,
    )
    .map_err(|e| Error::invalid(e.to_string()))?;
    let children = doc
        .pointer("/tracks/children/0/children")
        .and_then(|v| v.as_array())
        .ok_or_else(|| Error::invalid("OTIO first video track is required"))?;
    let mut bindings = BTreeMap::new();
    let mut ignored = Vec::new();
    for clip in children {
        let Some(slot) = clip
            .pointer("/metadata/graft/slot")
            .and_then(|v| v.as_str())
        else {
            ignored.push(clip.get("name").cloned().unwrap_or_default());
            continue;
        };
        if score.slot(slot).is_none() {
            ignored.push(serde_json::Value::String(slot.into()));
            continue;
        }
        let target = clip
            .pointer("/media_reference/target_url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::invalid("OTIO clip media_reference.target_url is required"))?;
        let material = target.strip_prefix("graft://").ok_or_else(|| {
            Error::invalid("OTIO import supports graft:// content-addressed media only")
        })?;
        let source = parse_otio_range(
            clip.get("source_range")
                .ok_or_else(|| Error::invalid("OTIO clip source_range is required"))?,
        )?;
        let audio = clip
            .pointer("/metadata/graft/audio")
            .cloned()
            .and_then(|value| serde_json::from_value::<AudioBinding>(value).ok());
        bindings.insert(
            slot.to_string(),
            Binding {
                material: material.into(),
                source,
                params: None,
                audio,
            },
        );
    }
    let scion = Scion {
        graft: GRAFT_SCHEMA.into(),
        id: id.into(),
        concept,
        parent: None,
        change_request: None,
        dest,
        layers: vec![BindingLayer {
            name: Layer::Base,
            bindings,
        }],
    };
    scion.validate_against_score(score)?;
    effective_bindings(score, &scion)?;
    let loss = loss_report(
        id,
        [
            "cuts",
            "graft media references",
            "slot markers",
            "rational timing",
        ],
        [],
        [
            "effects",
            "grades",
            "generators",
            "transitions",
            "non-graft metadata",
        ],
        &ignored,
    );
    Ok((scion, loss))
}

fn loss_report(
    scion: &str,
    preserved: impl IntoIterator<Item = &'static str>,
    flattened: impl IntoIterator<Item = &'static str>,
    lost: impl IntoIterator<Item = &'static str>,
    ignored: &[serde_json::Value],
) -> serde_json::Value {
    serde_json::json!({
        "format": "otio",
        "scion": scion,
        "preserved": preserved.into_iter().collect::<Vec<_>>(),
        "flattened": flattened.into_iter().collect::<Vec<_>>(),
        "lost": lost.into_iter().collect::<Vec<_>>(),
        "ignored_clips": ignored
    })
}

fn otio_range(value: TimedRange) -> serde_json::Value {
    serde_json::json!({
        "OTIO_SCHEMA": "TimeRange.1",
        "start_time": {
            "OTIO_SCHEMA": "RationalTime.1",
            "value": value.range.start,
            "rate": value.rate.as_f64()
        },
        "duration": {
            "OTIO_SCHEMA": "RationalTime.1",
            "value": value.range.duration,
            "rate": value.rate.as_f64()
        },
        "metadata": {
            "graft_rate": value.rate
        }
    })
}

fn otio_frame_range(range: FrameRange, rate: FrameRate) -> serde_json::Value {
    otio_range(TimedRange { rate, range })
}

fn parse_otio_range(value: &serde_json::Value) -> Result<TimedRange> {
    let rate: FrameRate = serde_json::from_value(
        value
            .pointer("/metadata/graft_rate")
            .cloned()
            .ok_or_else(|| Error::invalid("OTIO range metadata.graft_rate is required"))?,
    )
    .map_err(|e| Error::invalid(e.to_string()))?;
    let start = value
        .pointer("/start_time/value")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| Error::invalid("OTIO range start_time.value must be an integer frame"))?;
    let duration = value
        .pointer("/duration/value")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| {
            Error::invalid("OTIO range duration.value must be an integer frame count")
        })?;
    Ok(TimedRange {
        rate,
        range: FrameRange::new(start, duration),
    })
}

fn loss_path(out: &Path) -> PathBuf {
    let name = out
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "export.otio".into());
    out.with_file_name(format!("{name}.loss.json"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use graft_score::{load_scion, load_score};

    #[test]
    fn example_roundtrip_preserves_bindings() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/hook-v3-body-v1-9x16");
        let score = load_score(&dir.join("score.json")).unwrap();
        let scion = load_scion(&dir.join("scion.json")).unwrap();
        let tmp = std::env::temp_dir().join(format!("graft-otio-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let out = tmp.join("timeline.otio.json");
        export_otio(&score, &scion, &out).unwrap();
        let (imported, loss) = import_otio(&score, &out, "imported").unwrap();
        let before = effective_bindings(&score, &scion).unwrap();
        let after = effective_bindings(&score, &imported).unwrap();
        assert_eq!(before.len(), after.len());
        for (id, binding) in before {
            assert_eq!(after[&id].material, binding.material);
            assert_eq!(after[&id].source, binding.source);
        }
        assert_eq!(loss["lost"][0], "effects");
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
