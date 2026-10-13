// SPDX-License-Identifier: Apache-2.0

//! Lossy OTIO guest adapter. Depends on graft-score only.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use graft_score::{
    effective_bindings, AudioBinding, Binding, BindingLayer, Error, FrameRange, FrameRate, Join,
    Layer, Marker, Result, Scion, Score, TimedRange, GRAFT_SCHEMA,
};

#[derive(Debug)]
pub struct OtioImport {
    pub scion: Scion,
    pub loss: serde_json::Value,
    pub joins: Vec<Join>,
    pub markers: Vec<Marker>,
}

fn clip_json(score: &Score, slot: &graft_score::Slot, binding: &Binding) -> serde_json::Value {
    serde_json::json!({
        "OTIO_SCHEMA": "Clip.2",
        "name": slot.id,
        "source_range": otio_range(binding.source),
        "metadata": {
            "graft": {
                "slot": slot.id,
                "role": slot.role.as_str(),
                "audio": binding.audio,
                "params": binding.params,
                "nested": binding.nested,
                "x": binding.x,
                "y": binding.y,
                "opacity": binding.opacity,
                "motion": binding.motion,
                "remap": binding.remap
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
}

fn track_json(name: &str, kind: &str, clips: Vec<serde_json::Value>) -> serde_json::Value {
    serde_json::json!({
        "OTIO_SCHEMA": "Track.1",
        "name": name,
        "kind": kind,
        "source_range": serde_json::Value::Null,
        "metadata": {},
        "effects": [],
        "markers": [],
        "children": clips
    })
}

pub fn export_otio(score: &Score, scion: &Scion, out: &Path) -> Result<serde_json::Value> {
    let bindings = effective_bindings(score, scion)?;
    let mut picture = Vec::new();
    let mut overlays = Vec::new();
    let mut audio = Vec::new();
    for slot in &score.slots {
        let Some(binding) = bindings.get(&slot.id) else {
            continue;
        };
        let clip = clip_json(score, slot, binding);
        match slot.role {
            graft_score::Role::Vo | graft_score::Role::Bed | graft_score::Role::Sfx => {
                audio.push(clip);
            }
            graft_score::Role::Brand | graft_score::Role::Overlay | graft_score::Role::Captions => {
                overlays.push(clip);
            }
            _ if slot.role.is_spine() => picture.push(clip),
            _ => overlays.push(clip),
        }
    }
    let timeline_markers: Vec<serde_json::Value> = score
        .markers
        .iter()
        .map(|marker| {
            serde_json::json!({
                "OTIO_SCHEMA": "Marker.2",
                "name": marker.id,
                "marked_range": otio_frame_range(marker.range, score.clock.rate),
                "color": "CYAN",
                "metadata": {"graft": {"marker": marker.id}}
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
            "scion.graph (graft sidecar)",
            "joins (graft sidecar)",
            "overlay bindings",
        ],
        ["scion inheritance", "layer strength"],
        [
            "effects",
            "native OTIO grades",
            "working-depth execution",
            "generators",
        ],
        &[],
    );
    let mut tracks = vec![track_json("picture", "Video", picture)];
    if !overlays.is_empty() {
        tracks.push(track_json("overlays", "Video", overlays));
    }
    if !audio.is_empty() {
        tracks.push(track_json("audio", "Audio", audio));
    }
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
                "graph": scion.graph,
                "joins": score.joins,
                "markers": score.markers,
                "loss": loss
            }
        },
        "tracks": {
            "OTIO_SCHEMA": "Stack.1",
            "name": "tracks",
            "source_range": serde_json::Value::Null,
            "metadata": {},
            "effects": [],
            "markers": timeline_markers,
            "children": tracks
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

pub fn import_otio(score: &Score, file: &Path, id: &str) -> Result<OtioImport> {
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
    let tracks = doc
        .pointer("/tracks/children")
        .and_then(|v| v.as_array())
        .ok_or_else(|| Error::invalid("OTIO tracks.children is required"))?;
    let mut children = Vec::new();
    for track in tracks {
        if let Some(clips) = track.get("children").and_then(|v| v.as_array()) {
            children.extend(clips.iter());
        }
    }
    if children.is_empty() {
        return Err(Error::invalid("OTIO timeline has no clips"));
    }
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
        let params = clip
            .pointer("/metadata/graft/params")
            .filter(|value| !value.is_null())
            .cloned();
        let nested = clip
            .pointer("/metadata/graft/nested")
            .and_then(|value| value.as_str())
            .filter(|id| !id.is_empty())
            .map(str::to_string);
        let x = clip
            .pointer("/metadata/graft/x")
            .and_then(|value| value.as_str())
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        let y = clip
            .pointer("/metadata/graft/y")
            .and_then(|value| value.as_str())
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        let opacity = clip
            .pointer("/metadata/graft/opacity")
            .and_then(|value| value.as_f64());
        let motion = clip
            .pointer("/metadata/graft/motion")
            .cloned()
            .and_then(|value| serde_json::from_value(value).ok());
        let remap = clip
            .pointer("/metadata/graft/remap")
            .cloned()
            .and_then(|value| serde_json::from_value(value).ok())
            .unwrap_or_default();
        bindings.insert(
            slot.to_string(),
            Binding {
                material: material.into(),
                source,
                params,
                audio,
                nested,
                x,
                y,
                opacity,
                motion,
                remap,
            },
        );
    }
    let graph = graft
        .get("graph")
        .cloned()
        .and_then(|value| serde_json::from_value(value).ok());
    let scion = Scion {
        graft: GRAFT_SCHEMA.into(),
        id: id.into(),
        concept,
        parent: None,
        change_request: None,
        dest,
        graph,
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
            "scion.graph (graft sidecar)",
            "overlay bindings",
            "score.joins (graft sidecar)",
            "score.markers (graft sidecar)",
        ],
        [],
        [
            "effects",
            "native OTIO grades",
            "working-depth execution",
            "generators",
            "non-graft metadata",
        ],
        &ignored,
    );
    let joins = graft
        .get("joins")
        .cloned()
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or_default();
    let markers = graft
        .get("markers")
        .cloned()
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or_default();
    Ok(OtioImport {
        scion,
        loss,
        joins,
        markers,
    })
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
        let imported = import_otio(&score, &out, "imported").unwrap();
        let loss = imported.loss;
        let imported = imported.scion;
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

    #[test]
    fn sidecar_restores_graph_and_overlay_track() {
        use graft_score::{CompositionGraph, GraphMode, GraphNode, Role};
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/hook-v3-body-v1-9x16");
        let mut score = load_score(&dir.join("score.json")).unwrap();
        score.slots.push(graft_score::Slot {
            id: "overlay".into(),
            role: Role::Overlay,
            range: graft_score::FrameRange::new(0, 90),
            optional: true,
            window: None,
        });
        let mut scion = load_scion(&dir.join("scion.json")).unwrap();
        scion.graph = Some(CompositionGraph {
            mode: GraphMode::Append,
            nodes: vec![GraphNode {
                id: "eq".into(),
                op: "eq".into(),
                inputs: vec!["#picture".into()],
                material: None,
                source: None,
                place: None,
                enable: Some("between(t,0,3)".into()),
                role: None,
                params: Some(serde_json::json!({"saturation": "1.1"})),

                keys: None,
                nodes: Vec::new(),
            }],
        });
        let overlay_mat = scion
            .layers
            .first()
            .and_then(|layer| layer.bindings.values().next())
            .map(|b| b.material.clone())
            .unwrap();
        let hook = scion.layers[0].bindings["hook"].clone();
        scion.layers[0].bindings.insert(
            "overlay".into(),
            graft_score::Binding {
                material: overlay_mat,
                source: hook.source,
                params: None,
                audio: None,
                nested: None,
                x: Some("8".into()),
                y: Some("16".into()),
                opacity: Some(0.8),
                motion: None,
                remap: Vec::new(),
            },
        );
        scion.validate_against_score(&score).unwrap();
        let tmp = std::env::temp_dir().join(format!("graft-otio-graph-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let out = tmp.join("timeline.otio.json");
        score.joins.push(graft_score::Join {
            left: "hook".into(),
            right: "body".into(),
            transition: "fadeblack".into(),
            duration_frames: 6,
        });
        score.markers.push(graft_score::Marker {
            id: "punch".into(),
            range: graft_score::FrameRange::new(0, 30),
        });
        export_otio(&score, &scion, &out).unwrap();
        let imported = import_otio(&score, &out, "imported").unwrap();
        assert_eq!(imported.scion.graph, scion.graph);
        assert!(imported.scion.layers[0].bindings.contains_key("overlay"));
        assert_eq!(
            imported.scion.layers[0].bindings["overlay"].x.as_deref(),
            Some("8")
        );
        assert_eq!(
            imported.scion.layers[0].bindings["overlay"].y.as_deref(),
            Some("16")
        );
        assert_eq!(
            imported.scion.layers[0].bindings["overlay"].opacity,
            Some(0.8)
        );
        assert_eq!(imported.joins[0].transition, "fadeblack");
        assert_eq!(imported.markers[0].id, "punch");
        assert!(imported.loss["preserved"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v.as_str() == Some("scion.graph (graft sidecar)")));
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
