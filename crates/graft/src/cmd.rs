// SPDX-License-Identifier: Apache-2.0

//! One function per subcommand. Rules live in the libraries, not here.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use graft_cas::{Fs, Kind, Store};
use graft_compile::{
    compile, dirty_from_signal, lower, materials_present, prev_overlay, probe_bytes,
    write_time_map_artifact, FfmpegX264, FrameIntra, MaterialKind, Unimplemented,
};
use graft_score::{
    is_material_id, load_scion, load_score, load_time_map, parse_range, parse_wh, require_slot_id,
    save_json, Binding, Dest, Encoder, Scion, Score, Slot, TimeMap, Window, GRAFT_SCHEMA,
};

use crate::paths;

pub fn init(dir: &Path) -> Result<()> {
    std::fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
    let path = paths::score(dir);
    if path.exists() {
        bail!("{} already exists", path.display());
    }
    let score = Score {
        graft: GRAFT_SCHEMA.into(),
        concept: uuid::Uuid::new_v4().to_string(),
        clock: graft_score::Clock {
            fps: 30.0,
            duration_s: 3.0,
        },
        slots: vec![Slot {
            id: "hook".into(),
            role: graft_score::Role::Hook,
            span: [0.0, 3.0],
            optional: false,
            window: Some(Window {
                kind: "hook_rate".into(),
                span: [0.0, 3.0],
            }),
        }],
        layers: vec![graft_score::Layer::Base],
        dest_default: Some("9x16".into()),
        spill_threshold_s: graft_score::DEFAULT_SPILL_S,
    };
    score.validate()?;
    save_json(&path, &score)?;
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
    let window_span = window.as_deref().map(parse_range).transpose()?;
    let span = match (span.as_deref().map(parse_range).transpose()?, window_span) {
        (Some(s), _) => s.as_span(),
        (None, Some(w)) => w.as_span(),
        (None, None) => {
            if let Some(existing) = score.slot(&id) {
                existing.span
            } else {
                bail!("new slot {id} needs --span T0-T1 or --window T0-T1");
            }
        }
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
    let window = window_span.map(|r| Window {
        kind: window_kind,
        span: r.as_span(),
    });
    if let Some(existing) = score.slot_mut(&id) {
        existing.role = role;
        existing.span = span;
        existing.optional = optional;
        if window.is_some() {
            existing.window = window;
        }
    } else {
        score.slots.push(Slot {
            id,
            role,
            span,
            optional,
            window,
        });
    }
    score.recompute_duration();
    score.validate()?;
    save_json(&path, &score)?;
    eprintln!("wrote {}", path.display());
    Ok(())
}

pub fn bind(
    dir: &Path,
    slot: String,
    material: String,
    in_s: Option<f64>,
    out_s: Option<f64>,
    speed: Option<f64>,
) -> Result<()> {
    require_slot_id(&slot)?;
    let score = load_score(&paths::score(dir))?;
    let Some(slot_def) = score.slot(&slot) else {
        bail!("score has no slot {slot}");
    };
    let scion_file = paths::scion(dir);
    if !scion_file.exists() {
        bail!("no scion.json — run `graft scion` first");
    }
    let mut scion = load_scion(&scion_file)?;
    if scion.concept != score.concept {
        bail!("scion.concept != score.concept");
    }
    let material = resolve_material(dir, &material)?;
    if let Some(s) = speed {
        if (s - 1.0).abs() > 1e-9 {
            eprintln!("warning: speed {s} is in the action key; retime is not applied yet");
        }
    }
    let in_s = in_s.unwrap_or(0.0);
    let out_s = out_s.unwrap_or(in_s + (slot_def.end() - slot_def.start()));
    let params = speed.map(|s| serde_json::json!({ "speed": s }));
    let binding = Binding {
        material,
        in_s,
        out_s,
        params,
    };
    binding.validate(&slot)?;
    scion.bindings.insert(slot, binding);
    scion.validate()?;
    save_json(&scion_file, &scion)?;
    eprintln!("wrote {}", scion_file.display());
    Ok(())
}

fn resolve_material(dir: &Path, spec: &str) -> Result<String> {
    if is_material_id(spec) {
        return Ok(spec.to_string());
    }
    let path = Path::new(spec);
    if path.is_file() {
        let bytes = std::fs::read(path).with_context(|| format!("read {}", path.display()))?;
        match probe_bytes(&bytes) {
            MaterialKind::Gfi1 {
                width,
                height,
                nframes,
            } => eprintln!("probe: graft-intra {width}x{height} {nframes} frames"),
            MaterialKind::Media(p) => {
                eprintln!(
                    "probe: {} {}x{} {:.3}s @ {:.3} fps{}",
                    p.video_codec,
                    p.width,
                    p.height,
                    p.duration_s,
                    p.fps,
                    if p.has_audio {
                        " (audio present; compile is video-only)"
                    } else {
                        ""
                    }
                );
            }
            MaterialKind::Opaque { bytes: n } => {
                eprintln!("probe: not a media file ({n} bytes); hashed as essence anyway");
            }
        }
        let store = Fs::open(paths::graft_dir(dir))?;
        let id = store.put_blob(Kind::Material, &bytes)?;
        eprintln!("put {} → {id}", path.display());
        return Ok(id.to_string());
    }
    bail!("material must be blake3:<64 hex> or a file that exists (got {spec:?})");
}

pub fn scion(
    dir: &Path,
    dest_id: String,
    dest: String,
    id: Option<String>,
    pix_fmt: String,
    color: String,
    encoder: String,
) -> Result<()> {
    if dest_id.contains(':') {
        bail!("dest id is 9x16, not 9:16 — ratio is not a slot");
    }
    let score = load_score(&paths::score(dir))?;
    let (width, height) = parse_wh(&dest)?;
    let dest = Dest {
        id: dest_id.clone(),
        width,
        height,
        fps: score.clock.fps,
        pix_fmt,
        color,
        encoder: Encoder::named(&encoder)?,
    };
    dest.validate()?;
    let path = paths::scion(dir);
    let mut bindings = std::collections::BTreeMap::new();
    if path.exists() {
        let existing = load_scion(&path)?;
        if existing.concept != score.concept {
            bail!("existing scion.concept != score.concept");
        }
        bindings = existing.bindings;
    }
    let scion = Scion {
        graft: GRAFT_SCHEMA.into(),
        id: id.unwrap_or(dest_id),
        concept: score.concept.clone(),
        dest,
        bindings,
    };
    scion.validate()?;
    save_json(&path, &scion)?;
    eprintln!("wrote {}", path.display());
    Ok(())
}

fn load_time_map_or_derive(dir: &Path, score: &Score, scion: Option<&Scion>) -> Result<TimeMap> {
    let path = paths::time_map(dir);
    if path.exists() {
        return Ok(load_time_map(&path)?);
    }
    let scion_id = scion.map(|s| s.id.as_str()).unwrap_or("unbound");
    let dest_id = scion
        .map(|s| s.dest.id.as_str())
        .or(score.dest_default.as_deref())
        .unwrap_or("9x16");
    Ok(TimeMap::from_score(score, scion_id, dest_id))
}

pub fn signal(dir: &Path, kind: String, t: String, dest: Option<String>) -> Result<()> {
    let score = load_score(&paths::score(dir))?;
    let scion = if paths::scion(dir).exists() {
        Some(load_scion(&paths::scion(dir))?)
    } else {
        None
    };
    let time_map = load_time_map_or_derive(dir, &score, scion.as_ref())?;
    let range = parse_range(&t)?;
    let dest_id = dest
        .or_else(|| scion.as_ref().map(|s| s.dest.id.clone()))
        .or_else(|| Some(time_map.dest_id.clone()));
    let dirty = dirty_from_signal(&time_map, &score, &kind, range.start, range.end, dest_id);
    paths::print_json(&dirty)
}

pub fn dirty(dir: &Path, prev: Option<PathBuf>) -> Result<()> {
    let score = load_score(&paths::score(dir))?;
    let scion = load_scion(&paths::scion(dir))?;
    scion.validate_against_score(&score)?;
    let prev = prev
        .map(|p| load_scion(&paths::resolve_in_dir(dir, p)))
        .transpose()
        .map_err(anyhow::Error::from)?;
    let store = Fs::open(paths::graft_dir(dir))?;
    let outcome = compile_with_backend(&score, &scion, &store, false)?;
    let mut plan = outcome.plan;
    if let Some(prev) = prev.as_ref() {
        plan.prev_overlay = Some(prev_overlay(&score, &scion, prev));
    }
    let slots: Vec<String> = plan
        .slots
        .iter()
        .filter(|s| s.cache == "miss")
        .map(|s| s.id.clone())
        .collect();
    let kerfs: Vec<Vec<String>> = plan
        .kerfs
        .iter()
        .filter(|k| k.cache == "miss")
        .map(|k| k.join.clone())
        .collect();
    let clean: Vec<String> = plan
        .slots
        .iter()
        .filter(|s| s.cache == "hit")
        .map(|s| s.id.clone())
        .collect();
    let mut out = serde_json::json!({
        "graft": GRAFT_SCHEMA,
        "kind": "compile",
        "scion": scion.id,
        "scion_hash": plan.scion_hash,
        "slots": slots,
        "kerfs": kerfs,
        "clean": clean,
        "warnings": [],
    });
    if let Some(overlay) = plan.prev_overlay {
        out["prev_overlay"] = serde_json::to_value(overlay)?;
    }
    paths::print_json(&out)
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
        match FfmpegX264::from_env() {
            Ok(ff) => return Ok(compile(score, scion, store, &ff, &ff)?),
            Err(e) => {
                let graph = lower(score, scion)?;
                if materials_present(&graph, store) {
                    return Err(anyhow::Error::msg(e.to_string()));
                }
                return Ok(compile(
                    score,
                    scion,
                    store,
                    &Unimplemented,
                    &graft_compile::ConcatUnimplemented,
                )?);
            }
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

pub fn compile_cmd(dir: &Path, prev: Option<PathBuf>, out: Option<PathBuf>) -> Result<()> {
    let score = load_score(&paths::score(dir))?;
    let scion = load_scion(&paths::scion(dir))?;
    let prev = prev
        .map(|p| load_scion(&paths::resolve_in_dir(dir, p)))
        .transpose()
        .map_err(anyhow::Error::from)?;
    let store_root = paths::graft_dir(dir);
    let store = Fs::open(&store_root)?;
    let mut outcome = compile_with_backend(&score, &scion, &store, true)?;
    if let Some(prev) = prev.as_ref() {
        outcome.plan.prev_overlay = Some(prev_overlay(&score, &scion, prev));
    }
    write_time_map_artifact(&store_root, &outcome.graph.concat.key, &outcome.time_map)
        .map_err(anyhow::Error::from)?;
    if let Some(blob) = &outcome.concat {
        let bytes = store.get_blob(Kind::Concat, blob)?;
        let name = if FrameIntra::supports(&scion.dest) {
            "dest.gfi1"
        } else {
            "dest.mp4"
        };
        let dest_path =
            graft_compile::time_map_artifact_dir(&store_root, &outcome.graph.concat.key).join(name);
        std::fs::write(&dest_path, &bytes)
            .with_context(|| format!("write {}", dest_path.display()))?;
        eprintln!("wrote {}", dest_path.display());
        if let Some(user_out) = out {
            let user_out = paths::resolve_in_dir(dir, user_out);
            if let Some(parent) = user_out.parent() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("create {}", parent.display()))?;
            }
            std::fs::write(&user_out, &bytes)
                .with_context(|| format!("write {}", user_out.display()))?;
            eprintln!("wrote {}", user_out.display());
        }
    } else if out.is_some() {
        bail!("nothing to write — compile did not encode (missing essence or no backend)");
    }
    paths::print_json(&outcome.plan)?;
    if !outcome.encoded {
        eprintln!(
            "plan only — dest encoder is {}; put materials in CAS to encode (x264 needs ffmpeg)",
            scion.dest.encoder.impl_name
        );
    }
    Ok(())
}
