// SPDX-License-Identifier: Apache-2.0

//! Frame-grain encode: copy `[in, out)` frames. Kerf is a no-op splice.
//! This is the first row of the grain table (image-seq / All-I), not a stub.

use graft_score::Dest;

use crate::concat::{ConcatBackend, ConcatError, ConcatRequest};
use crate::encode::{
    AudioEncodeRequest, EncodeBackend, EncodeError, KerfEncodeRequest, SlotEncodeRequest,
};
use crate::grain::Grain;

pub const MAGIC: &[u8; 4] = b"GFI1";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IntraSeq {
    pub width: u32,
    pub height: u32,
    pub fps_num: u32,
    pub fps_den: u32,
    pub frames: Vec<Vec<u8>>,
}

impl IntraSeq {
    pub fn frame_bytes(&self) -> usize {
        self.width as usize * self.height as usize * 3
    }

    pub fn fps(&self) -> f64 {
        self.fps_num as f64 / self.fps_den as f64
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&self.width.to_le_bytes());
        out.extend_from_slice(&self.height.to_le_bytes());
        out.extend_from_slice(&self.fps_num.to_le_bytes());
        out.extend_from_slice(&self.fps_den.to_le_bytes());
        out.extend_from_slice(&(self.frames.len() as u32).to_le_bytes());
        for frame in &self.frames {
            out.extend_from_slice(frame);
        }
        out
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, EncodeError> {
        const HEADER: usize = 4 + 4 * 5;
        if bytes.len() < HEADER || &bytes[..4] != MAGIC {
            return Err(EncodeError::Invalid(
                "material is not graft-intra (GFI1)".into(),
            ));
        }
        let width = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
        let height = u32::from_le_bytes(bytes[8..12].try_into().unwrap());
        let fps_num = u32::from_le_bytes(bytes[12..16].try_into().unwrap());
        let fps_den = u32::from_le_bytes(bytes[16..20].try_into().unwrap());
        let nframes = u32::from_le_bytes(bytes[20..24].try_into().unwrap()) as usize;
        if width == 0 || height == 0 || fps_num == 0 || fps_den == 0 {
            return Err(EncodeError::Invalid("GFI1 header has a zero field".into()));
        }
        let frame_bytes = width as usize * height as usize * 3;
        let expected = HEADER + nframes * frame_bytes;
        if bytes.len() != expected {
            return Err(EncodeError::Invalid(format!(
                "GFI1 size {} != header+frames {expected}",
                bytes.len()
            )));
        }
        let mut frames = Vec::with_capacity(nframes);
        let mut off = HEADER;
        for _ in 0..nframes {
            frames.push(bytes[off..off + frame_bytes].to_vec());
            off += frame_bytes;
        }
        Ok(Self {
            width,
            height,
            fps_num,
            fps_den,
            frames,
        })
    }

    pub fn slice_seconds(&self, in_s: f64, out_s: f64) -> Result<Self, EncodeError> {
        let fps = self.fps();
        let start = frame_index(in_s, fps) as usize;
        let end = frame_index(out_s, fps) as usize;
        if end > self.frames.len() || start >= end {
            return Err(EncodeError::Invalid(format!(
                "slice [{in_s},{out_s}) → frames [{start},{end}) outside 0..{}",
                self.frames.len()
            )));
        }
        Ok(Self {
            width: self.width,
            height: self.height,
            fps_num: self.fps_num,
            fps_den: self.fps_den,
            frames: self.frames[start..end].to_vec(),
        })
    }

    /// Nearest-neighbor resample to a dest frame count (`source_duration / speed`).
    pub fn resample_frames(&self, dest_frames: usize) -> Result<Self, EncodeError> {
        if dest_frames == 0 {
            return Err(EncodeError::Invalid("retime dest frame count is 0".into()));
        }
        if dest_frames == self.frames.len() {
            return Ok(self.clone());
        }
        let src_n = self.frames.len() as f64;
        let frames = (0..dest_frames)
            .map(|i| {
                let src = ((i as f64 + 0.5) * src_n / dest_frames as f64).floor() as usize;
                self.frames[src.min(self.frames.len() - 1)].clone()
            })
            .collect();
        Ok(Self {
            width: self.width,
            height: self.height,
            fps_num: self.fps_num,
            fps_den: self.fps_den,
            frames,
        })
    }

