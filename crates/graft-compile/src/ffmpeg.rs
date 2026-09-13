// SPDX-License-Identifier: Apache-2.0

//! Long-GOP encode via a **system** ffmpeg/x264. graft does not link libx264
//! (GPL). Set `FFMPEG` / `FFPROBE` or put both on `PATH`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use graft_score::{Dest, Encoder, RateControlMode};

use crate::concat::{ConcatBackend, ConcatError, ConcatRequest};
use crate::encode::{
    AudioEncodeRequest, EncodeBackend, EncodeError, KerfEncodeRequest, SlotEncodeRequest,
};
use crate::grain::Grain;

pub fn ffmpeg_bin() -> PathBuf {
    std::env::var_os("FFMPEG")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("ffmpeg"))
}

pub fn ffprobe_bin() -> PathBuf {
    std::env::var_os("FFPROBE")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("ffprobe"))
}

fn run(bin: &Path, args: &[&str]) -> Result<std::process::Output, String> {
    Command::new(bin)
        .args(args)
        .output()
        .map_err(|e| format!("{}: {e}", bin.display()))
}

fn run_ok(bin: &Path, args: &[&str]) -> Result<Vec<u8>, String> {
    let out = run(bin, args)?;
    if !out.status.success() {
        return Err(format!(
            "{} {} failed: {}",
            bin.display(),
            args.join(" "),
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    Ok(out.stdout)
}

/// Video stream facts from ffprobe. Audio is reported, not compiled (v0.2).
#[derive(Clone, Debug)]
pub struct Probe {
    pub duration_s: f64,
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub video_codec: String,
    pub pix_fmt: String,
    pub color_space: String,
    pub time_base: String,
    pub frame_count: Option<u64>,
    pub has_audio: bool,
}

#[derive(Clone, Debug)]
pub enum MaterialKind {
    Gfi1 {
        width: u32,
        height: u32,
        nframes: u32,
    },
    Media(Probe),
    Opaque {
        bytes: usize,
    },
}

pub fn probe_bytes(bytes: &[u8]) -> MaterialKind {
    if bytes.len() >= 4 && &bytes[..4] == crate::intra::MAGIC {
        if let Ok(seq) = crate::intra::IntraSeq::decode(bytes) {
            return MaterialKind::Gfi1 {
                width: seq.width,
                height: seq.height,
                nframes: seq.frames.len() as u32,
            };
        }
    }
    match probe_media(bytes) {
        Ok(p) => MaterialKind::Media(p),
        Err(_) => MaterialKind::Opaque { bytes: bytes.len() },
    }
}

pub fn probe_media(bytes: &[u8]) -> Result<Probe, EncodeError> {
    let dir = tempfile::tempdir().map_err(|e| EncodeError::Invalid(e.to_string()))?;
    let path = dir.path().join("in.bin");
    fs::write(&path, bytes).map_err(|e| EncodeError::Invalid(e.to_string()))?;
    probe_path(&path)
}

pub fn probe_path(path: &Path) -> Result<Probe, EncodeError> {
    let ffprobe = ffprobe_bin();
    let path_s = path
        .to_str()
        .ok_or_else(|| EncodeError::Invalid("path".into()))?;
    let raw = run_ok(
        &ffprobe,
        &[
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height,avg_frame_rate,codec_name,pix_fmt,color_space,time_base,nb_frames:format=duration",
            "-of",
            "json",
            path_s,
        ],
    )
    .map_err(EncodeError::Invalid)?;
    let v: serde_json::Value =
        serde_json::from_slice(&raw).map_err(|e| EncodeError::Invalid(e.to_string()))?;
    let stream = v
        .get("streams")
        .and_then(|s| s.get(0))
        .ok_or_else(|| EncodeError::Invalid("ffprobe: no video stream".into()))?;
    let width = stream.get("width").and_then(|w| w.as_u64()).unwrap_or(0) as u32;
    let height = stream.get("height").and_then(|h| h.as_u64()).unwrap_or(0) as u32;
    let codec = stream
        .get("codec_name")
        .and_then(|c| c.as_str())
        .unwrap_or("unknown")
        .to_string();
    let pix_fmt = stream
        .get("pix_fmt")
        .and_then(|value| value.as_str())
        .unwrap_or("unknown")
        .to_string();
    let color_space = stream
        .get("color_space")
        .and_then(|value| value.as_str())
        .unwrap_or("unknown")
        .to_string();
    let time_base = stream
        .get("time_base")
        .and_then(|value| value.as_str())
        .unwrap_or("unknown")
        .to_string();
    let frame_count = stream
        .get("nb_frames")
        .and_then(|value| value.as_str())
        .and_then(|value| value.parse().ok());
    let fps = parse_rate(
        stream
            .get("avg_frame_rate")
            .and_then(|r| r.as_str())
            .unwrap_or("0/1"),
    );
    let duration_s = v
        .pointer("/format/duration")
        .and_then(|d| d.as_str())
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0);
    let audio = run_ok(
        &ffprobe,
        &[
            "-v",
            "error",
            "-select_streams",
            "a:0",
            "-show_entries",
            "stream=index",
            "-of",
            "csv=p=0",
            path_s,
        ],
    )
    .ok();
    let has_audio = audio.map(|a| !a.is_empty()).unwrap_or(false);
    if width == 0 || height == 0 {
        return Err(EncodeError::Invalid("ffprobe: video has zero size".into()));
    }
    Ok(Probe {
        duration_s,
        width,
        height,
        fps,
        video_codec: codec,
        pix_fmt,
        color_space,
        time_base,
        frame_count,
        has_audio,
    })
}

fn parse_rate(s: &str) -> f64 {
    let mut it = s.split('/');
    let n: f64 = it.next().and_then(|x| x.parse().ok()).unwrap_or(0.0);
    let d: f64 = it.next().and_then(|x| x.parse().ok()).unwrap_or(1.0);
    if d == 0.0 {
        0.0
    } else {
        n / d
    }
}

/// libx264 via ffmpeg. Slot encodes are closed-GOP (IDR at t=0 of the slot).
/// Concat bitstream-copies those files. Kerf is empty at an IDR join.
pub struct FfmpegX264 {
    ffmpeg: PathBuf,
}

impl FfmpegX264 {
    pub fn supports(dest: &Dest) -> bool {
        let n = dest.encoder.impl_name.to_ascii_lowercase();
        n == "x264" || n == "libx264"
    }

    pub fn from_env() -> Result<Self, EncodeError> {
        let ffmpeg = ffmpeg_bin();
        let ffprobe = ffprobe_bin();
        run_ok(&ffmpeg, &["-hide_banner", "-version"]).map_err(|e| {
            EncodeError::Invalid(format!(
                "graft compile needs ffmpeg on PATH (or $FFMPEG). {e}"
            ))
        })?;
        run_ok(&ffprobe, &["-hide_banner", "-version"]).map_err(|e| {
            EncodeError::Invalid(format!(
                "graft compile needs ffprobe on PATH (or $FFPROBE). {e}"
            ))
        })?;
        Ok(Self { ffmpeg })
    }

    pub fn available() -> bool {
        Self::from_env().is_ok()
    }

    pub fn runtime_toolchain() -> Result<serde_json::Value, EncodeError> {
        let ffmpeg = ffmpeg_bin();
        let ffprobe = ffprobe_bin();
        let version =
            run_ok(&ffmpeg, &["-hide_banner", "-version"]).map_err(EncodeError::Invalid)?;
        let encoder = run_ok(&ffmpeg, &["-hide_banner", "-h", "encoder=libx264"])
            .map_err(EncodeError::Invalid)?;
        let mut identity = version.clone();
        identity.extend_from_slice(&encoder);
        Ok(serde_json::json!({
            "contract": "ffmpeg-libx264-closed-gop-v1",
            "ffmpeg": ffmpeg,
            "ffprobe": ffprobe,
            "build_digest": graft_score::material_id(&identity),
            "threads": 1,
            "open_gop": false,
            "stitchable": true,
            "vf": "scale=bicubic,fps=dest.rate",
            "audio": "independent-aac-v1"
        }))
    }
}

fn x264_args(enc: &Encoder, dest: &Dest) -> Result<Vec<String>, EncodeError> {
    if enc.sc_threshold != 0 {
        return Err(EncodeError::Invalid("sc_threshold must be 0".into()));
    }
    let profile = enc.profile.as_deref().unwrap_or("high");
    let preset = enc.preset.as_deref().unwrap_or("medium");
    let crf = enc
        .rate_control
        .as_ref()
        .filter(|r| r.mode == RateControlMode::Crf)
        .and_then(|r| r.crf)
        .unwrap_or(18.0);
    let params = format!(
        "keyint={}:min-keyint={}:scenecut=0:open-gop=0:stitchable=1:threads=1:sliced-threads=0:sync-lookahead=0",
        enc.keyint, enc.keyint
    );
    Ok(vec![
        "-an".into(),
        "-c:v".into(),
        "libx264".into(),
        "-profile:v".into(),
        profile.into(),
        "-preset".into(),
        preset.into(),
        "-crf".into(),
        format!("{crf:.3}"),
        "-pix_fmt".into(),
        dest.pix_fmt.clone(),
        "-x264-params".into(),
        params,
        "-vf".into(),
        format!(
            "scale={}:{}:flags=bicubic,fps={}",
            dest.width,
            dest.height,
            dest.rate.as_f64()
        ),
        "-movflags".into(),
        "+faststart".into(),
    ])
}

impl EncodeBackend for FfmpegX264 {
    fn enabled(&self) -> bool {
        true
    }

    fn grain(&self, dest: &Dest) -> Grain {
        Grain::Gop {
            keyint: dest.encoder.keyint,
        }
    }

    fn encode_slot(&self, req: &SlotEncodeRequest<'_>) -> Result<Vec<u8>, EncodeError> {
        let dir = tempfile::tempdir().map_err(|e| EncodeError::Invalid(e.to_string()))?;
        let input = dir.path().join("material.bin");
        let output = dir.path().join("slot.mp4");
        fs::write(&input, req.material).map_err(|e| EncodeError::Invalid(e.to_string()))?;
        let mut args: Vec<String> = vec![
            "-hide_banner".into(),
            "-loglevel".into(),
            "error".into(),
            "-y".into(),
            "-ss".into(),
            format!("{:.3}", req.action.binding.in_s()),
            "-to".into(),
            format!("{:.3}", req.action.binding.out_s()),
            "-i".into(),
            input.to_string_lossy().into_owned(),
        ];
        args.extend(x264_args(&req.dest.encoder, req.dest)?);
        args.push(output.to_string_lossy().into_owned());
        let str_args: Vec<&str> = args.iter().map(String::as_str).collect();
        run_ok(&self.ffmpeg, &str_args).map_err(EncodeError::Invalid)?;
        fs::read(&output).map_err(|e| EncodeError::Invalid(e.to_string()))
    }

    fn encode_audio(&self, req: &AudioEncodeRequest<'_>) -> Result<Vec<u8>, EncodeError> {
        let dir = tempfile::tempdir().map_err(|e| EncodeError::Invalid(e.to_string()))?;
        let input = dir.path().join("material.bin");
        let output = dir.path().join("audio.m4a");
        fs::write(&input, req.material).map_err(|e| EncodeError::Invalid(e.to_string()))?;
        let args = [
            "-hide_banner".to_string(),
            "-loglevel".to_string(),
            "error".to_string(),
            "-y".to_string(),
            "-ss".to_string(),
            format!("{:.6}", req.action.binding.source.start_seconds()),
            "-to".to_string(),
            format!("{:.6}", req.action.binding.source.end_seconds()),
            "-i".to_string(),
            input.to_string_lossy().into_owned(),
            "-vn".to_string(),
            "-c:a".to_string(),
            "aac".to_string(),
            "-ar".to_string(),
            "48000".to_string(),
            "-ac".to_string(),
            "2".to_string(),
            "-b:a".to_string(),
            "192k".to_string(),
            output.to_string_lossy().into_owned(),
        ];
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        run_ok(&self.ffmpeg, &refs).map_err(EncodeError::Invalid)?;
        fs::read(&output).map_err(|e| EncodeError::Invalid(e.to_string()))
    }

    fn encode_kerf(&self, req: &KerfEncodeRequest<'_>) -> Result<Vec<u8>, EncodeError> {
        let _ = (req.left, req.right, req.dest);
        // Each SlotEncode is a closed-GOP file starting on IDR. The join is
        // file-aligned; bitstream-copy concat is the kerf. Mid-GOP splice is
        // a later fill of this same node.
        Ok(Vec::new())
    }
}

impl ConcatBackend for FfmpegX264 {
    fn enabled(&self) -> bool {
        true
    }

    fn concat(&self, req: &ConcatRequest<'_>) -> Result<Vec<u8>, ConcatError> {
        let dir = tempfile::tempdir().map_err(|e| ConcatError::Invalid(e.to_string()))?;
        let mut list = String::new();
        let mut n = 0u32;
        let mut baseline: Option<Probe> = None;
        let mut expected_duration = 0.0;
        for part in req.parts {
            if part.empty || part.bytes.is_empty() {
                continue;
            }
            let probe = probe_media(&part.bytes)
                .map_err(|e| ConcatError::Invalid(format!("probe concat part: {e}")))?;
            if probe.width != req.dest.width
                || probe.height != req.dest.height
                || (probe.fps - req.dest.rate.as_f64()).abs() > 0.001
                || probe.pix_fmt != req.dest.pix_fmt
                || probe.video_codec != "h264"
            {
                return Err(ConcatError::Invalid(format!(
                    "incompatible concat part: {}x{} {}fps {} {}",
                    probe.width, probe.height, probe.fps, probe.video_codec, probe.pix_fmt
                )));
            }
            if let Some(first) = &baseline {
                if first.time_base != probe.time_base
                    || first.pix_fmt != probe.pix_fmt
                    || first.color_space != probe.color_space
                {
                    return Err(ConcatError::Invalid(
                        "concat parts disagree on time base, pixel format, or color".into(),
                    ));
                }
            } else {
                baseline = Some(probe.clone());
            }
            expected_duration += probe.duration_s;
            let p = dir.path().join(format!("p{n}.mp4"));
            fs::write(&p, &part.bytes).map_err(|e| ConcatError::Invalid(e.to_string()))?;
            let path = p
                .to_str()
                .ok_or_else(|| ConcatError::Invalid("concat path".into()))?
                .replace('\\', "/")
                .replace('\'', r"'\''");
            list.push_str(&format!("file '{path}'\n"));
            n += 1;
        }
        if n == 0 {
            return Err(ConcatError::Invalid("concat has no video parts".into()));
        }
        let list_path = dir.path().join("list.txt");
        fs::write(&list_path, list).map_err(|e| ConcatError::Invalid(e.to_string()))?;
        let out = dir.path().join("dest.mp4");
        let list_s = list_path.to_string_lossy().into_owned();
        let out_s = out.to_string_lossy().into_owned();
        run_ok(
            &self.ffmpeg,
            &[
                "-hide_banner",
                "-loglevel",
                "error",
                "-y",
                "-f",
                "concat",
                "-safe",
                "0",
                "-i",
                &list_s,
                "-c",
                "copy",
                "-movflags",
                "+faststart",
                &out_s,
            ],
        )
        .map_err(ConcatError::Invalid)?;
        let bytes = fs::read(&out).map_err(|e| ConcatError::Invalid(e.to_string()))?;
        let has_audio = req
            .audio_parts
            .iter()
            .any(|part| !part.empty && !part.bytes.is_empty());
        let muxed = if has_audio {
            mux_audio(&self.ffmpeg, dir.path(), &out, req.audio_parts)?
        } else {
            bytes
        };
        let linked = probe_media(&muxed)
            .map_err(|e| ConcatError::Invalid(format!("verify linked output: {e}")))?;
        let tolerance = 1.5 / req.dest.rate.as_f64();
        if (linked.duration_s - expected_duration).abs() > tolerance {
            return Err(ConcatError::Invalid(format!(
                "linked duration {:.3}s differs from parts {:.3}s",
                linked.duration_s, expected_duration
            )));
        }
        if linked.width != req.dest.width
            || linked.height != req.dest.height
            || linked.video_codec != "h264"
            || linked.pix_fmt != req.dest.pix_fmt
        {
            return Err(ConcatError::Invalid(format!(
                "linked stream layout {}x{} {} {} is incompatible with dest",
                linked.width, linked.height, linked.video_codec, linked.pix_fmt
            )));
        }
        if let Some(frames) = linked.frame_count {
            let expected_frames = (expected_duration * req.dest.rate.as_f64()).round() as u64;
            if frames.abs_diff(expected_frames) > 2 {
                return Err(ConcatError::Invalid(format!(
                    "linked frame count {frames} differs from parts {expected_frames}"
                )));
            }
        }
        if has_audio && !linked.has_audio {
            return Err(ConcatError::Invalid(
                "linked output is missing synchronized audio".into(),
            ));
        }
        Ok(muxed)
    }
}

fn mux_audio(
    ffmpeg: &Path,
    dir: &Path,
    video: &Path,
    audio_parts: &[crate::concat::ConcatPartBytes],
) -> Result<Vec<u8>, ConcatError> {
    let mut list = String::new();
    for (i, part) in audio_parts.iter().enumerate() {
        if part.empty || part.bytes.is_empty() {
            continue;
        }
        let path = dir.join(format!("a{i}.m4a"));
        fs::write(&path, &part.bytes).map_err(|e| ConcatError::Invalid(e.to_string()))?;
        let path = path
            .to_str()
            .ok_or_else(|| ConcatError::Invalid("audio concat path".into()))?
            .replace('\\', "/")
            .replace('\'', r"'\''");
        list.push_str(&format!("file '{path}'\n"));
    }
    if list.is_empty() {
        return fs::read(video).map_err(|e| ConcatError::Invalid(e.to_string()));
    }
    let list_path = dir.join("audio.txt");
    fs::write(&list_path, list).map_err(|e| ConcatError::Invalid(e.to_string()))?;
    let audio = dir.join("audio.m4a");
    let list_s = list_path.to_string_lossy().into_owned();
    let audio_s = audio.to_string_lossy().into_owned();
    run_ok(
        ffmpeg,
        &[
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "concat",
            "-safe",
            "0",
            "-i",
            &list_s,
            "-c",
            "copy",
            &audio_s,
        ],
    )
    .map_err(ConcatError::Invalid)?;
    let muxed = dir.join("muxed.mp4");
    let video_s = video.to_string_lossy().into_owned();
    let muxed_s = muxed.to_string_lossy().into_owned();
    run_ok(
        ffmpeg,
        &[
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-i",
            &video_s,
            "-i",
            &audio_s,
            "-c:v",
            "copy",
            "-c:a",
            "copy",
            "-shortest",
            "-movflags",
            "+faststart",
            &muxed_s,
        ],
    )
    .map_err(ConcatError::Invalid)?;
    fs::read(&muxed).map_err(|e| ConcatError::Invalid(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::compile;
    use graft_cas::{Kind, Memory, Store};
    use graft_score::{
        Binding, BindingLayer, Clock, Dest, Encoder, FrameRange, FrameRate, Layer, Role, Scion,
        Score, Slot, TimedRange, Window, GRAFT_SCHEMA,
    };
    use std::collections::BTreeMap;

    fn color_mp4(ff: &FfmpegX264, color: &str, seconds: f64, tag_w: u32) -> Vec<u8> {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("c.mp4");
        let src = format!("color=c={color}:s={tag_w}x{tag_w}:d={seconds}:r=10");
        run_ok(
            &ff.ffmpeg,
            &[
                "-hide_banner",
                "-loglevel",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                &src,
                "-an",
                "-c:v",
                "libx264",
                "-pix_fmt",
                "yuv420p",
                "-x264-params",
                "keyint=10:min-keyint=10:scenecut=0:threads=1",
                out.to_str().unwrap(),
            ],
        )
        .expect("lavfi color mp4");
        fs::read(&out).unwrap()
    }

    fn score() -> Score {
        Score {
            graft: GRAFT_SCHEMA.into(),
            concept: "11111111-1111-4111-8111-111111111111".into(),
            clock: Clock {
                rate: FrameRate::new(10, 1),
                duration_frames: 40,
            },
            slots: vec![
                Slot {
                    id: "hook".into(),
                    role: Role::Hook,
                    range: FrameRange::new(0, 10),
                    optional: false,
                    window: Some(Window {
                        kind: "hook_rate".into(),
                        range: FrameRange::new(0, 10),
                    }),
                },
                Slot {
                    id: "body".into(),
                    role: Role::Body,
                    range: FrameRange::new(10, 20),
                    optional: false,
                    window: None,
                },
                Slot {
                    id: "cta".into(),
                    role: Role::Cta,
                    range: FrameRange::new(30, 10),
                    optional: false,
                    window: None,
                },
            ],
            layers: vec![Layer::Base],
            dest_default: Some("9x16".into()),
            spill_threshold_frames: 4,
        }
    }

    fn dest() -> Dest {
        let mut enc = Encoder::default_x264();
        enc.keyint = 10;
        Dest {
            id: "9x16".into(),
            width: 64,
            height: 64,
            rate: FrameRate::new(10, 1),
            pix_fmt: "yuv420p".into(),
            color: "bt709".into(),
            encoder: enc,
        }
    }

    fn bind(material: &str, in_s: f64, out_s: f64) -> Binding {
        Binding {
            material: material.into(),
            source: TimedRange::from_seconds(FrameRate::new(10, 1), in_s, out_s).unwrap(),
            params: None,
            audio: None,
        }
    }

    #[test]
    fn hook_swap_does_not_change_body_blob_on_x264() {
        let Ok(ff) = FfmpegX264::from_env() else {
            eprintln!("skip hook_swap_does_not_change_body_blob_on_x264 — install ffmpeg");
            return;
        };
        let score = score();
        let dest = dest();
        let store = Memory::new();
        let hook_v2 = store
            .put_blob(Kind::Material, &color_mp4(&ff, "red", 1.2, 64))
            .unwrap();
        let hook_v3 = store
            .put_blob(Kind::Material, &color_mp4(&ff, "blue", 1.2, 64))
            .unwrap();
        let body_v1 = store
            .put_blob(Kind::Material, &color_mp4(&ff, "green", 2.2, 64))
            .unwrap();
        let cta_v1 = store
            .put_blob(Kind::Material, &color_mp4(&ff, "yellow", 1.2, 64))
            .unwrap();

        let scion = |hook: &str| {
            let mut bindings = BTreeMap::new();
            bindings.insert("hook".into(), bind(hook, 0.0, 1.0));
            bindings.insert("body".into(), bind(body_v1.as_str(), 0.0, 2.0));
            bindings.insert("cta".into(), bind(cta_v1.as_str(), 0.0, 1.0));
            Scion {
                graft: GRAFT_SCHEMA.into(),
                id: "9x16".into(),
                concept: score.concept.clone(),
                parent: None,
                change_request: None,
                dest: dest.clone(),
                layers: vec![BindingLayer {
                    name: Layer::Base,
                    bindings,
                }],
            }
        };

        let first = compile(&score, &scion(hook_v2.as_str()), &store, &ff, &ff).unwrap();
        assert!(first.encoded);
        assert!(first.concat.is_some());
        let body_key = first
            .graph
            .slots
            .iter()
            .find(|s| s.slot.id == "body")
            .unwrap()
            .key
            .clone();
        let body_v2 = store.get_action(&body_key).unwrap().unwrap();
        let body_bytes_v2 = store.get_blob(Kind::SlotEncode, &body_v2.blob).unwrap();
        let concat_v2 = first.concat.clone().unwrap();

        let second = compile(&score, &scion(hook_v3.as_str()), &store, &ff, &ff).unwrap();
        let body_plan = second.plan.slots.iter().find(|s| s.id == "body").unwrap();
        let hook_plan = second.plan.slots.iter().find(|s| s.id == "hook").unwrap();
        assert_eq!(hook_plan.cache, "miss");
        assert_eq!(body_plan.cache, "hit");
        let body_v3 = store.get_action(&body_key).unwrap().unwrap();
        assert_eq!(body_v2.blob, body_v3.blob);
        let body_bytes_v3 = store.get_blob(Kind::SlotEncode, &body_v3.blob).unwrap();
        assert_eq!(body_bytes_v2, body_bytes_v3);
        assert_ne!(concat_v2, second.concat.unwrap());
        let dest_id = store
            .get_action(&second.graph.concat.key)
            .unwrap()
            .unwrap()
            .blob;
        let mp4 = store.get_blob(Kind::Concat, &dest_id).unwrap();
        assert!(mp4.windows(4).any(|w| w == b"ftyp"));
    }
}
