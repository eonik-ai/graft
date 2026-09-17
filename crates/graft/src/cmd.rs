// SPDX-License-Identifier: Apache-2.0

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use anyhow::{bail, Context, Result};
use graft_cas::{Fs, Kind, Object, Remote, Store};
use graft_compile::{
    compile, declared_signal_range, dirty_from_signal_range, lower, materials_present,
    prev_overlay, probe_bytes, write_time_map_artifact, FfmpegX264, FrameIntra, MaterialKind,
    Unimplemented,
};
use graft_score::{
    effective_bindings, load_scion, load_score, load_time_map, merge_scions, parse_range, parse_wh,
    require_slot_id, resolve_scion, save_json, semantic_diff, AudioBinding, Binding, BindingLayer,
    BuildRecord, ChangeRequest, Dest, Encoder, Feedback, FeedbackResolution, FrameRange, FrameRate,
    Layer, Scion, Score, Slot, TimeMap, TimedRange, Window, DEFAULT_SPILL_FRAMES, GRAFT_SCHEMA,
};

use crate::{paths, preview};

pub fn init(dir: &Path, from: Option<PathBuf>) -> Result<()> {
    std::fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
    let path = paths::score(dir);
    if path.exists() {
        bail!("{} already exists", path.display());
    }
    if let Some(from) = from {
        return init_from_takes(dir, &paths::resolve_in_dir(dir, from));
    }
    let score = Score {
        graft: GRAFT_SCHEMA.into(),
        concept: uuid::Uuid::new_v4().to_string(),
        clock: graft_score::Clock {
            rate: FrameRate::new(30, 1),
            duration_frames: 90,
        },
        slots: vec![Slot {
            id: "hook".into(),
            role: graft_score::Role::Hook,
            range: FrameRange::new(0, 90),
            optional: false,
            window: Some(Window {
                kind: "hook_rate".into(),
                range: FrameRange::new(0, 90),
            }),
        }],
        layers: vec![Layer::Base, Layer::Copy, Layer::Grade, Layer::Legal],
        dest_default: Some("9x16".into()),
        spill_threshold_frames: DEFAULT_SPILL_FRAMES,
        joins: Vec::new(),
    };
    score.validate()?;
    save_json(&path, &score)?;
    std::fs::create_dir_all(paths::scions(dir))?;
    std::fs::create_dir_all(paths::feedback_dir(dir))?;
    std::fs::create_dir_all(paths::graft_dir(dir))?;
    eprintln!("wrote {}", path.display());
    Ok(())
}

pub fn slot(
    dir: &Path,
    id: String,
    role: Option<String>,
    span: Option<String>,
    window: Option<String>,
    window_kind: String,
    optional: bool,
) -> Result<()> {
    require_slot_id(&id)?;
    let path = paths::score(dir);
    let mut score = load_score(&path)?;
    let rate = score.clock.rate;
    let window_range = window
        .as_deref()
        .map(parse_range)
        .transpose()?
        .map(|r| FrameRange::from_seconds(rate, r.start, r.end))
        .transpose()?;
    let range = match (span.as_deref().map(parse_range).transpose()?, window_range) {
        (Some(s), _) => FrameRange::from_seconds(rate, s.start, s.end)?,
        (None, Some(w)) => w,
        (None, None) => score
            .slot(&id)
            .map(|slot| slot.range)
            .ok_or_else(|| anyhow::anyhow!("new slot {id} needs --span T0-T1 or --window T0-T1"))?,
    };
    let role = match role {
        Some(name) => graft_score::Role::parse(&name)?,
        None => graft_score::Role::parse(&id).or_else(|_| {
            score
                .slot(&id)
                .map(|s| s.role)
                .ok_or_else(|| graft_score::Error::invalid(format!("slot {id}: pass --role")))
        })?,
    };
    let declared = window_range.map(|range| Window {
        kind: window_kind,
        range,
    });
    if let Some(existing) = score.slot_mut(&id) {
        existing.role = role;
        existing.range = range;
        existing.optional = optional;
        if declared.is_some() {
            existing.window = declared;
        }
    } else {
        score.slots.push(Slot {
            id,
            role,
            range,
            optional,
            window: declared,
        });
    }
    score.recompute_duration();
    score.validate()?;
    save_json(&path, &score)?;
    eprintln!("wrote {}", path.display());
    Ok(())
}

pub fn scion_create(
    dir: &Path,
    id: String,
    dest_id: Option<String>,
    dest: String,
    pix_fmt: String,
    color: String,
    encoder: String,
) -> Result<()> {
    require_scion_id(&id)?;
    if paths::scion(dir, &id).exists() {
        bail!("scion {id} already exists");
    }
    let score = load_score(&paths::score(dir))?;
    let dest_id = dest_id
        .or_else(|| score.dest_default.clone())
        .unwrap_or_else(|| id.clone());
    let (width, height) = parse_wh(&dest)?;
    let mut encoder = Encoder::named(&encoder)?;
    if matches!(encoder.impl_name.as_str(), "x264" | "libx264") {
        encoder.toolchain = Some(FfmpegX264::runtime_toolchain()?);
        if let Some(digest) = encoder
            .toolchain
            .as_ref()
            .and_then(|value| value.get("build_digest"))
            .and_then(|value| value.as_str())
        {
            encoder.version = digest.to_string();
        }
    }
    let scion = Scion {
        graft: GRAFT_SCHEMA.into(),
        id: id.clone(),
        concept: score.concept,
        parent: None,
        change_request: None,
        dest: Dest {
            id: dest_id,
            width,
            height,
            rate: score.clock.rate,
            pix_fmt,
            color,
            encoder,
        },
        layers: vec![BindingLayer {
            name: Layer::Base,
            bindings: BTreeMap::new(),
        }],
    };
    scion.validate()?;
    save_json(&paths::scion(dir, &id), &scion)?;
    write_head(dir, &id)?;
    eprintln!("wrote {}", paths::scion(dir, &id).display());
    Ok(())
}

pub fn scion_fork(dir: &Path, source: String, id: String) -> Result<()> {
    require_scion_id(&id)?;
    if paths::scion(dir, &id).exists() {
        bail!("scion {id} already exists");
    }
    let all = load_scions(dir)?;
    let parent = all
        .get(&source)
        .with_context(|| format!("unknown scion {source}"))?;
    let child = Scion {
        graft: GRAFT_SCHEMA.into(),
        id: id.clone(),
        concept: parent.concept.clone(),
        parent: Some(source),
        change_request: None,
        dest: parent.dest.clone(),
        layers: Vec::new(),
    };
    child.validate()?;
    save_json(&paths::scion(dir, &id), &child)?;
    write_head(dir, &id)?;
    eprintln!("wrote {}", paths::scion(dir, &id).display());
    Ok(())
}

pub fn scion_list(dir: &Path) -> Result<()> {
    let selected = selected_scion_id(dir, None).ok();
    let rows: Vec<serde_json::Value> = load_scions(dir)?
        .values()
        .map(|scion| {
            serde_json::json!({
                "id": scion.id,
                "parent": scion.parent,
                "dest": scion.dest.id,
                "selected": selected.as_deref() == Some(scion.id.as_str())
            })
        })
        .collect();
    paths::print_json(&rows)
}

