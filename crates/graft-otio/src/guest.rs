// SPDX-License-Identifier: Apache-2.0

//! One flattened guest timeline.
//!
//! OTIO, FCPXML 1.9, FCP7 XMEML, and MLT are views of that timeline.
//! Photoshop and After Effects are not timelines. CapCut draft JSON is refused.
//! A graft sidecar, embedded and written beside the file, restores the graph,
//! joins, markers, and overlay place. A cut edited in the guest wins over that
//! sidecar. This crate does not hash media and does not call the compiler.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use graft_score::{
    is_material_id, Binding, BindingLayer, Dest, Encoder, Error, FrameRange, FrameRate, Join,
    Layer, Marker, Result, Role, Scion, Score, TimedRange, GRAFT_SCHEMA,
};
use serde_json::{json, Value};

use super::{GuestSlot, OtioImport};

pub const RESOLVE_OTIO: &str = "resolve.otio";
pub const FINAL_CUT: &str = "finalcut.fcpxml";
pub const PREMIERE: &str = "premiere.xml";
pub const SHOTCUT: &str = "shotcut.mlt";
pub const LOSS_NAME: &str = "loss.json";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Lane {
    Picture,
    Overlay,
    Audio,
}

#[derive(Clone, Debug)]
struct GuestClip {
    slot: String,
    role: String,
    lane: Lane,
    path: Option<String>,
    binding: Binding,
    dest: FrameRange,
}

struct GuestTimeline {
    scion_id: String,
    concept: String,
    dest: Dest,
    rate: FrameRate,
    duration: u64,
    graph: Option<graft_score::CompositionGraph>,
    joins: Vec<Join>,
    markers: Vec<Marker>,
    clips: Vec<GuestClip>,
}

#[derive(Clone, Debug)]
pub struct ParsedClip {
    pub slot: String,
    pub material: Option<String>,
    pub path: Option<String>,
    pub source: TimedRange,
    pub dest: FrameRange,
    pub meta: Value,
}

#[derive(Clone, Debug)]
pub struct ParsedGuest {
    pub format: String,
    pub payload: Option<Value>,
    pub clips: Vec<ParsedClip>,
    pub ignored: Vec<Value>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub rate: Option<FrameRate>,
}

pub fn is_capcut_draft(path: &Path, text: &str) -> bool {
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or_default();
    if name == "draft_info.json" || name == "draft_content.json" || name == "draft_meta_info.json" {
        return true;
    }
    let Ok(value) = serde_json::from_str::<Value>(text) else {
        return false;
    };
    value.get("draft_materials").is_some() || value.get("draft_fold_path").is_some()
}

pub fn write_otio(
    score: &Score,
    scion: &Scion,
    out: &Path,
    media: &BTreeMap<String, String>,
) -> Result<Value> {
    let timeline = flatten(score, scion, media)?;
    let payload = payload_value(&timeline);
    let loss = single_loss("otio", &timeline, &otio_export_loss(&scion.id));
    let document = otio_document(&timeline, &payload, &loss);
    graft_score::save_json(out, &document)?;
    graft_score::save_json(&sidecar_path(out), &payload)?;
    let loss_file = loss_path(out);
    graft_score::save_json(&loss_file, &loss)?;
    Ok(wrote(out, &loss_file, loss))
}

pub fn export_fcpxml(
    score: &Score,
    scion: &Scion,
    out: &Path,
    media: &BTreeMap<String, String>,
) -> Result<Value> {
    let timeline = flatten(score, scion, media)?;
    let payload = payload_value(&timeline);
    let loss = single_loss("fcpxml", &timeline, &guest_file_loss("fcpxml", &scion.id));
    write_text(out, &fcpxml_document(&timeline, &payload)?)?;
    graft_score::save_json(&sidecar_path(out), &payload)?;
    let loss_file = loss_path(out);
    graft_score::save_json(&loss_file, &loss)?;
    Ok(wrote(out, &loss_file, loss))
}

pub fn export_xmeml(
    score: &Score,
    scion: &Scion,
    out: &Path,
    media: &BTreeMap<String, String>,
) -> Result<Value> {
    let timeline = flatten(score, scion, media)?;
    let payload = payload_value(&timeline);
    let loss = single_loss("xmeml", &timeline, &guest_file_loss("xmeml", &scion.id));
    write_text(out, &xmeml_document(&timeline, &payload)?)?;
    graft_score::save_json(&sidecar_path(out), &payload)?;
    let loss_file = loss_path(out);
    graft_score::save_json(&loss_file, &loss)?;
    Ok(wrote(out, &loss_file, loss))
}

pub fn export_mlt(
    score: &Score,
    scion: &Scion,
    out: &Path,
    media: &BTreeMap<String, String>,
) -> Result<Value> {
    let timeline = flatten(score, scion, media)?;
    let payload = payload_value(&timeline);
    let loss = single_loss("mlt", &timeline, &guest_file_loss("mlt", &scion.id));
    write_text(out, &mlt_document(&timeline, &payload)?)?;
    graft_score::save_json(&sidecar_path(out), &payload)?;
    let loss_file = loss_path(out);
    graft_score::save_json(&loss_file, &loss)?;
    Ok(wrote(out, &loss_file, loss))
}

pub fn export_bundle(
    score: &Score,
    scion: &Scion,
    out_dir: &Path,
    media: &BTreeMap<String, String>,
) -> Result<Value> {
    if out_dir.exists() && !out_dir.is_dir() {
        return Err(Error::invalid(format!(
            "{} is a file. guest export writes a directory",
            out_dir.display()
        )));
    }
    fs::create_dir_all(out_dir).map_err(|e| Error::io(out_dir, e))?;
    let timeline = flatten(score, scion, media)?;
    let payload = payload_value(&timeline);
    let loss = bundle_loss(&timeline);
    let files = [
        (
            RESOLVE_OTIO,
            otio_document(&timeline, &payload, &loss).to_string(),
        ),
        (FINAL_CUT, fcpxml_document(&timeline, &payload)?),
        (PREMIERE, xmeml_document(&timeline, &payload)?),
        (SHOTCUT, mlt_document(&timeline, &payload)?),
    ];
    for (name, body) in files {
        let path = out_dir.join(name);
        if name == RESOLVE_OTIO {
            let doc: Value = serde_json::from_str(&body).map_err(|e| Error::json(&path, e))?;
            graft_score::save_json(&path, &doc)?;
        } else {
            write_text(&path, &body)?;
        }
        graft_score::save_json(&sidecar_path(&path), &payload)?;
    }
    let loss_file = out_dir.join(LOSS_NAME);
    graft_score::save_json(&loss_file, &loss)?;
    Ok(wrote(out_dir, &loss_file, loss))
}

pub fn parse_guest_file(path: &Path) -> Result<ParsedGuest> {
    let text = fs::read_to_string(path).map_err(|e| Error::io(path, e))?;
    let text = text.trim_start_matches('\u{feff}').trim();
    if is_capcut_draft(path, text) {
        return Err(Error::invalid(
            "CapCut has no public edit API. draft JSON is not a guest format.",
        ));
    }
    if text.starts_with('{') {
        let doc: Value = serde_json::from_str(text).map_err(|e| Error::json(path, e))?;
        return parse_otio(path, &doc);
    }
    if text.contains("<fcpxml") {
        return parse_fcpxml(path, text);
    }
    if text.contains("<xmeml") {
        return parse_xmeml(path, text);
    }
    if text.contains("<mlt") {
        return parse_mlt(path, text);
    }
    Err(Error::invalid(format!(
        "{} is not OTIO, FCPXML, FCP7 XML, or MLT",
        path.display()
    )))
}

pub fn parse_guest_dir(dir: &Path) -> Result<ParsedGuest> {
    let names = [FINAL_CUT, PREMIERE, RESOLVE_OTIO, SHOTCUT];
    let mut parsed = Vec::new();
    for name in names {
        let path = dir.join(name);
        if path.is_file() {
            parsed.push((name, parse_guest_file(&path)?));
        }
    }
    if parsed.is_empty() {
        return Err(Error::invalid(format!(
            "{} has no resolve.otio, finalcut.fcpxml, premiere.xml, or shotcut.mlt",
            dir.display()
        )));
    }
    let fingerprint = fingerprint_guest(&parsed[0].1);
    for (name, guest) in parsed.iter().skip(1) {
        if fingerprint_guest(guest) != fingerprint {
            return Err(Error::invalid(format!(
                "guest files in {} disagree. {} and {} are not the same cut. Import the file you edited.",
                dir.display(),
                parsed[0].0,
                name
            )));
        }
    }
    Ok(parsed[0].1.clone())
}

pub fn finish_import(score: &Score, id: &str, parsed: &ParsedGuest) -> Result<OtioImport> {
    let payload = parsed.payload.as_ref();
    let concept = payload
        .and_then(|p| p.get("concept"))
        .and_then(|v| v.as_str())
        .unwrap_or(score.concept.as_str());
    if concept != score.concept {
        let kind = if parsed.format == "otio" {
            "OTIO"
        } else {
            "guest"
        };
        return Err(Error::invalid(format!(
            "{kind} concept does not match score concept"
        )));
    }
    let dest = dest_from_guest(score, parsed)?;
    let mut bindings = BTreeMap::new();
    let mut ignored = parsed.ignored.clone();
    let mut slots = Vec::new();
    let mut retimed = Vec::new();
    for clip in &parsed.clips {
        if score.slot(&clip.slot).is_none() {
            ignored.push(json!(clip.slot));
            continue;
        }
        if bindings.contains_key(&clip.slot) {
            ignored.push(json!({"duplicate": clip.slot}));
            continue;
        }
        let Some(material) = clip.material.as_deref().filter(|m| is_material_id(m)) else {
            ignored.push(json!({
                "slot": clip.slot,
                "path": clip.path,
            }));
            continue;
        };
        let mut binding = binding_from_meta(material, clip.source, &clip.meta)?;
        if source_from_meta(&clip.meta).is_some_and(|source| source != binding.source) {
            retimed.push(clip.slot.clone());
        }
        fit_speed(&mut binding, clip.dest.duration, dest.rate);
        bindings.insert(clip.slot.clone(), binding);
        slots.push(GuestSlot {
            id: clip.slot.clone(),
            range: clip.dest,
        });
    }
    if bindings.is_empty() {
        return Err(Error::invalid(
            "guest timeline has no clips with graft media. Offline paths were not hashed.",
        ));
    }
    let graph = payload
        .and_then(|p| p.get("graph"))
        .filter(|v| !v.is_null())
        .cloned()
        .and_then(|v| serde_json::from_value(v).ok());
    let joins = payload
        .and_then(|p| p.get("joins"))
        .cloned()
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default();
    let markers = payload
        .and_then(|p| p.get("markers"))
        .cloned()
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default();
    let scion = Scion {
        graft: GRAFT_SCHEMA.into(),
        id: id.into(),
        concept: concept.into(),
        parent: None,
        change_request: None,
        dest,
        graph,
        layers: vec![BindingLayer {
            name: Layer::Base,
            bindings,
        }],
    };
    let mut loss = import_loss(&parsed.format, id, &ignored);
    if !retimed.is_empty() {
        if let Some(arr) = loss["flattened"].as_array_mut() {
            arr.push(json!(format!(
                "guest timing replaced the sidecar source on {}",
                retimed.join(", ")
            )));
        }
    }
    let imported = OtioImport {
        scion,
        loss,
        joins,
        markers,
        slots,
    };
    let mut check = score.clone();
    apply_guest_edits(&mut check, &imported);
    check.validate()?;
    imported.scion.validate_against_score(&check)?;
    graft_score::effective_bindings(&check, &imported.scion)?;
    Ok(imported)
}

pub fn apply_guest_edits(score: &mut Score, imported: &OtioImport) -> bool {
    let mut changed = false;
    for join in &imported.joins {
        if let Some(existing) = score
            .joins
            .iter_mut()
            .find(|row| row.left == join.left && row.right == join.right)
        {
            if existing != join {
                *existing = join.clone();
                changed = true;
            }
        } else {
            score.joins.push(join.clone());
            changed = true;
        }
    }
    for marker in &imported.markers {
        if let Some(existing) = score.markers.iter_mut().find(|row| row.id == marker.id) {
            if existing != marker {
                *existing = marker.clone();
                changed = true;
            }
        } else {
            score.markers.push(marker.clone());
            changed = true;
        }
    }
    for slot in &imported.slots {
        if let Some(existing) = score.slots.iter_mut().find(|row| row.id == slot.id) {
            if existing.range != slot.range {
                existing.range = slot.range;
                changed = true;
            }
        }
    }
    let end = score.slots.iter().map(|s| s.range.end()).max().unwrap_or(0);
    if end > score.clock.duration_frames as i64 {
        score.clock.duration_frames = end as u64;
        changed = true;
    }
    changed
}

