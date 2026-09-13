// SPDX-License-Identifier: Apache-2.0

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};
use graft_cas::{BlobId, Fs, Kind, Store};
use graft_score::{effective_bindings, Scion, Score};

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
        filters.push(format!(
            "[{inputs}:v]trim=start={:.6}:end={:.6},setpts=PTS-STARTPTS,scale={}:{},fps={}[v{inputs}]",
            binding.in_s(),
            binding.out_s(),
            scion.dest.width.min(960),
            scion.dest.height.min(960),
            scion.dest.rate.as_f64()
        ));
        inputs += 1;
    }
    if inputs == 0 {
        bail!("preview has no bound picture slots");
    }
    let labels = (0..inputs)
        .map(|i| format!("[v{i}]"))
        .collect::<Vec<_>>()
        .join("");
    filters.push(format!("{labels}concat=n={inputs}:v=1:a=0[outv]"));
    args.extend([
        "-filter_complex".into(),
        filters.join(";"),
        "-map".into(),
        "[outv]".into(),
        "-an".into(),
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