pub fn scion_show(dir: &Path, id: String) -> Result<()> {
    let all = load_scions(dir)?;
    let raw = all
        .get(&id)
        .with_context(|| format!("unknown scion {id}"))?;
    let resolved = resolve_scion(&all, &id)?;
    paths::print_json(&serde_json::json!({"raw": raw, "resolved": resolved}))
}

pub fn scion_use(dir: &Path, id: String) -> Result<()> {
    if !paths::scion(dir, &id).is_file() {
        bail!("unknown scion {id}");
    }
    write_head(dir, &id)
}

#[allow(clippy::too_many_arguments)]
pub fn bind(
    dir: &Path,
    slot: String,
    material: String,
    in_s: Option<f64>,
    out_s: Option<f64>,
    speed: Option<f64>,
    scion: Option<String>,
    layer: String,
) -> Result<()> {
    require_slot_id(&slot)?;
    let score = load_score(&paths::score(dir))?;
    let slot_def = score
        .slot(&slot)
        .with_context(|| format!("score has no slot {slot}"))?;
    let id = selected_scion_id(dir, scion)?;
    let path = paths::scion(dir, &id);
    let mut scion = load_scion(&path)?;
    let layer = Layer::parse(&layer)?;
    if !score.layers.contains(&layer) {
        bail!("layer {layer} is not declared by score.layers");
    }
    let resolved = resolve_material(dir, &material, slot_def.role)?;
    let rate = resolved
        .probe
        .as_ref()
        .filter(|probe| probe.fps.is_finite() && probe.fps > 0.0)
        .map(|probe| FrameRate::from_f64(probe.fps))
        .transpose()?
        .unwrap_or(score.clock.rate);
    let start = in_s.unwrap_or(0.0);
    let duration = score
        .clock
        .rate
        .seconds_from_frames(slot_def.range.duration as i64);
    let end = out_s.unwrap_or(start + duration);
    let source = TimedRange::from_seconds(rate, start, end)?;
    let params = speed.map(|value| serde_json::json!({ "speed": value }));
    let audio = resolved
        .probe
        .as_ref()
        .filter(|probe| probe.has_audio)
        .map(|_| AudioBinding {
            material: resolved.id.clone(),
            source,
        });
    scion.ensure_layer(layer).bindings.insert(
        slot,
        Binding {
            material: resolved.id,
            source,
            params,
            audio,
        },
    );
    scion.validate()?;
    save_json(&path, &scion)?;
    eprintln!("wrote {}", path.display());
    Ok(())
}

pub fn dirty(dir: &Path, scion: Option<String>, prev: Option<PathBuf>) -> Result<()> {
    let score = load_score(&paths::score(dir))?;
    let (_, scion) = load_selected_scion(dir, scion)?;
    let prev = prev
        .map(|path| load_scion(&paths::resolve_in_dir(dir, path)))
        .transpose()?;
    let store = Fs::open(paths::graft_dir(dir))?;
    let outcome = compile_with_backend(&score, &scion, &store, false)?;
    let mut plan = outcome.plan;
    if let Some(previous) = &prev {
        plan.prev_overlay = Some(prev_overlay(&score, &scion, previous));
    }
    let slots: Vec<String> = plan
        .slots
        .iter()
        .filter(|slot| slot.cache == "miss")
        .map(|slot| slot.id.clone())
        .collect();
    let kerfs: Vec<Vec<String>> = plan
        .kerfs
        .iter()
        .filter(|kerf| kerf.cache == "miss")
        .map(|kerf| kerf.join.clone())
        .collect();
    let clean: Vec<String> = plan
        .slots
        .iter()
        .filter(|slot| slot.cache == "hit")
        .map(|slot| slot.id.clone())
        .collect();
    let captions: Vec<String> = plan
        .captions
        .iter()
        .filter(|slot| slot.cache == "miss")
        .map(|slot| slot.id.clone())
        .collect();
    let overlay_audio: Vec<String> = plan
        .overlay_audio
        .iter()
        .filter(|slot| slot.cache == "miss")
        .map(|slot| slot.id.clone())
        .collect();
    let mut mixes = Vec::new();
    if plan.audio_mix == Some("miss") {
        mixes.push("audio_mix");
    }
    if plan.overlay_mix == Some("miss") {
        mixes.push("overlay_mix");
    }
    paths::print_json(&serde_json::json!({
        "graft": GRAFT_SCHEMA,
        "kind": "compile",
        "scion": scion.id,
        "scion_hash": plan.scion_hash,
        "slots": slots,
        "kerfs": kerfs,
        "clean": clean,
        "overlay_audio": overlay_audio,
        "captions": captions,
        "mixes": mixes,
        "prev_overlay": plan.prev_overlay
    }))
}

pub fn signal(dir: &Path, kind: String, t: Option<String>, build: String) -> Result<()> {
    let score = load_score(&paths::score(dir))?;
    let (_, time_map) = load_build(dir, &build)?;
    let range = signal_range(&score, &time_map, &kind, t.as_deref())?;
    let dirty = dirty_from_signal_range(
        &time_map,
        &score,
        &kind,
        range,
        Some(time_map.dest_id.clone()),
    );
    paths::print_json(&dirty)
}

pub fn compile_cmd(
    dir: &Path,
    scion: Option<String>,
    prev: Option<PathBuf>,
    out: Option<PathBuf>,
) -> Result<()> {
    let report = compile_report(dir, scion, prev, out)?;
    paths::print_json(&report.json)
}

fn compile_report(
    dir: &Path,
    scion: Option<String>,
    prev: Option<PathBuf>,
    out: Option<PathBuf>,
) -> Result<CompileReport> {
    let score = load_score(&paths::score(dir))?;
    let (_, scion) = load_selected_scion(dir, scion)?;
    let previous = prev
        .map(|path| load_scion(&paths::resolve_in_dir(dir, path)))
        .transpose()?;
    let store_root = paths::graft_dir(dir);
    let store = Fs::open(&store_root)?;
    let started = Instant::now();
    let mut outcome = compile_with_backend(&score, &scion, &store, true)?;
    let encode_ms = started.elapsed().as_millis() as u64;
    if let Some(previous) = &previous {
        outcome.plan.prev_overlay = Some(prev_overlay(&score, &scion, previous));
    }
    let map_path =
        write_time_map_artifact(&store_root, &outcome.graph.concat.key, &outcome.time_map)?;
    let mut output_path = None;
    if let Some(blob) = &outcome.concat {
        let bytes = store.get_blob(Kind::Concat, blob)?;
        let name = if FrameIntra::supports(&scion.dest) {
            "dest.gfi1"
        } else {
            "dest.mp4"
        };
        let build_dir =
            graft_compile::time_map_artifact_dir(&store_root, &outcome.graph.concat.key);
        let dest_path = build_dir.join(name);
        std::fs::write(&dest_path, &bytes)?;
        output_path = Some(dest_path.clone());
        eprintln!("wrote {}", dest_path.display());
        if let Some(user_out) = out {
            let user_out = paths::resolve_in_dir(dir, user_out);
            if let Some(parent) = user_out.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&user_out, &bytes)?;
            eprintln!("wrote {}", user_out.display());
            output_path = Some(user_out.clone());
            for (slot_id, blob) in &outcome.captions {
                let text = store.get_blob(Kind::Captions, blob)?;
                let sidecar = user_out.with_extension("vtt");
                std::fs::write(&sidecar, text)?;
                eprintln!("wrote {} ({slot_id})", sidecar.display());
            }
        }
    }
    let build = BuildRecord {
        graft: GRAFT_SCHEMA.into(),
        id: outcome.plan.scion_hash.clone(),
        scion: scion.id.clone(),
        scion_hash: outcome.plan.scion_hash.clone(),
        dest_id: scion.dest.id.clone(),
        time_map: relative_to(dir, &map_path),
        output: output_path.as_ref().map(|path| relative_to(dir, path)),
    };
    build.validate()?;
    let build_path = paths::build(dir, &build.id).join("build.json");
    save_json(&build_path, &build)?;
    if !outcome.encoded {
        eprintln!("plan only — missing essence or no enabled backend");
    }
    let ledger = ledger_from_plan(&scion.id, &outcome.plan, encode_ms, outcome.encoded, &build);
    print_ledger_table(&ledger);
    Ok(CompileReport {
        json: serde_json::json!({"build": build, "plan": outcome.plan, "ledger": ledger}),
        encoded: outcome.encoded,
    })
}