fn flatten(
    score: &Score,
    scion: &Scion,
    media: &BTreeMap<String, String>,
) -> Result<GuestTimeline> {
    let bindings = graft_score::effective_bindings(score, scion)?;
    let mut clips = Vec::new();
    for slot in &score.slots {
        let Some(binding) = bindings.get(&slot.id) else {
            continue;
        };
        let path = media
            .get(&binding.material)
            .filter(|p| Path::new(p).is_file())
            .cloned();
        clips.push(GuestClip {
            slot: slot.id.clone(),
            role: slot.role.as_str().to_string(),
            lane: lane_of(slot.role),
            path,
            binding: binding.clone(),
            dest: slot.range,
        });
    }
    Ok(GuestTimeline {
        scion_id: scion.id.clone(),
        concept: scion.concept.clone(),
        dest: scion.dest.clone(),
        rate: score.clock.rate,
        duration: score.clock.duration_frames,
        graph: scion.graph.clone(),
        joins: score.joins.clone(),
        markers: score.markers.clone(),
        clips,
    })
}

fn lane_of(role: Role) -> Lane {
    match role {
        Role::Vo | Role::Bed | Role::Sfx => Lane::Audio,
        Role::Brand | Role::Overlay | Role::Captions => Lane::Overlay,
        Role::Hook | Role::Body | Role::Proof | Role::Cta => Lane::Picture,
    }
}

fn payload_value(timeline: &GuestTimeline) -> Value {
    let mut clips = serde_json::Map::new();
    for clip in &timeline.clips {
        clips.insert(clip.slot.clone(), clip_meta(clip));
    }
    json!({
        "format": GRAFT_SCHEMA,
        "concept": timeline.concept,
        "scion": timeline.scion_id,
        "dest": timeline.dest,
        "graph": timeline.graph,
        "joins": timeline.joins,
        "markers": timeline.markers,
        "clips": clips,
    })
}

fn clip_meta(clip: &GuestClip) -> Value {
    json!({
        "slot": clip.slot,
        "role": clip.role,
        "material": clip.binding.material,
        "source": clip.binding.source,
        "dest": {"start": clip.dest.start, "duration": clip.dest.duration},
        "audio": clip.binding.audio,
        "params": clip.binding.params,
        "nested": clip.binding.nested,
        "x": clip.binding.x,
        "y": clip.binding.y,
        "opacity": clip.binding.opacity,
        "motion": clip.binding.motion,
        "remap": clip.binding.remap,
    })
}

fn clips_in(timeline: &GuestTimeline, lane: Lane) -> Vec<&GuestClip> {
    let mut clips: Vec<&GuestClip> = timeline.clips.iter().filter(|c| c.lane == lane).collect();
    clips.sort_by(|a, b| {
        a.dest
            .start
            .cmp(&b.dest.start)
            .then_with(|| a.slot.cmp(&b.slot))
    });
    clips
}

fn pack_tracks<'a>(clips: &[&'a GuestClip]) -> Vec<Vec<&'a GuestClip>> {
    let mut tracks: Vec<(i64, Vec<&'a GuestClip>)> = Vec::new();
    for clip in clips {
        let end = clip.dest.end();
        if let Some(track) = tracks
            .iter_mut()
            .find(|(cursor, _)| *cursor <= clip.dest.start)
        {
            track.0 = end;
            track.1.push(*clip);
        } else {
            tracks.push((end, vec![*clip]));
        }
    }
    tracks.into_iter().map(|(_, clips)| clips).collect()
}

fn otio_document(timeline: &GuestTimeline, payload: &Value, loss: &Value) -> Value {
    let mut graft = payload.clone();
    if let Some(obj) = graft.as_object_mut() {
        obj.insert("loss".into(), loss.clone());
    }
    let mut tracks = Vec::new();
    for (name, kind, lane) in [
        ("picture", "Video", Lane::Picture),
        ("overlays", "Video", Lane::Overlay),
        ("audio", "Audio", Lane::Audio),
    ] {
        let packed = pack_tracks(&clips_in(timeline, lane));
        for (index, track) in packed.into_iter().enumerate() {
            let track_name = if index == 0 {
                name.to_string()
            } else {
                format!("{name}-{}", index + 1)
            };
            tracks.push(track_json(&track_name, kind, otio_items(timeline, &track)));
        }
    }
    if tracks.is_empty() {
        tracks.push(track_json("picture", "Video", Vec::new()));
    }
    let timeline_markers: Vec<Value> = timeline
        .markers
        .iter()
        .map(|marker| {
            json!({
                "OTIO_SCHEMA": "Marker.2",
                "name": marker.id,
                "marked_range": otio_frame_range(marker.range, timeline.rate),
                "color": "CYAN",
                "metadata": {"graft": {"marker": marker.id}}
            })
        })
        .collect();
    json!({
        "OTIO_SCHEMA": "Timeline.1",
        "name": timeline.scion_id,
        "global_start_time": Value::Null,
        "metadata": {"graft": graft},
        "tracks": {
            "OTIO_SCHEMA": "Stack.1",
            "name": "tracks",
            "source_range": Value::Null,
            "metadata": {},
            "effects": [],
            "markers": timeline_markers,
            "children": tracks
        }
    })
}

fn otio_items(timeline: &GuestTimeline, clips: &[&GuestClip]) -> Vec<Value> {
    let mut cursor = 0_i64;
    let mut items = Vec::new();
    for clip in clips {
        if clip.dest.start > cursor {
            let hole = (clip.dest.start - cursor) as u64;
            items.push(json!({
                "OTIO_SCHEMA": "Gap.1",
                "name": "gap",
                "source_range": otio_frame_range(FrameRange::new(0, hole), timeline.rate),
                "metadata": {},
                "effects": [],
                "markers": []
            }));
        }
        items.push(otio_clip(timeline, clip));
        cursor = clip.dest.start + otio_timeline_duration(clip) as i64;
    }
    items
}

fn otio_timeline_duration(clip: &GuestClip) -> u64 {
    let linear = !clip.binding.remap_is_curve() && (clip.binding.speed() - 1.0).abs() <= 1e-3;
    if linear && clip.binding.source.range.duration == clip.dest.duration {
        clip.binding.source.range.duration
    } else {
        clip.dest.duration
    }
}

fn otio_clip(timeline: &GuestTimeline, clip: &GuestClip) -> Value {
    let linear = !clip.binding.remap_is_curve() && (clip.binding.speed() - 1.0).abs() <= 1e-3;
    let same = linear && clip.binding.source.range.duration == clip.dest.duration;
    let source_range = if same {
        clip.binding.source
    } else {
        TimedRange {
            rate: clip.binding.source.rate,
            range: FrameRange::new(clip.binding.source.range.start, clip.dest.duration),
        }
    };
    let mut effects = Vec::new();
    if !same {
        let dest_s = timeline.rate.seconds_from_frames(clip.dest.duration as i64);
        let source_s = clip
            .binding
            .source
            .rate
            .seconds_from_frames(clip.binding.source.range.duration as i64);
        let scalar = if dest_s <= 1e-9 {
            1.0
        } else {
            source_s / dest_s
        };
        effects.push(json!({
            "OTIO_SCHEMA": "LinearTimeWarp.1",
            "name": "speed",
            "effect_name": "LinearTimeWarp",
            "time_scalar": scalar,
            "enabled": true,
            "metadata": {}
        }));
    }
    json!({
        "OTIO_SCHEMA": "Clip.2",
        "name": clip.slot,
        "source_range": otio_range(source_range),
        "metadata": {"graft": clip_meta(clip)},
        "media_reference": {
            "OTIO_SCHEMA": "ExternalReference.1",
            "target_url": media_target(&clip.binding.material, clip.path.as_deref()),
            "available_range": Value::Null,
            "metadata": {}
        },
        "effects": effects,
        "markers": [{
            "OTIO_SCHEMA": "Marker.2",
            "name": format!("graft:{}", clip.slot),
            "marked_range": otio_frame_range(clip.dest, timeline.rate),
            "color": "GREEN",
            "metadata": {"graft": {"slot": clip.slot}}
        }]
    })
}

fn track_json(name: &str, kind: &str, clips: Vec<Value>) -> Value {
    json!({
        "OTIO_SCHEMA": "Track.1",
        "name": name,
        "kind": kind,
        "source_range": Value::Null,
        "metadata": {},
        "effects": [],
        "markers": [],
        "children": clips
    })
}

fn fcpxml_document(timeline: &GuestTimeline, payload: &Value) -> Result<String> {
    let payload_b64 =
        b64_encode(&serde_json::to_vec(payload).map_err(|e| Error::invalid(e.to_string()))?);
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str("<!DOCTYPE fcpxml>\n");
    out.push_str("<fcpxml version=\"1.9\">\n");
    out.push_str("  <resources>\n");
    let frame = frame_duration(timeline.dest.rate);
    out.push_str(&format!(
        "    <format id=\"f1\" name=\"graft-{w}x{h}\" frameDuration=\"{frame}\" width=\"{w}\" height=\"{h}\"/>\n",
        w = timeline.dest.width,
        h = timeline.dest.height,
    ));
    for (index, clip) in timeline.clips.iter().enumerate() {
        let src = clip
            .path
            .as_deref()
            .map(file_url)
            .unwrap_or_else(|| format!("graft://{}", clip.binding.material));
        let end = clip.binding.source.range.end().max(1);
        let duration = fcpx_time(end, clip.binding.source.rate);
        let video = if clip.lane == Lane::Audio { "0" } else { "1" };
        let audio = if clip.lane == Lane::Overlay { "0" } else { "1" };
        out.push_str(&format!(
            "    <asset id=\"r{index}\" name=\"{name}\" src=\"{src}\" start=\"0s\" duration=\"{duration}\" hasVideo=\"{video}\" hasAudio=\"{audio}\" format=\"f1\">\n      <metadata>\n        <md key=\"com.eonik.graft.material\" value=\"{mat}\"/>\n      </metadata>\n    </asset>\n",
            name = xml_escape(&clip.slot),
            mat = xml_escape(&clip.binding.material),
        ));
    }
    out.push_str("  </resources>\n");
    out.push_str("  <library>\n    <event name=\"graft\">\n");
    out.push_str(&format!(
        "      <project name=\"{}\">\n",
        xml_escape(&timeline.scion_id)
    ));
    let seq_dur = fcpx_time(timeline.duration as i64, timeline.rate);
    out.push_str(&format!(
        "        <sequence format=\"f1\" duration=\"{seq_dur}\" tcStart=\"0s\" tcFormat=\"NDF\">\n"
    ));
    out.push_str("          <metadata>\n");
    out.push_str(&format!(
        "            <md key=\"com.eonik.graft\" value=\"{payload_b64}\"/>\n"
    ));
    out.push_str("          </metadata>\n          <spine>\n");
    let picture = clips_in(timeline, Lane::Picture);
    let overlays = clips_in(timeline, Lane::Overlay);
    let audio = clips_in(timeline, Lane::Audio);
    if picture.is_empty() {
        let gap = GuestClip {
            slot: "gap".into(),
            role: "gap".into(),
            lane: Lane::Picture,
            path: None,
            binding: timeline
                .clips
                .first()
                .map(|c| c.binding.clone())
                .ok_or_else(|| Error::invalid("guest timeline has no clips"))?,
            dest: FrameRange::new(0, timeline.duration.max(1)),
        };
        write_fcpx_container(&mut out, timeline, &gap, "gap", 6, &overlays, &audio)?;
    } else {
        let mut cursor = 0_i64;
        for clip in &picture {
            if clip.dest.start > cursor {
                let hole = FrameRange::new(cursor, (clip.dest.start - cursor) as u64);
                write_fcpx_gap(&mut out, timeline, hole, 6);
            }
            let mine_over: Vec<&GuestClip> = overlays
                .iter()
                .copied()
                .filter(|c| c.dest.start >= clip.dest.start && c.dest.start < clip.dest.end())
                .collect();
            let mine_audio: Vec<&GuestClip> = audio
                .iter()
                .copied()
                .filter(|c| c.dest.start >= clip.dest.start && c.dest.start < clip.dest.end())
                .collect();
            write_fcpx_container(
                &mut out,
                timeline,
                clip,
                "asset-clip",
                6,
                &mine_over,
                &mine_audio,
            )?;
            cursor = clip.dest.end();
        }
        let rest_over: Vec<&GuestClip> = overlays
            .iter()
            .copied()
            .filter(|c| {
                !picture
                    .iter()
                    .any(|p| c.dest.start >= p.dest.start && c.dest.start < p.dest.end())
            })
            .collect();
        let rest_audio: Vec<&GuestClip> = audio
            .iter()
            .copied()
            .filter(|c| {
                !picture
                    .iter()
                    .any(|p| c.dest.start >= p.dest.start && c.dest.start < p.dest.end())
            })
            .collect();
        if !rest_over.is_empty() || !rest_audio.is_empty() {
            let start = picture.last().map(|c| c.dest.end()).unwrap_or(0);
            let dur = timeline.duration.saturating_sub(start.max(0) as u64).max(1);
            let gap = GuestClip {
                slot: "gap".into(),
                role: "gap".into(),
                lane: Lane::Picture,
                path: None,
                binding: picture.last().unwrap().binding.clone(),
                dest: FrameRange::new(start, dur),
            };
            write_fcpx_container(&mut out, timeline, &gap, "gap", 6, &rest_over, &rest_audio)?;
        }
    }
    out.push_str("          </spine>\n        </sequence>\n      </project>\n    </event>\n  </library>\n</fcpxml>\n");
    Ok(out)
}

