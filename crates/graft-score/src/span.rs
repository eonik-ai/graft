// SPDX-License-Identifier: Apache-2.0

use crate::Error;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Range {
    pub start: f64,
    pub end: f64,
}

impl Range {
    pub fn new(start: f64, end: f64) -> Result<Self, Error> {
        if !start.is_finite() || !end.is_finite() {
            return Err(Error::invalid("range bounds must be finite"));
        }
        if start < 0.0 || end < 0.0 {
            return Err(Error::invalid("range bounds must be >= 0"));
        }
        if end <= start {
            return Err(Error::invalid(format!(
                "range end must be greater than start (got {start}-{end})"
            )));
        }
        Ok(Self { start, end })
    }

    pub fn as_span(self) -> [f64; 2] {
        [self.start, self.end]
    }
}

/// Parse `0-3` / `0.0-3.0` on the score or dest clock.
pub fn parse_range(spec: &str) -> Result<Range, Error> {
    let spec = spec.trim();
    let Some((a, b)) = spec.split_once('-') else {
        return Err(Error::invalid(format!("expected T0-T1 (got {spec:?})")));
    };
    if b.contains('-') {
        return Err(Error::invalid(format!("expected T0-T1 (got {spec:?})")));
    }
    let start = parse_seconds(a)?;
    let end = parse_seconds(b)?;
    Range::new(start, end)
}

fn parse_seconds(s: &str) -> Result<f64, Error> {
    s.trim()
        .parse::<f64>()
        .map_err(|_| Error::invalid(format!("not a number: {s:?}")))
}

/// Parse `1080x1920`. `9:16` is an aspect ratio, not a dest.
pub fn parse_wh(spec: &str) -> Result<(u32, u32), Error> {
    let spec = spec.trim();
    if spec.contains(':') {
        return Err(Error::invalid(
            "9:16 is an aspect, not a dest; pass pixels like 1080x1920 (dest id is 9x16)",
        ));
    }
    let Some((w, h)) = spec.split_once('x') else {
        return Err(Error::invalid(format!(
            "expected WIDTHxHEIGHT (got {spec:?})"
        )));
    };
    let width: u32 = w
        .parse()
        .map_err(|_| Error::invalid(format!("width is not a positive integer: {w:?}")))?;
    let height: u32 = h
        .parse()
        .map_err(|_| Error::invalid(format!("height is not a positive integer: {h:?}")))?;
    if width == 0 || height == 0 {
        return Err(Error::invalid("width and height must be >= 1"));
    }
    Ok((width, height))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hook_window() {
        let r = parse_range("0-3").unwrap();
        assert_eq!(r.start, 0.0);
        assert_eq!(r.end, 3.0);
    }

    #[test]
    fn dest_is_pixels() {
        assert_eq!(parse_wh("1080x1920").unwrap(), (1080, 1920));
        assert!(parse_wh("9:16").is_err());
    }
}