    pub fn trim_frames(&self, head: usize, tail: usize) -> Result<Self, ConcatError> {
        if head + tail > self.frames.len() {
            return Err(ConcatError::Invalid(format!(
                "intra trim {head}+{tail} exceeds {} frames",
                self.frames.len()
            )));
        }
        Ok(Self {
            width: self.width,
            height: self.height,
            fps_num: self.fps_num,
            fps_den: self.fps_den,
            frames: self.frames[head..self.frames.len() - tail].to_vec(),
        })
    }

    pub fn fade_join(
        left: &Self,
        right: &Self,
        duration_frames: usize,
    ) -> Result<Self, EncodeError> {
        if duration_frames == 0
            || duration_frames > left.frames.len()
            || duration_frames > right.frames.len()
        {
            return Err(EncodeError::Invalid(
                r#"{"error":"invalid_fade","reason":"fade duration does not fit both sides"}"#
                    .into(),
            ));
        }
        if left.width != right.width
            || left.height != right.height
            || left.fps_num != right.fps_num
            || left.fps_den != right.fps_den
        {
            return Err(EncodeError::Invalid(
                "fade kerf requires matching geometry and fps".into(),
            ));
        }
        let mut frames = Vec::with_capacity(duration_frames * 2);
        let left_start = left.frames.len() - duration_frames;
        for i in 0..duration_frames {
            let t = (i as f64 + 1.0) / (duration_frames as f64 + 1.0);
            frames.push(blend_rgb(&left.frames[left_start + i], &[], 1.0 - t));
        }
        for i in 0..duration_frames {
            let t = (i as f64 + 1.0) / (duration_frames as f64 + 1.0);
            frames.push(blend_rgb(&right.frames[i], &[], t));
        }
        Ok(Self {
            width: left.width,
            height: left.height,
            fps_num: left.fps_num,
            fps_den: left.fps_den,
            frames,
        })
    }

    pub fn concat_seqs(parts: &[Self]) -> Result<Self, ConcatError> {
        let Some(first) = parts.first() else {
            return Err(ConcatError::Invalid("concat has no parts".into()));
        };
        let mut frames = Vec::new();
        for part in parts {
            if part.width != first.width
                || part.height != first.height
                || part.fps_num != first.fps_num
                || part.fps_den != first.fps_den
            {
                return Err(ConcatError::Invalid(
                    "intra concat requires matching geometry and fps".into(),
                ));
            }
            frames.extend(part.frames.iter().cloned());
        }
        Ok(Self {
            width: first.width,
            height: first.height,
            fps_num: first.fps_num,
            fps_den: first.fps_den,
            frames,
        })
    }
}

fn blend_rgb(src: &[u8], _other: &[u8], keep: f64) -> Vec<u8> {
    let keep = keep.clamp(0.0, 1.0);
    src.iter()
        .map(|b| ((*b as f64) * keep).round() as u8)
        .collect()
}

pub fn frame_index(t: f64, fps: f64) -> u32 {
    (t * fps + 1e-9).floor() as u32
}

#[cfg(test)]
fn fps_ratio(fps: f64) -> (u32, u32) {
    let ms = (fps * 1000.0).round() as u32;
    if ms == 0 {
        (1, 1)
    } else if ms % 1000 == 0 {
        (ms / 1000, 1)
    } else {
        (ms, 1000)
    }
}