fn write_fcpx_gap(out: &mut String, timeline: &GuestTimeline, range: FrameRange, indent: usize) {
    let offset = fcpx_time(range.start, timeline.rate);
    let duration = fcpx_time(range.duration as i64, timeline.rate);
    pad(out, indent);
    out.push_str(&format!(
        "<gap name=\"gap\" offset=\"{offset}\" duration=\"{duration}\" start=\"0s\"/>\n"
    ));
}

fn write_fcpx_container(
    out: &mut String,
    timeline: &GuestTimeline,
    clip: &GuestClip,
    tag: &str,
    indent: usize,
    overlays: &[&GuestClip],
    audio: &[&GuestClip],
) -> Result<()> {
    write_fcpx_open(out, timeline, clip, tag, indent, false)?;
    write_fcpx_meta(out, clip, indent + 1)?;
    if tag != "gap" {
        write_time_map(out, timeline, clip, indent + 1);
    }
    for (lane, child) in (1_i64..).zip(overlays.iter()) {
        write_fcpx_connected(out, timeline, clip, child, lane, indent + 1)?;
    }
    for (index, child) in audio.iter().enumerate() {
        let lane = -1 - index as i64;
        write_fcpx_connected(out, timeline, clip, child, lane, indent + 1)?;
    }
    pad(out, indent);
    out.push_str(&format!("</{tag}>\n"));
    Ok(())
}

fn write_fcpx_connected(
    out: &mut String,
    timeline: &GuestTimeline,
    parent: &GuestClip,
    clip: &GuestClip,
    lane: i64,
    indent: usize,
) -> Result<()> {
    let relative = clip.dest.start - parent.dest.start;
    let offset = fcpx_time(relative, timeline.rate);
    let start = fcpx_time(clip.binding.source.range.start, clip.binding.source.rate);
    let duration = fcpx_time(clip.dest.duration as i64, timeline.rate);
    let asset = asset_index(timeline, &clip.slot).unwrap_or(0);
    let meta = clip_b64(clip)?;
    pad(out, indent);
    out.push_str(&format!(
        "<asset-clip ref=\"r{asset}\" lane=\"{lane}\" name=\"{name}\" offset=\"{offset}\" start=\"{start}\" duration=\"{duration}\" tcFormat=\"NDF\" format=\"f1\">\n",
        name = xml_escape(&clip.slot),
    ));
    write_fcpx_meta_raw(out, clip, &meta, indent + 1)?;
    write_time_map(out, timeline, clip, indent + 1);
    pad(out, indent);
    out.push_str("</asset-clip>\n");
    Ok(())
}

fn write_fcpx_open(
    out: &mut String,
    timeline: &GuestTimeline,
    clip: &GuestClip,
    tag: &str,
    indent: usize,
    _self_close: bool,
) -> Result<()> {
    let offset = fcpx_time(clip.dest.start, timeline.rate);
    let duration = fcpx_time(clip.dest.duration as i64, timeline.rate);
    pad(out, indent);
    if tag == "gap" {
        out.push_str(&format!(
            "<gap name=\"gap\" offset=\"{offset}\" duration=\"{duration}\" start=\"0s\">\n"
        ));
        return Ok(());
    }
    let start = fcpx_time(clip.binding.source.range.start, clip.binding.source.rate);
    let asset = asset_index(timeline, &clip.slot).unwrap_or(0);
    out.push_str(&format!(
        "<asset-clip ref=\"r{asset}\" name=\"{name}\" offset=\"{offset}\" start=\"{start}\" duration=\"{duration}\" tcFormat=\"NDF\" format=\"f1\">\n",
        name = xml_escape(&clip.slot),
    ));
    Ok(())
}

fn write_fcpx_meta(out: &mut String, clip: &GuestClip, indent: usize) -> Result<()> {
    if clip.slot == "gap" {
        return Ok(());
    }
    let meta = clip_b64(clip)?;
    write_fcpx_meta_raw(out, clip, &meta, indent)?;
    Ok(())
}

fn write_fcpx_meta_raw(
    out: &mut String,
    clip: &GuestClip,
    meta: &str,
    indent: usize,
) -> Result<()> {
    pad(out, indent);
    out.push_str("<metadata>\n");
    pad(out, indent + 1);
    out.push_str(&format!(
        "<md key=\"com.eonik.graft.slot\" value=\"{}\"/>\n",
        xml_escape(&clip.slot)
    ));
    pad(out, indent + 1);
    out.push_str(&format!(
        "<md key=\"com.eonik.graft.material\" value=\"{}\"/>\n",
        xml_escape(&clip.binding.material)
    ));
    pad(out, indent + 1);
    out.push_str(&format!(
        "<md key=\"com.eonik.graft.clip\" value=\"{meta}\"/>\n"
    ));
    pad(out, indent);
    out.push_str("</metadata>\n");
    Ok(())
}

fn write_time_map(out: &mut String, timeline: &GuestTimeline, clip: &GuestClip, indent: usize) {
    let linear = !clip.binding.remap_is_curve() && (clip.binding.speed() - 1.0).abs() <= 1e-3;
    let same = linear && clip.binding.source.range.duration == clip.dest.duration;
    if same {
        return;
    }
    let dest_s = timeline.rate.seconds_from_frames(clip.dest.duration as i64);
    let spans = clip.binding.remap_spans(dest_s);
    pad(out, indent);
    out.push_str("<timeMap>\n");
    let mut t = 0.0_f64;
    let mut src = clip.binding.in_s();
    let first = fcpx_seconds(src, clip.binding.source.rate);
    pad(out, indent + 1);
    out.push_str(&format!(
        "<timept time=\"0s\" value=\"{first}\" interp=\"linear\"/>\n"
    ));
    if spans.len() <= 1 {
        let end = fcpx_seconds(clip.binding.out_s(), clip.binding.source.rate);
        let dest = fcpx_time(clip.dest.duration as i64, timeline.rate);
        pad(out, indent + 1);
        out.push_str(&format!(
            "<timept time=\"{dest}\" value=\"{end}\" interp=\"linear\"/>\n"
        ));
    } else {
        for span in &spans {
            t += (span.src_out - span.src_in) / span.speed.max(1e-6);
            src = span.src_out;
            let time = fcpx_seconds(t, timeline.rate);
            let value = fcpx_seconds(src, clip.binding.source.rate);
            pad(out, indent + 1);
            out.push_str(&format!(
                "<timept time=\"{time}\" value=\"{value}\" interp=\"linear\"/>\n"
            ));
        }
    }
    pad(out, indent);
    out.push_str("</timeMap>\n");
}

fn asset_index(timeline: &GuestTimeline, slot: &str) -> Option<usize> {
    timeline.clips.iter().position(|c| c.slot == slot)
}

fn xmeml_document(timeline: &GuestTimeline, payload: &Value) -> Result<String> {
    let payload_b64 =
        b64_encode(&serde_json::to_vec(payload).map_err(|e| Error::invalid(e.to_string()))?);
    let (timebase, ntsc) = xmeml_rate(timeline.rate);
    let ntsc_flag = if ntsc { "TRUE" } else { "FALSE" };
    let duration = timeline
        .clips
        .iter()
        .map(|c| c.dest.end())
        .max()
        .unwrap_or(timeline.duration as i64)
        .max(timeline.duration as i64);
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str("<!DOCTYPE xmeml>\n");
    out.push_str("<xmeml version=\"5\">\n  <sequence>\n");
    out.push_str(&format!(
        "    <name>{}</name>\n",
        xml_escape(&timeline.scion_id)
    ));
    out.push_str(&format!("    <duration>{duration}</duration>\n"));
    out.push_str(&format!(
        "    <rate><timebase>{timebase}</timebase><ntsc>{ntsc_flag}</ntsc></rate>\n"
    ));
    out.push_str(&format!("    <comment>GRAFT:{payload_b64}</comment>\n"));
    out.push_str("    <media>\n      <video>\n        <format><samplecharacteristics>\n");
    out.push_str(&format!(
        "          <width>{}</width>\n          <height>{}</height>\n",
        timeline.dest.width, timeline.dest.height
    ));
    out.push_str("          <pixelaspectratio>square</pixelaspectratio>\n");
    out.push_str(&format!(
        "          <rate><timebase>{timebase}</timebase><ntsc>{ntsc_flag}</ntsc></rate>\n"
    ));
    out.push_str("        </samplecharacteristics></format>\n");
    write_xmeml_tracks(&mut out, timeline, Lane::Picture, "video")?;
    write_xmeml_tracks(&mut out, timeline, Lane::Overlay, "video")?;
    out.push_str("      </video>\n      <audio>\n");
    write_xmeml_tracks(&mut out, timeline, Lane::Audio, "audio")?;
    out.push_str("      </audio>\n    </media>\n  </sequence>\n</xmeml>\n");
    Ok(out)
}

fn write_xmeml_tracks(
    out: &mut String,
    timeline: &GuestTimeline,
    lane: Lane,
    kind: &str,
) -> Result<()> {
    let packed = pack_tracks(&clips_in(timeline, lane));
    for track in packed {
        out.push_str("        <track>\n");
        for clip in track {
            write_xmeml_clip(out, timeline, clip, kind)?;
        }
        out.push_str("        </track>\n");
    }
    Ok(())
}

fn write_xmeml_clip(
    out: &mut String,
    timeline: &GuestTimeline,
    clip: &GuestClip,
    kind: &str,
) -> Result<()> {
    let (timebase, ntsc) = xmeml_rate(timeline.rate);
    let ntsc_flag = if ntsc { "TRUE" } else { "FALSE" };
    let index = asset_index(timeline, &clip.slot).unwrap_or(0);
    let start = clip.dest.start;
    let end = clip.dest.end();
    let in_f = clip.binding.source.range.start;
    let out_f = clip.binding.source.range.end();
    let path = clip.path.as_deref().map(file_url).unwrap_or_default();
    let meta = clip_b64(clip)?;
    out.push_str(&format!(
        "          <clipitem id=\"clip-{index}\">\n            <name>{}</name>\n",
        xml_escape(&clip.slot)
    ));
    out.push_str(&format!(
        "            <duration>{}</duration>\n",
        clip.dest.duration
    ));
    out.push_str(&format!(
        "            <rate><timebase>{timebase}</timebase><ntsc>{ntsc_flag}</ntsc></rate>\n"
    ));
    out.push_str(&format!(
        "            <start>{start}</start>\n            <end>{end}</end>\n            <in>{in_f}</in>\n            <out>{out_f}</out>\n"
    ));
    out.push_str(&format!(
        "            <comment>GRAFTCLIP:{meta}</comment>\n"
    ));
    out.push_str(&format!(
        "            <file id=\"file-{index}\">\n              <name>{}</name>\n",
        xml_escape(&clip.slot)
    ));
    if !path.is_empty() {
        out.push_str(&format!(
            "              <pathurl>{}</pathurl>\n",
            xml_escape(&path)
        ));
    }
    out.push_str(&format!(
        "              <duration>{out_f}</duration>\n              <rate><timebase>{timebase}</timebase><ntsc>{ntsc_flag}</ntsc></rate>\n              <media><{kind}><duration>{out_f}</duration></{kind}></media>\n            </file>\n          </clipitem>\n"
    ));
    Ok(())
}

