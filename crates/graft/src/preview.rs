// SPDX-License-Identifier: Apache-2.0

use std::path::Path;

use anyhow::{bail, Result};
use graft_cas::{Fs, Kind, Store};
use graft_compile::{
    compile_in, lower, materials_present, ConcatUnimplemented, FfmpegX264, FrameIntra, NestFamily,
    Unimplemented,
};
use graft_score::{scion_has_nested, Scion, Score};

/// Same compile as `ship` / `swap`. Preview is not a second compositor.
pub fn render(
    project: &Path,
    score: &Score,
    scion: &Scion,
    store: &Fs,
    out: &Path,
    family: &NestFamily,
) -> Result<()> {
    let outcome = if FrameIntra::supports(&scion.dest) && !scion_has_nested(scion) {
        compile_in(score, scion, store, &FrameIntra, &FrameIntra, family)?
    } else if FfmpegX264::supports(&scion.dest) {
        let ff = FfmpegX264::from_env()?;
        if scion_has_nested(scion) {
            compile_in(score, scion, store, &ff, &ff, family)?
        } else {
            let graph = lower(score, scion)?;
            if materials_present(&graph, store) {
                compile_in(score, scion, store, &ff, &ff, family)?
            } else {
                compile_in(
                    score,
                    scion,
                    store,
                    &Unimplemented,
                    &ConcatUnimplemented,
                    family,
                )?
            }
        }
    } else {
        compile_in(
            score,
            scion,
            store,
            &Unimplemented,
            &ConcatUnimplemented,
            family,
        )?
    };
    if !outcome.encoded {
        bail!("preview compiled a plan only — bind real takes and install ffmpeg");
    }
    let Some(blob) = outcome.concat else {
        bail!("preview produced no dest");
    };
    let bytes = store.get_blob(Kind::Concat, &blob)?;
    let out = if out.is_absolute() {
        out.to_path_buf()
    } else {
        project.join(out)
    };
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&out, bytes)?;
    eprintln!("wrote {}", out.display());
    Ok(())
}