pub fn diff(dir: &Path, left: String, right: String) -> Result<()> {
    let score = load_score(&paths::score(dir))?;
    let all = load_scions(dir)?;
    let left = resolve_scion(&all, &left)?;
    let right = resolve_scion(&all, &right)?;
    paths::print_json(&semantic_diff(&score, &left, &right)?)
}

pub fn merge(dir: &Path, base: String, ours: String, theirs: String, id: String) -> Result<()> {
    if paths::scion(dir, &id).exists() {
        bail!("scion {id} already exists");
    }
    let score = load_score(&paths::score(dir))?;
    let all = load_scions(dir)?;
    let result = merge_scions(
        &score,
        &id,
        &resolve_scion(&all, &base)?,
        &resolve_scion(&all, &ours)?,
        &resolve_scion(&all, &theirs)?,
    )?;
    if let Some(merged) = &result.merged {
        save_json(&paths::scion(dir, &id), merged)?;
        write_head(dir, &id)?;
    }
    paths::print_json(&result)?;
    if result.merged.is_none() {
        bail!("semantic merge has {} conflict(s)", result.conflicts.len());
    }
    Ok(())
}

pub fn feedback_ingest(dir: &Path, file: PathBuf) -> Result<()> {
    let file = paths::resolve_in_dir(dir, file);
    let items = if file.extension().and_then(|ext| ext.to_str()) == Some("csv") {
        parse_feedback_csv(&file)?
    } else {
        parse_feedback_json(&file)?
    };
    let score = load_score(&paths::score(dir))?;
    let mut written = Vec::new();
    for raw in items {
        let id = raw
            .get("id")
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let build = string_field(&raw, "build")?;
        let kind = string_field(&raw, "kind")?;
        let (_, time_map) = load_build(dir, &build)?;
        let range = if let Some(range) = raw.get("range") {
            serde_json::from_value(range.clone())?
        } else if let Some(declared) = declared_signal_range(&score, &kind) {
            declared
        } else {
            let t0 = number_field(&raw, "t0_s")?;
            let t1 = number_field(&raw, "t1_s")?;
            FrameRange::from_seconds(time_map.rate, t0, t1)?
        };
        let dirty = dirty_from_signal_range(
            &time_map,
            &score,
            &kind,
            range,
            Some(time_map.dest_id.clone()),
        );
        let feedback = Feedback {
            graft: GRAFT_SCHEMA.into(),
            id: id.clone(),
            build,
            kind,
            range: dirty.signal.range,
            raw,
            resolved: Some(FeedbackResolution {
                slots: dirty.slots,
                kerfs: dirty.kerfs,
                warnings: dirty.warnings,
            }),
        };
        feedback.validate()?;
        save_json(&paths::feedback(dir, &id), &feedback)?;
        written.push(id);
    }
    paths::print_json(&serde_json::json!({"written": written}))
}

pub fn feedback_list(dir: &Path) -> Result<()> {
    let mut values = Vec::new();
    if paths::feedback_dir(dir).is_dir() {
        for entry in std::fs::read_dir(paths::feedback_dir(dir))? {
            let path = entry?.path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
                values.push(graft_score::load_json::<serde_json::Value>(&path)?);
            }
        }
    }
    values.sort_by_key(|value| {
        value
            .get("id")
            .and_then(|id| id.as_str())
            .unwrap_or("")
            .to_string()
    });
    paths::print_json(&values)
}

pub fn feedback_show(dir: &Path, id: String) -> Result<()> {
    let feedback: Feedback = graft_score::load_json(&paths::feedback(dir, &id))?;
    feedback.validate()?;
    paths::print_json(&feedback)
}

pub fn iterate(dir: &Path, from: String, feedback: String, scion: String) -> Result<()> {
    let value = iterate_inner(dir, from, feedback, scion)?;
    paths::print_json(&value)
}

fn iterate_inner(
    dir: &Path,
    from: String,
    feedback: String,
    scion: String,
) -> Result<serde_json::Value> {
    if paths::scion(dir, &scion).exists() {
        bail!("scion {scion} already exists");
    }
    let (build, _) = load_build(dir, &from)?;
    let item: Feedback = graft_score::load_json(&paths::feedback(dir, &feedback))?;
    if item.build != build.id {
        bail!(
            "feedback {} addresses build {}, not {}",
            item.id,
            item.build,
            build.id
        );
    }
    let resolved = item
        .resolved
        .as_ref()
        .context("feedback has not been resolved")?;
    let all = load_scions(dir)?;
    let parent = all
        .get(&build.scion)
        .with_context(|| format!("build source scion {} is missing", build.scion))?;
    let child = Scion {
        graft: GRAFT_SCHEMA.into(),
        id: scion.clone(),
        concept: parent.concept.clone(),
        parent: Some(parent.id.clone()),
        change_request: Some(ChangeRequest {
            feedback: item.id,
            slots: resolved.slots.clone(),
        }),
        dest: parent.dest.clone(),
        layers: Vec::new(),
    };
    child.validate()?;
    save_json(&paths::scion(dir, &scion), &child)?;
    write_head(dir, &scion)?;
    Ok(serde_json::json!({
        "scion": scion,
        "parent": parent.id,
        "feedback": feedback,
        "dirty_slots": resolved.slots,
        "creative_replacement": "required"
    }))
}

pub fn export(dir: &Path, format: &str, scion: Option<String>, out: PathBuf) -> Result<()> {
    let score = load_score(&paths::score(dir))?;
    let (_, scion) = load_selected_scion(dir, scion)?;
    match format {
        "otio" => {
            let out = paths::resolve_in_dir(dir, out);
            paths::print_json(&graft_otio::export_otio(&score, &scion, &out)?)
        }
        _ => bail!("unsupported export format {format}"),
    }
}