fn mlt_document(timeline: &GuestTimeline, payload: &Value) -> Result<String> {
    let payload_b64 =
        b64_encode(&serde_json::to_vec(payload).map_err(|e| Error::invalid(e.to_string()))?);
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n");
    out.push_str(&format!(
        "<mlt version=\"7.0.0\" title=\"{}\">\n",
        xml_escape(&timeline.scion_id)
    ));
    out.push_str(&format!(
        "  <profile description=\"graft\" width=\"{w}\" height=\"{h}\" progressive=\"1\" sample_aspect_num=\"1\" sample_aspect_den=\"1\" display_aspect_num=\"{w}\" display_aspect_den=\"{h}\" frame_rate_num=\"{num}\" frame_rate_den=\"{den}\" colorspace=\"709\"/>\n",
        w = timeline.dest.width,
        h = timeline.dest.height,
        num = timeline.dest.rate.num,
        den = timeline.dest.rate.den,
    ));
    out.push_str(&format!(
        "  <property name=\"graft.payload\">{payload_b64}</property>\n"
    ));
    for (index, clip) in timeline.clips.iter().enumerate() {
        let resource = clip
            .path
            .clone()
            .unwrap_or_else(|| format!("graft://{}", clip.binding.material));
        let in_f = clip.binding.source.range.start;
        let out_f = clip.binding.source.range.end().saturating_sub(1).max(in_f);
        let meta = clip_b64(clip)?;
        out.push_str(&format!(
            "  <producer id=\"p{index}\" in=\"{in_f}\" out=\"{out_f}\">\n    <property name=\"resource\">{}</property>\n    <property name=\"graft.material\">{}</property>\n    <property name=\"graft.slot\">{}</property>\n    <property name=\"graft.clip\">{meta}</property>\n  </producer>\n",
            xml_escape(&resource),
            xml_escape(&clip.binding.material),
            xml_escape(&clip.slot),
        ));
    }
    let mut playlists = Vec::new();
    for lane in [Lane::Picture, Lane::Overlay, Lane::Audio] {
        for track in pack_tracks(&clips_in(timeline, lane)) {
            playlists.push(track);
        }
    }
    for (index, track) in playlists.iter().enumerate() {
        out.push_str(&format!("  <playlist id=\"track{index}\">\n"));
        let mut cursor = 0_i64;
        for clip in track {
            if clip.dest.start > cursor {
                out.push_str(&format!(
                    "    <blank length=\"{}\"/>\n",
                    clip.dest.start - cursor
                ));
            }
            let producer = asset_index(timeline, &clip.slot).unwrap_or(0);
            let in_f = clip.binding.source.range.start;
            let out_f = clip.binding.source.range.end().saturating_sub(1).max(in_f);
            out.push_str(&format!(
                "    <entry producer=\"p{producer}\" in=\"{in_f}\" out=\"{out_f}\">\n      <property name=\"graft.dest_start\">{}</property>\n      <property name=\"graft.dest_duration\">{}</property>\n    </entry>\n",
                clip.dest.start, clip.dest.duration
            ));
            cursor = clip.dest.end();
        }
        out.push_str("  </playlist>\n");
    }
    out.push_str("  <tractor id=\"timeline\">\n    <multitrack>\n");
    for index in 0..playlists.len() {
        out.push_str(&format!("      <track producer=\"track{index}\"/>\n"));
    }
    out.push_str("    </multitrack>\n  </tractor>\n</mlt>\n");
    Ok(out)
}

fn parse_otio(path: &Path, doc: &Value) -> Result<ParsedGuest> {
    if doc.get("OTIO_SCHEMA").and_then(|v| v.as_str()) != Some("Timeline.1") {
        return Err(Error::invalid(format!(
            "{} is not an OTIO Timeline.1 document",
            path.display()
        )));
    }
    let embedded = doc.pointer("/metadata/graft").cloned();
    let payload = load_payload(path, embedded);
    let mut clips = Vec::new();
    let mut ignored = Vec::new();
    let tracks = doc
        .pointer("/tracks/children")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    for track in tracks {
        if track.get("OTIO_SCHEMA").and_then(|v| v.as_str()) == Some("Track.1")
            || track.get("children").is_some()
        {
            let (found, skip) = walk_otio_children(
                track
                    .get("children")
                    .and_then(|v| v.as_array())
                    .map(Vec::as_slice)
                    .unwrap_or(&[]),
                &payload,
            )?;
            clips.extend(found);
            ignored.extend(skip);
        }
    }
    Ok(ParsedGuest {
        format: "otio".into(),
        payload,
        clips,
        ignored,
        width: None,
        height: None,
        rate: None,
    })
}

fn walk_otio_children(
    children: &[Value],
    payload: &Option<Value>,
) -> Result<(Vec<ParsedClip>, Vec<Value>)> {
    let mut cursor = 0_i64;
    let mut clips = Vec::new();
    let mut ignored = Vec::new();
    for child in children {
        let schema = child
            .get("OTIO_SCHEMA")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if schema.starts_with("Gap") {
            let dur = otio_duration(child.get("source_range"))?;
            cursor += dur as i64;
            continue;
        }
        if schema.starts_with("Track") || schema.starts_with("Stack") {
            let (found, skip) = walk_otio_children(
                child
                    .get("children")
                    .and_then(|v| v.as_array())
                    .map(Vec::as_slice)
                    .unwrap_or(&[]),
                payload,
            )?;
            clips.extend(found);
            ignored.extend(skip);
            continue;
        }
        let dur = otio_duration(child.get("source_range")).unwrap_or(0);
        let meta = child
            .pointer("/metadata/graft")
            .cloned()
            .unwrap_or(Value::Null);
        let dest = if dur > 0 {
            FrameRange::new(cursor, dur)
        } else {
            dest_from_meta(&meta).unwrap_or(FrameRange::new(cursor, 1))
        };
        cursor += dest.duration as i64;
        let slot = meta
            .get("slot")
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .or_else(|| {
                child
                    .get("name")
                    .and_then(|v| v.as_str())
                    .map(str::to_string)
            });
        let Some(slot) = slot else {
            ignored.push(child.get("name").cloned().unwrap_or(Value::Null));
            continue;
        };
        let target = child
            .pointer("/media_reference/target_url")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let material = material_from_meta(&meta).or_else(|| graft_url_material(target));
        let path = file_url_path(target);
        let structural = child
            .get("source_range")
            .and_then(|v| parse_otio_range(v).ok());
        let warped = child
            .get("effects")
            .and_then(|v| v.as_array())
            .is_some_and(|effects| {
                effects.iter().any(|effect| {
                    effect.get("OTIO_SCHEMA").and_then(|v| v.as_str()) == Some("LinearTimeWarp.1")
                })
            });
        let source = if warped {
            source_from_meta(&meta)
                .or(structural)
                .or_else(|| source_from_payload(payload, &slot))
        } else {
            structural
                .or_else(|| source_from_meta(&meta))
                .or_else(|| source_from_payload(payload, &slot))
        }
        .ok_or_else(|| Error::invalid(format!("OTIO clip {slot} has no source range")))?;
        clips.push(ParsedClip {
            slot,
            material,
            path,
            source,
            dest,
            meta,
        });
    }
    Ok((clips, ignored))
}

fn parse_fcpxml(path: &Path, xml: &str) -> Result<ParsedGuest> {
    let embedded = md_exact(xml, "com.eonik.graft")
        .into_iter()
        .next()
        .and_then(|b64| decode_json(&b64).ok());
    let payload = load_payload(path, embedded);
    let rate = format_rate(xml).or_else(|| payload_rate(&payload));
    let (width, height) = format_size(xml);
    let assets = fcpx_assets(xml);
    let mut clips = Vec::new();
    let mut ignored = Vec::new();
    for clip in fcpx_asset_clips(xml, rate.unwrap_or(FrameRate::new(30, 1)))? {
        let attrs = &clip.attrs;
        let inner = clip.inner;
        let parent_start = clip.parent_start;
        let mds = direct_mds(inner);
        let meta = mds
            .get("com.eonik.graft.clip")
            .and_then(|b64| decode_json(b64).ok())
            .unwrap_or(Value::Null);
        let slot = mds
            .get("com.eonik.graft.slot")
            .cloned()
            .or_else(|| {
                meta.get("slot")
                    .and_then(|v| v.as_str())
                    .map(str::to_string)
            })
            .or_else(|| attrs.get("name").cloned())
            .unwrap_or_default();
        if slot.is_empty() || slot == "gap" {
            ignored.push(json!(attrs.get("name").cloned().unwrap_or_default()));
            continue;
        }
        let source_rate = source_from_meta(&meta)
            .map(|s| s.rate)
            .or_else(|| source_from_payload(&payload, &slot).map(|s| s.rate))
            .or(rate)
            .unwrap_or(FrameRate::new(30, 1));
        let dest_rate = rate.unwrap_or(source_rate);
        let duration_s = attrs.get("duration").map(String::as_str).unwrap_or("0s");
        let start_s = attrs.get("start").map(String::as_str).unwrap_or("0s");
        let offset_s = attrs.get("offset").map(String::as_str).unwrap_or("0s");
        let dest_dur = frames_from_fcpx(duration_s, dest_rate)? as u64;
        let offset = frames_from_fcpx(offset_s, dest_rate)?;
        let dest_start = if attrs.contains_key("lane") {
            parent_start + offset
        } else {
            offset
        };
        let (source_start, source_dur) = if let Some((a, b)) = time_map_range(inner, source_rate)? {
            (a, b)
        } else {
            (
                frames_from_fcpx(start_s, source_rate)?,
                frames_from_fcpx(duration_s, source_rate)? as u64,
            )
        };
        if dest_dur == 0 || source_dur == 0 {
            return Err(Error::invalid(format!(
                "FCPXML clip {slot} has an empty range"
            )));
        }
        let ref_id = attrs.get("ref").map(String::as_str).unwrap_or("");
        let asset = assets.get(ref_id);
        let material = mds
            .get("com.eonik.graft.material")
            .cloned()
            .filter(|m| is_material_id(m))
            .or_else(|| material_from_meta(&meta))
            .or_else(|| asset.and_then(|a| a.material.clone()))
            .or_else(|| {
                payload
                    .as_ref()
                    .and_then(|p| p.pointer(&format!("/clips/{slot}/material")))
                    .and_then(|v| v.as_str())
                    .filter(|m| is_material_id(m))
                    .map(str::to_string)
            });
        let path = asset.and_then(|a| a.path.clone());
        let mut source = source_from_meta(&meta)
            .or_else(|| source_from_payload(&payload, &slot))
            .unwrap_or(TimedRange {
                rate: source_rate,
                range: FrameRange::new(source_start, source_dur),
            });
        source.range = FrameRange::new(source_start, source_dur);
        source.rate = source_rate;
        let dest = FrameRange::new(dest_start, dest_dur);
        clips.push(ParsedClip {
            slot,
            material,
            path,
            source,
            dest,
            meta,
        });
    }
    Ok(ParsedGuest {
        format: "fcpxml".into(),
        payload,
        clips,
        ignored,
        width,
        height,
        rate,
    })
}

