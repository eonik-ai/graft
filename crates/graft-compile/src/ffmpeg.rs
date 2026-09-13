// SPDX-License-Identifier: Apache-2.0

//! Long-GOP encode via a **system** ffmpeg/x264. graft does not link libx264
//! (GPL). Set `FFMPEG` / `FFPROBE` or put both on `PATH`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use graft_score::{Dest, Encoder, RateControlMode};

use crate::concat::{ConcatBackend, ConcatError, ConcatRequest};
use crate::encode::{EncodeBackend, EncodeError, KerfEncodeRequest, SlotEncodeRequest};
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
            "stream=width,height,avg_frame_rate,codec_name:format=duration",
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
        "keyint={}:min-keyint={}:scenecut=0:threads=1:sliced-threads=0:sync-lookahead=0",
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
            dest.width, dest.height, dest.fps
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
            format!("{:.3}", req.action.binding.in_s),
            "-to".into(),
            format!("{:.3}", req.action.binding.out_s),
            "-i".into(),
            input.to_string_lossy().into_owned(),
        ];
        args.extend(x264_args(&req.dest.encoder, req.dest)?);
        args.push(output.to_string_lossy().into_owned());
        let str_args: Vec<&str> = args.iter().map(String::as_str).collect();
        run_ok(&self.ffmpeg, &str_args).map_err(EncodeError::Invalid)?;
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
        for part in req.parts {
            if part.empty || part.bytes.is_empty() {
                continue;
            }
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
        fs::read(&out).map_err(|e| ConcatError::Invalid(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::compile;
    use graft_cas::{Kind, Memory, Store};
    use graft_score::{
        Binding, Clock, Dest, Encoder, Layer, Role, Scion, Score, Slot, Window, GRAFT_SCHEMA,
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
                fps: 10.0,
                duration_s: 4.0,
            },
            slots: vec![
                Slot {
                    id: "hook".into(),
                    role: Role::Hook,
                    span: [0.0, 1.0],
                    optional: false,
                    window: Some(Window {
                        kind: "hook_rate".into(),
                        span: [0.0, 1.0],
                    }),
                },
                Slot {
                    id: "body".into(),
                    role: Role::Body,
                    span: [1.0, 3.0],
                    optional: false,
                    window: None,
                },
                Slot {
                    id: "cta".into(),
                    role: Role::Cta,
                    span: [3.0, 4.0],
                    optional: false,
                    window: None,
                },
            ],
            layers: vec![Layer::Base],
            dest_default: Some("9x16".into()),
            spill_threshold_s: 0.35,
        }
    }

    fn dest() -> Dest {
        let mut enc = Encoder::default_x264();
        enc.keyint = 10;
        Dest {
            id: "9x16".into(),
            width: 64,
            height: 64,
            fps: 10.0,
            pix_fmt: "yuv420p".into(),
            color: "bt709".into(),
            encoder: enc,
        }
    }

    fn bind(material: &str, in_s: f64, out_s: f64) -> Binding {
        Binding {
            material: material.into(),
            in_s,
            out_s,
            params: None,
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
                dest: dest.clone(),
                bindings,
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
