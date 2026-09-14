// SPDX-License-Identifier: Apache-2.0

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};
use graft_cas::{BlobId, Fs, Kind, Store};
use graft_score::{effective_bindings, Binding, Scion, Score};

pub fn render(project: &Path, score: &Score, scion: &Scion, store: &Fs, out: &Path) -> Result<()> {
    let bindings = effective_bindings(score, scion)?;
    let temp = tempfile::tempdir()?;
    let mut args = vec![
        "-hide_banner".to_string(),
        "-loglevel".to_string(),
        "error".to_string(),
        "-y".to_string(),
    ];
    let mut filters = Vec::new();
    let mut inputs = 0usize;
    let mut audio_labels = Vec::new();
    let any_audio = score
        .spine()
        .into_iter()
        .filter_map(|slot| bindings.get(&slot.id))
        .any(|binding| binding.audio.is_some());
    for slot in score.spine() {
        let Some(binding) = bindings.get(&slot.id) else {
            continue;
        };
        let id = BlobId::parse(&binding.material)?;
        let bytes = store.get_blob(Kind::Material, &id)?;
        let path = temp.path().join(format!("{inputs}.media"));
        std::fs::write(&path, bytes)?;
        args.push("-i".into());
        args.push(path.to_string_lossy().into_owned());
        let speed = binding.speed();
        let setpts = if (speed - 1.0).abs() > 1e-9 {
            format!("setpts=(PTS-STARTPTS)/{speed:.6}")
        } else {
            "setpts=PTS-STARTPTS".into()
        };
        filters.push(format!(
            "[{inputs}:v]trim=start={:.6}:end={:.6},{setpts},scale={}:{},fps={}[v{inputs}]",
            binding.in_s(),
            binding.out_s(),
            scion.dest.width.min(960),
            scion.dest.height.min(960),
            scion.dest.rate.as_f64()
        ));
        if any_audio {
            audio_labels.push(preview_audio_filter(
                inputs,
                binding,
                slot.range.duration,
                &scion.dest.rate,
                &mut args,
                &mut filters,
            )?);
        }
        inputs += 1;
    }
    if inputs == 0 {
        bail!("preview has no bound picture slots");
    }
    if any_audio {
        let paired = (0..inputs)
            .map(|i| format!("[v{i}]{}", audio_labels[i]))
            .collect::<Vec<_>>()
            .join("");
        filters.push(format!("{paired}concat=n={inputs}:v=1:a=1[basev][basea]"));
    } else {
        let vlabels = (0..inputs)
            .map(|i| format!("[v{i}]"))
            .collect::<Vec<_>>()
            .join("");
        filters.push(format!("{vlabels}concat=n={inputs}:v=1:a=0[basev]"));
    }
    let mut picture = "basev".to_string();
    if let Some(slot) = score
        .slots
        .iter()
        .find(|s| s.role == graft_score::Role::Brand)
    {
        if let Some(binding) = bindings.get(&slot.id) {
            let id = BlobId::parse(&binding.material)?;
            let bytes = store.get_blob(Kind::Material, &id)?;
            let path = temp.path().join("brand.media");
            std::fs::write(&path, bytes)?;
            args.push("-i".into());
            args.push(path.to_string_lossy().into_owned());
            let brand_i = args.iter().filter(|a| *a == "-i").count() - 1;
            filters.push(format!(
                "[{brand_i}:v]scale={}:{}:flags=bicubic[brand];[{picture}][brand]overlay=0:0:shortest=1[branded]",
                scion.dest.width.min(960),
                scion.dest.height.min(960)
            ));
            picture = "branded".into();
        }
    }
    let mut overlay_audio = Vec::new();
    for slot in score
        .slots
        .iter()
        .filter(|s| matches!(s.role, graft_score::Role::Vo | graft_score::Role::Bed))
    {
        let Some(binding) = bindings.get(&slot.id) else {
            continue;
        };
        let id = BlobId::parse(&binding.material)?;
        let bytes = store.get_blob(Kind::Material, &id)?;
        let path = temp.path().join(format!("{}.audio", slot.id));
        std::fs::write(&path, bytes)?;
        args.push("-i".into());
        args.push(path.to_string_lossy().into_owned());
        let idx = args.iter().filter(|a| *a == "-i").count() - 1;
        let delay = (scion.dest.rate.seconds_from_frames(slot.range.start) * 1000.0).round() as i64;
        filters.push(format!(
            "[{idx}:a]adelay={delay}|{delay},asetpts=PTS-STARTPTS[ov{idx}]"
        ));
        overlay_audio.push(format!("[ov{idx}]"));
    }
    if overlay_audio.is_empty() {
        filters.push(format!("[{picture}]copy[outv]"));
        if any_audio {
            filters.push("[basea]anull[outa]".into());
        }
    } else if any_audio {
        let n = overlay_audio.len() + 1;
        filters.push(format!(
            "[basea]{}amix=inputs={n}:duration=longest:normalize=0[outa]",
            overlay_audio.join("")
        ));
        filters.push(format!("[{picture}]copy[outv]"));
    } else {
        filters.push(format!(
            "{}amix=inputs={}:duration=longest:normalize=0[outa]",
            overlay_audio.join(""),
            overlay_audio.len()
        ));
        filters.push(format!("[{picture}]copy[outv]"));
    }
    let map_audio = any_audio
        || score.slots.iter().any(|s| {
            matches!(s.role, graft_score::Role::Vo | graft_score::Role::Bed)
                && bindings.contains_key(&s.id)
        });
    args.extend(["-filter_complex".into(), filters.join(";")]);
    args.extend(["-map".into(), "[outv]".into()]);
    if map_audio {
        args.extend(["-map".into(), "[outa]".into(), "-c:a".into(), "aac".into()]);
    } else {
        args.push("-an".into());
    }
    args.extend([
        "-c:v".into(),
        "libx264".into(),
        "-preset".into(),
        "veryfast".into(),
        "-crf".into(),
        "24".into(),
        "-pix_fmt".into(),
        "yuv420p".into(),
        "-movflags".into(),
        "+faststart".into(),
    ]);
    let out = if out.is_absolute() {
        out.to_path_buf()
    } else {
        project.join(out)
    };
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    args.push(out.to_string_lossy().into_owned());
    let ffmpeg = std::env::var_os("FFMPEG")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("ffmpeg"));
    let output = Command::new(&ffmpeg)
        .args(&args)
        .output()
        .with_context(|| format!("run {}", ffmpeg.display()))?;
    if !output.status.success() {
        bail!(
            "preview ffmpeg failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    eprintln!("wrote {}", out.display());
    Ok(())
}

fn preview_audio_filter(
    inputs: usize,
    binding: &Binding,
    dest_frames: u64,
    dest_rate: &graft_score::FrameRate,
    args: &mut Vec<String>,
    filters: &mut Vec<String>,
) -> Result<String> {
    let speed = binding.speed();
    let dest_s = dest_rate.seconds_from_frames(dest_frames as i64);
    if let Some(audio) = &binding.audio {
        let atempo = if (speed - 1.0).abs() > 1e-9 {
            format!(",{}", atempo_chain(speed)?)
        } else {
            String::new()
        };
        filters.push(format!(
            "[{inputs}:a]atrim=start={:.6}:end={:.6},asetpts=PTS-STARTPTS{atempo}[a{inputs}]",
            audio.source.start_seconds(),
            audio.source.end_seconds()
        ));
        Ok(format!("[a{inputs}]"))
    } else {
        args.extend([
            "-f".into(),
            "lavfi".into(),
            "-t".into(),
            format!("{dest_s:.6}"),
            "-i".into(),
            "anullsrc=r=48000:cl=stereo".into(),
        ]);
        let silence = args.iter().filter(|a| *a == "-i").count() - 1;
        filters.push(format!("[{silence}:a]asetpts=PTS-STARTPTS[a{inputs}]"));
        Ok(format!("[a{inputs}]"))
    }
}

fn atempo_chain(speed: f64) -> Result<String> {
    if speed <= 0.0 || !speed.is_finite() {
        bail!("binding speed must be finite and > 0");
    }
    let mut remaining = speed;
    let mut parts = Vec::new();
    while remaining > 2.0 + 1e-9 {
        parts.push("atempo=2.0".into());
        remaining /= 2.0;
    }
    while remaining < 0.5 - 1e-9 {
        parts.push("atempo=0.5".into());
        remaining *= 2.0;
    }
    parts.push(format!("atempo={remaining:.6}"));
    Ok(parts.join(","))
}