fn parse_xmeml(path: &Path, xml: &str) -> Result<ParsedGuest> {
    let embedded = comment_payload(xml, "GRAFT:");
    let payload = load_payload(path, embedded);
    let (width, height) = xmeml_size(xml);
    let rate = xmeml_sequence_rate(xml).or_else(|| payload_rate(&payload));
    let mut clips = Vec::new();
    let mut ignored = Vec::new();
    for inner in element_inners(xml, "clipitem") {
        let name = text_of(inner, "name").unwrap_or_default();
        let comment = text_of(inner, "comment").unwrap_or_default();
        let meta = comment
            .strip_prefix("GRAFTCLIP:")
            .and_then(|b64| decode_json(b64).ok())
            .unwrap_or(Value::Null);
        let slot = meta
            .get("slot")
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .unwrap_or(name);
        if slot.is_empty() {
            ignored.push(json!(text_of(inner, "name").unwrap_or_default()));
            continue;
        }
        let in_f = text_of(inner, "in").and_then(|s| s.parse::<i64>().ok());
        let out_f = text_of(inner, "out").and_then(|s| s.parse::<i64>().ok());
        let start = text_of(inner, "start").and_then(|s| s.parse::<i64>().ok());
        let end = text_of(inner, "end").and_then(|s| s.parse::<i64>().ok());
        let source_rate = source_from_meta(&meta)
            .map(|s| s.rate)
            .or(rate)
            .unwrap_or(FrameRate::new(30, 1));
        let (source_start, source_dur) = match (in_f, out_f) {
            (Some(a), Some(b)) if b > a => (a, (b - a) as u64),
            _ => {
                let fallback = source_from_meta(&meta)
                    .or_else(|| source_from_payload(&payload, &slot))
                    .ok_or_else(|| Error::invalid(format!("XMEML clip {slot} has no in/out")))?;
                (fallback.range.start, fallback.range.duration)
            }
        };
        let (dest_start, dest_dur) = match (start, end) {
            (Some(a), Some(b)) if b > a => (a, (b - a) as u64),
            _ => dest_from_meta(&meta)
                .map(|r| (r.start, r.duration))
                .unwrap_or((0, source_dur)),
        };
        let pathurl = text_of(inner, "pathurl").unwrap_or_default();
        let path = if pathurl.is_empty() {
            None
        } else {
            file_url_path(&pathurl)
        };
        let material = material_from_meta(&meta).or_else(|| {
            payload
                .as_ref()
                .and_then(|p| p.pointer(&format!("/clips/{slot}/material")))
                .and_then(|v| v.as_str())
                .filter(|m| is_material_id(m))
                .map(str::to_string)
        });
        let mut source = source_from_meta(&meta).unwrap_or(TimedRange {
            rate: source_rate,
            range: FrameRange::new(source_start, source_dur),
        });
        source.range = FrameRange::new(source_start, source_dur);
        clips.push(ParsedClip {
            slot,
            material,
            path,
            source,
            dest: FrameRange::new(dest_start, dest_dur),
            meta,
        });
    }
    Ok(ParsedGuest {
        format: "xmeml".into(),
        payload,
        clips,
        ignored,
        width,
        height,
        rate,
    })
}

fn parse_mlt(path: &Path, xml: &str) -> Result<ParsedGuest> {
    let embedded = property_value(xml, "graft.payload").and_then(|b64| decode_json(&b64).ok());
    let payload = load_payload(path, embedded);
    let width = attr_on(xml, "profile", "width").and_then(|s| s.parse().ok());
    let height = attr_on(xml, "profile", "height").and_then(|s| s.parse().ok());
    let rate = match (
        attr_on(xml, "profile", "frame_rate_num").and_then(|s| s.parse().ok()),
        attr_on(xml, "profile", "frame_rate_den").and_then(|s| s.parse().ok()),
    ) {
        (Some(num), Some(den)) => Some(FrameRate::new(num, den)),
        _ => payload_rate(&payload),
    };
    let mut producers = BTreeMap::new();
    for inner in element_inners(xml, "producer") {
        let id = attr_of_first(inner, "producer", "id").or_else(|| attr_before(xml, inner, "id"));
        let Some(id) = id else { continue };
        let props = properties(inner);
        producers.insert(id, props);
    }
    let mut clips = Vec::new();
    let mut ignored = Vec::new();
    for playlist in element_inners(xml, "playlist") {
        let mut cursor = 0_i64;
        for item in mlt_playlist_items(playlist) {
            match item {
                MltItem::Blank(len) => cursor += len,
                MltItem::Entry {
                    producer,
                    in_f,
                    out_f,
                    dest_start,
                    dest_duration,
                } => {
                    let props = producers.get(&producer);
                    let meta = props
                        .and_then(|p| p.get("graft.clip"))
                        .and_then(|b64| decode_json(b64).ok())
                        .unwrap_or(Value::Null);
                    let slot = props
                        .and_then(|p| p.get("graft.slot"))
                        .cloned()
                        .or_else(|| {
                            meta.get("slot")
                                .and_then(|v| v.as_str())
                                .map(str::to_string)
                        })
                        .unwrap_or_default();
                    if slot.is_empty() {
                        ignored.push(json!(producer));
                        continue;
                    }
                    let source_dur = out_f.saturating_sub(in_f).saturating_add(1).max(1) as u64;
                    let rate = source_from_meta(&meta)
                        .map(|s| s.rate)
                        .or(rate)
                        .unwrap_or(FrameRate::new(30, 1));
                    let mut source = source_from_meta(&meta).unwrap_or(TimedRange {
                        rate,
                        range: FrameRange::new(in_f, source_dur),
                    });
                    source.range = FrameRange::new(in_f, source_dur);
                    let dest = FrameRange::new(
                        dest_start.unwrap_or(cursor),
                        dest_duration.unwrap_or(source_dur),
                    );
                    let resource = props.and_then(|p| p.get("resource")).cloned();
                    let material = props
                        .and_then(|p| p.get("graft.material"))
                        .cloned()
                        .filter(|m| is_material_id(m))
                        .or_else(|| material_from_meta(&meta))
                        .or_else(|| resource.as_deref().and_then(graft_url_material));
                    let path = resource.as_deref().and_then(|r| {
                        if r.starts_with("graft://") || r.is_empty() {
                            None
                        } else {
                            Some(r.to_string())
                        }
                    });
                    cursor = dest.end();
                    clips.push(ParsedClip {
                        slot,
                        material,
                        path,
                        source,
                        dest,
                        meta,
                    });
                }
            }
        }
    }
    Ok(ParsedGuest {
        format: "mlt".into(),
        payload,
        clips,
        ignored,
        width,
        height,
        rate,
    })
}

enum MltItem {
    Blank(i64),
    Entry {
        producer: String,
        in_f: i64,
        out_f: i64,
        dest_start: Option<i64>,
        dest_duration: Option<u64>,
    },
}

fn mlt_playlist_items(inner: &str) -> Vec<MltItem> {
    let mut items = Vec::new();
    let mut i = 0;
    while let Some(at) = find_tag_start(inner, i) {
        let Some(tag) = parse_tag(inner, at) else {
            break;
        };
        if tag.closing {
            i = tag.end;
            continue;
        }
        if tag.name == "blank" {
            let len = tag
                .attrs
                .get("length")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            items.push(MltItem::Blank(len));
        } else if tag.name == "entry" {
            let producer = tag.attrs.get("producer").cloned().unwrap_or_default();
            let in_f = tag
                .attrs
                .get("in")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            let out_f = tag
                .attrs
                .get("out")
                .and_then(|s| s.parse().ok())
                .unwrap_or(in_f);
            let entry_inner = if tag.self_closing {
                ""
            } else {
                element_inner(inner, tag.end, "entry")
            };
            let props = properties(entry_inner);
            let dest_start = props.get("graft.dest_start").and_then(|s| s.parse().ok());
            let dest_duration = props
                .get("graft.dest_duration")
                .and_then(|s| s.parse().ok());
            items.push(MltItem::Entry {
                producer,
                in_f,
                out_f,
                dest_start,
                dest_duration,
            });
        }
        i = tag.end;
    }
    items
}

struct FcpxAsset {
    path: Option<String>,
    material: Option<String>,
}

fn fcpx_assets(xml: &str) -> BTreeMap<String, FcpxAsset> {
    let mut out = BTreeMap::new();
    let mut i = 0;
    while let Some(at) = find_tag_start(xml, i) {
        let Some(tag) = parse_tag(xml, at) else { break };
        if !tag.closing && tag.name == "asset" {
            let id = tag.attrs.get("id").cloned().unwrap_or_default();
            let src = tag.attrs.get("src").cloned();
            let path = src.as_deref().and_then(|s| {
                if s.starts_with("file:") {
                    file_url_path(s)
                } else if s.starts_with("graft://") {
                    None
                } else if s.starts_with('/') {
                    Some(s.to_string())
                } else {
                    None
                }
            });
            let inner = if tag.self_closing {
                ""
            } else {
                element_inner(xml, tag.end, "asset")
            };
            let mds = direct_mds(inner);
            let material = mds
                .get("com.eonik.graft.material")
                .cloned()
                .filter(|m| is_material_id(m))
                .or_else(|| src.as_deref().and_then(graft_url_material));
            if !id.is_empty() {
                out.insert(id, FcpxAsset { path, material });
            }
        }
        i = tag.end;
    }
    out
}

struct FcpxSeen<'a> {
    attrs: BTreeMap<String, String>,
    inner: &'a str,
    parent_start: i64,
}

fn fcpx_asset_clips(xml: &str, rate: FrameRate) -> Result<Vec<FcpxSeen<'_>>> {
    let mut out = Vec::new();
    let mut stack: Vec<i64> = Vec::new();
    let mut i = 0;
    while let Some(at) = find_tag_start(xml, i) {
        let Some(tag) = parse_tag(xml, at) else { break };
        if tag.closing {
            if tag.name == "asset-clip" || tag.name == "gap" || tag.name == "spine" {
                stack.pop();
            }
            i = tag.end;
            continue;
        }
        if tag.name == "asset-clip" && !tag.self_closing {
            let offset = tag
                .attrs
                .get("offset")
                .map(|s| frames_from_fcpx(s, rate).unwrap_or(0))
                .unwrap_or(0);
            let parent = *stack.last().unwrap_or(&0);
            let dest_start = if tag.attrs.contains_key("lane") {
                parent + offset
            } else {
                offset
            };
            let inner = element_inner(xml, tag.end, "asset-clip");
            out.push(FcpxSeen {
                attrs: tag.attrs.clone(),
                inner,
                parent_start: parent,
            });
            stack.push(dest_start);
        } else if tag.name == "gap" && !tag.self_closing {
            let offset = tag
                .attrs
                .get("offset")
                .map(|s| frames_from_fcpx(s, rate).unwrap_or(0))
                .unwrap_or(0);
            stack.push(offset);
        } else if tag.name == "spine" && !tag.self_closing {
            stack.push(0);
        }
        i = tag.end;
    }
    Ok(out)
}

fn dest_from_guest(score: &Score, parsed: &ParsedGuest) -> Result<Dest> {
    if let Some(dest) = parsed
        .payload
        .as_ref()
        .and_then(|p| p.get("dest"))
        .filter(|v| !v.is_null())
    {
        return serde_json::from_value(dest.clone()).map_err(|e| Error::invalid(e.to_string()));
    }
    let width = parsed
        .width
        .ok_or_else(|| Error::invalid("guest file has no frame size and no graft sidecar"))?;
    let height = parsed
        .height
        .ok_or_else(|| Error::invalid("guest file has no frame size and no graft sidecar"))?;
    let rate = parsed.rate.unwrap_or(score.clock.rate);
    let id = score
        .dest_default
        .clone()
        .unwrap_or_else(|| format!("{width}x{height}"))
        .replace(':', "x");
    Ok(Dest {
        id,
        width,
        height,
        rate,
        pix_fmt: "yuv420p".into(),
        color: "bt709".into(),
        working: None,
        encoder: Encoder::default_x264(),
    })
}

fn binding_from_meta(material: &str, source: TimedRange, meta: &Value) -> Result<Binding> {
    let audio = meta
        .get("audio")
        .filter(|v| !v.is_null())
        .cloned()
        .and_then(|v| serde_json::from_value(v).ok());
    let params = meta.get("params").filter(|v| !v.is_null()).cloned();
    let nested = meta
        .get("nested")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let x = meta
        .get("x")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let y = meta
        .get("y")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let opacity = meta.get("opacity").and_then(|v| v.as_f64());
    let motion = meta
        .get("motion")
        .filter(|v| !v.is_null())
        .cloned()
        .and_then(|v| serde_json::from_value(v).ok());
    let remap = meta
        .get("remap")
        .filter(|v| !v.is_null())
        .cloned()
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default();
    Ok(Binding {
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
    })
}

fn fit_speed(binding: &mut Binding, dest_frames: u64, dest_rate: FrameRate) {
    if binding.remap_is_curve() || dest_frames == 0 {
        return;
    }
    let source_s = binding
        .source
        .rate
        .seconds_from_frames(binding.source.range.duration as i64);
    let dest_s = dest_rate.seconds_from_frames(dest_frames as i64);
    if dest_s <= 1e-9 {
        return;
    }
    let ratio = source_s / dest_s;
    if (ratio - binding.speed()).abs() <= 1e-3 {
        return;
    }
    let mut params = binding.params.take().unwrap_or_else(|| json!({}));
    if let Some(obj) = params.as_object_mut() {
        obj.insert("speed".into(), json!(ratio));
        binding.params = Some(params);
    }
}

fn source_from_meta(meta: &Value) -> Option<TimedRange> {
    meta.get("source")
        .filter(|v| !v.is_null())
        .cloned()
        .and_then(|v| serde_json::from_value(v).ok())
}

fn source_from_payload(payload: &Option<Value>, slot: &str) -> Option<TimedRange> {
    payload
        .as_ref()
        .and_then(|p| p.pointer(&format!("/clips/{slot}/source")))
        .cloned()
        .and_then(|v| serde_json::from_value(v).ok())
}

