// SPDX-License-Identifier: Apache-2.0

use crate::Error;

const UUID_RE: &str = "uuid";
const SLOT_ID_RE: &str = "slot";

pub fn require_uuid(s: &str, field: &str) -> Result<(), Error> {
    if looks_like(UUID_RE, s) {
        Ok(())
    } else {
        Err(Error::invalid(format!("{field} is not a UUID: {s:?}")))
    }
}

pub fn require_slot_id(s: &str) -> Result<(), Error> {
    if looks_like(SLOT_ID_RE, s) {
        Ok(())
    } else {
        Err(Error::invalid(format!(
            "slot id must match [a-z][a-z0-9_]* (got {s:?})"
        )))
    }
}

#[allow(dead_code)]
pub fn validate_span(span: [f64; 2], what: &str) -> Result<(), Error> {
    if span[0] < 0.0 || span[1] < 0.0 {
        return Err(Error::invalid(format!("{what}: span bounds must be >= 0")));
    }
    if span[1] <= span[0] {
        return Err(Error::invalid(format!(
            "{what}: span end must be greater than start"
        )));
    }
    if !span[0].is_finite() || !span[1].is_finite() {
        return Err(Error::invalid(format!(
            "{what}: span bounds must be finite"
        )));
    }
    Ok(())
}

pub fn looks_like_hex64(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}

fn looks_like(kind: &str, s: &str) -> bool {
    match kind {
        "uuid" => {
            let b = s.as_bytes();
            if b.len() != 36 {
                return false;
            }
            let hex = |c: u8| c.is_ascii_digit() || (b'a'..=b'f').contains(&c);
            b.iter().enumerate().all(|(i, &c)| match i {
                8 | 13 | 18 | 23 => c == b'-',
                _ => hex(c),
            })
        }
        "slot" => {
            let mut chars = s.chars();
            match chars.next() {
                Some(c) if c.is_ascii_lowercase() => {
                    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
                }
                _ => false,
            }
        }
        _ => false,
    }
}