pub fn import(dir: &Path, format: &str, file: PathBuf, id: String) -> Result<()> {
    if paths::scion(dir, &id).exists() {
        bail!("scion {id} already exists");
    }
    let score = load_score(&paths::score(dir))?;
    match format {
        "otio" => {
            let (scion, loss) =
                graft_otio::import_otio(&score, &paths::resolve_in_dir(dir, file), &id)?;
            save_json(&paths::scion(dir, &id), &scion)?;
            save_json(&paths::scion(dir, &id).with_extension("loss.json"), &loss)?;
            write_head(dir, &id)?;
            paths::print_json(&serde_json::json!({"scion": id, "loss": loss}))
        }
        _ => bail!("unsupported import format {format}"),
    }
}

pub fn preview(dir: &Path, scion: Option<String>, out: PathBuf) -> Result<()> {
    let score = load_score(&paths::score(dir))?;
    let (_, scion) = load_selected_scion(dir, scion)?;
    let store = Fs::open(paths::graft_dir(dir))?;
    preview::render(dir, &score, &scion, &store, &out)
}

pub fn migrate(dir: &Path) -> Result<()> {
    let score_path = paths::score(dir);
    let old_score: serde_json::Value = graft_score::load_json(&score_path)?;
    if old_score.get("graft").and_then(|v| v.as_str()) != Some("0.1.0") {
        bail!("migrate expects a 0.1.0 score.json");
    }
    let fps = old_score
        .pointer("/clock/fps")
        .and_then(|v| v.as_f64())
        .context("old score clock.fps")?;
    let rate = FrameRate::from_f64(fps)?;
    let duration = old_score
        .pointer("/clock/duration_s")
        .and_then(|v| v.as_f64())
        .context("old score clock.duration_s")?;
    let mut new_score = old_score.clone();
    new_score["graft"] = serde_json::json!(GRAFT_SCHEMA);
    new_score["clock"] = serde_json::json!({
        "rate": rate,
        "duration_frames": rate.frames_from_seconds(duration)?
    });
    if let Some(slots) = new_score.get_mut("slots").and_then(|v| v.as_array_mut()) {
        for slot in slots {
            if let Some(span) = slot.get("span").and_then(|v| v.as_array()) {
                let range = FrameRange::from_seconds(
                    rate,
                    span[0].as_f64().context("slot span start")?,
                    span[1].as_f64().context("slot span end")?,
                )?;
                slot.as_object_mut().unwrap().remove("span");
                slot["range"] = serde_json::to_value(range)?;
            }
            if let Some(window) = slot.get_mut("window") {
                if let Some(span) = window.get("span").and_then(|v| v.as_array()) {
                    let range = FrameRange::from_seconds(
                        rate,
                        span[0].as_f64().context("window span start")?,
                        span[1].as_f64().context("window span end")?,
                    )?;
                    window.as_object_mut().unwrap().remove("span");
                    window["range"] = serde_json::to_value(range)?;
                }
            }
        }
    }
    new_score
        .as_object_mut()
        .unwrap()
        .remove("spill_threshold_s");
    new_score["spill_threshold_frames"] = serde_json::json!(DEFAULT_SPILL_FRAMES);
    let backup = dir.join("score.0.1.0.json");
    std::fs::copy(&score_path, &backup)?;
    save_json(&score_path, &new_score)?;

    let old_scion_path = dir.join("scion.json");
    if old_scion_path.is_file() {
        let old: serde_json::Value = graft_score::load_json(&old_scion_path)?;
        let id = string_field(&old, "id")?;
        let mut bindings = serde_json::Map::new();
        for (slot, binding) in old
            .get("bindings")
            .and_then(|v| v.as_object())
            .context("old scion.bindings")?
        {
            let start = number_field(binding, "in_s")?;
            let end = number_field(binding, "out_s")?;
            bindings.insert(
                slot.clone(),
                serde_json::json!({
                    "material": binding["material"],
                    "source": TimedRange::from_seconds(rate, start, end)?,
                    "params": binding.get("params").cloned()
                }),
            );
        }
        let mut dest = old["dest"].clone();
        dest.as_object_mut().unwrap().remove("fps");
        dest["rate"] = serde_json::to_value(rate)?;
        let migrated = serde_json::json!({
            "graft": GRAFT_SCHEMA,
            "id": id,
            "concept": old["concept"],
            "dest": dest,
            "layers": [{"name": "base", "bindings": bindings}]
        });
        std::fs::create_dir_all(paths::scions(dir))?;
        save_json(&paths::scion(dir, &id), &migrated)?;
        std::fs::rename(&old_scion_path, dir.join("scion.0.1.0.json"))?;
        write_head(dir, &id)?;
    }
    eprintln!("migrated 0.1.0 documents; backups retain the old format");
    Ok(())
}

fn compile_with_backend(
    score: &Score,
    scion: &Scion,
    store: &Fs,
    encode: bool,
) -> Result<graft_compile::CompileOutcome> {
    if !encode {
        return Ok(compile(
            score,
            scion,
            store,
            &Unimplemented,
            &graft_compile::ConcatUnimplemented,
        )?);
    }
    if FrameIntra::supports(&scion.dest) {
        return Ok(compile(score, scion, store, &FrameIntra, &FrameIntra)?);
    }
    if FfmpegX264::supports(&scion.dest) {
        let ff = FfmpegX264::from_env()?;
        let graph = lower(score, scion)?;
        if materials_present(&graph, store) {
            return Ok(compile(score, scion, store, &ff, &ff)?);
        }
    }
    Ok(compile(
        score,
        scion,
        store,
        &Unimplemented,
        &graft_compile::ConcatUnimplemented,
    )?)
}

struct ResolvedMaterial {
    id: String,
    probe: Option<graft_compile::Probe>,
}

fn resolve_material(dir: &Path, spec: &str, role: graft_score::Role) -> Result<ResolvedMaterial> {
    if graft_score::is_material_id(spec) {
        return Ok(ResolvedMaterial {
            id: spec.into(),
            probe: None,
        });
    }
    let path = Path::new(spec);
    if !path.is_file() {
        bail!("material must be blake3:<64 hex> or an existing file");
    }
    let bytes = std::fs::read(path)?;
    let probe = match probe_bytes(&bytes) {
        MaterialKind::Gfi1 {
            width,
            height,
            nframes,
        } => {
            eprintln!("probe: graft-intra {width}x{height} {nframes} frames");
            None
        }
        MaterialKind::Media(probe) => {
            eprintln!(
                "probe: {} {}x{} {:.3}s @ {:.3} fps{}",
                probe.video_codec,
                probe.width,
                probe.height,
                probe.duration_s,
                probe.fps,
                if probe.has_audio { " + audio" } else { "" }
            );
            Some(probe)
        }
        MaterialKind::Opaque { bytes: size } => {
            if role == graft_score::Role::Captions && looks_like_captions(&bytes) {
                eprintln!("probe: captions {size} bytes");
                None
            } else {
                bail!("material is not decodable media ({size} bytes)")
            }
        }
    };
    let store = Fs::open(paths::graft_dir(dir))?;
    let id = store.put_blob(Kind::Material, &bytes)?.to_string();
    Ok(ResolvedMaterial { id, probe })
}