fn dest_from_meta(meta: &Value) -> Option<FrameRange> {
    let start = meta.pointer("/dest/start").and_then(json_i64)?;
    let duration = meta.pointer("/dest/duration").and_then(json_u64)?;
    if duration == 0 {
        return None;
    }
    Some(FrameRange::new(start, duration))
}

fn material_from_meta(meta: &Value) -> Option<String> {
    meta.get("material")
        .and_then(|v| v.as_str())
        .filter(|m| is_material_id(m))
        .map(str::to_string)
}

fn graft_url_material(url: &str) -> Option<String> {
    url.strip_prefix("graft://")
        .filter(|m| is_material_id(m))
        .map(str::to_string)
}

fn file_url_path(url: &str) -> Option<String> {
    if !url.starts_with("file:") {
        return None;
    }
    let path = file_url_to_path(url);
    if path.is_empty() {
        None
    } else {
        Some(path)
    }
}

fn load_payload(path: &Path, embedded: Option<Value>) -> Option<Value> {
    if embedded
        .as_ref()
        .and_then(|v| v.get("concept"))
        .and_then(|v| v.as_str())
        .is_some()
    {
        return embedded;
    }
    let side = sidecar_path(path);
    graft_score::load_json::<Value>(&side).ok().or(embedded)
}

fn fingerprint_guest(guest: &ParsedGuest) -> Value {
    let mut clips: Vec<Value> = guest
        .clips
        .iter()
        .map(|c| {
            json!({
                "slot": c.slot,
                "material": c.material,
                "source": c.source,
                "dest": {"start": c.dest.start, "duration": c.dest.duration},
            })
        })
        .collect();
    clips.sort_by(|a, b| {
        a["slot"]
            .as_str()
            .unwrap_or("")
            .cmp(b["slot"].as_str().unwrap_or(""))
    });
    json!({
        "clips": clips,
        "graph": guest.payload.as_ref().and_then(|v| v.get("graph")),
        "joins": guest.payload.as_ref().and_then(|v| v.get("joins")),
        "markers": guest.payload.as_ref().and_then(|v| v.get("markers")),
    })
}

fn import_loss(format: &str, scion: &str, ignored: &[Value]) -> Value {
    let (preserved, lost) = if format == "otio" {
        (
            vec![
                "cuts",
                "graft media references",
                "slot markers",
                "rational timing",
                "scion.graph (graft sidecar)",
                "overlay bindings",
                "score.joins (graft sidecar)",
                "score.markers (graft sidecar)",
            ],
            vec![
                "effects",
                "native OTIO grades",
                "working-depth execution",
                "generators",
                "non-graft metadata",
            ],
        )
    } else {
        (
            vec![
                "cuts",
                "graft media references",
                "rational timing",
                "scion.graph (graft sidecar)",
                "overlay bindings",
                "score.joins (graft sidecar)",
                "score.markers (graft sidecar)",
            ],
            vec![
                "effects",
                "native grades",
                "plugins",
                "working-depth execution",
                "generators",
            ],
        )
    };
    json!({
        "format": format,
        "scion": scion,
        "preserved": preserved,
        "flattened": [],
        "lost": lost,
        "ignored_clips": ignored,
    })
}

fn otio_export_loss(scion: &str) -> Value {
    json!({
        "format": "otio",
        "scion": scion,
        "preserved": [
            "cuts",
            "media references",
            "slot markers",
            "rational timing",
            "dest metadata",
            "scion.graph (graft sidecar)",
            "joins (graft sidecar)",
            "overlay bindings"
        ],
        "flattened": ["scion inheritance", "layer strength"],
        "lost": [
            "effects",
            "native OTIO grades",
            "working-depth execution",
            "generators"
        ],
        "ignored_clips": []
    })
}

fn guest_file_loss(format: &str, scion: &str) -> Value {
    json!({
        "format": format,
        "scion": scion,
        "preserved": [
            "cuts",
            "slot ids",
            "source in and out",
            "dest placement",
            "material hashes",
            "scion.graph (graft sidecar)",
            "joins (graft sidecar)",
            "markers (graft sidecar)",
            "overlay bindings"
        ],
        "flattened": ["scion inheritance", "layer strength"],
        "lost": [
            "effects",
            "native grades",
            "plugins",
            "working-depth execution",
            "generators"
        ],
        "ignored_clips": []
    })
}

fn single_loss(format: &str, timeline: &GuestTimeline, base: &Value) -> Value {
    let mut loss = base.clone();
    if let Some(obj) = loss.as_object_mut() {
        obj.insert("format".into(), json!(format));
        obj.insert(
            "offline".into(),
            json!(timeline
                .clips
                .iter()
                .filter(|c| c.path.is_none())
                .map(|c| c.slot.clone())
                .collect::<Vec<_>>()),
        );
    }
    loss
}

fn bundle_loss(timeline: &GuestTimeline) -> Value {
    let offline: Vec<String> = timeline
        .clips
        .iter()
        .filter(|c| c.path.is_none())
        .map(|c| c.slot.clone())
        .collect();
    json!({
        "format": "guest",
        "scion": timeline.scion_id,
        "principle": "Universal means these files. Each app opens the interchange it already imports.",
        "files": {
            "davinci_resolve": RESOLVE_OTIO,
            "final_cut_pro": FINAL_CUT,
            "premiere_pro": PREMIERE,
            "shotcut": SHOTCUT,
            "kdenlive": SHOTCUT
        },
        "no_timeline": {
            "after_effects": "After Effects has no public composition format. Rasterize a graphic to an overlay still and bind that slot.",
            "photoshop": "Photoshop authors stills. Bind the still as an overlay or brand plate.",
            "capcut": "CapCut has no public edit API. Import of draft JSON is refused.",
            "avid": "Avid has no public timeline XML. Open resolve.otio where the app imports OTIO. AAF is not a guest format."
        },
        "eonik": "eonik is a host. It calls graft compile on the recipe. It does not import a timeline.",
        "preserved": [
            "cuts",
            "slot ids",
            "source in and out",
            "dest placement",
            "material hashes",
            "file paths when the workspace media index has them",
            "scion.graph",
            "joins",
            "markers",
            "overlay place",
            "remap"
        ],
        "flattened": ["scion inheritance", "layer strength"],
        "lost": [
            "effects",
            "native grades",
            "plugins",
            "working-depth execution",
            "generators",
            "stepwise remap plays as one speed in the guest; graft import restores the curve"
        ],
        "offline": offline,
        "ignored_clips": []
    })
}

fn wrote(out: &Path, loss_file: &Path, loss: Value) -> Value {
    json!({
        "wrote": out,
        "loss_report": loss_file,
        "loss": loss,
    })
}

fn media_target(material: &str, path: Option<&str>) -> String {
    match path {
        Some(path) => file_url(path),
        None => format!("graft://{material}"),
    }
}

fn clip_b64(clip: &GuestClip) -> Result<String> {
    let bytes = serde_json::to_vec(&clip_meta(clip)).map_err(|e| Error::invalid(e.to_string()))?;
    Ok(b64_encode(&bytes))
}

fn decode_json(b64: &str) -> Result<Value> {
    let bytes = b64_decode(b64)?;
    serde_json::from_slice(&bytes).map_err(|e| Error::invalid(e.to_string()))
}

fn otio_range(value: TimedRange) -> Value {
    json!({
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
        "metadata": {"graft_rate": value.rate}
    })
}

fn otio_frame_range(range: FrameRange, rate: FrameRate) -> Value {
    otio_range(TimedRange { rate, range })
}

fn parse_otio_range(value: &Value) -> Result<TimedRange> {
    let rate = if let Some(raw) = value.pointer("/metadata/graft_rate") {
        serde_json::from_value(raw.clone()).map_err(|e| Error::invalid(e.to_string()))?
    } else {
        let rate = value
            .pointer("/duration/rate")
            .and_then(|v| v.as_f64())
            .unwrap_or(30.0);
        FrameRate::from_f64(rate)?
    };
    let start = value
        .pointer("/start_time/value")
        .and_then(json_i64)
        .ok_or_else(|| Error::invalid("OTIO range start_time.value must be a frame"))?;
    let duration = value
        .pointer("/duration/value")
        .and_then(json_u64)
        .ok_or_else(|| Error::invalid("OTIO range duration.value must be a frame count"))?;
    Ok(TimedRange {
        rate,
        range: FrameRange::new(start, duration),
    })
}

fn otio_duration(range: Option<&Value>) -> Result<u64> {
    let Some(range) = range else {
        return Ok(0);
    };
    Ok(parse_otio_range(range)?.range.duration)
}

fn sidecar_path(out: &Path) -> PathBuf {
    let name = out
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "export".into());
    out.with_file_name(format!("{name}.graft.json"))
}

fn loss_path(out: &Path) -> PathBuf {
    let name = out
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "export".into());
    out.with_file_name(format!("{name}.loss.json"))
}

fn write_text(path: &Path, text: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
        }
    }
    fs::write(path, text).map_err(|e| Error::io(path, e))
}

fn xmeml_rate(rate: FrameRate) -> (u32, bool) {
    if rate.den == 1001 {
        let tb = (rate.num as f64 / rate.den as f64).round() as u32;
        (tb.max(1), true)
    } else {
        (rate.num.max(1), false)
    }
}

fn frame_duration(rate: FrameRate) -> String {
    format!("{}/{}s", rate.den, rate.num)
}

fn fcpx_time(frames: i64, rate: FrameRate) -> String {
    if frames == 0 {
        return "0s".into();
    }
    let neg = frames < 0;
    let frames = frames.unsigned_abs();
    let mut n = frames * rate.den as u64;
    let mut d = rate.num as u64;
    let g = gcd_u64(n, d);
    n /= g;
    d /= g;
    let sign = if neg { "-" } else { "" };
    if d == 1 {
        format!("{sign}{n}s")
    } else {
        format!("{sign}{n}/{d}s")
    }
}

fn fcpx_seconds(seconds: f64, rate: FrameRate) -> String {
    if !seconds.is_finite() {
        return "0s".into();
    }
    let frames = rate.frames_from_seconds(seconds.max(0.0)).unwrap_or(0);
    fcpx_time(frames, rate)
}

fn frames_from_fcpx(text: &str, rate: FrameRate) -> Result<i64> {
    let (n, d) = parse_fcpx_seconds(text)?;
    let num = n * rate.num as i128;
    let den = d * rate.den as i128;
    if den == 0 {
        return Err(Error::invalid("bad frame rate"));
    }
    let frames = if num >= 0 {
        (num + den / 2) / den
    } else {
        (num - den / 2) / den
    };
    i64::try_from(frames).map_err(|_| Error::invalid("fcpx time overflows"))
}

fn parse_fcpx_seconds(text: &str) -> Result<(i128, i128)> {
    let t = text.trim().trim_end_matches('s').trim();
    if t.is_empty() {
        return Err(Error::invalid(format!("empty FCPXML time {text}")));
    }
    let (n, d) = if let Some((a, b)) = t.split_once('/') {
        (
            a.parse::<i128>()
                .map_err(|_| Error::invalid(format!("bad FCPXML time {text}")))?,
            b.parse::<i128>()
                .map_err(|_| Error::invalid(format!("bad FCPXML time {text}")))?,
        )
    } else {
        (
            t.parse::<i128>()
                .map_err(|_| Error::invalid(format!("bad FCPXML time {text}")))?,
            1,
        )
    };
    if d == 0 {
        return Err(Error::invalid(format!("bad FCPXML time {text}")));
    }
    Ok((n, d))
}

fn time_map_range(inner: &str, rate: FrameRate) -> Result<Option<(i64, u64)>> {
    let Some(map) = element_inners(inner, "timeMap").into_iter().next() else {
        return Ok(None);
    };
    let mut values = Vec::new();
    let mut i = 0;
    while let Some(at) = find_tag_start(map, i) {
        let Some(tag) = parse_tag(map, at) else { break };
        if !tag.closing && tag.name == "timept" {
            if let Some(value) = tag.attrs.get("value") {
                values.push(frames_from_fcpx(value, rate)?);
            }
        }
        i = tag.end;
    }
    if values.len() < 2 {
        return Ok(None);
    }
    let start = values[0];
    let end = *values.last().unwrap();
    if end <= start {
        return Err(Error::invalid("FCPXML timeMap is empty"));
    }
    Ok(Some((start, (end - start) as u64)))
}

fn format_rate(xml: &str) -> Option<FrameRate> {
    let frame = attr_on(xml, "format", "frameDuration")?;
    let (n, d) = parse_fcpx_seconds(&frame).ok()?;
    if n <= 0 || d <= 0 {
        return None;
    }
    Some(FrameRate::new(d as u32, n as u32))
}

