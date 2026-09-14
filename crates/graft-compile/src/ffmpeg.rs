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

/// Video stream facts from ffprobe. Audio duration is reported when present.
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
    pub audio_duration_s: Option<f64>,
    pub starts_on_idr: bool,
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
    let audio_duration_s = if has_audio {
        probe_audio_duration(path).ok()
    } else {
        None
    };
    let starts_on_idr = first_packet_is_keyframe(path).unwrap_or(false);
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
        audio_duration_s,
        starts_on_idr,
    })
}

fn probe_audio_duration(path: &Path) -> Result<f64, EncodeError> {
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
            "a:0",
            "-show_entries",
            "stream=duration:format=duration",
            "-of",
            "json",
            path_s,
        ],
    )
    .map_err(EncodeError::Invalid)?;
    let v: serde_json::Value =
        serde_json::from_slice(&raw).map_err(|e| EncodeError::Invalid(e.to_string()))?;
    v.pointer("/streams/0/duration")
        .and_then(|d| d.as_str())
        .and_then(|s| s.parse().ok())
        .or_else(|| {
            v.pointer("/format/duration")
                .and_then(|d| d.as_str())
                .and_then(|s| s.parse().ok())
        })
        .ok_or_else(|| EncodeError::Invalid("ffprobe: audio duration missing".into()))
}

fn first_packet_is_keyframe(path: &Path) -> Result<bool, EncodeError> {
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
            "-show_frames",
            "-read_intervals",
            "%+#1",
            "-show_entries",
            "frame=key_frame,pict_type",
            "-of",
            "json",
            path_s,
        ],
    )
    .map_err(EncodeError::Invalid)?;
    let v: serde_json::Value =
        serde_json::from_slice(&raw).map_err(|e| EncodeError::Invalid(e.to_string()))?;
    let frame = v
        .get("frames")
        .and_then(|frames| frames.get(0))
        .ok_or_else(|| EncodeError::Invalid("ffprobe: no video frame".into()))?;
    let key = frame
        .get("key_frame")
        .and_then(|value| value.as_u64())
        .unwrap_or(0)
        == 1;
    let pict = frame
        .get("pict_type")
        .and_then(|value| value.as_str())
        .unwrap_or("");
    Ok(key && pict == "I")
}