fn looks_like_captions(bytes: &[u8]) -> bool {
    let text = String::from_utf8_lossy(bytes);
    let trimmed = text.trim_start();
    trimmed.starts_with("WEBVTT") || trimmed.contains("-->")
}

fn load_scions(dir: &Path) -> Result<BTreeMap<String, Scion>> {
    let mut out = BTreeMap::new();
    if !paths::scions(dir).is_dir() {
        return Ok(out);
    }
    for entry in std::fs::read_dir(paths::scions(dir))? {
        let path = entry?.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json")
            || path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(".loss.json"))
        {
            continue;
        }
        let scion = load_scion(&path)?;
        if out.insert(scion.id.clone(), scion).is_some() {
            bail!("duplicate scion id in {}", paths::scions(dir).display());
        }
    }
    if paths::score(dir).is_file() {
        let score = load_score(&paths::score(dir))?;
        graft_score::validate_family(&score, &out)?;
    }
    Ok(out)
}

fn selected_scion_id(dir: &Path, explicit: Option<String>) -> Result<String> {
    if let Some(id) = explicit {
        return Ok(id);
    }
    if let Ok(id) = std::fs::read_to_string(paths::head(dir)) {
        let id = id.trim();
        if !id.is_empty() {
            return Ok(id.into());
        }
    }
    let all = load_scions(dir)?;
    match all.len() {
        1 => Ok(all.keys().next().unwrap().clone()),
        0 => bail!("no scions — run `graft scion create`"),
        _ => bail!("multiple scions — pass --scion or run `graft scion use`"),
    }
}

fn load_selected_scion(dir: &Path, explicit: Option<String>) -> Result<(String, Scion)> {
    let id = selected_scion_id(dir, explicit)?;
    let all = load_scions(dir)?;
    let resolved = resolve_scion(&all, &id)?;
    Ok((id, resolved))
}

fn write_head(dir: &Path, id: &str) -> Result<()> {
    std::fs::create_dir_all(paths::graft_dir(dir))?;
    std::fs::write(paths::head(dir), format!("{id}\n"))?;
    eprintln!("selected scion {id}");
    Ok(())
}

fn load_build(dir: &Path, id: &str) -> Result<(BuildRecord, TimeMap)> {
    let root = paths::build(dir, id);
    let build: BuildRecord = graft_score::load_json(&root.join("build.json"))?;
    build.validate()?;
    let time_map = load_time_map(&paths::resolve_in_dir(dir, PathBuf::from(&build.time_map)))?;
    if time_map.build != build.id || time_map.scion_hash != build.scion_hash {
        bail!("build and time-map identity disagree");
    }
    Ok((build, time_map))
}

fn signal_range(
    score: &Score,
    time_map: &TimeMap,
    kind: &str,
    text: Option<&str>,
) -> Result<FrameRange> {
    if let Some(range) = declared_signal_range(score, kind) {
        return Ok(range);
    }
    let text =
        text.with_context(|| format!("signal {kind} has no declared window; pass --t T0-T1"))?;
    let parsed = parse_range(text)?;
    Ok(FrameRange::from_seconds(
        time_map.rate,
        parsed.start,
        parsed.end,
    )?)
}

fn require_scion_id(id: &str) -> Result<()> {
    if id.is_empty()
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'@' | b'+'))
    {
        bail!("scion id must use letters, digits, -, _, +, or @");
    }
    Ok(())
}

fn relative_to(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .into_owned()
}

fn parse_feedback_json(path: &Path) -> Result<Vec<serde_json::Value>> {
    let value: serde_json::Value = graft_score::load_json(path)?;
    Ok(match value {
        serde_json::Value::Array(values) => values,
        value => vec![value],
    })
}

fn parse_feedback_csv(path: &Path) -> Result<Vec<serde_json::Value>> {
    let text = std::fs::read_to_string(path)?;
    let mut lines = text.lines();
    let headers: Vec<&str> = lines
        .next()
        .context("feedback CSV is empty")?
        .split(',')
        .map(str::trim)
        .collect();
    let mut out = Vec::new();
    for (line_no, line) in lines.enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let values: Vec<&str> = line.split(',').map(str::trim).collect();
        if values.len() != headers.len() {
            bail!(
                "feedback CSV line {} has the wrong column count",
                line_no + 2
            );
        }
        let mut object = serde_json::Map::new();
        for (header, value) in headers.iter().zip(values) {
            let value = match *header {
                "t0_s" | "t1_s" => serde_json::json!(value.parse::<f64>()?),
                _ => serde_json::json!(value),
            };
            object.insert((*header).into(), value);
        }
        out.push(serde_json::Value::Object(object));
    }
    Ok(out)
}

fn string_field(value: &serde_json::Value, key: &str) -> Result<String> {
    value
        .get(key)
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .with_context(|| format!("{key} must be a string"))
}

fn number_field(value: &serde_json::Value, key: &str) -> Result<f64> {
    value
        .get(key)
        .and_then(|value| value.as_f64())
        .with_context(|| format!("{key} must be a number"))
}

struct CompileReport {
    json: serde_json::Value,
    encoded: bool,
}

const TAKE_EXTS: &[&str] = &[
    "mov", "mp4", "mkv", "webm", "m4v", "wav", "aiff", "aif", "m4a", "aac", "vtt", "srt",
];
const ROLE_ORDER: &[&str] = &[
    "hook", "body", "proof", "cta", "vo", "captions", "bed", "brand",
];

struct DiscoveredTake {
    slot: String,
    role: graft_score::Role,
    path: PathBuf,
}

pub fn ship(
    dir: &Path,
    from: PathBuf,
    out: PathBuf,
    scion: Option<String>,
    dest: Option<String>,
    encoder: String,
) -> Result<()> {
    let from = paths::resolve_in_dir(dir, from);
    let existed = paths::score(dir).is_file();
    if !existed {
        init(dir, Some(from.clone()))?;
    } else {
        add_missing_slots_from_takes(dir, &from)?;
    }
    ensure_root_scion(dir, scion.clone(), dest, encoder, &from)?;
    if existed {
        bind_takes(dir, &from)?;
    }
    require_spine_bound(dir)?;
    let report = compile_report(dir, scion, None, Some(out))?;
    if !report.encoded {
        bail!("ship compiled a plan only — bind real takes and install ffmpeg");
    }
    paths::print_json(&report.json)
}

pub fn swap(
    dir: &Path,
    slot: String,
    file: Option<PathBuf>,
    from: Option<PathBuf>,
    out: Option<PathBuf>,
    out_dir: Option<PathBuf>,
    scion: Option<String>,
) -> Result<()> {
    require_slot_id(&slot)?;
    match (file, from) {
        (None, None) => bail!("graft swap {slot} needs a take file or --from <dir>"),
        (Some(_), Some(_)) => bail!("pass a take file or --from, not both"),
        (Some(file), None) => {
            let report = swap_one(dir, &slot, &paths::resolve_in_dir(dir, file), scion, out)?;
            paths::print_json(&report.json)
        }
        (None, Some(from)) => {
            let from = paths::resolve_in_dir(dir, from);
            let files = list_pool_takes(&from)?;
            if files.is_empty() {
                bail!("no takes in {}", from.display());
            }
            let out_dir =
                paths::resolve_in_dir(dir, out_dir.unwrap_or_else(|| PathBuf::from("out")));
            std::fs::create_dir_all(&out_dir)?;
            let parent = selected_scion_id(dir, scion)?;
            let mut variants = Vec::new();
            for file in files {
                let stem = file.file_stem().and_then(|s| s.to_str()).unwrap_or("take");
                let dest = out_dir.join(format!("{slot}-{stem}.mp4"));
                let report = swap_one(dir, &slot, &file, Some(parent.clone()), Some(dest))?;
                variants.push(report.json);
            }
            paths::print_json(&serde_json::json!({
                "parent": parent,
                "slot": slot,
                "variants": variants
            }))
        }
    }
}

