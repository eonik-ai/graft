// SPDX-License-Identifier: Apache-2.0

use graft_score::Dest;

/// Smallest replaceable unit. See docs/compile.md grain table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Grain {
    /// Intra / image-seq / All-I. Kerf may be a no-op splice.
    Frame,
    /// Long-GOP. Dirty slot plus typically one GOP each side of a join.
    Gop { keyint: u32 },
}

impl Grain {
    pub fn from_dest(dest: &Dest) -> Self {
        from_impl(&dest.encoder.impl_name, dest.encoder.keyint)
    }

    pub fn kerf_is_noop(self) -> bool {
        matches!(self, Self::Frame)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Frame => "frame",
            Self::Gop { .. } => "gop",
        }
    }
}

fn from_impl(impl_name: &str, keyint: u32) -> Grain {
    let n = impl_name.to_ascii_lowercase();
    if n == "graft-intra"
        || n.contains("prores")
        || n.contains("dnx")
        || n.contains("jpeg2000")
        || n.contains("mjpeg")
        || n == "png"
        || n == "dpx"
        || n == "exr"
        || n == "y4m"
        || n.contains("image-seq")
        || n.contains("all-i")
        || n.contains("alli")
    {
        Grain::Frame
    } else {
        Grain::Gop { keyint }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use graft_score::{Dest, Encoder, FrameRate};

    #[test]
    fn x264_is_gop_intra_is_frame() {
        let mut dest = Dest {
            id: "9x16".into(),
            width: 1080,
            height: 1920,
            rate: FrameRate::new(30, 1),
            pix_fmt: "yuv420p".into(),
            color: "bt709".into(),
            encoder: Encoder::default_x264(),
        };
        assert_eq!(Grain::from_dest(&dest), Grain::Gop { keyint: 30 });
        dest.encoder = Encoder::graft_intra();
        assert!(Grain::from_dest(&dest).kerf_is_noop());
        dest.encoder.impl_name = "prores".into();
        assert!(Grain::from_dest(&dest).kerf_is_noop());
    }
}