fn atempo_filter(speed: f64) -> Result<String, EncodeError> {
    if speed <= 0.0 || !speed.is_finite() {
        return Err(EncodeError::Invalid(
            "binding speed must be finite and > 0".into(),
        ));
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

fn silence_aac(ffmpeg: &Path, duration_s: f64) -> Result<Vec<u8>, ConcatError> {
    let dir = tempfile::tempdir().map_err(|e| ConcatError::Invalid(e.to_string()))?;
    let out = dir.path().join("silence.m4a");
    let out_s = out.to_string_lossy().into_owned();
    run_ok(
        ffmpeg,
        &[
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "anullsrc=r=48000:cl=stereo",
            "-t",
            &format!("{duration_s:.6}"),
            "-c:a",
            "aac",
            "-ar",
            "48000",
            "-ac",
            "2",
            "-b:a",
            "192k",
            &out_s,
        ],
    )
    .map_err(ConcatError::Invalid)?;
    fs::read(&out).map_err(|e| ConcatError::Invalid(e.to_string()))
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

fn x264_args(enc: &Encoder, dest: &Dest, speed: f64) -> Result<Vec<String>, EncodeError> {
    if enc.sc_threshold != 0 {
        return Err(EncodeError::Invalid("sc_threshold must be 0".into()));
    }
    if speed <= 0.0 || !speed.is_finite() {
        return Err(EncodeError::Invalid(
            "binding speed must be finite and > 0".into(),
        ));
    }
    let profile = enc.profile.as_deref().unwrap_or("high");
    let preset = enc.preset.as_deref().unwrap_or("medium");
    let params = format!(
        "keyint={}:min-keyint={}:scenecut=0:open-gop=0:stitchable=1:threads=1:sliced-threads=0:sync-lookahead=0",
        enc.keyint, enc.keyint
    );
    let vf = if (speed - 1.0).abs() > 1e-9 {
        format!(
            "setpts=PTS/{speed:.6},scale={}:{}:flags=bicubic,fps={}",
            dest.width,
            dest.height,
            dest.rate.as_f64()
        )
    } else {
        format!(
            "scale={}:{}:flags=bicubic,fps={}",
            dest.width,
            dest.height,
            dest.rate.as_f64()
        )
    };
    let mut args = vec![
        "-an".into(),
        "-c:v".into(),
        "libx264".into(),
        "-profile:v".into(),
        profile.into(),
        "-preset".into(),
        preset.into(),
    ];
    if let Some(level) = enc.level.as_deref() {
        args.push("-level:v".into());
        args.push(level.into());
    }
    match enc.rate_control.as_ref() {
        Some(rc) if rc.mode == RateControlMode::Bitrate => {
            let bitrate = rc.bitrate.as_deref().ok_or_else(|| {
                EncodeError::Invalid(
                    r#"{"error":"invalid_rate_control","reason":"bitrate mode needs encoder.rate_control.bitrate"}"#.into(),
                )
            })?;
            args.push("-b:v".into());
            args.push(bitrate.into());
        }
        _ => {
            let crf = enc
                .rate_control
                .as_ref()
                .and_then(|r| r.crf)
                .unwrap_or(18.0);
            args.push("-crf".into());
            args.push(format!("{crf:.3}"));
        }
    }
    args.extend([
        "-pix_fmt".into(),
        dest.pix_fmt.clone(),
        "-x264-params".into(),
        params,
        "-vf".into(),
        vf,
        "-movflags".into(),
        "+faststart".into(),
    ]);
    Ok(args)
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
        args.extend(x264_args(
            &req.dest.encoder,
            req.dest,
            req.action.binding.speed(),
        )?);
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
        ];
        let mut args = args.to_vec();
        if (req.action.speed - 1.0).abs() > 1e-9 {
            args.push("-filter:a".into());
            args.push(atempo_filter(req.action.speed)?);
        }
        args.push(output.to_string_lossy().into_owned());
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        run_ok(&self.ffmpeg, &refs).map_err(EncodeError::Invalid)?;
        fs::read(&output).map_err(|e| EncodeError::Invalid(e.to_string()))
    }

    fn encode_kerf(&self, req: &KerfEncodeRequest<'_>) -> Result<Vec<u8>, EncodeError> {
        if req.action.transition.eq_ignore_ascii_case("fade") {
            return encode_fade_kerf(self, req);
        }
        if req.action.noop {
            return Ok(Vec::new());
        }
        let dir = tempfile::tempdir().map_err(|e| EncodeError::Invalid(e.to_string()))?;
        let left_path = dir.path().join("left.mp4");
        let right_path = dir.path().join("right.mp4");
        fs::write(&left_path, req.left).map_err(|e| EncodeError::Invalid(e.to_string()))?;
        fs::write(&right_path, req.right).map_err(|e| EncodeError::Invalid(e.to_string()))?;
        let left = probe_path(&left_path)?;
        let right = probe_path(&right_path)?;
        if left.starts_on_idr && right.starts_on_idr {
            return Ok(Vec::new());
        }
        let gop_s = req.dest.encoder.keyint as f64 / req.dest.rate.as_f64();
        if gop_s <= 0.0 {
            return Err(EncodeError::Invalid(
                r#"{"error":"kerf_unproven","reason":"dest keyint/rate cannot form a GOP"}"#.into(),
            ));
        }
        let left_out = dir.path().join("left-gop.mp4");
        let right_out = dir.path().join("right-gop.mp4");
        let mut left_args = vec![
            "-hide_banner".into(),
            "-loglevel".into(),
            "error".into(),
            "-y".into(),
            "-sseof".into(),
            format!("-{gop_s:.6}"),
            "-i".into(),
            left_path.to_string_lossy().into_owned(),
            "-t".into(),
            format!("{gop_s:.6}"),
        ];
        left_args.extend(x264_args(&req.dest.encoder, req.dest, 1.0)?);
        left_args.push(left_out.to_string_lossy().into_owned());
        let left_refs: Vec<&str> = left_args.iter().map(String::as_str).collect();
        run_ok(&self.ffmpeg, &left_refs).map_err(|e| {
            EncodeError::Invalid(format!(
                r#"{{"error":"kerf_unproven","reason":"left GOP encode failed: {e}"}}"#
            ))
        })?;
        let mut right_args = vec![
            "-hide_banner".into(),
            "-loglevel".into(),
            "error".into(),
            "-y".into(),
            "-ss".into(),
            "0".into(),
            "-i".into(),
            right_path.to_string_lossy().into_owned(),
            "-t".into(),
            format!("{gop_s:.6}"),
        ];
        right_args.extend(x264_args(&req.dest.encoder, req.dest, 1.0)?);
        right_args.push(right_out.to_string_lossy().into_owned());
        let right_refs: Vec<&str> = right_args.iter().map(String::as_str).collect();
        run_ok(&self.ffmpeg, &right_refs).map_err(|e| {
            EncodeError::Invalid(format!(
                r#"{{"error":"kerf_unproven","reason":"right GOP encode failed: {e}"}}"#
            ))
        })?;
        let list = dir.path().join("kerf.txt");
        let kerf_out = dir.path().join("kerf.mp4");
        fs::write(
            &list,
            format!(
                "file '{}'\nfile '{}'\n",
                left_out.to_string_lossy().replace('\'', r"'\''"),
                right_out.to_string_lossy().replace('\'', r"'\''")
            ),
        )
        .map_err(|e| EncodeError::Invalid(e.to_string()))?;
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
                list.to_str().unwrap(),
                "-c",
                "copy",
                kerf_out.to_str().unwrap(),
            ],
        )
        .map_err(|e| {
            EncodeError::Invalid(format!(
                r#"{{"error":"kerf_unproven","reason":"kerf concat failed: {e}"}}"#
            ))
        })?;
        let bytes = fs::read(&kerf_out).map_err(|e| EncodeError::Invalid(e.to_string()))?;
        let probed = probe_media(&bytes)?;
        if !probed.starts_on_idr
            || probed.video_codec != "h264"
            || probed.width != req.dest.width
            || probed.height != req.dest.height
            || probed.pix_fmt != req.dest.pix_fmt
            || (probed.fps - req.dest.rate.as_f64()).abs() > 0.001
        {
            return Err(EncodeError::Invalid(format!(
                r#"{{"error":"kerf_unproven","reason":"kerf stream {}x{} {} {} idr={} tb={} does not match dest {}x{} {} closed-GOP contract"}}"#,
                probed.width,
                probed.height,
                probed.video_codec,
                probed.pix_fmt,
                probed.starts_on_idr,
                probed.time_base,
                req.dest.width,
                req.dest.height,
                req.dest.pix_fmt
            )));
        }
        Ok(bytes)
    }

    fn mix_audio(
        &self,
        dest: &Dest,
        parts: &[crate::encode::AudioMixPart<'_>],
    ) -> Result<Vec<u8>, EncodeError> {
        if parts.is_empty() {
            return Err(EncodeError::Invalid("audio_mix has no parts".into()));
        }
        let _ = dest;
        let dir = tempfile::tempdir().map_err(|e| EncodeError::Invalid(e.to_string()))?;
        let mix_dur = parts
            .iter()
            .map(|p| p.start_s + p.duration_s)
            .fold(0.0, f64::max);
        let mut args = vec![
            "-hide_banner".into(),
            "-loglevel".into(),
            "error".into(),
            "-y".into(),
        ];
        let mut filters = Vec::new();
        for (i, part) in parts.iter().enumerate() {
            let path = dir.path().join(format!("a{i}.m4a"));
            fs::write(&path, part.bytes).map_err(|e| EncodeError::Invalid(e.to_string()))?;
            args.push("-i".into());
            args.push(path.to_string_lossy().into_owned());
            let delay_ms = (part.start_s * 1000.0).round().max(0.0) as i64;
            filters.push(format!(
                "[{i}:a]adelay={delay_ms}|{delay_ms},apad=whole_dur={mix_dur:.6}[a{i}]"
            ));
        }
        let labels = (0..parts.len())
            .map(|i| format!("[a{i}]"))
            .collect::<Vec<_>>()
            .join("");
        filters.push(format!(
            "{labels}amix=inputs={}:duration=longest:normalize=0[outa]",
            parts.len()
        ));
        let out = dir.path().join("mix.m4a");
        args.extend([
            "-filter_complex".into(),
            filters.join(";"),
            "-map".into(),
            "[outa]".into(),
            "-c:a".into(),
            "aac".into(),
            "-ar".into(),
            "48000".into(),
            "-ac".into(),
            "2".into(),
            out.to_string_lossy().into_owned(),
        ]);
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        run_ok(&self.ffmpeg, &refs).map_err(EncodeError::Invalid)?;
        fs::read(&out).map_err(|e| EncodeError::Invalid(e.to_string()))
    }

    fn overlay_brand(
        &self,
        dest: &Dest,
        picture: &[u8],
        brand: &[u8],
    ) -> Result<Vec<u8>, EncodeError> {
        let dir = tempfile::tempdir().map_err(|e| EncodeError::Invalid(e.to_string()))?;
        let pic = dir.path().join("pic.mp4");
        let logo = dir.path().join("brand.bin");
        let out = dir.path().join("over.mp4");
        fs::write(&pic, picture).map_err(|e| EncodeError::Invalid(e.to_string()))?;
        fs::write(&logo, brand).map_err(|e| EncodeError::Invalid(e.to_string()))?;
        let mut args = vec![
            "-hide_banner".into(),
            "-loglevel".into(),
            "error".into(),
            "-y".into(),
            "-i".into(),
            pic.to_string_lossy().into_owned(),
            "-i".into(),
            logo.to_string_lossy().into_owned(),
            "-filter_complex".into(),
            format!(
                "[1:v]scale={}:{}:flags=bicubic[bg];[0:v][bg]overlay=0:0:eof_action=repeat",
                dest.width, dest.height
            ),
        ];
        let mut enc = x264_args(&dest.encoder, dest, 1.0)?;
        if let Some(i) = enc.iter().position(|a| a == "-vf") {
            enc.remove(i);
            if i < enc.len() {
                enc.remove(i);
            }
        }
        enc.retain(|a| a != "-an");
        args.extend(enc);
        args.push(out.to_string_lossy().into_owned());
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        run_ok(&self.ffmpeg, &refs).map_err(EncodeError::Invalid)?;
        fs::read(&out).map_err(|e| EncodeError::Invalid(e.to_string()))
    }
}

