// SPDX-License-Identifier: Apache-2.0

//! Dirty-set, action graph, action cache schedule, intra + Long-GOP encode.

mod concat;
mod encode;
mod ffmpeg;
mod filters;
mod flatten;
mod grain;
mod graph;
mod intra;
mod keys;
mod pipeline;
mod plan;
mod schedule;
mod signal;
mod working;

pub use concat::{ConcatBackend, ConcatError, ConcatRequest, Unimplemented as ConcatUnimplemented};
pub use encode::{
    AudioMixPart, EncodeBackend, EncodeError, KerfEncodeRequest, SlotEncodeRequest, Unimplemented,
};
pub use ffmpeg::{probe_bytes, probe_path, FfmpegX264, MaterialKind, Probe};
pub use filters::{classify, ensure_op, is_lavfi_source, render, routes_audio, FilterClass};
pub use flatten::{flatten, Flattened};
pub use grain::Grain;
pub use graph::{
    lower, ActionGraph, AudioEncodeAction, AudioMixAction, CaptionsAction, ConcatAction,
    ConcatPart, KerfAction, NodeAction, OverlayMixAction, SlotEncodeAction,
};
pub use intra::{FrameIntra, IntraSeq};
pub use keys::{
    audio_encode_key, audio_mix_key, captions_key, deliver_key, kerf_key, node_key,
    overlay_mix_key, scion_hash, slot_encode_key, spine_picture_key, working_audio_key,
    working_picture_key,
};
pub use pipeline::{
    compile, compile_in, materials_present, CompileError, CompileOutcome, Nest, NestFamily,
};
pub use plan::{
    plan_from_schedule, prev_overlay, time_map_artifact_dir, write_time_map_artifact, CompilePlan,
    KerfPlan, PrevOverlay, SlotPlan,
};
pub use schedule::{schedule, CacheStatus, Schedule};
pub use signal::{
    covering_graph_nodes, declared_signal_range, dirty_from_signal, dirty_from_signal_range,
    DirtySet, Signal,
};
