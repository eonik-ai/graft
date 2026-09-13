// SPDX-License-Identifier: Apache-2.0

mod cmd;
mod paths;

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "graft",
    about = "compiler for video composition",
    long_about = "The score is source. Essence is immutable. The mp4 is a compile.\n\
                  Dirty is the action cache. x264 shells out to ffmpeg; graft-intra is frame grain."
)]
struct Cli {
    /// Project directory (like git -C).
    #[arg(short = 'C', long = "dir", default_value = ".", global = true)]
    dir: PathBuf,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Write a new score.json (concept UUID, hook 0–3).
    Init,
    /// Add or update a slot on the score.
    Slot {
        id: String,
        #[arg(long)]
        role: Option<String>,
        #[arg(long)]
        span: Option<String>,
        #[arg(long)]
        window: Option<String>,
        #[arg(long = "window-kind", default_value = "hook_rate")]
        window_kind: String,
        #[arg(long)]
        optional: bool,
    },
    /// Bind a slot to a material hash, or put a local file into `.graft/blobs/material`.
    Bind {
        slot: String,
        /// `blake3:<64 hex>` or a path whose bytes are stored in CAS.
        material: String,
        #[arg(long = "in")]
        in_s: Option<f64>,
        #[arg(long = "out")]
        out_s: Option<f64>,
        #[arg(long)]
        speed: Option<f64>,
    },
    /// Write dest + encoder fingerprint. 9:16 is a dest id, not a slot.
    Scion {
        dest_id: String,
        #[arg(long)]
        dest: String,
        #[arg(long)]
        id: Option<String>,
        #[arg(long, default_value = "yuv420p")]
        pix_fmt: String,
        #[arg(long, default_value = "bt709")]
        color: String,
        /// Dest encoder fingerprint: x264 (default) or graft-intra.
        #[arg(long, default_value = "x264")]
        encoder: String,
    },
    /// Print the compile dirty set from the action cache. `--prev` is debug only.
    Dirty {
        #[arg(long)]
        prev: Option<PathBuf>,
    },
    /// Address a platform metric through the time map.
    Signal {
        #[arg(long)]
        kind: String,
        #[arg(long)]
        t: String,
        #[arg(long)]
        dest: Option<String>,
    },
    /// Incremental compile. Encodes x264 via ffmpeg, or graft-intra frame copy.
    Compile {
        #[arg(long)]
        prev: Option<PathBuf>,
        /// Write the dest file here (mp4 or gfi1). Also stored under `.graft/builds/`.
        #[arg(long)]
        out: Option<PathBuf>,
    },
}

fn main() {
    if let Err(err) = run() {
        eprintln!("graft: {err:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let dir = &cli.dir;
    match cli.command {
        Command::Init => cmd::init(dir),
        Command::Slot {
            id,
            role,
            span,
            window,
            window_kind,
            optional,
        } => cmd::slot(dir, id, role, span, window, window_kind, optional),
        Command::Bind {
            slot,
            material,
            in_s,
            out_s,
            speed,
        } => cmd::bind(dir, slot, material, in_s, out_s, speed),
        Command::Scion {
            dest_id,
            dest,
            id,
            pix_fmt,
            color,
            encoder,
        } => cmd::scion(dir, dest_id, dest, id, pix_fmt, color, encoder),
        Command::Dirty { prev } => cmd::dirty(dir, prev),
        Command::Signal { kind, t, dest } => cmd::signal(dir, kind, t, dest),
        Command::Compile { prev, out } => cmd::compile_cmd(dir, prev, out),
    }
}