fn encode_fade_kerf(ff: &FfmpegX264, req: &KerfEncodeRequest<'_>) -> Result<Vec<u8>, EncodeError> {
    if req.action.duration_frames == 0 {
        return Err(EncodeError::Invalid(
            r#"{"error":"invalid_fade","reason":"duration_frames must be > 0"}"#.into(),
        ));
    }
    let fade_s = req.action.duration_frames as f64 / req.dest.rate.as_f64();
    let dir = tempfile::tempdir().map_err(|e| EncodeError::Invalid(e.to_string()))?;
    let left_path = dir.path().join("left.mp4");
    let right_path = dir.path().join("right.mp4");
    fs::write(&left_path, req.left).map_err(|e| EncodeError::Invalid(e.to_string()))?;
    fs::write(&right_path, req.right).map_err(|e| EncodeError::Invalid(e.to_string()))?;
    let left_out = dir.path().join("left-fade.mp4");
    let right_out = dir.path().join("right-fade.mp4");
    let mut left_args = vec![
        "-hide_banner".into(),
        "-loglevel".into(),
        "error".into(),
        "-y".into(),
        "-sseof".into(),
        format!("-{fade_s:.6}"),
        "-i".into(),
        left_path.to_string_lossy().into_owned(),
        "-t".into(),
        format!("{fade_s:.6}"),
    ];
    left_args.extend(x264_args(&req.dest.encoder, req.dest, 1.0)?);
    // x264_args already includes -vf scale; replace by chaining fade into that vf.
    replace_vf(
        &mut left_args,
        &format!(
            "fade=t=out:st=0:d={fade_s:.6},scale={}:{}:flags=bicubic,fps={}",
            req.dest.width,
            req.dest.height,
            req.dest.rate.as_f64()
        ),
    );
    left_args.push(left_out.to_string_lossy().into_owned());
    let left_refs: Vec<&str> = left_args.iter().map(String::as_str).collect();
    run_ok(&ff.ffmpeg, &left_refs).map_err(|e| {
        EncodeError::Invalid(format!(
            r#"{{"error":"kerf_unproven","reason":"left fade encode failed: {e}"}}"#
        ))
    })?;
    let mut right_args = vec![
        "-hide_banner".into(),
        "-loglevel".into(),
        "error".into(),
        "-y".into(),
        "-ss".into(),
        "0".into(),
        "-i".into(),
        right_path.to_string_lossy().into_owned(),
        "-t".into(),
        format!("{fade_s:.6}"),
    ];
    right_args.extend(x264_args(&req.dest.encoder, req.dest, 1.0)?);
    replace_vf(
        &mut right_args,
        &format!(
            "fade=t=in:st=0:d={fade_s:.6},scale={}:{}:flags=bicubic,fps={}",
            req.dest.width,
            req.dest.height,
            req.dest.rate.as_f64()
        ),
    );
    right_args.push(right_out.to_string_lossy().into_owned());
    let right_refs: Vec<&str> = right_args.iter().map(String::as_str).collect();
    run_ok(&ff.ffmpeg, &right_refs).map_err(|e| {
        EncodeError::Invalid(format!(
            r#"{{"error":"kerf_unproven","reason":"right fade encode failed: {e}"}}"#
        ))
    })?;
    let list = dir.path().join("kerf.txt");
    let kerf_out = dir.path().join("kerf.mp4");
    fs::write(
        &list,
        format!(
            "file '{}'\nfile '{}'\n",
            left_out.to_string_lossy().replace('\'', r"'\''"),
            right_out.to_string_lossy().replace('\'', r"'\''")
        ),
    )
    .map_err(|e| EncodeError::Invalid(e.to_string()))?;
    run_ok(
        &ff.ffmpeg,
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
            list.to_str().unwrap(),
            "-c",
            "copy",
            kerf_out.to_str().unwrap(),
        ],
    )
    .map_err(|e| {
        EncodeError::Invalid(format!(
            r#"{{"error":"kerf_unproven","reason":"fade kerf concat failed: {e}"}}"#
        ))
    })?;
    let bytes = fs::read(&kerf_out).map_err(|e| EncodeError::Invalid(e.to_string()))?;
    let probed = probe_media(&bytes)?;
    if !probed.starts_on_idr
        || probed.video_codec != "h264"
        || probed.width != req.dest.width
        || probed.height != req.dest.height
    {
        return Err(EncodeError::Invalid(
            r#"{"error":"kerf_unproven","reason":"fade kerf stream does not match dest closed-GOP contract"}"#
                .into(),
        ));
    }
    Ok(bytes)
}