pub fn address(
    dir: &Path,
    kind: String,
    t: Option<String>,
    build: String,
    scion: Option<String>,
) -> Result<()> {
    let score = load_score(&paths::score(dir))?;
    let (record, time_map) = load_build(dir, &build)?;
    let range = signal_range(&score, &time_map, &kind, t.as_deref())?;
    let dirty = dirty_from_signal_range(
        &time_map,
        &score,
        &kind,
        range,
        Some(time_map.dest_id.clone()),
    );
    let feedback_id = unique_feedback_id(dir, &kind)?;
    let feedback = Feedback {
        graft: GRAFT_SCHEMA.into(),
        id: feedback_id.clone(),
        build: build.clone(),
        kind: kind.clone(),
        range: dirty.signal.range,
        raw: serde_json::json!({ "kind": kind, "build": build }),
        resolved: Some(FeedbackResolution {
            slots: dirty.slots.clone(),
            kerfs: dirty.kerfs.clone(),
            warnings: dirty.warnings.clone(),
        }),
    };
    feedback.validate()?;
    std::fs::create_dir_all(paths::feedback_dir(dir))?;
    save_json(&paths::feedback(dir, &feedback_id), &feedback)?;
    let slot = dirty.slots.first().cloned().unwrap_or_else(|| kind.clone());
    let child = match scion {
        Some(id) => id,
        None => unique_scion_id(dir, &format!("{slot}-next"))?,
    };
    let mut iterated = iterate_inner(dir, record.id.clone(), feedback_id.clone(), child.clone())?;
    let next = format!("graft swap {slot} <file>");
    eprintln!("next  {next}");
    if let Some(obj) = iterated.as_object_mut() {
        obj.insert("kind".into(), serde_json::json!(kind));
        obj.insert("build".into(), serde_json::json!(build));
        obj.insert("kerfs".into(), serde_json::json!(dirty.kerfs));
        obj.insert("clean".into(), serde_json::json!(dirty.clean));
        obj.insert("next".into(), serde_json::json!(next));
    }
    paths::print_json(&iterated)
}

fn swap_one(
    dir: &Path,
    slot: &str,
    file: &Path,
    parent: Option<String>,
    out: Option<PathBuf>,
) -> Result<CompileReport> {
    let score = load_score(&paths::score(dir))?;
    if score.slot(slot).is_none() {
        bail!("score has no slot {slot}");
    }
    let source_id = selected_scion_id(dir, parent)?;
    let all = load_scions(dir)?;
    let source = all
        .get(&source_id)
        .with_context(|| format!("unknown scion {source_id}"))?;
    let binds_locally = source
        .layers
        .iter()
        .any(|layer| layer.bindings.contains_key(slot));
    let child_id = if binds_locally {
        let stem = file.file_stem().and_then(|s| s.to_str()).unwrap_or("v");
        unique_scion_id(dir, &format!("{slot}-{}", sanitize_id_part(stem)))?
    } else {
        source_id.clone()
    };
    if child_id != source_id {
        scion_fork(dir, source_id.clone(), child_id.clone())?;
    }
    bind(
        dir,
        slot.to_string(),
        file.to_string_lossy().into_owned(),
        None,
        None,
        None,
        Some(child_id.clone()),
        "base".into(),
    )?;
    let report = compile_report(dir, Some(child_id), None, out)?;
    if !report.encoded {
        bail!("swap compiled a plan only — bind real takes and ship first");
    }
    assert_swap_reuse(&report.json, slot)?;
    Ok(report)
}

fn assert_swap_reuse(json: &serde_json::Value, slot: &str) -> Result<()> {
    let mut recoded = Vec::new();
    if let Some(slots) = json["plan"]["slots"].as_array() {
        for entry in slots {
            let id = entry["id"].as_str().unwrap_or("");
            if id != slot && entry["cache"] != "hit" {
                recoded.push(id.to_string());
            }
        }
    }
    if !recoded.is_empty() {
        bail!(
            "swap {slot} recoded clean siblings: {} (principle 14)",
            recoded.join(", ")
        );
    }
    Ok(())
}

fn init_from_takes(dir: &Path, from: &Path) -> Result<()> {
    let takes = discover_takes(from)?;
    if takes.is_empty() {
        bail!(
            "no named takes in {} (expected hook.mov, body.mov, cta.mov, …)",
            from.display()
        );
    }
    let (rate, _) = dest_from_takes(&takes)?;
    let slots = slots_from_takes(&takes, rate)?;
    let score = Score {
        graft: GRAFT_SCHEMA.into(),
        concept: uuid::Uuid::new_v4().to_string(),
        clock: graft_score::Clock {
            rate,
            duration_frames: 1,
        },
        slots,
        layers: vec![Layer::Base, Layer::Copy, Layer::Grade, Layer::Legal],
        dest_default: Some("9x16".into()),
        spill_threshold_frames: DEFAULT_SPILL_FRAMES,
        joins: Vec::new(),
    };
    let mut score = score;
    score.recompute_duration();
    score.validate()?;
    save_json(&paths::score(dir), &score)?;
    std::fs::create_dir_all(paths::scions(dir))?;
    std::fs::create_dir_all(paths::feedback_dir(dir))?;
    std::fs::create_dir_all(paths::graft_dir(dir))?;
    eprintln!("wrote {}", paths::score(dir).display());
    ensure_root_scion(dir, None, None, "x264".into(), from)?;
    bind_takes(dir, from)
}

fn add_missing_slots_from_takes(dir: &Path, from: &Path) -> Result<()> {
    let takes = discover_takes(from)?;
    if takes.is_empty() {
        bail!(
            "no named takes in {} (expected hook.mov, body.mov, cta.mov, …)",
            from.display()
        );
    }
    let path = paths::score(dir);
    let mut score = load_score(&path)?;
    let rate = score.clock.rate;
    let existing: Vec<String> = score.slots.iter().map(|s| s.id.clone()).collect();
    let added = slots_from_takes(&takes, rate)?
        .into_iter()
        .filter(|slot| !existing.iter().any(|id| id == &slot.id))
        .collect::<Vec<_>>();
    if added.is_empty() {
        return Ok(());
    }
    let mut cursor = score.clock.duration_frames as i64;
    for mut slot in added {
        if slot.role.is_spine() {
            let duration = slot.range.duration;
            slot.range = FrameRange::new(cursor, duration);
            if slot.role == graft_score::Role::Hook {
                slot.window = Some(hook_window(rate, FrameRange::new(cursor, duration)));
            }
            cursor += duration as i64;
        } else {
            slot.range = FrameRange::new(0, (cursor.max(1)) as u64);
        }
        score.slots.push(slot);
    }
    score.recompute_duration();
    score.validate()?;
    save_json(&path, &score)?;
    eprintln!("wrote {}", path.display());
    Ok(())
}

