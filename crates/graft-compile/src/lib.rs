// SPDX-License-Identifier: Apache-2.0

//! Dirty-set, action graph, action cache schedule, intra + Long-GOP encode.

mod concat;
mod encode;
mod ffmpeg;
mod flatten;
mod grain;
mod graph;
mod intra;
mod keys;
mod pipeline;
mod plan;
mod schedule;
mod signal;

pub use concat::{ConcatBackend, ConcatError, ConcatRequest, Unimplemented as ConcatUnimplemented};
pub use encode::{EncodeBackend, EncodeError, KerfEncodeRequest, SlotEncodeRequest, Unimplemented};
pub use ffmpeg::{probe_bytes, probe_path, FfmpegX264, MaterialKind, Probe};
pub use flatten::{flatten, Flattened};
pub use grain::Grain;
pub use graph::{lower, ActionGraph, ConcatAction, ConcatPart, KerfAction, SlotEncodeAction};
pub use intra::{FrameIntra, IntraSeq};
pub use keys::{kerf_key, scion_hash, slot_encode_key};
pub use pipeline::{compile, materials_present, CompileError, CompileOutcome};
pub use plan::{
    plan_from_schedule, prev_overlay, time_map_artifact_dir, write_time_map_artifact, CompilePlan,
    KerfPlan, PrevOverlay, SlotPlan,
};
pub use schedule::{schedule, CacheStatus, Schedule};
pub use signal::{dirty_from_signal, DirtySet, Signal};
