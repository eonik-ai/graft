// SPDX-License-Identifier: Apache-2.0

mod cmd;
mod paths;
mod preview;

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser, Debug)]
#[command(
    name = "graft",
    about = "local-first composition workspace",
    long_about = "The score is source. Essence is immutable. The mp4 is a compile.\n\
                  Git owns recipe history. graft owns scions, layers, and compile."
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
    /// Bind a slot on a selected scion and layer.
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
        #[arg(long)]
        scion: Option<String>,
        #[arg(long, default_value = "base")]
        layer: String,
    },
    /// Create, fork, inspect, or select variants.
    Scion {
        #[command(subcommand)]
        command: ScionCommand,
    },
    /// Print the compile dirty set from the action cache.
    Dirty {
        #[arg(long)]
        scion: Option<String>,
        #[arg(long)]
        prev: Option<PathBuf>,
    },
    /// Address a platform metric through an exact build time map.
    Signal {
        #[arg(long)]
        kind: String,
        #[arg(long)]
        t: Option<String>,
        #[arg(long)]
        build: String,
    },
    /// Incremental compile. Encodes x264 via ffmpeg, or graft-intra frame copy.
    Compile {
        #[arg(long)]
        scion: Option<String>,
        #[arg(long)]
        prev: Option<PathBuf>,
        /// Write the dest file here (mp4 or gfi1). Also stored under `.graft/builds/`.
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Semantic diff between two flattened scions.
    Diff { left: String, right: String },
    /// Three-way semantic merge; writes a new scion only when conflict-free.
    Merge {
        base: String,
        ours: String,
        theirs: String,
        #[arg(long)]
        id: String,
    },
    /// Ingest and inspect platform feedback.
    Feedback {
        #[command(subcommand)]
        command: FeedbackCommand,
    },
    /// Fork a build's source scion for an addressed feedback item.
    Iterate {
        #[arg(long)]
        from: String,
        #[arg(long)]
        feedback: String,
        #[arg(long)]
        scion: String,
    },
    /// Export a flattened scion through a lossy guest adapter.
    Export {
        format: AdapterFormat,
        #[arg(long)]
        scion: Option<String>,
        #[arg(long)]
        out: PathBuf,
    },
    /// Import a supported guest document as a new scion.
    Import {
        format: AdapterFormat,
        file: PathBuf,
        #[arg(long)]
        scion: String,
    },
    /// Decode and composite a local preview outside the delivery cache.
    Preview {
        #[arg(long)]
        scion: Option<String>,
        #[arg(long, default_value = "preview.mp4")]
        out: PathBuf,
    },
    /// Upgrade 0.1.0 score/scion/time-map documents to 0.2.0.
    Migrate,
    /// Porcelain-light Git and recipe status. Does not replace git.
    Status,
    /// Object-store missing-blob discovery and resumable sync.
    Store {
        #[command(subcommand)]
        command: StoreCommand,
    },
}

#[derive(Subcommand, Debug)]
enum StoreCommand {
    /// List blobs present locally but missing from a remote object store.
    Missing {
        /// Filesystem path, `https://`, or `s3://bucket/prefix`.
        #[arg(long)]
        remote: String,
    },
    /// Push missing local blobs to a remote object-store root.
    Push {
        /// Filesystem path, `https://`, or `s3://bucket/prefix`.
        #[arg(long)]
        remote: String,
    },
    /// Pull missing remote blobs into the local store.
    Pull {
        /// Filesystem path, `https://`, or `s3://bucket/prefix`.
        #[arg(long)]
        remote: String,
    },
}

#[derive(Subcommand, Debug)]
enum ScionCommand {
    /// Create a root scion.
    Create {
        id: String,
        #[arg(long = "dest-id")]
        dest_id: Option<String>,
        #[arg(long)]
        dest: String,
        #[arg(long, default_value = "yuv420p")]
        pix_fmt: String,
        #[arg(long, default_value = "bt709")]
        color: String,
        /// Dest encoder fingerprint: x264 (default) or graft-intra.
        #[arg(long, default_value = "x264")]
        encoder: String,
    },
    /// Fork a child scion which inherits bindings from its parent.
    Fork { source: String, id: String },
    /// List scions in this workspace.
    List,
    /// Print one raw and flattened scion.
    Show { id: String },
    /// Select the default scion in untracked `.graft/HEAD`.
    Use { id: String },
}

#[derive(Subcommand, Debug)]
enum FeedbackCommand {
    /// Ingest JSON or CSV feedback and resolve it through its build.
    Ingest { file: PathBuf },
    /// List tracked feedback items.
    List,
    /// Print one feedback item.
    Show { id: String },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum AdapterFormat {
    Otio,
}

impl AdapterFormat {
    fn as_str(self) -> &'static str {
        match self {
            Self::Otio => "otio",
        }
    }
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
            scion,
            layer,
        } => cmd::bind(dir, slot, material, in_s, out_s, speed, scion, layer),
        Command::Scion { command } => match command {
            ScionCommand::Create {
                id,
                dest_id,
                dest,
                pix_fmt,
                color,
                encoder,
            } => cmd::scion_create(dir, id, dest_id, dest, pix_fmt, color, encoder),
            ScionCommand::Fork { source, id } => cmd::scion_fork(dir, source, id),
            ScionCommand::List => cmd::scion_list(dir),
            ScionCommand::Show { id } => cmd::scion_show(dir, id),
            ScionCommand::Use { id } => cmd::scion_use(dir, id),
        },
        Command::Dirty { scion, prev } => cmd::dirty(dir, scion, prev),
        Command::Signal { kind, t, build } => cmd::signal(dir, kind, t, build),
        Command::Compile { scion, prev, out } => cmd::compile_cmd(dir, scion, prev, out),
        Command::Diff { left, right } => cmd::diff(dir, left, right),
        Command::Merge {
            base,
            ours,
            theirs,
            id,
        } => cmd::merge(dir, base, ours, theirs, id),
        Command::Feedback { command } => match command {
            FeedbackCommand::Ingest { file } => cmd::feedback_ingest(dir, file),
            FeedbackCommand::List => cmd::feedback_list(dir),
            FeedbackCommand::Show { id } => cmd::feedback_show(dir, id),
        },
        Command::Iterate {
            from,
            feedback,
            scion,
        } => cmd::iterate(dir, from, feedback, scion),
        Command::Export { format, scion, out } => cmd::export(dir, format.as_str(), scion, out),
        Command::Import {
            format,
            file,
            scion,
        } => cmd::import(dir, format.as_str(), file, scion),
        Command::Preview { scion, out } => cmd::preview(dir, scion, out),
        Command::Migrate => cmd::migrate(dir),
        Command::Status => cmd::status(dir),
        Command::Store { command } => match command {
            StoreCommand::Missing { remote } => cmd::store_missing(dir, remote),
            StoreCommand::Push { remote } => cmd::store_push(dir, remote),
            StoreCommand::Pull { remote } => cmd::store_pull(dir, remote),
        },
    }
}