fn replace_vf(args: &mut Vec<String>, vf: &str) {
    if let Some(i) = args.iter().position(|a| a == "-vf") {
        if i + 1 < args.len() {
            args[i + 1] = vf.into();
            return;
        }
    }
    args.push("-vf".into());
    args.push(vf.into());
}

fn trim_h264_part(
    ff: &FfmpegX264,
    dest: &Dest,
    bytes: &[u8],
    head_frames: u64,
    tail_frames: u64,
) -> Result<(Vec<u8>, f64), ConcatError> {
    let probe = probe_media(bytes).map_err(|e| ConcatError::Invalid(format!("probe trim: {e}")))?;
    let fps = dest.rate.as_f64();
    let head_s = head_frames as f64 / fps;
    let tail_s = tail_frames as f64 / fps;
    let keep_s = (probe.duration_s - head_s - tail_s).max(0.0);
    if keep_s <= 0.0 {
        return Err(ConcatError::Invalid(
            "fade trim removed the whole slot".into(),
        ));
    }
    if head_frames == 0 && tail_frames == 0 {
        return Ok((bytes.to_vec(), probe.duration_s));
    }
    let dir = tempfile::tempdir().map_err(|e| ConcatError::Invalid(e.to_string()))?;
    let input = dir.path().join("in.mp4");
    let output = dir.path().join("trim.mp4");
    fs::write(&input, bytes).map_err(|e| ConcatError::Invalid(e.to_string()))?;
    let mut args = vec![
        "-hide_banner".into(),
        "-loglevel".into(),
        "error".into(),
        "-y".into(),
        "-ss".into(),
        format!("{head_s:.6}"),
        "-i".into(),
        input.to_string_lossy().into_owned(),
        "-t".into(),
        format!("{keep_s:.6}"),
    ];
    args.extend(
        x264_args(&dest.encoder, dest, 1.0).map_err(|e| ConcatError::Invalid(e.to_string()))?,
    );
    args.push(output.to_string_lossy().into_owned());
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    run_ok(&ff.ffmpeg, &refs).map_err(ConcatError::Invalid)?;
    let out = fs::read(&output).map_err(|e| ConcatError::Invalid(e.to_string()))?;
    let trimmed =
        probe_media(&out).map_err(|e| ConcatError::Invalid(format!("probe trimmed part: {e}")))?;
    Ok((out, trimmed.duration_s))
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
            let (bytes, part_duration) = if part.trim_head_frames > 0 || part.trim_tail_frames > 0 {
                trim_h264_part(
                    self,
                    req.dest,
                    &part.bytes,
                    part.trim_head_frames,
                    part.trim_tail_frames,
                )?
            } else {
                let probe = probe_media(&part.bytes)
                    .map_err(|e| ConcatError::Invalid(format!("probe concat part: {e}")))?;
                (part.bytes.clone(), probe.duration_s)
            };
            let probe = probe_media(&bytes)
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
                if first.pix_fmt != probe.pix_fmt || first.color_space != probe.color_space {
                    return Err(ConcatError::Invalid(
                        "concat parts disagree on pixel format or color".into(),
                    ));
                }
            } else {
                baseline = Some(probe.clone());
            }
            expected_duration += part_duration;
            let p = dir.path().join(format!("p{n}.mp4"));
            fs::write(&p, &bytes).map_err(|e| ConcatError::Invalid(e.to_string()))?;
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
        if has_audio {
            let audio_s = linked.audio_duration_s.unwrap_or(0.0);
            if (audio_s - expected_duration).abs() > tolerance {
                return Err(ConcatError::Invalid(format!(
                    "linked audio duration {:.3}s differs from picture {:.3}s",
                    audio_s, expected_duration
                )));
            }
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
        let path = dir.join(format!("a{i}.m4a"));
        let bytes = if part.empty || part.bytes.is_empty() {
            if part.duration_s <= 0.0 {
                continue;
            }
            silence_aac(ffmpeg, part.duration_s)?
        } else {
            part.bytes.clone()
        };
        fs::write(&path, bytes).map_err(|e| ConcatError::Invalid(e.to_string()))?;
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
        Binding, BindingLayer, Clock, Dest, Encoder, FrameRange, FrameRate, Join, Layer,
        RateControl, RateControlMode, Role, Scion, Score, Slot, TimedRange, Window, GRAFT_SCHEMA,
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

    fn sine_m4a(ff: &FfmpegX264, seconds: f64) -> Vec<u8> {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("a.m4a");
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
                &format!("sine=frequency=440:duration={seconds}"),
                "-c:a",
                "aac",
                "-ar",
                "48000",
                "-ac",
                "2",
                out.to_str().unwrap(),
            ],
        )
        .expect("lavfi sine");
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
            joins: Vec::new(),
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
        let kerf = second
            .graph
            .kerfs
            .iter()
            .find(|k| k.left == "hook")
            .unwrap();
        let kerf_bytes = store
            .get_blob(
                Kind::Kerf,
                &store.get_action(&kerf.key).unwrap().unwrap().blob,
            )
            .unwrap();
        assert!(kerf_bytes.is_empty());
    }

    fn bind_speed(material: &str, in_s: f64, out_s: f64, speed: f64) -> Binding {
        Binding {
            material: material.into(),
            source: TimedRange::from_seconds(FrameRate::new(10, 1), in_s, out_s).unwrap(),
            params: Some(serde_json::json!({ "speed": speed })),
            audio: None,
        }
    }

    #[test]
    fn speed_retime_invalidates_only_that_slot_on_x264() {
        let Ok(ff) = FfmpegX264::from_env() else {
            eprintln!("skip speed_retime_invalidates_only_that_slot_on_x264 — install ffmpeg");
            return;
        };
        let score = score();
        let dest = dest();
        let store = Memory::new();
        let hook = store
            .put_blob(Kind::Material, &color_mp4(&ff, "red", 2.2, 64))
            .unwrap();
        let body = store
            .put_blob(Kind::Material, &color_mp4(&ff, "green", 2.2, 64))
            .unwrap();
        let cta = store
            .put_blob(Kind::Material, &color_mp4(&ff, "yellow", 1.2, 64))
            .unwrap();
        let scion = |hook_speed: f64, hook_out: f64| {
            let mut bindings = BTreeMap::new();
            bindings.insert(
                "hook".into(),
                bind_speed(hook.as_str(), 0.0, hook_out, hook_speed),
            );
            bindings.insert("body".into(), bind(body.as_str(), 0.0, 2.0));
            bindings.insert("cta".into(), bind(cta.as_str(), 0.0, 1.0));
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
        let first = compile(&score, &scion(1.0, 1.0), &store, &ff, &ff).unwrap();
        let body_key = first
            .graph
            .slots
            .iter()
            .find(|s| s.slot.id == "body")
            .unwrap()
            .key
            .clone();
        let body_blob = store.get_action(&body_key).unwrap().unwrap().blob;
        let second = compile(&score, &scion(2.0, 2.0), &store, &ff, &ff).unwrap();
        assert_eq!(
            second
                .plan
                .slots
                .iter()
                .find(|s| s.id == "hook")
                .unwrap()
                .cache,
            "miss"
        );
        assert_eq!(
            second
                .plan
                .slots
                .iter()
                .find(|s| s.id == "body")
                .unwrap()
                .cache,
            "hit"
        );
        assert_eq!(
            store.get_action(&body_key).unwrap().unwrap().blob,
            body_blob
        );
        let hook_key = second
            .graph
            .slots
            .iter()
            .find(|s| s.slot.id == "hook")
            .unwrap()
            .key
            .clone();
        let hook_mp4 = store
            .get_blob(
                Kind::SlotEncode,
                &store.get_action(&hook_key).unwrap().unwrap().blob,
            )
            .unwrap();
        let probed = probe_media(&hook_mp4).unwrap();
        assert!((probed.duration_s - 1.0).abs() < 0.15);
    }

    fn mid_gop_copy(ff: &FfmpegX264, color: &str) -> Vec<u8> {
        let long = color_mp4(ff, color, 3.2, 64);
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("long.mp4");
        let out = dir.path().join("mid.mp4");
        fs::write(&src, long).unwrap();
        run_ok(
            &ff.ffmpeg,
            &[
                "-hide_banner",
                "-loglevel",
                "error",
                "-y",
                "-ss",
                "0.35",
                "-i",
                src.to_str().unwrap(),
                "-t",
                "1.0",
                "-c",
                "copy",
                out.to_str().unwrap(),
            ],
        )
        .expect("mid-GOP copy");
        fs::read(&out).unwrap()
    }

    #[test]
    fn non_idr_join_fills_kerf_without_rewriting_closed_gop_slots() {
        let Ok(ff) = FfmpegX264::from_env() else {
            eprintln!(
                "skip non_idr_join_fills_kerf_without_rewriting_closed_gop_slots — install ffmpeg"
            );
            return;
        };
        let dest = dest();
        let closed = color_mp4(&ff, "green", 1.2, 64);
        let closed_probe = probe_media(&closed).unwrap();
        assert!(closed_probe.starts_on_idr);
        let empty = ff
            .encode_kerf(&KerfEncodeRequest {
                action: &crate::graph::KerfAction {
                    left: "hook".into(),
                    right: "body".into(),
                    key: graft_cas::ActionKey::from_canonical(&serde_json::json!({"t":"closed"})),
                    left_key: graft_cas::ActionKey::from_canonical(&serde_json::json!({"l":1})),
                    right_key: graft_cas::ActionKey::from_canonical(&serde_json::json!({"r":1})),
                    transition: "cut".into(),
                    duration_frames: 0,
                    grain: Grain::Gop { keyint: 10 },
                    noop: false,
                },
                dest: &dest,
                left: &closed,
                right: &closed,
            })
            .unwrap();
        assert!(empty.is_empty());

        let mid = mid_gop_copy(&ff, "red");
        let mid_probe = probe_media(&mid).unwrap();
        if mid_probe.starts_on_idr {
            eprintln!("skip non-IDR kerf fill — stream copy still started on IDR");
            return;
        }
        let filled = ff
            .encode_kerf(&KerfEncodeRequest {
                action: &crate::graph::KerfAction {
                    left: "hook".into(),
                    right: "body".into(),
                    key: graft_cas::ActionKey::from_canonical(&serde_json::json!({"t":"open"})),
                    left_key: graft_cas::ActionKey::from_canonical(&serde_json::json!({"l":2})),
                    right_key: graft_cas::ActionKey::from_canonical(&serde_json::json!({"r":2})),
                    transition: "cut".into(),
                    duration_frames: 0,
                    grain: Grain::Gop { keyint: 10 },
                    noop: false,
                },
                dest: &dest,
                left: &mid,
                right: &mid,
            })
            .unwrap();
        assert!(!filled.is_empty());
        let kerf = probe_media(&filled).unwrap();
        assert!(kerf.starts_on_idr);
        assert_eq!(kerf.video_codec, "h264");
        assert!(!closed.is_empty());
    }

    #[test]
    fn fade_hook_swap_reuses_body_and_duration_change_misses_kerf_on_x264() {
        let Ok(ff) = FfmpegX264::from_env() else {
            eprintln!(
                "skip fade_hook_swap_reuses_body_and_duration_change_misses_kerf_on_x264 — install ffmpeg"
            );
            return;
        };
        let mut score = score();
        score.joins = vec![Join {
            left: "hook".into(),
            right: "body".into(),
            transition: "fade".into(),
            duration_frames: 2,
        }];
        let dest = dest();
        let store = Memory::new();
        let hook = store
            .put_blob(Kind::Material, &color_mp4(&ff, "red", 1.2, 64))
            .unwrap();
        let hook2 = store
            .put_blob(Kind::Material, &color_mp4(&ff, "blue", 1.2, 64))
            .unwrap();
        let body = store
            .put_blob(Kind::Material, &color_mp4(&ff, "green", 2.2, 64))
            .unwrap();
        let cta = store
            .put_blob(Kind::Material, &color_mp4(&ff, "yellow", 1.2, 64))
            .unwrap();
        let scion = |hook_id: &str| {
            let mut bindings = BTreeMap::new();
            bindings.insert("hook".into(), bind(hook_id, 0.0, 1.0));
            bindings.insert("body".into(), bind(body.as_str(), 0.0, 2.0));
            bindings.insert("cta".into(), bind(cta.as_str(), 0.0, 1.0));
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
        let first = compile(&score, &scion(hook.as_str()), &store, &ff, &ff).unwrap();
        let body_key = first
            .graph
            .slots
            .iter()
            .find(|s| s.slot.id == "body")
            .unwrap()
            .key
            .clone();
        let body_blob = store.get_action(&body_key).unwrap().unwrap().blob;
        let kerf = first
            .graph
            .kerfs
            .iter()
            .find(|k| k.left == "hook" && k.right == "body")
            .unwrap();
        assert_eq!(kerf.transition, "fade");
        let kerf_bytes = store
            .get_blob(
                Kind::Kerf,
                &store.get_action(&kerf.key).unwrap().unwrap().blob,
            )
            .unwrap();
        assert!(!kerf_bytes.is_empty());
        let second = compile(&score, &scion(hook2.as_str()), &store, &ff, &ff).unwrap();
        assert_eq!(
            second
                .plan
                .slots
                .iter()
                .find(|s| s.id == "body")
                .unwrap()
                .cache,
            "hit"
        );
        assert_eq!(
            store.get_action(&body_key).unwrap().unwrap().blob,
            body_blob
        );
        score.joins[0].duration_frames = 3;
        let third = compile(&score, &scion(hook2.as_str()), &store, &ff, &ff).unwrap();
        assert_eq!(
            third
                .plan
                .kerfs
                .iter()
                .find(|k| k.join == ["hook", "body"])
                .unwrap()
                .cache,
            "miss"
        );
        assert_eq!(
            third
                .plan
                .slots
                .iter()
                .find(|s| s.id == "body")
                .unwrap()
                .cache,
            "hit"
        );
    }

    #[test]
    fn dest_encoder_fingerprint_misses_every_slot_on_x264() {
        let Ok(ff) = FfmpegX264::from_env() else {
            eprintln!("skip dest_encoder_fingerprint_misses_every_slot_on_x264 — install ffmpeg");
            return;
        };
        let score = score();
        let mut dest = dest();
        dest.encoder.level = Some("3.1".into());
        dest.encoder.rate_control = Some(RateControl {
            mode: RateControlMode::Crf,
            crf: Some(20.0),
            bitrate: None,
        });
        let store = Memory::new();
        let hook = store
            .put_blob(Kind::Material, &color_mp4(&ff, "red", 1.2, 64))
            .unwrap();
        let body = store
            .put_blob(Kind::Material, &color_mp4(&ff, "green", 2.2, 64))
            .unwrap();
        let cta = store
            .put_blob(Kind::Material, &color_mp4(&ff, "yellow", 1.2, 64))
            .unwrap();
        let scion = |dest: Dest| {
            let mut bindings = BTreeMap::new();
            bindings.insert("hook".into(), bind(hook.as_str(), 0.0, 1.0));
            bindings.insert("body".into(), bind(body.as_str(), 0.0, 2.0));
            bindings.insert("cta".into(), bind(cta.as_str(), 0.0, 1.0));
            Scion {
                graft: GRAFT_SCHEMA.into(),
                id: "9x16".into(),
                concept: score.concept.clone(),
                parent: None,
                change_request: None,
                dest,
                layers: vec![BindingLayer {
                    name: Layer::Base,
                    bindings,
                }],
            }
        };
        let first = compile(&score, &scion(dest.clone()), &store, &ff, &ff).unwrap();
        dest.encoder.rate_control = Some(RateControl {
            mode: RateControlMode::Bitrate,
            crf: None,
            bitrate: Some("800k".into()),
        });
        dest.encoder.level = Some("4.0".into());
        // Encoder fingerprint is on dest, so every slot_encode misses.
        let second = compile(&score, &scion(dest), &store, &ff, &ff).unwrap();
        assert!(second.plan.slots.iter().all(|s| s.cache == "miss"));
        assert!(first.encoded && second.encoded);
    }

    #[test]
    fn vo_bed_captions_compile_keeps_body_hit_on_x264() {
        let Ok(ff) = FfmpegX264::from_env() else {
            eprintln!("skip vo_bed_captions_compile_keeps_body_hit_on_x264 — install ffmpeg");
            return;
        };
        let mut score = score();
        score.slots.push(Slot {
            id: "vo".into(),
            role: Role::Vo,
            range: FrameRange::new(0, 10),
            optional: false,
            window: Some(Window {
                kind: "vo_hold".into(),
                range: FrameRange::new(0, 10),
            }),
        });
        score.slots.push(Slot {
            id: "bed".into(),
            role: Role::Bed,
            range: FrameRange::new(0, 40),
            optional: false,
            window: None,
        });
        score.slots.push(Slot {
            id: "captions".into(),
            role: Role::Captions,
            range: FrameRange::new(0, 40),
            optional: false,
            window: None,
        });
        score.slots.push(Slot {
            id: "brand".into(),
            role: Role::Brand,
            range: FrameRange::new(0, 40),
            optional: false,
            window: None,
        });
        let dest = dest();
        let store = Memory::new();
        let hook = store
            .put_blob(Kind::Material, &color_mp4(&ff, "red", 1.2, 64))
            .unwrap();
        let hook2 = store
            .put_blob(Kind::Material, &color_mp4(&ff, "blue", 1.2, 64))
            .unwrap();
        let body = store
            .put_blob(Kind::Material, &color_mp4(&ff, "green", 2.2, 64))
            .unwrap();
        let cta = store
            .put_blob(Kind::Material, &color_mp4(&ff, "yellow", 1.2, 64))
            .unwrap();
        let vo = store.put_blob(Kind::Material, &sine_m4a(&ff, 1.2)).unwrap();
        let bed = store.put_blob(Kind::Material, &sine_m4a(&ff, 4.2)).unwrap();
        let captions = store.put_blob(Kind::Material, b"WEBVTT\n\nHi\n").unwrap();
        let captions2 = store.put_blob(Kind::Material, b"WEBVTT\n\nBye\n").unwrap();
        let brand = store
            .put_blob(Kind::Material, &color_mp4(&ff, "white", 1.2, 64))
            .unwrap();
        let scion = |hook_id: &str, cap: &str| {
            let mut bindings = BTreeMap::new();
            bindings.insert("hook".into(), bind(hook_id, 0.0, 1.0));
            bindings.insert("body".into(), bind(body.as_str(), 0.0, 2.0));
            bindings.insert("cta".into(), bind(cta.as_str(), 0.0, 1.0));
            bindings.insert("vo".into(), bind(vo.as_str(), 0.0, 1.0));
            bindings.insert("bed".into(), bind(bed.as_str(), 0.0, 4.0));
            bindings.insert("captions".into(), bind(cap, 0.0, 4.0));
            bindings.insert("brand".into(), bind(brand.as_str(), 0.0, 4.0));
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
        let first = compile(
            &score,
            &scion(hook.as_str(), captions.as_str()),
            &store,
            &ff,
            &ff,
        )
        .unwrap();
        assert!(first.encoded);
        assert_eq!(first.captions.len(), 1);
        assert!(first.graph.audio_mix.is_some());
        assert!(first.graph.overlay_mix.is_some());
        let dest_bytes = store
            .get_blob(Kind::Concat, first.concat.as_ref().unwrap())
            .unwrap();
        assert!(probe_media(&dest_bytes).unwrap().has_audio);
        let body_key = first
            .graph
            .slots
            .iter()
            .find(|s| s.slot.id == "body")
            .unwrap()
            .key
            .clone();
        let body_blob = store.get_action(&body_key).unwrap().unwrap().blob;
        let vo_key = first
            .graph
            .overlay_audio
            .iter()
            .find(|a| a.slot.id == "vo")
            .unwrap()
            .key
            .clone();
        let vo_blob = store.get_action(&vo_key).unwrap().unwrap().blob;
        let bed_key = first
            .graph
            .overlay_audio
            .iter()
            .find(|a| a.slot.id == "bed")
            .unwrap()
            .key
            .clone();
        let bed_blob = store.get_action(&bed_key).unwrap().unwrap().blob;
        let cap_blob = store
            .get_action(&first.graph.captions[0].key)
            .unwrap()
            .unwrap()
            .blob;
        let second = compile(
            &score,
            &scion(hook2.as_str(), captions.as_str()),
            &store,
            &ff,
            &ff,
        )
        .unwrap();
        assert_eq!(
            store.get_action(&body_key).unwrap().unwrap().blob,
            body_blob
        );
        assert_eq!(store.get_action(&vo_key).unwrap().unwrap().blob, vo_blob);
        assert_eq!(store.get_action(&bed_key).unwrap().unwrap().blob, bed_blob);
        assert_eq!(
            store
                .get_action(&second.graph.captions[0].key)
                .unwrap()
                .unwrap()
                .blob,
            cap_blob
        );
        assert_eq!(
            second
                .plan
                .slots
                .iter()
                .find(|s| s.id == "body")
                .unwrap()
                .cache,
            "hit"
        );
        let third = compile(
            &score,
            &scion(hook2.as_str(), captions2.as_str()),
            &store,
            &ff,
            &ff,
        )
        .unwrap();
        assert_eq!(
            third
                .plan
                .slots
                .iter()
                .find(|s| s.id == "body")
                .unwrap()
                .cache,
            "hit"
        );
        assert_eq!(store.get_action(&vo_key).unwrap().unwrap().blob, vo_blob);
        assert_eq!(store.get_action(&bed_key).unwrap().unwrap().blob, bed_blob);
        assert_eq!(
            third
                .plan
                .captions
                .iter()
                .find(|s| s.id == "captions")
                .unwrap()
                .cache,
            "miss"
        );
    }
}
