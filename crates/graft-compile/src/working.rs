// SPDX-License-Identifier: Apache-2.0

//! Working-depth ffmpeg nodes and a single delivery encode.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use graft_score::{xfade_filter_name, Binding, Dest, GraphNode, Marker};
use serde_json::Value;

use crate::encode::EncodeError;
use crate::ffmpeg::{ffmpeg_bin, probe_media};

fn run_ok(bin: &Path, args: &[&str]) -> Result<Vec<u8>, String> {
    let out = Command::new(bin)
        .args(args)
        .output()
        .map_err(|e| format!("{}: {e}", bin.display()))?;
    if !out.status.success() {
        return Err(format!(
            "{} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    Ok(out.stdout)
}

fn write_tmp(dir: &Path, name: &str, bytes: &[u8]) -> Result<PathBuf, EncodeError> {
    let path = dir.join(name);
    fs::write(&path, bytes).map_err(|e| EncodeError::Invalid(e.to_string()))?;
    Ok(path)
}

fn read_out(path: &Path) -> Result<Vec<u8>, EncodeError> {
    fs::read(path).map_err(|e| EncodeError::Invalid(e.to_string()))
}

pub fn working_vf(dest: &Dest, speed: f64) -> Result<String, EncodeError> {
    if speed <= 0.0 || !speed.is_finite() {
        return Err(EncodeError::Invalid("speed must be finite and > 0".into()));
    }
    let pix = dest.working_pix_fmt();
    let color = dest.working_color();
    let fps = dest.rate.as_f64();
    let mut chain = Vec::new();
    if (speed - 1.0).abs() > 1e-9 {
        chain.push(format!("setpts=PTS/{speed:.6}"));
    }
    chain.push(format!(
        "scale={}:{}:flags=bicubic",
        dest.width, dest.height
    ));
    chain.push(format!("fps={fps}"));
    chain.push(format!("format={pix}"));
    chain.push(format!(
        "setparams=range=tv:colorspace={color}:color_primaries={color}:color_trc={color}"
    ));
    Ok(chain.join(","))
}

/// Intra working encode. ProRes HQ (422) or 4444 — not delivery H.264.
fn working_video_encode(dest: &Dest) -> Vec<String> {
    let pix = dest.working_pix_fmt();
    let profile = if pix.contains("444") { "4" } else { "3" };
    vec![
        "-c:v".into(),
        "prores_ks".into(),
        "-profile:v".into(),
        profile.into(),
        "-pix_fmt".into(),
        pix,
    ]
}

/// Constant speed, or a stepwise remap folded as working pads.
pub fn encode_working_binding(
    dest: &Dest,
    material: &[u8],
    binding: &Binding,
    dest_duration_s: f64,
) -> Result<Vec<u8>, EncodeError> {
    if !binding.remap_is_curve() {
        return encode_working_video(
            dest,
            material,
            binding.in_s(),
            binding.out_s(),
            binding.speed(),
        );
    }
    let spans = binding.remap_spans(dest_duration_s);
    if spans.is_empty() {
        return Err(EncodeError::Invalid("remap produced no spans".into()));
    }
    let mut parts = Vec::with_capacity(spans.len());
    for span in spans {
        parts.push(encode_working_video(
            dest,
            material,
            span.src_in,
            span.src_out,
            span.speed,
        )?);
    }
    let refs: Vec<&[u8]> = parts.iter().map(Vec::as_slice).collect();
    working_concat_video(dest, &refs)
}

pub fn encode_working_video(
    dest: &Dest,
    material: &[u8],
    in_s: f64,
    out_s: f64,
    speed: f64,
) -> Result<Vec<u8>, EncodeError> {
    let ffmpeg = ffmpeg_bin();
    let dir = tempfile::tempdir().map_err(|e| EncodeError::Invalid(e.to_string()))?;
    let still = is_still_image(material);
    let input = write_tmp(dir.path(), still_input_name(material), material)?;
    let output = dir.path().join("w.mov");
    let vf = working_vf(dest, speed)?;
    let output_s = output.to_string_lossy().into_owned();
    let start = format!("{in_s:.6}");
    let end = format!("{out_s:.6}");
    let mut args = vec![
        "-hide_banner".into(),
        "-loglevel".into(),
        "error".into(),
        "-y".into(),
    ];
    if still {
        args.extend(["-loop".into(), "1".into()]);
    } else {
        args.extend(["-ss".into(), start, "-to".into(), end]);
    }
    args.extend(["-i".into(), input.to_string_lossy().into_owned()]);
    if still {
        let duration = (out_s - in_s).max(1.0 / 30.0);
        args.extend(["-t".into(), format!("{duration:.6}")]);
    }
    args.extend(["-an".into(), "-vf".into(), vf]);
    args.extend(working_video_encode(dest));
    args.push(output_s);
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    run_ok(&ffmpeg, &refs).map_err(EncodeError::Invalid)?;
    read_out(&output)
}

pub fn encode_working_audio(
    material: &[u8],
    in_s: f64,
    out_s: f64,
    speed: f64,
) -> Result<Vec<u8>, EncodeError> {
    let ffmpeg = ffmpeg_bin();
    let dir = tempfile::tempdir().map_err(|e| EncodeError::Invalid(e.to_string()))?;
    let input = write_tmp(dir.path(), "in.bin", material)?;
    let output = dir.path().join("a.wav");
    let mut af = vec![
        format!("atrim=start={in_s:.6}:end={out_s:.6}"),
        "asetpts=PTS-STARTPTS".into(),
        "aformat=sample_fmts=s32:sample_rates=48000:channel_layouts=stereo".into(),
    ];
    if (speed - 1.0).abs() > 1e-9 {
        af.insert(2, atempo_chain(speed)?);
    }
    let out_s = output.to_string_lossy().into_owned();
    run_ok(
        &ffmpeg,
        &[
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-i",
            input.to_str().unwrap(),
            "-vn",
            "-af",
            &af.join(","),
            "-c:a",
            "pcm_s24le",
            &out_s,
        ],
    )
    .map_err(EncodeError::Invalid)?;
    read_out(&output)
}

fn is_still_image(bytes: &[u8]) -> bool {
    still_input_name(bytes) != "in.bin"
}

fn still_input_name(bytes: &[u8]) -> &'static str {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        "in.png"
    } else if bytes.len() >= 3 && bytes[0] == 0xFF && bytes[1] == 0xD8 && bytes[2] == 0xFF {
        "in.jpg"
    } else if bytes.starts_with(b"GIF8") {
        "in.gif"
    } else if bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
        "in.webp"
    } else {
        "in.bin"
    }
}