fn format_size(xml: &str) -> (Option<u32>, Option<u32>) {
    (
        attr_on(xml, "format", "width").and_then(|s| s.parse().ok()),
        attr_on(xml, "format", "height").and_then(|s| s.parse().ok()),
    )
}

fn payload_rate(payload: &Option<Value>) -> Option<FrameRate> {
    payload
        .as_ref()
        .and_then(|p| p.pointer("/dest/rate"))
        .cloned()
        .and_then(|v| serde_json::from_value(v).ok())
}

fn xmeml_size(xml: &str) -> (Option<u32>, Option<u32>) {
    (
        text_of(xml, "width").and_then(|s| s.parse().ok()),
        text_of(xml, "height").and_then(|s| s.parse().ok()),
    )
}

fn xmeml_sequence_rate(xml: &str) -> Option<FrameRate> {
    let timebase = text_of(xml, "timebase")?.parse::<u32>().ok()?;
    let ntsc = text_of(xml, "ntsc").unwrap_or_default();
    if ntsc.eq_ignore_ascii_case("TRUE") {
        Some(FrameRate::new(timebase * 1000, 1001))
    } else {
        Some(FrameRate::new(timebase, 1))
    }
}

fn comment_payload(xml: &str, prefix: &str) -> Option<Value> {
    let mut i = 0;
    while let Some(rel) = xml[i..].find("<comment>") {
        let start = i + rel + "<comment>".len();
        let end = xml[start..].find("</comment>")? + start;
        let body = xml[start..end].trim();
        if let Some(b64) = body.strip_prefix(prefix) {
            if !b64.starts_with("CLIP:") && prefix == "GRAFT:" {
                return decode_json(b64).ok();
            }
            if prefix != "GRAFT:" {
                return decode_json(b64).ok();
            }
        }
        i = end;
    }
    None
}

fn md_exact(xml: &str, key: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = 0;
    while let Some(at) = find_tag_start(xml, i) {
        let Some(tag) = parse_tag(xml, at) else { break };
        if !tag.closing && tag.name == "md" && tag.attrs.get("key").map(String::as_str) == Some(key)
        {
            if let Some(value) = tag.attrs.get("value") {
                out.push(value.clone());
            }
        }
        i = tag.end;
    }
    out
}

fn direct_mds(inner: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let mut i = 0;
    while i < inner.len() {
        if inner[i..].starts_with("<asset-clip") || inner[i..].starts_with("<gap") {
            if let Some(tag) = parse_tag(inner, i) {
                if tag.self_closing || tag.closing {
                    i = tag.end;
                    continue;
                }
                let close = format!("</{}>", tag.name);
                if let Some(rel) = inner[tag.end..].find(&close) {
                    i = tag.end + rel + close.len();
                    continue;
                }
            }
        }
        if inner[i..].starts_with("<md ") || inner[i..].starts_with("<md\n") {
            if let Some(tag) = parse_tag(inner, i) {
                if let (Some(key), Some(value)) = (tag.attrs.get("key"), tag.attrs.get("value")) {
                    out.insert(key.clone(), value.clone());
                }
                i = tag.end;
                continue;
            }
        }
        i += inner[i..].chars().next().map(|c| c.len_utf8()).unwrap_or(1);
    }
    out
}

fn text_of(inner: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = inner.find(&open)? + open.len();
    let end = inner[start..].find(&close)? + start;
    Some(xml_unescape(inner[start..end].trim()))
}

fn property_value(xml: &str, name: &str) -> Option<String> {
    properties(xml).get(name).cloned()
}

fn properties(inner: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let mut i = 0;
    while let Some(at) = find_tag_start(inner, i) {
        let Some(tag) = parse_tag(inner, at) else {
            break;
        };
        if !tag.closing && tag.name == "property" {
            if let Some(name) = tag.attrs.get("name") {
                let value = if tag.self_closing {
                    String::new()
                } else {
                    let body = element_inner(inner, tag.end, "property");
                    xml_unescape(body.trim())
                };
                out.entry(name.clone()).or_insert(value);
            }
        }
        i = tag.end;
    }
    out
}

fn attr_on(xml: &str, element: &str, attr: &str) -> Option<String> {
    let mut i = 0;
    while let Some(at) = find_tag_start(xml, i) {
        let Some(tag) = parse_tag(xml, at) else { break };
        if !tag.closing && tag.name == element {
            if let Some(value) = tag.attrs.get(attr) {
                return Some(value.clone());
            }
        }
        i = tag.end;
    }
    None
}

fn attr_of_first(inner: &str, _element: &str, attr: &str) -> Option<String> {
    let at = find_tag_start(inner, 0)?;
    let tag = parse_tag(inner, at)?;
    tag.attrs.get(attr).cloned()
}

fn attr_before(xml: &str, inner: &str, attr: &str) -> Option<String> {
    let start = xml.find(inner)?;
    let open = xml[..start].rfind('<')?;
    parse_tag(xml, open)?.attrs.get(attr).cloned()
}

fn element_inners<'a>(xml: &'a str, name: &str) -> Vec<&'a str> {
    let mut out = Vec::new();
    let mut i = 0;
    while let Some(at) = find_tag_start(xml, i) {
        let Some(tag) = parse_tag(xml, at) else { break };
        if !tag.closing && tag.name == name && !tag.self_closing {
            out.push(element_inner(xml, tag.end, name));
        }
        i = tag.end;
    }
    out
}

fn element_inner<'a>(xml: &'a str, open_end: usize, name: &str) -> &'a str {
    let mut depth = 1;
    let mut i = open_end;
    while let Some(at) = find_tag_start(xml, i) {
        let Some(tag) = parse_tag(xml, at) else {
            break;
        };
        if tag.name == name {
            if tag.closing {
                depth -= 1;
                if depth == 0 {
                    return &xml[open_end..at];
                }
            } else if !tag.self_closing {
                depth += 1;
            }
        }
        i = tag.end;
    }
    &xml[open_end..]
}

struct Tag {
    name: String,
    attrs: BTreeMap<String, String>,
    closing: bool,
    self_closing: bool,
    end: usize,
}

fn find_tag_start(xml: &str, from: usize) -> Option<usize> {
    let mut i = from;
    while let Some(rel) = xml[i..].find('<') {
        let at = i + rel;
        let rest = &xml[at..];
        if rest.starts_with("<!--") {
            if let Some(end) = rest.find("-->") {
                i = at + end + 3;
                continue;
            }
            return None;
        }
        if rest.starts_with("<?") {
            if let Some(end) = rest.find("?>") {
                i = at + end + 2;
                continue;
            }
            return None;
        }
        if rest.starts_with("<!") {
            if let Some(end) = rest.find('>') {
                i = at + end + 1;
                continue;
            }
            return None;
        }
        return Some(at);
    }
    None
}

fn parse_tag(xml: &str, at: usize) -> Option<Tag> {
    if !xml[at..].starts_with('<') {
        return None;
    }
    let gt = xml[at..].find('>')? + at;
    let mut body = &xml[at + 1..gt];
    let closing = body.starts_with('/');
    if closing {
        body = &body[1..];
    }
    let self_closing = body.trim_end().ends_with('/');
    if self_closing {
        body = body.trim_end();
        body = &body[..body.len() - 1];
    }
    let name = body.split_whitespace().next().unwrap_or("").to_string();
    if name.is_empty() {
        return None;
    }
    Some(Tag {
        name,
        attrs: attrs(body),
        closing,
        self_closing,
        end: gt + 1,
    })
}

fn attrs(body: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    let mut s = match body.find(char::is_whitespace) {
        Some(i) => body[i..].trim_start(),
        None => return map,
    };
    while !s.is_empty() {
        let Some(eq) = s.find('=') else { break };
        let key = s[..eq].trim();
        if key.is_empty() {
            break;
        }
        s = s[eq + 1..].trim_start();
        let Some(quote) = s.chars().next() else { break };
        if quote != '"' && quote != '\'' {
            break;
        }
        s = &s[quote.len_utf8()..];
        let Some(end) = s.find(quote) else { break };
        map.insert(key.to_string(), xml_unescape(&s[..end]));
        s = s[end + quote.len_utf8()..].trim_start();
    }
    map
}

fn pad(out: &mut String, indent: usize) {
    for _ in 0..indent {
        out.push_str("  ");
    }
}

fn xml_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

fn xml_unescape(text: &str) -> String {
    text.replace("&apos;", "'")
        .replace("&quot;", "\"")
        .replace("&gt;", ">")
        .replace("&lt;", "<")
        .replace("&amp;", "&")
}

fn file_url(path: &str) -> String {
    let mut out = String::from("file://");
    for ch in path.chars() {
        match ch {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '/' | '-' | '_' | '.' | '~' => out.push(ch),
            ' ' => out.push_str("%20"),
            other => {
                let mut buf = [0_u8; 4];
                for byte in other.encode_utf8(&mut buf).as_bytes() {
                    out.push_str(&format!("%{byte:02X}"));
                }
            }
        }
    }
    out
}

fn file_url_to_path(url: &str) -> String {
    let rest = url.strip_prefix("file://").unwrap_or(url);
    let rest = rest.strip_prefix("localhost").unwrap_or(rest);
    percent_decode(rest)
}

fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(v) =
                u8::from_str_radix(std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or(""), 16)
            {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn b64_encode(data: &[u8]) -> String {
    const B64: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    let mut i = 0;
    while i + 3 <= data.len() {
        let n = ((data[i] as u32) << 16) | ((data[i + 1] as u32) << 8) | data[i + 2] as u32;
        out.push(B64[((n >> 18) & 63) as usize] as char);
        out.push(B64[((n >> 12) & 63) as usize] as char);
        out.push(B64[((n >> 6) & 63) as usize] as char);
        out.push(B64[(n & 63) as usize] as char);
        i += 3;
    }
    if i < data.len() {
        let rem = data.len() - i;
        let mut n = (data[i] as u32) << 16;
        if rem == 2 {
            n |= (data[i + 1] as u32) << 8;
        }
        out.push(B64[((n >> 18) & 63) as usize] as char);
        out.push(B64[((n >> 12) & 63) as usize] as char);
        if rem == 2 {
            out.push(B64[((n >> 6) & 63) as usize] as char);
            out.push('=');
        } else {
            out.push('=');
            out.push('=');
        }
    }
    out
}

fn b64_decode(text: &str) -> Result<Vec<u8>> {
    fn val(c: u8) -> Result<u8> {
        match c {
            b'A'..=b'Z' => Ok(c - b'A'),
            b'a'..=b'z' => Ok(c - b'a' + 26),
            b'0'..=b'9' => Ok(c - b'0' + 52),
            b'+' => Ok(62),
            b'/' => Ok(63),
            _ => Err(Error::invalid("bad base64 in graft sidecar")),
        }
    }
    let bytes: Vec<u8> = text.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
    if bytes.is_empty() {
        return Ok(Vec::new());
    }
    if bytes.len() % 4 != 0 {
        return Err(Error::invalid("bad base64 in graft sidecar"));
    }
    let mut out = Vec::new();
    for chunk in bytes.chunks(4) {
        let pad = chunk.iter().filter(|c| **c == b'=').count();
        let a = val(chunk[0])?;
        let b = val(chunk[1])?;
        let c = if chunk[2] == b'=' { 0 } else { val(chunk[2])? };
        let d = if chunk[3] == b'=' { 0 } else { val(chunk[3])? };
        let n = ((a as u32) << 18) | ((b as u32) << 12) | ((c as u32) << 6) | d as u32;
        out.push((n >> 16) as u8);
        if pad < 2 {
            out.push((n >> 8) as u8);
        }
        if pad < 1 {
            out.push(n as u8);
        }
    }
    Ok(out)
}

fn json_i64(value: &Value) -> Option<i64> {
    value.as_i64().or_else(|| {
        value
            .as_u64()
            .and_then(|n| i64::try_from(n).ok())
            .or_else(|| value.as_f64().map(|n| n.round() as i64))
    })
}

fn json_u64(value: &Value) -> Option<u64> {
    value.as_u64().or_else(|| {
        value
            .as_i64()
            .and_then(|n| u64::try_from(n).ok())
            .or_else(|| value.as_f64().map(|n| n.round() as u64))
    })
}

fn gcd_u64(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    a
}

#[cfg(test)]
mod tests {
    use super::*;
    use graft_score::{load_scion, load_score, CompositionGraph, GraphMode, GraphNode};

    fn example() -> (Score, Scion) {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/hook-v3-body-v1-9x16");
        let score = load_score(&dir.join("score.json")).unwrap();
        let scion = load_scion(&dir.join("scion.json")).unwrap();
        (score, scion)
    }

    fn tmp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "graft-guest-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn with_graph(score: &mut Score, scion: &mut Scion) {
        score.slots.push(graft_score::Slot {
            id: "overlay".into(),
            role: Role::Overlay,
            range: FrameRange::new(0, 90),
            optional: true,
            window: None,
        });
        score.slots.push(graft_score::Slot {
            id: "vo".into(),
            role: Role::Vo,
            range: FrameRange::new(0, 90),
            optional: false,
            window: None,
        });
        score.slots.push(graft_score::Slot {
            id: "bed".into(),
            role: Role::Bed,
            range: FrameRange::new(0, 90),
            optional: false,
            window: None,
        });
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
                params: Some(json!({"saturation": "1.1"})),
                keys: None,
                nodes: Vec::new(),
            }],
        });
        let hook = scion.layers[0].bindings["hook"].clone();
        scion.layers[0].bindings.insert(
            "overlay".into(),
            Binding {
                material: hook.material.clone(),
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
        let audio = Binding {
            material: hook.material,
            source: TimedRange {
                rate: FrameRate::new(30, 1),
                range: FrameRange::new(0, 90),
            },
            params: Some(json!({"speed": 1.0})),
            audio: None,
            nested: None,
            x: None,
            y: None,
            opacity: None,
            motion: None,
            remap: Vec::new(),
        };
        scion.layers[0].bindings.insert("vo".into(), audio.clone());
        scion.layers[0].bindings.insert("bed".into(), audio);
        score.joins.push(Join {
            left: "hook".into(),
            right: "body".into(),
            transition: "fadeblack".into(),
            duration_frames: 6,
        });
        score.markers.push(Marker {
            id: "punch".into(),
            range: FrameRange::new(0, 30),
        });
        for slot in &mut score.slots {
            if slot.id == "hook" {
                slot.range = FrameRange::new(0, 30);
            }
            if slot.id == "body" {
                slot.range = FrameRange::new(60, 30);
            }
        }
        scion.layers[0]
            .bindings
            .get_mut("hook")
            .unwrap()
            .source
            .range
            .duration = 30;
        scion.layers[0]
            .bindings
            .get_mut("body")
            .unwrap()
            .source
            .range
            .duration = 30;
        scion.validate_against_score(score).unwrap();
    }

    fn assert_same(score: &Score, before: &Scion, imported: &OtioImport) {
        assert_eq!(imported.loss["lost"][0], "effects");
        assert_eq!(imported.scion.graph, before.graph);
        assert_eq!(imported.joins, score.joins);
        assert_eq!(imported.markers, score.markers);
        let left = graft_score::effective_bindings(score, before).unwrap();
        let mut check = score.clone();
        apply_guest_edits(&mut check, imported);
        let right = graft_score::effective_bindings(&check, &imported.scion).unwrap();
        assert_eq!(left.len(), right.len());
        for (id, binding) in left {
            assert_eq!(right[&id].material, binding.material, "{id}");
            assert_eq!(right[&id].source, binding.source, "{id}");
            assert_eq!(right[&id].x, binding.x, "{id}");
            assert_eq!(right[&id].y, binding.y, "{id}");
            assert_eq!(right[&id].opacity, binding.opacity, "{id}");
        }
        assert_eq!(
            imported.scion.layers[0].bindings["overlay"].x.as_deref(),
            Some("8")
        );
    }

    #[test]
    fn fcpx_time_roundtrips_frames() {
        let rate = FrameRate::new(30, 1);
        assert_eq!(fcpx_time(12, rate), "2/5s");
        assert_eq!(frames_from_fcpx("2/5s", rate).unwrap(), 12);
        assert_eq!(fcpx_time(90, rate), "3s");
        assert_eq!(frames_from_fcpx("3s", rate).unwrap(), 90);
        assert_eq!(frames_from_fcpx("1s", rate).unwrap(), 30);
    }

    #[test]
    fn one_timeline_roundtrips_every_guest_file() {
        let (mut score, mut scion) = example();
        with_graph(&mut score, &mut scion);
        let dir = tmp("round");
        let media = BTreeMap::new();
        write_otio(&score, &scion, &dir.join("t.otio"), &media).unwrap();
        export_fcpxml(&score, &scion, &dir.join("t.fcpxml"), &media).unwrap();
        export_xmeml(&score, &scion, &dir.join("t.xml"), &media).unwrap();
        export_mlt(&score, &scion, &dir.join("t.mlt"), &media).unwrap();
        export_bundle(&score, &scion, &dir.join("guest"), &media).unwrap();
        for name in ["t.otio", "t.fcpxml", "t.xml", "t.mlt"] {
            let parsed = parse_guest_file(&dir.join(name)).unwrap();
            let imported = finish_import(&score, "imported", &parsed).unwrap();
            assert_same(&score, &scion, &imported);
        }
        let bundled = parse_guest_dir(&dir.join("guest")).unwrap();
        let imported = finish_import(&score, "imported", &bundled).unwrap();
        assert_same(&score, &scion, &imported);
        let loss: Value = graft_score::load_json(&dir.join("guest").join(LOSS_NAME)).unwrap();
        assert_eq!(loss["files"]["premiere_pro"], PREMIERE);
        assert_eq!(loss["files"]["final_cut_pro"], FINAL_CUT);
        assert_eq!(loss["files"]["davinci_resolve"], RESOLVE_OTIO);
        assert_eq!(loss["files"]["shotcut"], SHOTCUT);
        assert!(loss["no_timeline"]["capcut"]
            .as_str()
            .unwrap()
            .contains("public"));
        assert!(loss["no_timeline"]["photoshop"].is_string());
        assert!(loss["no_timeline"]["after_effects"].is_string());
        assert!(loss["eonik"].as_str().unwrap().contains("host"));
        assert_eq!(loss["lost"][0], "effects");
        let otio: Value = graft_score::load_json(&dir.join("t.otio")).unwrap();
        let audio: Vec<&Value> = otio["tracks"]["children"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|t| t["kind"] == "Audio")
            .collect();
        assert!(
            audio.len() >= 2,
            "overlapping vo and bed need two audio tracks"
        );
        let picture = otio["tracks"]["children"]
            .as_array()
            .unwrap()
            .iter()
            .find(|t| t["name"] == "picture")
            .unwrap();
        assert!(picture["children"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["OTIO_SCHEMA"] == "Gap.1"));
        let xml = fs::read_to_string(dir.join("t.fcpxml")).unwrap();
        assert!(xml.contains("<fcpxml version=\"1.9\">"));
        assert!(xml.contains("<gap "));
        let xmeml = fs::read_to_string(dir.join("t.xml")).unwrap();
        assert!(xmeml.contains("<xmeml version=\"5\">"));
        assert!(xmeml.contains("<in>12</in>"));
        assert!(xmeml.contains("<out>42</out>"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn file_url_keeps_the_hash() {
        let (score, scion) = example();
        let dir = tmp("media");
        let media_file = dir.join("hook.mov");
        fs::write(&media_file, b"not-a-movie").unwrap();
        let hash = scion.layers[0].bindings["hook"].material.clone();
        let mut media = BTreeMap::new();
        media.insert(hash.clone(), media_file.to_string_lossy().into_owned());
        let out = dir.join("t.otio");
        write_otio(&score, &scion, &out, &media).unwrap();
        let doc: Value = graft_score::load_json(&out).unwrap();
        let url = doc["tracks"]["children"][0]["children"][0]["media_reference"]["target_url"]
            .as_str()
            .unwrap();
        assert!(url.starts_with("file://"), "{url}");
        assert_eq!(
            doc["tracks"]["children"][0]["children"][0]["metadata"]["graft"]["material"],
            hash
        );
        let parsed = parse_guest_file(&out).unwrap();
        let imported = finish_import(&score, "imported", &parsed).unwrap();
        assert_eq!(imported.scion.layers[0].bindings["hook"].material, hash);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn foreign_fcpxml_without_sidecar_uses_clip_hashes() {
        let (score, _scion) = example();
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<fcpxml version="1.9">
  <resources>
    <format id="f1" frameDuration="1/30s" width="1080" height="1920"/>
  </resources>
  <library><event><project><sequence format="f1" duration="23s" tcStart="0s">
    <spine>
      <asset-clip name="hook" offset="0s" start="0s" duration="3s">
        <metadata><md key="com.eonik.graft.material" value="blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"/></metadata>
      </asset-clip>
      <asset-clip name="body" offset="3s" start="0s" duration="17s">
        <metadata><md key="com.eonik.graft.material" value="blake3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"/></metadata>
      </asset-clip>
      <asset-clip name="cta" offset="20s" start="0s" duration="3s">
        <metadata><md key="com.eonik.graft.material" value="blake3:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"/></metadata>
      </asset-clip>
    </spine>
  </sequence></project></event></library>
</fcpxml>
"#;
        let dir = tmp("foreign");
        let path = dir.join("foreign.fcpxml");
        fs::write(&path, xml).unwrap();
        let parsed = parse_guest_file(&path).unwrap();
        assert!(parsed.payload.is_none());
        let imported = finish_import(&score, "from-foreign", &parsed).unwrap();
        assert_eq!(
            imported.scion.layers[0].bindings["hook"].material,
            "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        );
        assert_eq!(
            imported.scion.layers[0].bindings["body"]
                .source
                .range
                .duration,
            510
        );
        assert_eq!(imported.scion.dest.width, 1080);
        assert_eq!(imported.scion.dest.id, "9x16");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn guest_trim_replaces_source_and_slot() {
        let (score, scion) = example();
        let dir = tmp("trim");
        let out = dir.join("t.fcpxml");
        export_fcpxml(&score, &scion, &out, &BTreeMap::new()).unwrap();
        let xml = fs::read_to_string(&out).unwrap();
        let mut edited = None;
        let mut search = 0;
        while let Some(rel) = xml[search..].find("<asset-clip") {
            let at = search + rel;
            let gt = xml[at..].find('>').unwrap() + at;
            let open = &xml[at..=gt];
            if open.contains("name=\"hook\"") {
                let key = "duration=\"";
                let rel = open.find(key).unwrap();
                let start = at + rel + key.len();
                let end = xml[start..].find('"').unwrap() + start;
                let mut next = xml.clone();
                next.replace_range(start..end, "1s");
                edited = Some(next);
                break;
            }
            search = gt;
        }
        fs::write(&out, edited.expect("hook clip")).unwrap();
        let parsed = parse_guest_file(&out).unwrap();
        let imported = finish_import(&score, "trimmed", &parsed).unwrap();
        assert_eq!(
            imported.scion.layers[0].bindings["hook"]
                .source
                .range
                .duration,
            30
        );
        assert_eq!(
            imported.scion.layers[0].bindings["hook"].source.range.start,
            12
        );
        assert_eq!(
            imported
                .slots
                .iter()
                .find(|s| s.id == "hook")
                .unwrap()
                .range
                .duration,
            30
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn capcut_draft_is_refused() {
        let dir = tmp("capcut");
        let path = dir.join("draft_info.json");
        fs::write(&path, b"{\"draft_materials\":[]}").unwrap();
        let err = parse_guest_file(&path).unwrap_err().to_string();
        assert!(err.contains("CapCut"), "{err}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn diverged_guest_directory_is_refused() {
        let (score, scion) = example();
        let dir = tmp("diverge");
        let guest = dir.join("guest");
        export_bundle(&score, &scion, &guest, &BTreeMap::new()).unwrap();
        let xml = fs::read_to_string(guest.join(FINAL_CUT)).unwrap();
        let mut edited = None;
        let mut search = 0;
        while let Some(rel) = xml[search..].find("<asset-clip") {
            let at = search + rel;
            let gt = xml[at..].find('>').unwrap() + at;
            let open = &xml[at..=gt];
            if open.contains("name=\"hook\"") {
                let key = "duration=\"";
                let rel = open.find(key).unwrap();
                let start = at + rel + key.len();
                let end = xml[start..].find('"').unwrap() + start;
                let mut next = xml.clone();
                next.replace_range(start..end, "2s");
                edited = Some(next);
                break;
            }
            search = gt;
        }
        fs::write(guest.join(FINAL_CUT), edited.expect("hook clip")).unwrap();
        let err = parse_guest_dir(&guest).unwrap_err().to_string();
        assert!(err.contains("disagree"), "{err}");
        let _ = fs::remove_dir_all(&dir);
    }
}