fn ensure_root_scion(
    dir: &Path,
    scion: Option<String>,
    dest: Option<String>,
    encoder: String,
    from: &Path,
) -> Result<()> {
    if let Ok(id) = selected_scion_id(dir, scion.clone()) {
        if paths::scion(dir, &id).is_file() {
            return Ok(());
        }
    }
    let takes = discover_takes(from).unwrap_or_default();
    let (rate, probed) = dest_from_takes(&takes).unwrap_or((FrameRate::new(30, 1), None));
    let _ = rate;
    let spec = match dest {
        Some(value) => value,
        None => match probed {
            Some((w, h)) => format!("{w}x{h}"),
            None => "1080x1920".into(),
        },
    };
    let id = scion.unwrap_or_else(|| {
        load_score(&paths::score(dir))
            .ok()
            .and_then(|s| s.dest_default)
            .unwrap_or_else(|| "9x16".into())
    });
    if paths::scion(dir, &id).exists() {
        write_head(dir, &id)?;
        return Ok(());
    }
    scion_create(
        dir,
        id,
        None,
        spec,
        "yuv420p".into(),
        "bt709".into(),
        encoder,
    )
}

fn bind_takes(dir: &Path, from: &Path) -> Result<()> {
    let score = load_score(&paths::score(dir))?;
    let takes = discover_takes(from)?;
    let scion = selected_scion_id(dir, None)?;
    for take in takes {
        if score.slot(&take.slot).is_none() {
            continue;
        }
        bind(
            dir,
            take.slot,
            take.path.to_string_lossy().into_owned(),
            None,
            None,
            None,
            Some(scion.clone()),
            "base".into(),
        )?;
    }
    Ok(())
}

fn require_spine_bound(dir: &Path) -> Result<()> {
    let score = load_score(&paths::score(dir))?;
    let (_, scion) = load_selected_scion(dir, None)?;
    let bindings = effective_bindings(&score, &scion)?;
    let missing: Vec<&str> = score
        .slots
        .iter()
        .filter(|slot| slot.role.is_spine() && !slot.optional && !bindings.contains_key(&slot.id))
        .map(|slot| slot.id.as_str())
        .collect();
    if !missing.is_empty() {
        bail!(
            "required spine slots unbound: {} — add takes/<slot>.mov",
            missing.join(", ")
        );
    }
    Ok(())
}

fn discover_takes(from: &Path) -> Result<Vec<DiscoveredTake>> {
    if !from.is_dir() {
        bail!("{} is not a take directory", from.display());
    }
    let mut found = Vec::new();
    let mut unknown = Vec::new();
    for entry in std::fs::read_dir(from)? {
        let path = entry?.path();
        if !path.is_file() || !is_take_file(&path) {
            continue;
        }
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        match graft_score::Role::parse(&stem) {
            Ok(role) => {
                if found.iter().any(|t: &DiscoveredTake| t.slot == stem) {
                    bail!("duplicate take for slot {stem} in {}", from.display());
                }
                found.push(DiscoveredTake {
                    slot: stem,
                    role,
                    path,
                });
            }
            Err(_) => unknown.push(path),
        }
    }
    for path in &unknown {
        eprintln!("skip unknown take {}", path.display());
    }
    found.sort_by_key(|t| {
        ROLE_ORDER
            .iter()
            .position(|id| *id == t.slot)
            .unwrap_or(ROLE_ORDER.len())
    });
    Ok(found)
}