fn atempo_chain(speed: f64) -> Result<String, EncodeError> {
    if speed <= 0.0 || !speed.is_finite() {
        return Err(EncodeError::Invalid("speed must be finite and > 0".into()));
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

pub fn working_concat_video(dest: &Dest, parts: &[&[u8]]) -> Result<Vec<u8>, EncodeError> {
    if parts.is_empty() {
        return Err(EncodeError::Invalid("concat has no parts".into()));
    }
    if parts.len() == 1 {
        return Ok(parts[0].to_vec());
    }
    let ffmpeg = ffmpeg_bin();
    let dir = tempfile::tempdir().map_err(|e| EncodeError::Invalid(e.to_string()))?;
    let mut args = vec![
        "-hide_banner".into(),
        "-loglevel".into(),
        "error".into(),
        "-y".into(),
    ];
    let mut labels = String::new();
    for (i, part) in parts.iter().enumerate() {
        let p = write_tmp(dir.path(), &format!("p{i}.mov"), part)?;
        args.push("-i".into());
        args.push(p.to_string_lossy().into_owned());
        labels.push_str(&format!("[{i}:v]"));
    }
    let out = dir.path().join("c.mov");
    args.extend([
        "-filter_complex".into(),
        format!(
            "{labels}concat=n={}:v=1:a=0,format={},setparams=range=tv:colorspace={}:color_primaries={}:color_trc={}[v]",
            parts.len(),
            dest.working_pix_fmt(),
            dest.working_color(),
            dest.working_color(),
            dest.working_color()
        ),
        "-map".into(),
        "[v]".into(),
        "-an".into(),
    ]);
    args.extend(working_video_encode(dest));
    args.push(out.to_string_lossy().into_owned());
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    run_ok(&ffmpeg, &refs).map_err(EncodeError::Invalid)?;
    read_out(&out)
}

pub fn working_xfade(
    dest: &Dest,
    left: &[u8],
    right: &[u8],
    transition: &str,
    duration_s: f64,
    offset_s: f64,
) -> Result<Vec<u8>, EncodeError> {
    let name = xfade_filter_name(transition).map_err(|e| EncodeError::Invalid(e.to_string()))?;
    let ffmpeg = ffmpeg_bin();
    let dir = tempfile::tempdir().map_err(|e| EncodeError::Invalid(e.to_string()))?;
    let l = write_tmp(dir.path(), "l.mov", left)?;
    let r = write_tmp(dir.path(), "r.mov", right)?;
    let out = dir.path().join("x.mov");
    let filter = format!(
        "[0:v][1:v]xfade=transition={name}:duration={duration_s:.6}:offset={offset_s:.6},format={},setparams=range=tv:colorspace={}:color_primaries={}:color_trc={}[v]",
        dest.working_pix_fmt(),
        dest.working_color(),
        dest.working_color(),
        dest.working_color()
    );
    let mut args = vec![
        "-hide_banner".into(),
        "-loglevel".into(),
        "error".into(),
        "-y".into(),
        "-i".into(),
        l.to_string_lossy().into_owned(),
        "-i".into(),
        r.to_string_lossy().into_owned(),
        "-filter_complex".into(),
        filter,
        "-map".into(),
        "[v]".into(),
        "-an".into(),
    ];
    args.extend(working_video_encode(dest));
    args.push(out.to_string_lossy().into_owned());
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    run_ok(&ffmpeg, &refs).map_err(EncodeError::Invalid)?;
    read_out(&out)
}

pub fn working_single(
    dest: &Dest,
    input: &[u8],
    extra: Option<&[u8]>,
    filter: &str,
) -> Result<Vec<u8>, EncodeError> {
    let ffmpeg = ffmpeg_bin();
    let dir = tempfile::tempdir().map_err(|e| EncodeError::Invalid(e.to_string()))?;
    let a = write_tmp(dir.path(), "a.mov", input)?;
    let out = dir.path().join("n.mov");
    let mut args = vec![
        "-hide_banner".into(),
        "-loglevel".into(),
        "error".into(),
        "-y".into(),
        "-i".into(),
        a.to_string_lossy().into_owned(),
    ];
    if let Some(extra) = extra {
        let b = write_tmp(dir.path(), "b.bin", extra)?;
        args.push("-i".into());
        args.push(b.to_string_lossy().into_owned());
    }
    let vf = if filter.contains("[v]") {
        format!(
            "{filter};[v]format={},setparams=range=tv:colorspace={}:color_primaries={}:color_trc={}[outv]",
            dest.working_pix_fmt(),
            dest.working_color(),
            dest.working_color(),
            dest.working_color()
        )
    } else {
        format!(
            "[0:v]{filter},format={},setparams=range=tv:colorspace={}:color_primaries={}:color_trc={}[outv]",
            dest.working_pix_fmt(),
            dest.working_color(),
            dest.working_color(),
            dest.working_color()
        )
    };
    args.extend([
        "-filter_complex".into(),
        vf,
        "-map".into(),
        "[outv]".into(),
        "-an".into(),
    ]);
    args.extend(working_video_encode(dest));
    args.push(out.to_string_lossy().into_owned());
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    run_ok(&ffmpeg, &refs).map_err(EncodeError::Invalid)?;
    read_out(&out)
}

fn quote_filter_expr(expr: &str) -> String {
    crate::filters::quote_filter_arg(expr)
}

fn geq_time(expr: &str) -> String {
    let bytes = expr.as_bytes();
    let mut out = String::with_capacity(expr.len());
    let mut i = 0;
    while i < bytes.len() {
        let is_t = bytes[i] == b't';
        let prev_ok = i == 0 || !(bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_');
        let next_ok =
            i + 1 == bytes.len() || !(bytes[i + 1].is_ascii_alphanumeric() || bytes[i + 1] == b'_');
        if is_t && prev_ok && next_ok {
            out.push('T');
        } else {
            out.push(bytes[i] as char);
        }
        i += 1;
    }
    out
}

/// Multi-cam: later angles fold over the first with a delayed overlay.
pub fn switch_filter(node: &GraphNode) -> Result<String, EncodeError> {
    let cuts = node
        .params
        .as_ref()
        .and_then(|value| value.get("cuts"))
        .and_then(|value| value.as_array())
        .ok_or_else(|| EncodeError::Invalid("switch needs params.cuts".into()))?;
    let mut parsed = Vec::new();
    for cut in cuts {
        let t = cut.get("t").and_then(|value| value.as_f64()).unwrap_or(0.0);
        let map = cut.get("map").and_then(|value| value.as_u64()).unwrap_or(0) as usize;
        if !t.is_finite() || t < 0.0 {
            return Err(EncodeError::Invalid("switch cut time is invalid".into()));
        }
        parsed.push((t, map));
    }
    if parsed.is_empty() {
        return Err(EncodeError::Invalid("switch needs params.cuts".into()));
    }
    parsed.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    let base = parsed[0].1;
    if parsed.len() == 1 {
        return Ok(format!("[{base}:v]null[v]"));
    }
    let mut filter = String::new();
    let mut prev = format!("[{base}:v]");
    for (i, (t, map)) in parsed.iter().enumerate().skip(1) {
        let dest_label = if i + 1 == parsed.len() {
            "[v]".to_string()
        } else {
            format!("[sw{i}]")
        };
        filter.push_str(&format!(
            "[{map}:v]setpts=PTS+{t:.6}/TB[d{i}];{prev}[d{i}]overlay=x=0:y=0:enable='gte(t,{t:.6})':eof_action=pass:format=auto{dest_label}"
        ));
        if i + 1 != parsed.len() {
            filter.push(';');
        }
        prev = dest_label;
    }
    Ok(filter)
}

pub fn node_filter(
    dest: &Dest,
    node: &GraphNode,
    lut_path: Option<&Path>,
) -> Result<String, EncodeError> {
    if crate::filters::should_generic(&node.op, node) {
        return crate::filters::render(dest.rate.as_f64(), dest.width, dest.height, node);
    }
    let enable = node
        .enable
        .as_deref()
        .map(|e| format!(":enable='{e}'"))
        .unwrap_or_default();
    let consumed = std::cell::RefCell::new(std::collections::BTreeSet::new());
    let p = |key: &str, default: &str| -> String {
        consumed.borrow_mut().insert(key.to_string());
        if let Some(keys) = node
            .keys
            .as_ref()
            .and_then(|map| map.get(key))
            .filter(|keys| !keys.is_empty())
        {
            return graft_score::piecewise_eased(keys, "t");
        }
        node.params
            .as_ref()
            .and_then(|v| v.get(key))
            .and_then(|v| match v {
                Value::String(s) => Some(s.clone()),
                Value::Number(n) => Some(n.to_string()),
                Value::Bool(b) => Some(b.to_string()),
                _ => None,
            })
            .unwrap_or_else(|| default.to_string())
    };
    let filter = match node.op.as_str() {
        "lut3d" => {
            let path = lut_path
                .and_then(|p| p.to_str())
                .ok_or_else(|| EncodeError::Invalid("lut3d needs a .cube material".into()))?;
            format!(
                "lut3d=file={path}:interp={}{enable}",
                p("interp", "tetrahedral")
            )
        }
        "lut1d" => {
            let path = lut_path
                .and_then(|p| p.to_str())
                .ok_or_else(|| EncodeError::Invalid("lut1d needs a LUT material".into()))?;
            format!("lut1d=file={path}:interp={}{enable}", p("interp", "linear"))
        }
        "eq" => format!(
            "eq=brightness={}:contrast={}:saturation={}:gamma={}{enable}",
            p("brightness", "0"),
            p("contrast", "1"),
            p("saturation", "1"),
            p("gamma", "1")
        ),
        "hue" => format!("hue=h={}:s={}{enable}", p("h", "0"), p("s", "1")),
        "curves" => format!("curves=preset={}{enable}", p("preset", "none")),
        "colorbalance" => format!(
            "colorbalance=rs={}:gs={}:bs={}:rm={}:gm={}:bm={}:rh={}:gh={}:bh={}{enable}",
            p("rs", "0"),
            p("gs", "0"),
            p("bs", "0"),
            p("rm", "0"),
            p("gm", "0"),
            p("bm", "0"),
            p("rh", "0"),
            p("gh", "0"),
            p("bh", "0")
        ),
        "colortemperature" => format!(
            "colortemperature=temperature={}{enable}",
            p("temperature", "6500")
        ),
        "vibrance" => format!("vibrance=intensity={}{enable}", p("intensity", "0.3")),
        "unsharp" => format!(
            "unsharp=luma_msize_x={}:luma_amount={}{enable}",
            p("lx", "5"),
            p("la", "1")
        ),
        "vignette" => format!("vignette=a={}{enable}", p("a", "PI/5")),
        "noise" => format!("noise=alls={}:allf=t{enable}", p("alls", "8")),
        "gblur" => format!("gblur=sigma={}{enable}", p("sigma", "0.6")),
        "setrange" => format!("setrange=range={}{enable}", p("range", "tv")),
        "colorspace" => {
            let all = p("all", &dest.working_color());
            let primaries = p("primaries", &dest.working_primaries());
            let trc = p("trc", &dest.working_trc());
            let mut filter = format!(
                "colorspace=all={all}:primaries={primaries}:trc={trc}:range=tv"
            );
            let iprimaries = p("iprimaries", "");
            if !iprimaries.is_empty() {
                filter.push_str(&format!(":iprimaries={iprimaries}"));
            }
            let itrc = p("itrc", "");
            if !itrc.is_empty() {
                filter.push_str(&format!(":itrc={itrc}"));
            }
            format!("{filter}{enable}")
        }
        "tonemap" => format!("tonemap={}{enable}", p("tonemap", "hable")),
        "scale" => format!("scale={}:{}:flags=bicubic{enable}", dest.width, dest.height),
        "crop" => format!(
            "crop={}:{}:{}:{}{enable}",
            p("w", &dest.width.to_string()),
            p("h", &dest.height.to_string()),
            p("x", "0"),
            p("y", "0")
        ),
        "pad" => format!(
            "pad={}:{}:{}:{}{enable}",
            dest.width,
            dest.height,
            p("x", "0"),
            p("y", "0")
        ),
        "rotate" => format!("rotate={}{enable}", p("angle", "0")),
        "flip" => match p("axis", "h").as_str() {
            "v" => format!("vflip{enable}"),
            _ => format!("hflip{enable}"),
        },
        "zoompan" => format!(
            "zoompan=z={}:d={}:s={}x{}{enable}",
            p("z", "1"),
            p("d", "1"),
            dest.width,
            dest.height
        ),
        "reverse" => "reverse".into(),
        "freeze" => format!("tpad=stop_mode=clone:stop_duration={}", p("duration", "1")),
        "fade" => format!(
            "fade=t={}:st={}:d={}{enable}",
            p("type", "in"),
            p("start", "0"),
            p("duration", "0.3")
        ),
        "overlay" => {
            let enable = if node.enable.is_some() {
                format!(":enable='{}'", node.enable.as_deref().unwrap())
            } else {
                String::new()
            };
            let opacity = p("opacity", "1");
            let x = quote_filter_expr(&p("x", "0"));
            let y = quote_filter_expr(&p("y", "0"));
            if opacity.contains("lt(t") || opacity.contains("lt(T") || opacity.contains('(') {
                let expr = geq_time(&opacity);
                format!(
                    "[1:v]format=rgba,geq=r='r(X,Y)':g='g(X,Y)':b='b(X,Y)':a='alpha(X,Y)*({expr})'[ov];[0:v][ov]overlay=x={x}:y={y}{enable}:format=auto[v]"
                )
            } else if opacity != "1" && opacity != "1.000000" {
                format!(
                    "[1:v]format=rgba,colorchannelmixer=aa={opacity}[ov];[0:v][ov]overlay=x={x}:y={y}{enable}:format=auto[v]"
                )
            } else {
                format!("[0:v][1:v]overlay=x={x}:y={y}{enable}:format=auto[v]")
            }
        }
        "blend" => {
            let mode = graft_score::canonical_blend(&p("mode", "normal"))
                .map_err(|e| EncodeError::Invalid(e.to_string()))?;
            format!("[0:v][1:v]blend=all_mode={mode}[v]")
        }
        "chromakey" => format!(
            "chromakey=color={}:similarity={}:blend={}{enable}",
            p("color", "0x00ff00"),
            p("similarity", "0.2"),
            p("blend", "0.0")
        ),
        "colorkey" => format!(
            "colorkey=color={}:similarity={}:blend={}{enable}",
            p("color", "0x00ff00"),
            p("similarity", "0.2"),
            p("blend", "0.0")
        ),
        "lumakey" => format!(
            "lumakey=threshold={}:tolerance={}{enable}",
            p("threshold", "0"),
            p("tolerance", "0.1")
        ),
        "chromahold" => format!(
            "chromahold=color={}:similarity={}{enable}",
            p("color", "0x00ff00"),
            p("similarity", "0.3")
        ),
        "alphamerge" => "[0:v][1:v]alphamerge[v]".into(),
        "drawbox" => format!(
            "drawbox=x={}:y={}:w={}:h={}:color={}:t={}{enable}",
            p("x", "0"),
            p("y", "0"),
            p("w", "32"),
            p("h", "16"),
            p("color", "white"),
            p("t", "2")
        ),
        "drawgrid" => format!(
            "drawgrid=w={}:h={}:t={}:color={}{enable}",
            p("w", "16"),
            p("h", "16"),
            p("t", "1"),
            p("color", "white@0.4")
        ),
        "colorchannelmixer" => format!(
            "colorchannelmixer=rr={}:gg={}:bb={}:aa={}{enable}",
            p("rr", "1"),
            p("gg", "1"),
            p("bb", "1"),
            p("aa", "1")
        ),
        "setsar" => format!("setsar=sar={}{enable}", p("sar", "1")),
        "setdar" => format!("setdar=dar={}{enable}", p("dar", "9/16")),
        "perspective" => format!(
            "perspective=x0={}:y0={}:x1={}:y1={}:x2={}:y2={}:x3={}:y3={}{enable}",
            p("x0", "0"),
            p("y0", "0"),
            p("x1", &dest.width.to_string()),
            p("y1", "0"),
            p("x2", "0"),
            p("y2", &dest.height.to_string()),
            p("x3", &dest.width.to_string()),
            p("y3", &dest.height.to_string())
        ),
        "shear" => format!("shear=shx={}:shy={}{enable}", p("shx", "0"), p("shy", "0")),
        "scroll" => format!(
            "scroll=horizontal={}:vertical={}{enable}",
            p("horizontal", "0"),
            p("vertical", "0")
        ),
        "premultiply" => format!("premultiply=inplace={}{enable}", p("inplace", "1")),
        "unpremultiply" => format!("unpremultiply=inplace={}{enable}", p("inplace", "1")),
        "streamselect" => format!("streamselect=map={}{enable}", p("map", "0")),
        "switch" => switch_filter(node)?,
        "geq" => {
            let lum = p("lum", "");
            if !lum.is_empty() {
                format!("geq=lum='{lum}'{enable}")
            } else {
                format!(
                    "geq=r='{}':g='{}':b='{}':a='{}'{enable}",
                    p("r", "r(X,Y)"),
                    p("g", "g(X,Y)"),
                    p("b", "b(X,Y)"),
                    p("a", "alpha(X,Y)")
                )
            }
        }
        "selectivecolor" => {
            let mut filter = format!(
                "selectivecolor=correction_method={}",
                p("correction_method", "absolute")
            );
            for key in [
                "reds",
                "yellows",
                "greens",
                "cyans",
                "blues",
                "magentas",
                "whites",
                "neutrals",
                "blacks",
            ] {
                let value = p(key, "");
                if !value.is_empty() {
                    filter.push_str(&format!(":{key}={value}"));
                }
            }
            format!("{filter}{enable}")
        }
        "exposure" => format!(
            "exposure=exposure={}:black={}{enable}",
            p("exposure", "0"),
            p("black", "0")
        ),
        "colorhold" => format!(
            "colorhold=color={}:similarity={}:blend={}{enable}",
            p("color", "black"),
            p("similarity", "0.01"),
            p("blend", "0")
        ),
        "colorize" => format!(
            "colorize=hue={}:saturation={}:lightness={}:mix={}{enable}",
            p("hue", "0"),
            p("saturation", "0.5"),
            p("lightness", "0.5"),
            p("mix", "1")
        ),
        "deband" => format!(
            "deband=1thr={}:2thr={}:3thr={}:range={}:blur={}{enable}",
            p("1thr", "0.02"),
            p("2thr", "0.02"),
            p("3thr", "0.02"),
            p("range", "16"),
            p("blur", "1")
        ),
        "maskedmerge" => "[0:v][1:v][2:v]maskedmerge[v]".into(),
        "hstack" => format!("hstack=inputs={}{enable}", p("inputs", "2")),
        "vstack" => format!("vstack=inputs={}{enable}", p("inputs", "2")),
        "xstack" => format!(
            "xstack=inputs={}:layout={}{enable}",
            p("inputs", "2"),
            p("layout", "0_0|w0_0")
        ),
        "mix" => format!(
            "mix=inputs={}:weights={}{enable}",
            p("inputs", "2"),
            p("weights", "1 1")
        ),
        "lenscorrection" => format!(
            "lenscorrection=k1={}:k2={}{enable}",
            p("k1", "0"),
            p("k2", "0")
        ),
        "yadif" => format!(
            "yadif=mode={}:parity={}{enable}",
            p("mode", "0"),
            p("parity", "-1")
        ),
        "bwdif" => format!(
            "bwdif=mode={}:parity={}:deint={}{enable}",
            p("mode", "1"),
            p("parity", "-1"),
            p("deint", "all")
        ),
        "tile" => format!("tile=layout={}{enable}", p("layout", "1x1")),
        "haldclut" => {
            let path = lut_path
                .and_then(|p| p.to_str())
                .ok_or_else(|| EncodeError::Invalid("haldclut needs a CLUT material".into()))?;
            format!("haldclut=file={path}{enable}")
        }
        "colorcorrect" => format!(
            "colorcorrect=rl={}:bl={}:rh={}:bh={}:saturation={}{enable}",
            p("rl", "0"),
            p("bl", "0"),
            p("rh", "0"),
            p("bh", "0"),
            p("saturation", "1")
        ),
        "cdl" => format!(
            "colorchannelmixer=rr={}:gg={}:bb={}{enable},eq=gamma={}:brightness={}:saturation={}{enable}",
            p("slope", "1"),
            p("slope", "1"),
            p("slope", "1"),
            p("power", "1"),
            p("offset", "0"),
            p("sat", "1")
        ),
        "lgg" => format!(
            "colorcorrect=rl={}:bl={}:rh={}:bh={}:saturation={}{enable},eq=gamma={}{enable}",
            p("lift", "0"),
            p("lift", "0"),
            p("gain", "1"),
            p("gain", "1"),
            p("sat", "1"),
            p("gamma", "1")
        ),
        _other => {
            return crate::filters::render(dest.rate.as_f64(), dest.width, dest.height, node);
        }
    };
    if crate::filters::is_structural(&node.op) {
        Ok(filter)
    } else {
        crate::filters::append_declared(node, &filter, &consumed.into_inner())
    }
}

pub fn apply_video_node(
    dest: &Dest,
    node: &GraphNode,
    inputs: &[&[u8]],
    extra_material: Option<&[u8]>,
) -> Result<Vec<u8>, EncodeError> {
    match node.op.as_str() {
        "overlay" | "blend" | "alphamerge" => {
            if inputs.len() < 2 {
                return Err(EncodeError::Invalid(format!("{} needs two pads", node.op)));
            }
            let filter = node_filter(dest, node, None)?;
            n_input_video(dest, inputs, &filter)
        }
        "hstack" | "vstack" | "xstack" | "mix" => {
            if inputs.len() < 2 {
                return Err(EncodeError::Invalid(format!("{} needs two pads", node.op)));
            }
            let n = inputs.len();
            let p = |key: &str, default: &str| -> String {
                node.params
                    .as_ref()
                    .and_then(|v| v.get(key))
                    .and_then(|v| {
                        v.as_str()
                            .map(str::to_string)
                            .or_else(|| v.as_u64().map(|n| n.to_string()))
                    })
                    .unwrap_or_else(|| default.to_string())
            };
            let filter = match node.op.as_str() {
                "vstack" => format!("vstack=inputs={n}"),
                "xstack" => format!("xstack=inputs={n}:layout={}", p("layout", "0_0|w0_0")),
                "mix" => format!("mix=inputs={n}:weights={}", p("weights", "1 1")),
                _ => format!("hstack=inputs={n}"),
            };
            n_input_video(dest, inputs, &filter)
        }
        "maskedmerge" => {
            if inputs.len() < 3 {
                return Err(EncodeError::Invalid("maskedmerge needs three pads".into()));
            }
            let filter = node_filter(dest, node, None)?;
            n_input_video(dest, inputs, &filter)
        }
        "switch" => {
            if inputs.len() < 2 {
                return Err(EncodeError::Invalid("switch needs two pads".into()));
            }
            let filter = node_filter(dest, node, None)?;
            n_input_video(dest, inputs, &filter)
        }
        "haldclut" => {
            let clut = extra_material
                .ok_or_else(|| EncodeError::Invalid("haldclut material missing".into()))?;
            apply_haldclut(dest, inputs[0], clut, node.enable.as_deref())
        }
        "lut3d" | "lut1d" => {
            let dir = tempfile::tempdir().map_err(|e| EncodeError::Invalid(e.to_string()))?;
            let lut = extra_material
                .ok_or_else(|| EncodeError::Invalid("LUT material missing".into()))?;
            let lut_path = write_tmp(dir.path(), "look.cube", lut)?;
            let filter = node_filter(dest, node, Some(&lut_path))?;
            working_single(dest, inputs[0], None, &filter)
        }
        _ => {
            if inputs.is_empty() {
                return lavfi_video(dest, node);
            }
            let filter = node_filter(dest, node, None)?;
            if inputs.len() >= 2 {
                n_input_video(dest, inputs, &filter)
            } else {
                working_single(dest, inputs[0], extra_material, &filter)
            }
        }
    }
}

fn apply_haldclut(
    dest: &Dest,
    picture: &[u8],
    clut: &[u8],
    enable: Option<&str>,
) -> Result<Vec<u8>, EncodeError> {
    let ffmpeg = ffmpeg_bin();
    let dir = tempfile::tempdir().map_err(|e| EncodeError::Invalid(e.to_string()))?;
    let pic = write_tmp(dir.path(), "p.mov", picture)?;
    let png = clut.starts_with(b"\x89PNG");
    let clut_path = write_tmp(dir.path(), if png { "clut.png" } else { "clut.mov" }, clut)?;
    let enable = enable.map(|e| format!(":enable='{e}'")).unwrap_or_default();
    let fc = format!(
        "[0:v][1:v]haldclut{enable},format={},setparams=range=tv:colorspace={}:color_primaries={}:color_trc={}[outv]",
        dest.working_pix_fmt(),
        dest.working_color(),
        dest.working_color(),
        dest.working_color()
    );
    let out = dir.path().join("o.mov");
    let mut args = vec![
        "-hide_banner".into(),
        "-loglevel".into(),
        "error".into(),
        "-y".into(),
        "-i".into(),
        pic.to_string_lossy().into_owned(),
    ];
    if png {
        args.extend([
            "-loop".into(),
            "1".into(),
            "-i".into(),
            clut_path.to_string_lossy().into_owned(),
        ]);
    } else {
        args.extend(["-i".into(), clut_path.to_string_lossy().into_owned()]);
    }
    args.extend([
        "-filter_complex".into(),
        fc,
        "-map".into(),
        "[outv]".into(),
        "-an".into(),
    ]);
    if png {
        args.push("-shortest".into());
    }
    args.extend(working_video_encode(dest));
    args.push(out.to_string_lossy().into_owned());
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    run_ok(&ffmpeg, &refs).map_err(EncodeError::Invalid)?;
    read_out(&out)
}

fn n_input_video(dest: &Dest, inputs: &[&[u8]], filter: &str) -> Result<Vec<u8>, EncodeError> {
    let ffmpeg = ffmpeg_bin();
    let dir = tempfile::tempdir().map_err(|e| EncodeError::Invalid(e.to_string()))?;
    let mut paths = Vec::new();
    for (i, bytes) in inputs.iter().enumerate() {
        paths.push(write_tmp(dir.path(), &format!("p{i}.mov"), bytes)?);
    }
    let out = dir.path().join("o.mov");
    let labels: String = (0..inputs.len()).map(|i| format!("[{i}:v]")).collect();
    let fc = if filter.contains("[v]") {
        format!(
            "{filter};[v]format={},setparams=range=tv:colorspace={}:color_primaries={}:color_trc={}[outv]",
            dest.working_pix_fmt(),
            dest.working_color(),
            dest.working_color(),
            dest.working_color()
        )
    } else {
        format!(
            "{labels}{filter},format={},setparams=range=tv:colorspace={}:color_primaries={}:color_trc={}[outv]",
            dest.working_pix_fmt(),
            dest.working_color(),
            dest.working_color(),
            dest.working_color()
        )
    };
    let mut args = vec![
        "-hide_banner".into(),
        "-loglevel".into(),
        "error".into(),
        "-y".into(),
    ];
    for path in &paths {
        args.push("-i".into());
        args.push(path.to_string_lossy().into_owned());
    }
    args.extend([
        "-filter_complex".into(),
        fc,
        "-map".into(),
        "[outv]".into(),
        "-an".into(),
    ]);
    args.extend(working_video_encode(dest));
    args.push(out.to_string_lossy().into_owned());
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    run_ok(&ffmpeg, &refs).map_err(EncodeError::Invalid)?;
    read_out(&out)
}

pub fn mix_pcm(parts: &[(Vec<u8>, f64, f64)]) -> Result<Vec<u8>, EncodeError> {
    if parts.is_empty() {
        return Err(EncodeError::Invalid("audio mix has no parts".into()));
    }
    let ffmpeg = ffmpeg_bin();
    let dir = tempfile::tempdir().map_err(|e| EncodeError::Invalid(e.to_string()))?;
    let mix_dur = parts
        .iter()
        .map(|(_, start, dur)| start + dur)
        .fold(0.0, f64::max);
    let mut args = vec![
        "-hide_banner".into(),
        "-loglevel".into(),
        "error".into(),
        "-y".into(),
    ];
    let mut filters = Vec::new();
    for (i, (bytes, start, _)) in parts.iter().enumerate() {
        let p = write_tmp(dir.path(), &format!("a{i}.wav"), bytes)?;
        args.push("-i".into());
        args.push(p.to_string_lossy().into_owned());
        let delay_ms = (*start * 1000.0).round().max(0.0) as i64;
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
    let out = dir.path().join("mix.wav");
    args.extend([
        "-filter_complex".into(),
        filters.join(";"),
        "-map".into(),
        "[outa]".into(),
        "-c:a".into(),
        "pcm_s24le".into(),
        "-ar".into(),
        "48000".into(),
        "-ac".into(),
        "2".into(),
        out.to_string_lossy().into_owned(),
    ]);
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    run_ok(&ffmpeg, &refs).map_err(EncodeError::Invalid)?;
    read_out(&out)
}

pub fn apply_audio_node(
    dest: &Dest,
    node: &GraphNode,
    inputs: &[&[u8]],
    extra: Option<&[u8]>,
) -> Result<Vec<u8>, EncodeError> {
    if crate::filters::should_generic(&node.op, node) {
        let filter = crate::filters::render(dest.rate.as_f64(), dest.width, dest.height, node)?;
        return if inputs.len() >= 2 {
            n_input_audio(inputs, &filter)
        } else {
            pcm_af(inputs, &filter)
        };
    }
    let ffmpeg = ffmpeg_bin();
    let dir = tempfile::tempdir().map_err(|e| EncodeError::Invalid(e.to_string()))?;
    let consumed = std::cell::RefCell::new(std::collections::BTreeSet::new());
    let p = |key: &str, default: &str| -> String {
        consumed.borrow_mut().insert(key.to_string());
        if let Some(keys) = node
            .keys
            .as_ref()
            .and_then(|map| map.get(key))
            .filter(|keys| !keys.is_empty())
        {
            return graft_score::piecewise_eased(keys, "t");
        }
        node.params
            .as_ref()
            .and_then(|v| v.get(key))
            .map(|v| match v {
                Value::String(s) => s.clone(),
                other => other.to_string(),
            })
            .unwrap_or_else(|| default.to_string())
    };
    let declared = |filter: String| -> Result<String, EncodeError> {
        crate::filters::append_declared(node, &filter, &consumed.borrow())
    };
    match node.op.as_str() {
        "volume" => {
            let filter = declared(format!("volume={}", p("volume", "1")))?;
            pcm_af(inputs, &filter)
        }
        "loudnorm" => {
            let filter = declared(format!(
                "loudnorm=I={}:LRA={}:TP={}",
                p("I", "-14"),
                p("LRA", "7"),
                p("TP", "-1.5")
            ))?;
            pcm_af(inputs, &filter)
        }
        "afade" => {
            let filter = declared(format!(
                "afade=t={}:st={}:d={}:curve={}",
                p("type", "in"),
                p("start", "0"),
                p("duration", "0.3"),
                p("curve", "tri")
            ))?;
            let a = write_tmp(dir.path(), "a.wav", inputs[0])?;
            let out = dir.path().join("o.wav");
            run_ok(
                &ffmpeg,
                &[
                    "-hide_banner",
                    "-loglevel",
                    "error",
                    "-y",
                    "-i",
                    a.to_str().unwrap(),
                    "-af",
                    &filter,
                    "-c:a",
                    "pcm_s24le",
                    out.to_str().unwrap(),
                ],
            )
            .map_err(EncodeError::Invalid)?;
            read_out(&out)
        }
        "acrossfade" => {
            if inputs.len() < 2 {
                return Err(EncodeError::Invalid("acrossfade needs two pads".into()));
            }
            let a = write_tmp(dir.path(), "a.wav", inputs[0])?;
            let b = write_tmp(dir.path(), "b.wav", inputs[1])?;
            let out = dir.path().join("o.wav");
            run_ok(
                &ffmpeg,
                &[
                    "-hide_banner",
                    "-loglevel",
                    "error",
                    "-y",
                    "-i",
                    a.to_str().unwrap(),
                    "-i",
                    b.to_str().unwrap(),
                    "-filter_complex",
                    &format!(
                        "[0:a][1:a]acrossfade=d={}:c1={}:c2={}",
                        p("duration", "0.2"),
                        p("curve1", "tri"),
                        p("curve2", "tri")
                    ),
                    "-c:a",
                    "pcm_s24le",
                    out.to_str().unwrap(),
                ],
            )
            .map_err(EncodeError::Invalid)?;
            read_out(&out)
        }
        "amix" => {
            let owned: Vec<(Vec<u8>, f64, f64)> = inputs
                .iter()
                .map(|b| {
                    let dur = probe_media(b)
                        .ok()
                        .and_then(|p| p.audio_duration_s)
                        .unwrap_or(1.0);
                    ((*b).to_vec(), 0.0, dur)
                })
                .collect();
            mix_pcm(&owned)
        }
        "sidechain" => {
            if inputs.len() < 2 && extra.is_none() {
                return Err(EncodeError::Invalid("sidechain needs bed + key".into()));
            }
            let bed = write_tmp(dir.path(), "bed.wav", inputs[0])?;
            let key = write_tmp(
                dir.path(),
                "key.wav",
                inputs.get(1).copied().unwrap_or(inputs[0]),
            )?;
            let out = dir.path().join("o.wav");
            run_ok(
                &ffmpeg,
                &[
                    "-hide_banner",
                    "-loglevel",
                    "error",
                    "-y",
                    "-i",
                    bed.to_str().unwrap(),
                    "-i",
                    key.to_str().unwrap(),
                    "-filter_complex",
                    "sidechaincompress=threshold=0.05:ratio=8:attack=20:release=250",
                    "-c:a",
                    "pcm_s24le",
                    out.to_str().unwrap(),
                ],
            )
            .map_err(EncodeError::Invalid)?;
            read_out(&out)
        }
        "audio_source" => Err(EncodeError::Invalid(
            "audio_source is encoded as a source pad, not apply_audio_node".into(),
        )),
        "acompressor" => {
            let filter = declared(format!(
                "acompressor=threshold={}:ratio={}:attack={}:release={}",
                p("threshold", "0.1"),
                p("ratio", "4"),
                p("attack", "20"),
                p("release", "250")
            ))?;
            pcm_af(inputs, &filter)
        }
        "alimiter" => {
            let filter = declared(format!("alimiter=limit={}", p("limit", "1")))?;
            pcm_af(inputs, &filter)
        }
        "equalizer" => {
            let filter = declared(format!(
                "equalizer=f={}:t=q:w={}:g={}",
                p("f", "1000"),
                p("w", "1"),
                p("g", "0")
            ))?;
            pcm_af(inputs, &filter)
        }
        "highpass" => {
            let filter = declared(format!("highpass=f={}", p("f", "80")))?;
            pcm_af(inputs, &filter)
        }
        "lowpass" => {
            let filter = declared(format!("lowpass=f={}", p("f", "12000")))?;
            pcm_af(inputs, &filter)
        }
        "bass" => {
            let filter = declared(format!("bass=g={}", p("g", "0")))?;
            pcm_af(inputs, &filter)
        }
        "treble" => {
            let filter = declared(format!("treble=g={}", p("g", "0")))?;
            pcm_af(inputs, &filter)
        }
        "adelay" => {
            let filter = declared(format!("adelay={}|{}", p("delays", "0"), p("delays", "0")))?;
            pcm_af(inputs, &filter)
        }
        "aecho" => {
            let filter = declared(format!(
                "aecho={}:{}:{}:{}",
                p("in_gain", "0.8"),
                p("out_gain", "0.9"),
                p("delays", "60"),
                p("decays", "0.4")
            ))?;
            pcm_af(inputs, &filter)
        }
        "areverse" => {
            let filter = declared("areverse".into())?;
            pcm_af(inputs, &filter)
        }
        "pan" => {
            let filter = declared(format!("pan={}", p("layout", "stereo|c0=c0|c1=c1")))?;
            pcm_af(inputs, &filter)
        }
        _other => {
            let filter = crate::filters::render(dest.rate.as_f64(), dest.width, dest.height, node)?;
            if inputs.len() >= 2 {
                n_input_audio(inputs, &filter)
            } else {
                pcm_af(inputs, &filter)
            }
        }
    }
}

fn pcm_af(inputs: &[&[u8]], filter: &str) -> Result<Vec<u8>, EncodeError> {
    if inputs.is_empty() {
        return Err(EncodeError::Invalid("audio node needs a pad".into()));
    }
    let ffmpeg = ffmpeg_bin();
    let dir = tempfile::tempdir().map_err(|e| EncodeError::Invalid(e.to_string()))?;
    let a = write_tmp(dir.path(), "a.wav", inputs[0])?;
    let out = dir.path().join("o.wav");
    run_ok(
        &ffmpeg,
        &[
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-i",
            a.to_str().unwrap(),
            "-af",
            filter,
            "-c:a",
            "pcm_s24le",
            out.to_str().unwrap(),
        ],
    )
    .map_err(EncodeError::Invalid)?;
    read_out(&out)
}

fn n_input_audio(inputs: &[&[u8]], filter: &str) -> Result<Vec<u8>, EncodeError> {
    if inputs.len() < 2 {
        return Err(EncodeError::Invalid("audio filter needs two pads".into()));
    }
    let ffmpeg = ffmpeg_bin();
    let dir = tempfile::tempdir().map_err(|e| EncodeError::Invalid(e.to_string()))?;
    let mut paths = Vec::new();
    for (i, bytes) in inputs.iter().enumerate() {
        paths.push(write_tmp(dir.path(), &format!("a{i}.wav"), bytes)?);
    }
    let labels: String = (0..inputs.len()).map(|i| format!("[{i}:a]")).collect();
    let out = dir.path().join("o.wav");
    let mut args = vec![
        "-hide_banner".into(),
        "-loglevel".into(),
        "error".into(),
        "-y".into(),
    ];
    for path in &paths {
        args.push("-i".into());
        args.push(path.to_string_lossy().into_owned());
    }
    args.extend([
        "-filter_complex".into(),
        format!("{labels}{filter}[outa]"),
        "-map".into(),
        "[outa]".into(),
        "-c:a".into(),
        "pcm_s24le".into(),
        out.to_string_lossy().into_owned(),
    ]);
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    run_ok(&ffmpeg, &refs).map_err(EncodeError::Invalid)?;
    read_out(&out)
}

pub fn lavfi_video(dest: &Dest, node: &GraphNode) -> Result<Vec<u8>, EncodeError> {
    let spec = crate::filters::render(dest.rate.as_f64(), dest.width, dest.height, node)?;
    let ffmpeg = ffmpeg_bin();
    let dir = tempfile::tempdir().map_err(|e| EncodeError::Invalid(e.to_string()))?;
    let out = dir.path().join("o.mov");
    let vf = format!(
        "scale={}:{}:flags=bicubic,fps={},format={},setparams=range=tv:colorspace={}:color_primaries={}:color_trc={}",
        dest.width,
        dest.height,
        dest.rate.as_f64(),
        dest.working_pix_fmt(),
        dest.working_color(),
        dest.working_color(),
        dest.working_color()
    );
    let mut args = vec![
        "-hide_banner".into(),
        "-loglevel".into(),
        "error".into(),
        "-y".into(),
        "-f".into(),
        "lavfi".into(),
        "-i".into(),
        spec,
        "-vf".into(),
        vf,
        "-an".into(),
    ];
    args.extend(working_video_encode(dest));
    args.push(out.to_string_lossy().into_owned());
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    run_ok(&ffmpeg, &refs).map_err(EncodeError::Invalid)?;
    read_out(&out)
}

pub fn lavfi_audio(dest: &Dest, node: &GraphNode) -> Result<Vec<u8>, EncodeError> {
    let spec = crate::filters::render(dest.rate.as_f64(), dest.width, dest.height, node)?;
    let ffmpeg = ffmpeg_bin();
    let dir = tempfile::tempdir().map_err(|e| EncodeError::Invalid(e.to_string()))?;
    let out = dir.path().join("o.wav");
    run_ok(
        &ffmpeg,
        &[
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            &spec,
            "-c:a",
            "pcm_s24le",
            out.to_str().unwrap(),
        ],
    )
    .map_err(EncodeError::Invalid)?;
    read_out(&out)
}

pub fn deliver(
    dest: &Dest,
    picture: &[u8],
    audio: Option<&[u8]>,
    captions: Option<&[u8]>,
    markers: &[Marker],
) -> Result<Vec<u8>, EncodeError> {
    let ffmpeg = ffmpeg_bin();
    let dir = tempfile::tempdir().map_err(|e| EncodeError::Invalid(e.to_string()))?;
    let pic = write_tmp(dir.path(), "p.mov", picture)?;
    let out = dir.path().join("out.mp4");
    let mut args = vec![
        "-hide_banner".into(),
        "-loglevel".into(),
        "error".into(),
        "-y".into(),
        "-i".into(),
        pic.to_string_lossy().into_owned(),
    ];
    let mut next_in = 1usize;
    if let Some(audio) = audio {
        let a = write_tmp(dir.path(), "a.wav", audio)?;
        args.push("-i".into());
        args.push(a.to_string_lossy().into_owned());
        next_in = 2;
    }
    let caption_idx = if let Some(captions) = captions {
        let text = String::from_utf8_lossy(captions);
        let ext = if text.contains("WEBVTT") {
            "vtt"
        } else {
            "srt"
        };
        let cap = write_tmp(dir.path(), &format!("c.{ext}"), captions)?;
        args.push("-i".into());
        args.push(cap.to_string_lossy().into_owned());
        let idx = next_in;
        next_in += 1;
        Some(idx)
    } else {
        None
    };
    let chapter_idx = if markers.is_empty() {
        None
    } else {
        let meta = ffmetadata(dest, markers);
        let ch = write_tmp(dir.path(), "chapters.txt", meta.as_bytes())?;
        args.push("-i".into());
        args.push(ch.to_string_lossy().into_owned());
        Some(next_in)
    };
    let enc = dest.encoder.clone();
    let profile = enc.profile.as_deref().unwrap_or("high");
    let preset = enc.preset.as_deref().unwrap_or("medium");
    let params = format!(
        "keyint={}:min-keyint={}:scenecut=0:open-gop=0:stitchable=1:threads=1:sliced-threads=0:sync-lookahead=0",
        enc.keyint, enc.keyint
    );
    args.extend([
        "-filter_complex".into(),
        format!(
            "[0:v]format={},setparams=range=tv:colorspace={}:color_primaries={}:color_trc={}[v]",
            dest.pix_fmt, dest.color, dest.color, dest.color
        ),
        "-map".into(),
        "[v]".into(),
        "-c:v".into(),
        "libx264".into(),
        "-profile:v".into(),
        profile.into(),
        "-preset".into(),
        preset.into(),
        "-pix_fmt".into(),
        dest.pix_fmt.clone(),
        "-x264-params".into(),
        params,
        "-movflags".into(),
        "+faststart".into(),
    ]);
    if let Some(level) = enc.level.as_deref() {
        args.push("-level:v".into());
        args.push(level.into());
    }
    match enc.rate_control.as_ref() {
        Some(rc) if rc.mode == graft_score::RateControlMode::Bitrate => {
            args.push("-b:v".into());
            args.push(rc.bitrate.clone().unwrap_or_else(|| "4M".into()));
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
    if audio.is_some() {
        args.extend([
            "-map".into(),
            "1:a".into(),
            "-c:a".into(),
            "aac".into(),
            "-ar".into(),
            "48000".into(),
            "-ac".into(),
            "2".into(),
        ]);
    } else {
        args.push("-an".into());
    }
    if let Some(idx) = caption_idx {
        args.extend([
            "-map".into(),
            format!("{idx}:s"),
            "-c:s".into(),
            "mov_text".into(),
        ]);
    }
    if let Some(idx) = chapter_idx {
        args.extend([
            "-map_metadata".into(),
            idx.to_string(),
            "-map_chapters".into(),
            idx.to_string(),
        ]);
    }
    args.push(out.to_string_lossy().into_owned());
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    run_ok(&ffmpeg, &refs).map_err(EncodeError::Invalid)?;
    read_out(&out)
}

fn ffmetadata(dest: &Dest, markers: &[Marker]) -> String {
    let mut out = String::from(";FFMETADATA1\n");
    for marker in markers {
        let title = marker
            .id
            .replace('\\', "\\\\")
            .replace('=', "\\=")
            .replace(';', "\\;")
            .replace('#', "\\#")
            .replace('\n', " ");
        out.push_str(&format!(
            "[CHAPTER]\nTIMEBASE={}/{}\nSTART={}\nEND={}\ntitle={title}\n",
            dest.rate.den,
            dest.rate.num,
            marker.range.start,
            marker.range.end()
        ));
    }
    out
}

pub fn probe_duration_s(bytes: &[u8]) -> f64 {
    probe_media(bytes)
        .map(|p| {
            if p.duration_s > 0.0 {
                p.duration_s
            } else {
                p.audio_duration_s.unwrap_or(0.0)
            }
        })
        .unwrap_or(0.0)
}

pub fn encode_silence(duration_s: f64) -> Result<Vec<u8>, EncodeError> {
    let ffmpeg = ffmpeg_bin();
    let dir = tempfile::tempdir().map_err(|e| EncodeError::Invalid(e.to_string()))?;
    let out = dir.path().join("silence.wav");
    let dur = format!("{duration_s:.6}");
    run_ok(
        &ffmpeg,
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
            &dur,
            "-c:a",
            "pcm_s24le",
            out.to_str().unwrap(),
        ],
    )
    .map_err(EncodeError::Invalid)?;
    read_out(&out)
}

pub fn encode_gap_video(dest: &Dest, duration_s: f64) -> Result<Vec<u8>, EncodeError> {
    let ffmpeg = ffmpeg_bin();
    let dir = tempfile::tempdir().map_err(|e| EncodeError::Invalid(e.to_string()))?;
    let out = dir.path().join("gap.mov");
    let src = format!(
        "color=c=black:s={}x{}:r={}:d={duration_s:.6}",
        dest.width,
        dest.height,
        dest.rate.as_f64()
    );
    let vf = working_vf(dest, 1.0)?;
    let mut args = vec![
        "-hide_banner".into(),
        "-loglevel".into(),
        "error".into(),
        "-y".into(),
        "-f".into(),
        "lavfi".into(),
        "-i".into(),
        src,
        "-an".into(),
        "-vf".into(),
        vf,
    ];
    args.extend(working_video_encode(dest));
    args.push(out.to_string_lossy().into_owned());
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    run_ok(&ffmpeg, &refs).map_err(EncodeError::Invalid)?;
    read_out(&out)
}

pub fn working_concat_audio(parts: &[&[u8]]) -> Result<Vec<u8>, EncodeError> {
    if parts.is_empty() {
        return Err(EncodeError::Invalid("audio concat has no parts".into()));
    }
    if parts.len() == 1 {
        return Ok(parts[0].to_vec());
    }
    let ffmpeg = ffmpeg_bin();
    let dir = tempfile::tempdir().map_err(|e| EncodeError::Invalid(e.to_string()))?;
    let mut args = vec![
        "-hide_banner".into(),
        "-loglevel".into(),
        "error".into(),
        "-y".into(),
    ];
    let mut labels = String::new();
    for (i, part) in parts.iter().enumerate() {
        let p = write_tmp(dir.path(), &format!("a{i}.wav"), part)?;
        args.push("-i".into());
        args.push(p.to_string_lossy().into_owned());
        labels.push_str(&format!("[{i}:a]"));
    }
    let out = dir.path().join("c.wav");
    args.extend([
        "-filter_complex".into(),
        format!("{labels}concat=n={}:v=0:a=1[a]", parts.len()),
        "-map".into(),
        "[a]".into(),
        "-c:a".into(),
        "pcm_s24le".into(),
        out.to_string_lossy().into_owned(),
    ]);
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    run_ok(&ffmpeg, &refs).map_err(EncodeError::Invalid)?;
    read_out(&out)
}

pub fn working_acrossfade(
    left: &[u8],
    right: &[u8],
    duration_s: f64,
) -> Result<Vec<u8>, EncodeError> {
    let ffmpeg = ffmpeg_bin();
    let dir = tempfile::tempdir().map_err(|e| EncodeError::Invalid(e.to_string()))?;
    let a = write_tmp(dir.path(), "a.wav", left)?;
    let b = write_tmp(dir.path(), "b.wav", right)?;
    let out = dir.path().join("x.wav");
    run_ok(
        &ffmpeg,
        &[
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-i",
            a.to_str().unwrap(),
            "-i",
            b.to_str().unwrap(),
            "-filter_complex",
            &format!("[0:a][1:a]acrossfade=d={duration_s:.6}:c1=tri:c2=tri"),
            "-c:a",
            "pcm_s24le",
            out.to_str().unwrap(),
        ],
    )
    .map_err(EncodeError::Invalid)?;
    read_out(&out)
}

pub fn node_duration_s(dest: &Dest, node: &GraphNode) -> f64 {
    if let Some(place) = &node.place {
        return dest.rate.seconds_from_frames(place.duration as i64);
    }
    node.params
        .as_ref()
        .and_then(|v| v.get("duration"))
        .and_then(|v| v.as_f64())
        .unwrap_or(1.0)
}

pub fn gap_is_audio(node: &GraphNode) -> bool {
    node.role
        .as_deref()
        .map(|r| matches!(r, "sfx" | "vo" | "bed" | "audio"))
        .unwrap_or(false)
        || node
            .params
            .as_ref()
            .and_then(|v| v.get("kind"))
            .and_then(|v| v.as_str())
            == Some("audio")
}

#[cfg(test)]
mod tests {
    use super::switch_filter;
    use graft_score::GraphNode;

    #[test]
    fn switch_delays_later_angles() {
        let node = GraphNode {
            id: "switch".into(),
            op: "switch".into(),
            inputs: vec!["wide".into(), "close".into()],
            material: None,
            source: None,
            place: None,
            enable: None,
            role: None,
            params: Some(serde_json::json!({
                "cuts": [{"t": 0.0, "map": 0}, {"t": 0.5, "map": 1}]
            })),

            keys: None,
            nodes: Vec::new(),
        };
        let filter = switch_filter(&node).unwrap();
        assert!(filter.contains("setpts=PTS+0.500000/TB"));
        assert!(filter.contains("enable='gte(t,0.500000)'"));
        assert!(filter.ends_with("[v]"));
    }
}