#[cfg(test)]
fn paint_frame(width: u32, height: u32, tag: u8, index: u32) -> Vec<u8> {
    let n = width as usize * height as usize * 3;
    let mut px = vec![0u8; n];
    for (i, b) in px.iter_mut().enumerate() {
        *b = tag
            .wrapping_add((index % 251) as u8)
            .wrapping_add((i % 251) as u8);
    }
    px
}

#[cfg(test)]
fn make_material(width: u32, height: u32, fps: f64, nframes: u32, tag: u8) -> IntraSeq {
    let (fps_num, fps_den) = fps_ratio(fps);
    IntraSeq {
        width,
        height,
        fps_num,
        fps_den,
        frames: (0..nframes)
            .map(|i| paint_frame(width, height, tag, i))
            .collect(),
    }
}

/// In-tree intra backend. Materials are GFI1 frame packs generated in tests.
pub struct FrameIntra;

impl FrameIntra {
    pub fn supports(dest: &Dest) -> bool {
        dest.encoder.impl_name == "graft-intra"
    }
}

impl EncodeBackend for FrameIntra {
    fn enabled(&self) -> bool {
        true
    }

    fn grain(&self, _dest: &Dest) -> Grain {
        Grain::Frame
    }

    fn encode_slot(&self, req: &SlotEncodeRequest<'_>) -> Result<Vec<u8>, EncodeError> {
        let seq = IntraSeq::decode(req.material)?;
        if seq.width != req.dest.width || seq.height != req.dest.height {
            return Err(EncodeError::Invalid(format!(
                "material {}x{} != dest {}x{}",
                seq.width, seq.height, req.dest.width, req.dest.height
            )));
        }
        let sliced = seq.slice_seconds(req.action.binding.in_s(), req.action.binding.out_s())?;
        let dest_frames = req.action.slot.range.duration as usize;
        let retimed = if dest_frames == sliced.frames.len() {
            sliced
        } else {
            sliced.resample_frames(dest_frames)?
        };
        Ok(retimed.encode())
    }

    fn encode_kerf(&self, req: &KerfEncodeRequest<'_>) -> Result<Vec<u8>, EncodeError> {
        if req.action.transition.eq_ignore_ascii_case("fade") {
            let left = IntraSeq::decode(req.left)?;
            let right = IntraSeq::decode(req.right)?;
            return Ok(
                IntraSeq::fade_join(&left, &right, req.action.duration_frames as usize)?.encode(),
            );
        }
        if req.action.noop {
            return Ok(Vec::new());
        }
        Err(EncodeError::Invalid(
            "FrameIntra cut kerf is a no-op splice; fade is the filled intra kerf".into(),
        ))
    }

    fn encode_audio(&self, _req: &AudioEncodeRequest<'_>) -> Result<Vec<u8>, EncodeError> {
        Err(EncodeError::Invalid(
            "graft-intra does not carry audio; use x264/ffmpeg".into(),
        ))
    }

    fn overlay_brand(
        &self,
        dest: &Dest,
        picture: &[u8],
        brand: &[u8],
    ) -> Result<Vec<u8>, EncodeError> {
        let mut pic = IntraSeq::decode(picture)?;
        if pic.width != dest.width || pic.height != dest.height {
            return Err(EncodeError::Invalid("overlay picture geometry".into()));
        }
        let logo = IntraSeq::decode(brand)?;
        let stamp = logo
            .frames
            .first()
            .cloned()
            .ok_or_else(|| EncodeError::Invalid("brand material has no frames".into()))?;
        if stamp.len() != pic.frame_bytes() {
            return Err(EncodeError::Invalid(
                "brand frame size must match dest".into(),
            ));
        }
        for frame in &mut pic.frames {
            for (i, px) in frame.iter_mut().enumerate() {
                let mix = stamp[i];
                *px = ((*px as u16 + mix as u16) / 2) as u8;
            }
        }
        Ok(pic.encode())
    }
}

impl ConcatBackend for FrameIntra {
    fn enabled(&self) -> bool {
        true
    }