fn list_pool_takes(from: &Path) -> Result<Vec<PathBuf>> {
    if !from.is_dir() {
        bail!("{} is not a take directory", from.display());
    }
    let mut files = Vec::new();
    for entry in std::fs::read_dir(from)? {
        let path = entry?.path();
        if path.is_file() && is_take_file(&path) {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

fn is_take_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| TAKE_EXTS.contains(&ext.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

fn dest_from_takes(takes: &[DiscoveredTake]) -> Result<(FrameRate, Option<(u32, u32)>)> {
    for take in takes {
        if let Ok(probe) = graft_compile::probe_path(&take.path) {
            if probe.width > 0 && probe.height > 0 {
                let rate = FrameRate::from_f64(probe.fps).unwrap_or_else(|_| FrameRate::new(30, 1));
                return Ok((rate, Some((probe.width, probe.height))));
            }
        }
    }
    Ok((FrameRate::new(30, 1), None))
}

fn slots_from_takes(takes: &[DiscoveredTake], rate: FrameRate) -> Result<Vec<Slot>> {
    let mut cursor = 0_i64;
    let mut spine = Vec::new();
    let mut overlays = Vec::new();
    for take in takes {
        let duration = take_duration_frames(take, rate)?;
        if take.role.is_spine() {
            let range = FrameRange::new(cursor, duration);
            cursor += duration as i64;
            let window = if take.role == graft_score::Role::Hook {
                Some(hook_window(rate, range))
            } else {
                None
            };
            spine.push(Slot {
                id: take.slot.clone(),
                role: take.role,
                range,
                optional: false,
                window,
            });
        } else {
            overlays.push(take);
        }
    }
    if cursor <= 0 {
        cursor = rate.frames_from_seconds(3.0)?.max(1);
    }
    let full = FrameRange::new(0, cursor as u64);
    let mut slots = spine;
    for take in overlays {
        slots.push(Slot {
            id: take.slot.clone(),
            role: take.role,
            range: full,
            optional: true,
            window: None,
        });
    }
    if slots.is_empty() {
        bail!("no slots from takes");
    }
    Ok(slots)
}

fn take_duration_frames(take: &DiscoveredTake, rate: FrameRate) -> Result<u64> {
    if let Ok(probe) = graft_compile::probe_path(&take.path) {
        let frames = rate.frames_from_seconds(probe.duration_s)?.max(1) as u64;
        return Ok(frames);
    }
    let bytes = std::fs::read(&take.path)?;
    match probe_bytes(&bytes) {
        MaterialKind::Media(probe) => Ok(rate.frames_from_seconds(probe.duration_s)?.max(1) as u64),
        MaterialKind::Gfi1 { nframes, .. } => Ok(nframes.max(1) as u64),
        MaterialKind::Opaque { .. } if take.role == graft_score::Role::Captions => {
            Ok(rate.frames_from_seconds(3.0)?.max(1) as u64)
        }
        _ => bail!("cannot probe duration for {}", take.path.display()),
    }
}

fn hook_window(rate: FrameRate, hook: FrameRange) -> Window {
    let three = rate.frames_from_seconds(3.0).unwrap_or(90).max(1) as u64;
    let duration = hook.duration.min(three);
    Window {
        kind: "hook_rate".into(),
        range: FrameRange::new(hook.start, duration),
    }
}

fn ledger_from_plan(
    scion: &str,
    plan: &graft_compile::CompilePlan,
    encode_ms: u64,
    encoded: bool,
    build: &BuildRecord,
) -> serde_json::Value {
    let dirty: Vec<String> = plan
        .slots
        .iter()
        .filter(|slot| slot.cache == "miss")
        .map(|slot| slot.id.clone())
        .collect();
    let kerfs: Vec<Vec<String>> = plan
        .kerfs
        .iter()
        .filter(|kerf| kerf.cache == "miss")
        .map(|kerf| kerf.join.clone())
        .collect();
    let clean: Vec<serde_json::Value> = plan
        .slots
        .iter()
        .filter(|slot| slot.cache == "hit")
        .map(|slot| {
            serde_json::json!({
                "id": slot.id,
                "cache": slot.cache,
                "blob": slot.blob
            })
        })
        .collect();
    serde_json::json!({
        "scion": scion,
        "dirty": dirty,
        "kerfs": kerfs,
        "clean": clean,
        "encode_ms": encode_ms,
        "encoded": encoded,
        "out": build.output,
        "build": build.id
    })
}

fn print_ledger_table(ledger: &serde_json::Value) {
    let scion = ledger["scion"].as_str().unwrap_or("-");
    let ms = ledger["encode_ms"].as_u64().unwrap_or(0);
    eprintln!("reuse  scion={scion}  encode={:.2}s", ms as f64 / 1000.0);
    if let Some(dirty) = ledger["dirty"].as_array() {
        let names: Vec<&str> = dirty.iter().filter_map(|v| v.as_str()).collect();
        if !names.is_empty() {
            eprintln!("dirty  {}", names.join(", "));
        }
    }
    if let Some(kerfs) = ledger["kerfs"].as_array() {
        for kerf in kerfs {
            if let Some(pair) = kerf.as_array() {
                let left = pair.first().and_then(|v| v.as_str()).unwrap_or("?");
                let right = pair.get(1).and_then(|v| v.as_str()).unwrap_or("?");
                eprintln!("kerf   {left}→{right}");
            }
        }
    }
    if let Some(clean) = ledger["clean"].as_array() {
        for slot in clean {
            let id = slot["id"].as_str().unwrap_or("?");
            let blob = slot["blob"].as_str().unwrap_or("-");
            eprintln!("clean  {id}  {}  hit", short_blob(blob));
        }
    }
    if let Some(out) = ledger["out"].as_str() {
        eprintln!("out    {out}");
    }
}

fn short_blob(blob: &str) -> &str {
    blob.get(..19).unwrap_or(blob)
}

fn sanitize_id_part(raw: &str) -> String {
    let mut out = String::new();
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
            out.push(ch);
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    if out.is_empty() {
        "v".into()
    } else {
        out
    }
}

fn unique_scion_id(dir: &Path, want: &str) -> Result<String> {
    let base = sanitize_id_part(want);
    require_scion_id(&base)?;
    if !paths::scion(dir, &base).exists() {
        return Ok(base);
    }
    for i in 2..1000 {
        let id = format!("{base}-{i}");
        if !paths::scion(dir, &id).exists() {
            return Ok(id);
        }
    }
    bail!("could not allocate scion id from {base}")
}

fn unique_feedback_id(dir: &Path, kind: &str) -> Result<String> {
    let base = sanitize_id_part(kind);
    if !paths::feedback(dir, &base).exists() {
        return Ok(base);
    }
    for i in 2..1000 {
        let id = format!("{base}-{i}");
        if !paths::feedback(dir, &id).exists() {
            return Ok(id);
        }
    }
    Ok(uuid::Uuid::new_v4().to_string())
}

pub fn status(dir: &Path) -> Result<()> {
    let git_repo = Command::new("git")
        .args([
            "-C",
            &dir.display().to_string(),
            "rev-parse",
            "--is-inside-work-tree",
        ])
        .output()
        .ok()
        .filter(|out| out.status.success())
        .is_some();
    let mut recipe = Vec::new();
    if paths::score(dir).is_file() {
        recipe.push(paths::SCORE_FILE.to_string());
    }
    if paths::scions(dir).is_dir() {
        for entry in std::fs::read_dir(paths::scions(dir))? {
            let path = entry?.path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
                if let Ok(rel) = path.strip_prefix(dir) {
                    recipe.push(rel.to_string_lossy().into_owned());
                }
            }
        }
    }
    if paths::feedback_dir(dir).is_dir() {
        for entry in std::fs::read_dir(paths::feedback_dir(dir))? {
            let path = entry?.path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
                if let Ok(rel) = path.strip_prefix(dir) {
                    recipe.push(rel.to_string_lossy().into_owned());
                }
            }
        }
    }
    recipe.sort();
    let mut dirty = Vec::new();
    if git_repo {
        if let Ok(out) = Command::new("git")
            .args([
                "-C",
                &dir.display().to_string(),
                "status",
                "--porcelain",
                "--",
            ])
            .args(&recipe)
            .output()
        {
            dirty = String::from_utf8_lossy(&out.stdout)
                .lines()
                .filter(|line| !line.is_empty())
                .map(str::to_string)
                .collect();
        }
    }
    paths::print_json(&serde_json::json!({
        "git_repository": git_repo,
        "selected_scion": selected_scion_id(dir, None).ok(),
        "recipe_files": recipe,
        "commit_ready": dirty,
        "note": "graft does not replace git add/commit; these are the recipe files Git should own"
    }))
}

fn open_remote(dir: &Path, remote: &str) -> Result<Remote> {
    if remote.starts_with("s3://")
        || remote.starts_with("http://")
        || remote.starts_with("https://")
    {
        Ok(Remote::open(remote)?)
    } else {
        Ok(Remote::open(
            paths::resolve_in_dir(dir, PathBuf::from(remote))
                .to_string_lossy()
                .as_ref(),
        )?)
    }
}

pub fn store_missing(dir: &Path, remote: String) -> Result<()> {
    let local = Object::open(paths::graft_dir(dir))?;
    let remote = open_remote(dir, &remote)?;
    let wanted = local.list_blobs()?;
    paths::print_json(&serde_json::json!({
        "missing": remote
            .missing_blobs(&wanted)
            .into_iter()
            .map(|(kind, id)| format!("{kind}/{id}"))
            .collect::<Vec<_>>()
    }))
}

pub fn store_push(dir: &Path, remote: String) -> Result<()> {
    let local = Object::open(paths::graft_dir(dir))?;
    let remote = open_remote(dir, &remote)?;
    let wanted = local.list_blobs()?;
    let copied = remote.push_from(&local, &wanted)?;
    for (key, entry) in local.list_actions()? {
        remote.put_action(&key, entry)?;
    }
    paths::print_json(&serde_json::json!({ "pushed": copied }))
}

pub fn store_pull(dir: &Path, remote: String) -> Result<()> {
    let local = Object::open(paths::graft_dir(dir))?;
    let remote = open_remote(dir, &remote)?;
    let wanted = remote.list_blobs()?;
    let copied = remote.pull_into(&local, &wanted)?;
    for (key, entry) in remote.list_actions()? {
        local.put_action(&key, entry)?;
    }
    paths::print_json(&serde_json::json!({ "pulled": copied }))
}
