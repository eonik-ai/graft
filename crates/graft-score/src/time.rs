// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use crate::{Error, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrameRate {
    pub num: u32,
    pub den: u32,
}

impl FrameRate {
    pub const fn new(num: u32, den: u32) -> Self {
        Self { num, den }
    }

    pub fn validate(self, field: &str) -> Result<()> {
        if self.num == 0 || self.den == 0 {
            return Err(Error::invalid(format!(
                "{field} must be a positive rational"
            )));
        }
        Ok(())
    }

    pub fn as_f64(self) -> f64 {
        self.num as f64 / self.den as f64
    }

    pub fn from_f64(value: f64) -> Result<Self> {
        if !value.is_finite() || value <= 0.0 {
            return Err(Error::invalid("frame rate must be finite and > 0"));
        }
        let den = 1000_u32;
        let num = (value * den as f64).round() as u32;
        let divisor = gcd(num, den);
        Ok(Self::new(num / divisor, den / divisor))
    }

    pub fn frames_from_seconds(self, seconds: f64) -> Result<i64> {
        if !seconds.is_finite() || seconds < 0.0 {
            return Err(Error::invalid("seconds must be finite and >= 0"));
        }
        Ok((seconds * self.as_f64()).round() as i64)
    }

    pub fn seconds_from_frames(self, frames: i64) -> f64 {
        frames as f64 / self.as_f64()
    }
}

const fn gcd(mut a: u32, mut b: u32) -> u32 {
    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    a
}

impl Default for FrameRate {
    fn default() -> Self {
        Self::new(30, 1)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrameRange {
    pub start: i64,
    pub duration: u64,
}

impl FrameRange {
    pub const fn new(start: i64, duration: u64) -> Self {
        Self { start, duration }
    }

    pub fn validate(self, field: &str) -> Result<()> {
        if self.start < 0 {
            return Err(Error::invalid(format!("{field}.start must be >= 0")));
        }
        if self.duration == 0 {
            return Err(Error::invalid(format!("{field}.duration must be > 0")));
        }
        self.start
            .checked_add(self.duration as i64)
            .ok_or_else(|| Error::invalid(format!("{field} overflows")))?;
        Ok(())
    }

    pub fn end(self) -> i64 {
        self.start + self.duration as i64
    }

    pub fn overlap_frames(self, other: Self) -> u64 {
        let start = self.start.max(other.start);
        let end = self.end().min(other.end());
        end.saturating_sub(start).max(0) as u64
    }

    pub fn from_seconds(rate: FrameRate, start: f64, end: f64) -> Result<Self> {
        if end <= start {
            return Err(Error::invalid("range end must be greater than start"));
        }
        let start_frame = rate.frames_from_seconds(start)?;
        let end_frame = rate.frames_from_seconds(end)?;
        let duration = end_frame
            .checked_sub(start_frame)
            .filter(|v| *v > 0)
            .ok_or_else(|| Error::invalid("range is shorter than one frame"))?
            as u64;
        Ok(Self::new(start_frame, duration))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimedRange {
    pub rate: FrameRate,
    pub range: FrameRange,
}

impl TimedRange {
    pub fn validate(self, field: &str) -> Result<()> {
        self.rate.validate(&format!("{field}.rate"))?;
        self.range.validate(&format!("{field}.range"))
    }

    pub fn from_seconds(rate: FrameRate, start: f64, end: f64) -> Result<Self> {
        Ok(Self {
            rate,
            range: FrameRange::from_seconds(rate, start, end)?,
        })
    }

    pub fn start_seconds(self) -> f64 {
        self.rate.seconds_from_frames(self.range.start)
    }

    pub fn end_seconds(self) -> f64 {
        self.rate.seconds_from_frames(self.range.end())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disjoint_ranges_have_zero_overlap() {
        assert_eq!(
            FrameRange::new(600, 90).overlap_frames(FrameRange::new(0, 90)),
            0
        );
    }
}