    fn concat(&self, req: &ConcatRequest<'_>) -> Result<Vec<u8>, ConcatError> {
        let mut seqs = Vec::new();
        for part in req.parts {
            if part.empty || part.bytes.is_empty() {
                continue;
            }
            let seq =
                IntraSeq::decode(&part.bytes).map_err(|e| ConcatError::Invalid(e.to_string()))?;
            let trimmed = seq
                .trim_frames(
                    part.trim_head_frames as usize,
                    part.trim_tail_frames as usize,
                )
                .map_err(|e| ConcatError::Invalid(e.to_string()))?;
            if trimmed.frames.is_empty() {
                continue;
            }
            seqs.push(trimmed);
        }
        Ok(IntraSeq::concat_seqs(&seqs)?.encode())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::compile;
    use graft_cas::{Kind, Memory, Store};
    use graft_score::{
        Binding, BindingLayer, Clock, Dest, Encoder, FrameRange, FrameRate, Join, Layer, Role,
        Scion, Score, Slot, TimedRange, Window, GRAFT_SCHEMA,
    };
    use std::collections::BTreeMap;

    #[test]
    fn slice_is_half_open_frames() {
        let seq = make_material(4, 4, 10.0, 30, 1);
        let mid = seq.slice_seconds(1.0, 2.0).unwrap();
        assert_eq!(mid.frames.len(), 10);
        assert_eq!(mid.frames[0], seq.frames[10]);
        assert_eq!(mid.frames[9], seq.frames[19]);
    }

    #[test]
    fn roundtrip_bytes() {
        let seq = make_material(2, 2, 30.0, 3, 7);
        let bytes = seq.encode();
        assert_eq!(IntraSeq::decode(&bytes).unwrap(), seq);
    }

    fn intra_score() -> Score {
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

    fn intra_dest() -> Dest {
        Dest {
            id: "9x16".into(),
            width: 4,
            height: 4,
            rate: FrameRate::new(10, 1),
            pix_fmt: "rgb24".into(),
            color: "srgb".into(),
            encoder: Encoder::graft_intra(),
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

    /// Mission §1 intra: hook_v3 + body_v1 after hook_v2 bitstream-copies the body.
    #[test]
    fn hook_swap_does_not_change_body_blob_or_frame_bytes() {
        let score = intra_score();
        let dest = intra_dest();
        let store = Memory::new();
        let backend = FrameIntra;

        let hook_v2 = store
            .put_blob(Kind::Material, &make_material(4, 4, 10.0, 15, 2).encode())
            .unwrap();
        let hook_v3 = store
            .put_blob(Kind::Material, &make_material(4, 4, 10.0, 15, 3).encode())
            .unwrap();
        let body_v1 = store
            .put_blob(Kind::Material, &make_material(4, 4, 10.0, 25, 10).encode())
            .unwrap();
        let cta_v1 = store
            .put_blob(Kind::Material, &make_material(4, 4, 10.0, 15, 20).encode())
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

        let v2 = scion(hook_v2.as_str());
        let first = compile(&score, &v2, &store, &backend, &backend).unwrap();
        assert!(first.encoded);
        let body_slot = first
            .graph
            .slots
            .iter()
            .find(|s| s.slot.id == "body")
            .unwrap();
        let body_entry = store.get_action(&body_slot.key).unwrap().unwrap();
        let body_bytes_v2 = store.get_blob(Kind::SlotEncode, &body_entry.blob).unwrap();
        let hook_entry_v2 = store
            .get_action(
                &first
                    .graph
                    .slots
                    .iter()
                    .find(|s| s.slot.id == "hook")
                    .unwrap()
                    .key,
            )
            .unwrap()
            .unwrap();
        let concat_v2 = first.concat.clone().unwrap();

        let v3 = scion(hook_v3.as_str());
        let second = compile(&score, &v3, &store, &backend, &backend).unwrap();
        let hook_plan = second.plan.slots.iter().find(|s| s.id == "hook").unwrap();
        let body_plan = second.plan.slots.iter().find(|s| s.id == "body").unwrap();
        let cta_plan = second.plan.slots.iter().find(|s| s.id == "cta").unwrap();
        assert_eq!(hook_plan.cache, "miss");
        assert_eq!(body_plan.cache, "hit");
        assert_eq!(cta_plan.cache, "hit");
        let hb = second
            .plan
            .kerfs
            .iter()
            .find(|k| k.join == ["hook", "body"])
            .unwrap();
        let bc = second
            .plan
            .kerfs
            .iter()
            .find(|k| k.join == ["body", "cta"])
            .unwrap();
        assert_eq!(hb.cache, "miss");
        assert_eq!(bc.cache, "hit");

        let body_entry_v3 = store.get_action(&body_slot.key).unwrap().unwrap();
        assert_eq!(body_entry.blob, body_entry_v3.blob);
        let body_bytes_v3 = store
            .get_blob(Kind::SlotEncode, &body_entry_v3.blob)
            .unwrap();
        assert_eq!(body_bytes_v2, body_bytes_v3);

        let hook_entry_v3 = store
            .get_action(
                &second
                    .graph
                    .slots
                    .iter()
                    .find(|s| s.slot.id == "hook")
                    .unwrap()
                    .key,
            )
            .unwrap()
            .unwrap();
        assert_ne!(hook_entry_v2.blob, hook_entry_v3.blob);

        let concat_v3 = second.concat.unwrap();
        assert_ne!(concat_v2, concat_v3);

        let body_seq = IntraSeq::decode(&body_bytes_v3).unwrap();
        assert_eq!(body_seq.frames.len(), 20);
        assert_eq!(body_seq.frames[0], paint_frame(4, 4, 10, 0));
        assert_eq!(body_seq.frames[19], paint_frame(4, 4, 10, 19));
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
    fn speed_retime_changes_hook_frames_and_keeps_body_hit() {
        let score = intra_score();
        let dest = intra_dest();
        let store = Memory::new();
        let backend = FrameIntra;
        let hook = store
            .put_blob(Kind::Material, &make_material(4, 4, 10.0, 25, 2).encode())
            .unwrap();
        let body = store
            .put_blob(Kind::Material, &make_material(4, 4, 10.0, 25, 10).encode())
            .unwrap();
        let cta = store
            .put_blob(Kind::Material, &make_material(4, 4, 10.0, 15, 20).encode())
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
        let first = compile(&score, &scion(1.0, 1.0), &store, &backend, &backend).unwrap();
        let body_key = first
            .graph
            .slots
            .iter()
            .find(|s| s.slot.id == "body")
            .unwrap()
            .key
            .clone();
        let body_blob = store.get_action(&body_key).unwrap().unwrap().blob;
        let second = compile(&score, &scion(2.0, 2.0), &store, &backend, &backend).unwrap();
        let hook_plan = second.plan.slots.iter().find(|s| s.id == "hook").unwrap();
        let body_plan = second.plan.slots.iter().find(|s| s.id == "body").unwrap();
        let cta_plan = second.plan.slots.iter().find(|s| s.id == "cta").unwrap();
        assert_eq!(hook_plan.cache, "miss");
        assert_eq!(body_plan.cache, "hit");
        assert_eq!(cta_plan.cache, "hit");
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
        let hook_bytes = store
            .get_blob(
                Kind::SlotEncode,
                &store.get_action(&hook_key).unwrap().unwrap().blob,
            )
            .unwrap();
        assert_eq!(IntraSeq::decode(&hook_bytes).unwrap().frames.len(), 10);
    }

    #[test]
    fn fade_hook_body_keeps_body_blob_and_changing_duration_misses_only_kerf() {
        let mut score = intra_score();
        score.joins = vec![Join {
            left: "hook".into(),
            right: "body".into(),
            transition: "fade".into(),
            duration_frames: 2,
        }];
        let dest = intra_dest();
        let store = Memory::new();
        let backend = FrameIntra;
        let hook = store
            .put_blob(Kind::Material, &make_material(4, 4, 10.0, 15, 2).encode())
            .unwrap();
        let body = store
            .put_blob(Kind::Material, &make_material(4, 4, 10.0, 25, 10).encode())
            .unwrap();
        let cta = store
            .put_blob(Kind::Material, &make_material(4, 4, 10.0, 15, 20).encode())
            .unwrap();
        let scion = {
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
                dest: dest.clone(),
                layers: vec![BindingLayer {
                    name: Layer::Base,
                    bindings,
                }],
            }
        };
        let first = compile(&score, &scion, &store, &backend, &backend).unwrap();
        assert!(first.encoded);
        let body_key = first
            .graph
            .slots
            .iter()
            .find(|s| s.slot.id == "body")
            .unwrap()
            .key
            .clone();
        let body_blob = store.get_action(&body_key).unwrap().unwrap().blob;
        let hook_body = first
            .graph
            .kerfs
            .iter()
            .find(|k| k.left == "hook" && k.right == "body")
            .unwrap();
        assert_eq!(hook_body.transition, "fade");
        assert!(!hook_body.noop);
        let kerf_bytes = store
            .get_blob(
                Kind::Kerf,
                &store.get_action(&hook_body.key).unwrap().unwrap().blob,
            )
            .unwrap();
        assert!(!kerf_bytes.is_empty());
        assert_eq!(IntraSeq::decode(&kerf_bytes).unwrap().frames.len(), 4);

        let hook_v2 = store
            .put_blob(Kind::Material, &make_material(4, 4, 10.0, 15, 3).encode())
            .unwrap();
        let mut swap = scion.clone();
        swap.layers[0]
            .bindings
            .insert("hook".into(), bind(hook_v2.as_str(), 0.0, 1.0));
        let second = compile(&score, &swap, &store, &backend, &backend).unwrap();
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
        let third = compile(&score, &swap, &store, &backend, &backend).unwrap();
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
        assert_eq!(
            third
                .plan
                .slots
                .iter()
                .find(|s| s.id == "hook")
                .unwrap()
                .cache,
            "hit"
        );
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
            store.get_action(&body_key).unwrap().unwrap().blob,
            body_blob
        );
    }

    #[test]
    fn proof_slot_stays_hit_across_hook_swap() {
        let mut score = intra_score();
        score.slots.insert(
            2,
            Slot {
                id: "proof".into(),
                role: Role::Proof,
                range: FrameRange::new(20, 10),
                optional: false,
                window: None,
            },
        );
        score.clock.duration_frames = 40;
        let dest = intra_dest();
        let store = Memory::new();
        let backend = FrameIntra;
        let hook = store
            .put_blob(Kind::Material, &make_material(4, 4, 10.0, 15, 2).encode())
            .unwrap();
        let hook2 = store
            .put_blob(Kind::Material, &make_material(4, 4, 10.0, 15, 4).encode())
            .unwrap();
        let body = store
            .put_blob(Kind::Material, &make_material(4, 4, 10.0, 25, 10).encode())
            .unwrap();
        let proof = store
            .put_blob(Kind::Material, &make_material(4, 4, 10.0, 15, 12).encode())
            .unwrap();
        let cta = store
            .put_blob(Kind::Material, &make_material(4, 4, 10.0, 15, 20).encode())
            .unwrap();
        let scion = |hook_id: &str| {
            let mut bindings = BTreeMap::new();
            bindings.insert("hook".into(), bind(hook_id, 0.0, 1.0));
            bindings.insert("body".into(), bind(body.as_str(), 0.0, 2.0));
            bindings.insert("proof".into(), bind(proof.as_str(), 0.0, 1.0));
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
        let first = compile(&score, &scion(hook.as_str()), &store, &backend, &backend).unwrap();
        let proof_key = first
            .graph
            .slots
            .iter()
            .find(|s| s.slot.id == "proof")
            .unwrap()
            .key
            .clone();
        let proof_blob = store.get_action(&proof_key).unwrap().unwrap().blob;
        let body_key = first
            .graph
            .slots
            .iter()
            .find(|s| s.slot.id == "body")
            .unwrap()
            .key
            .clone();
        let body_blob = store.get_action(&body_key).unwrap().unwrap().blob;
        let second = compile(&score, &scion(hook2.as_str()), &store, &backend, &backend).unwrap();
        assert_eq!(
            second
                .plan
                .slots
                .iter()
                .find(|s| s.id == "proof")
                .unwrap()
                .cache,
            "hit"
        );
        assert_eq!(
            store.get_action(&proof_key).unwrap().unwrap().blob,
            proof_blob
        );
        assert_eq!(
            store.get_action(&body_key).unwrap().unwrap().blob,
            body_blob
        );
    }

    #[test]
    fn captions_and_brand_do_not_recut_body() {
        let mut score = intra_score();
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
        let dest = intra_dest();
        let store = Memory::new();
        let backend = FrameIntra;
        let hook = store
            .put_blob(Kind::Material, &make_material(4, 4, 10.0, 15, 2).encode())
            .unwrap();
        let hook2 = store
            .put_blob(Kind::Material, &make_material(4, 4, 10.0, 15, 9).encode())
            .unwrap();
        let body = store
            .put_blob(Kind::Material, &make_material(4, 4, 10.0, 25, 10).encode())
            .unwrap();
        let cta = store
            .put_blob(Kind::Material, &make_material(4, 4, 10.0, 15, 20).encode())
            .unwrap();
        let captions = store
            .put_blob(Kind::Material, b"WEBVTT\n\n00:00.000 --> 00:04.000\nHi\n")
            .unwrap();
        let captions2 = store
            .put_blob(Kind::Material, b"WEBVTT\n\n00:00.000 --> 00:04.000\nBye\n")
            .unwrap();
        let brand = store
            .put_blob(Kind::Material, &make_material(4, 4, 10.0, 4, 77).encode())
            .unwrap();
        let scion = |hook_id: &str, cap: &str| {
            let mut bindings = BTreeMap::new();
            bindings.insert("hook".into(), bind(hook_id, 0.0, 1.0));
            bindings.insert("body".into(), bind(body.as_str(), 0.0, 2.0));
            bindings.insert("cta".into(), bind(cta.as_str(), 0.0, 1.0));
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
            &backend,
            &backend,
        )
        .unwrap();
        assert_eq!(first.captions.len(), 1);
        assert_eq!(first.plan.overlay_mix, Some("miss"));
        let body_key = first
            .graph
            .slots
            .iter()
            .find(|s| s.slot.id == "body")
            .unwrap()
            .key
            .clone();
        let body_blob = store.get_action(&body_key).unwrap().unwrap().blob;
        let cap_key = first.graph.captions[0].key.clone();
        let cap_blob = store.get_action(&cap_key).unwrap().unwrap().blob;
        let brand_material = first
            .graph
            .overlay_mix
            .as_ref()
            .unwrap()
            .brand
            .material
            .clone();
        let second = compile(
            &score,
            &scion(hook2.as_str(), captions.as_str()),
            &store,
            &backend,
            &backend,
        )
        .unwrap();
        assert_eq!(
            store.get_action(&body_key).unwrap().unwrap().blob,
            body_blob
        );
        assert_eq!(store.get_action(&cap_key).unwrap().unwrap().blob, cap_blob);
        assert_eq!(
            second.graph.overlay_mix.as_ref().unwrap().brand.material,
            brand_material
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
            &backend,
            &backend,
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
        assert_ne!(
            store
                .get_action(&third.graph.captions[0].key)
                .unwrap()
                .unwrap()
                .blob,
            cap_blob
        );
        assert_eq!(
            store.get_action(&body_key).unwrap().unwrap().blob,
            body_blob
        );
    }
}
