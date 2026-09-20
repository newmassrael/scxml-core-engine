// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! What a literal in an SCE document denotes, and a second one of the
//! same shape.
//!
//! Two gates need this and they need the same answer.
//! `a_declared_attribute_must_reach_the_ir` mutates an attribute and
//! requires the IR to move; `a_rendering_shows_the_literal_the_author_
//! wrote` mutates one and requires the PAGE to move, then compares the
//! spelling that moved against the one the author used. Both stand on
//! "a different value of this same shape", and two copies of that would
//! be two things to keep true — with the quieter copy deciding what the
//! noisier one measures.
//!
//! # ⚠ Same shape, not merely different
//!
//! The mutant has to survive XSD validation and the parser, or the
//! measurement turns into a measurement of the error path. So a hex
//! literal stays hex and keeps its digit count, a byte string stays an
//! even number of hex digits, and a rational stays a rational. Appending
//! a character is the last resort, for a value with no numeric shape at
//! all.

#![allow(dead_code)]

/// What a numeric literal denotes, when it denotes anything.
///
/// Two carriers rather than one: `0x10` and `16` are the same integer,
/// while `7.0` and `7` are the same number written by different types.
/// Reading everything as `f64` would make `18446744073709551615` and
/// `18446744073709551616` equal, and this tree holds hex values that
/// large.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Num {
    Int(i128),
    Real(f64),
}

impl Num {
    /// Whether two literals denote the same number, across carriers.
    pub fn same(self, other: Self) -> bool {
        match (self, other) {
            (Num::Int(a), Num::Int(b)) => a == b,
            (Num::Real(a), Num::Real(b)) => a == b,
            // `7.0` in a document against `7` on a page is precisely
            // the loss one of these gates looks for, so the carriers do
            // compare — through the real, never by truncating.
            (Num::Int(a), Num::Real(b)) | (Num::Real(b), Num::Int(a)) => (a as f64) == b,
        }
    }
}

/// The number this text denotes, or `None` if it is not one.
pub fn number(text: &str) -> Option<Num> {
    let t = text.trim();
    if t.is_empty() {
        return None;
    }
    let (negative, body) = match t.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, t.strip_prefix('+').unwrap_or(t)),
    };
    if let Some(hex) = body.strip_prefix("0x").or_else(|| body.strip_prefix("0X")) {
        if hex.is_empty() || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        return i128::from_str_radix(hex, 16)
            .ok()
            .map(|n| Num::Int(if negative { -n } else { n }));
    }
    if let Ok(n) = body.parse::<i128>() {
        return Some(Num::Int(if negative { -n } else { n }));
    }
    // A real only when it is written like one. Without this every
    // integer would read as a real too, and the carriers would stop
    // telling `7` from `7.0`.
    if body.contains('.') || body.contains('e') || body.contains('E') {
        if let Ok(f) = body.parse::<f64>() {
            return Some(Num::Real(if negative { -f } else { f }));
        }
    }
    None
}

/// A different value written the way this one is written.
///
/// ⚠ Never the same text. A caller mutating a document and diffing the
/// result reads "nothing changed" as "the value is not used", so a
/// mutation that produced the original would be reported as an absence.
pub fn second_value_of_same_shape(current: &str) -> String {
    let v = current.trim();

    // A `0x` literal: keep the prefix and the digit count, so a field
    // whose width the parser checks still validates.
    if let Some(digits) = v.strip_prefix("0x").or_else(|| v.strip_prefix("0X")) {
        if !digits.is_empty() && digits.chars().all(|c| c.is_ascii_hexdigit()) {
            if let Ok(n) = u128::from_str_radix(digits, 16) {
                // ⚠ Masked to the written width, not merely incremented.
                // `0xFFFF + 1` is `0x10000`, five digits where the author
                // wrote four — and a field whose width the parser checks
                // then rejects the mutant, turning a measurement into a
                // measurement of the error path. `a_hex_literal_stays_hex
                // _and_keeps_its_width` caught exactly this.
                let bits = 4u32 * digits.len() as u32;
                let next = n.wrapping_add(1);
                let masked = if bits < 128 {
                    next & ((1u128 << bits) - 1)
                } else {
                    next
                };
                return format!("0x{masked:0width$X}", width = digits.len());
            }
        }
    }

    // A bare hex string, as a byte payload is written. Changing the last
    // digit keeps the hex alphabet and the even length a byte string
    // needs.
    if v.len() >= 2 && v.len().is_multiple_of(2) && v.chars().all(|c| c.is_ascii_hexdigit()) {
        let (head, last) = v.split_at(v.len() - 1);
        return match u8::from_str_radix(last, 16) {
            Ok(15) => format!("{head}e"),
            Ok(d) => format!("{head}{:x}", d + 1),
            Err(_) => format!("{v}z"),
        };
    }

    if let Ok(n) = v.parse::<i64>() {
        return n.saturating_add(1).to_string();
    }

    // A rational, as a scale or an offset is written. `+ 1` rather than
    // a digit edit, so the result is still a rational and still parses.
    if v.contains('.') {
        if let Ok(f) = v.parse::<f64>() {
            return format!("{}", f + 1.0);
        }
    }

    format!("{v}z")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_second_value_is_never_the_first() {
        for v in [
            "0x00", "0xFF", "0x0f", "deadbeef", "0", "-40", "0.5", "7.0", "name", "",
        ] {
            assert_ne!(second_value_of_same_shape(v), v, "{v} mutated to itself");
        }
    }

    #[test]
    fn a_hex_literal_stays_hex_and_keeps_its_width() {
        assert_eq!(second_value_of_same_shape("0x00"), "0x01");
        assert_eq!(second_value_of_same_shape("0x0f"), "0x10");
        assert_eq!(second_value_of_same_shape("0xFFFF"), "0x0000");
    }

    #[test]
    fn a_byte_string_keeps_an_even_number_of_hex_digits() {
        let out = second_value_of_same_shape("deadbeef");
        assert_eq!(out.len(), 8, "{out}");
        assert!(out.chars().all(|c| c.is_ascii_hexdigit()), "{out}");
    }

    #[test]
    fn carriers_compare_across_the_dot() {
        assert!(number("7.0").unwrap().same(number("7").unwrap()));
        assert!(number("0x10").unwrap().same(number("16").unwrap()));
        assert!(!number("7.5").unwrap().same(number("7").unwrap()));
    }

    #[test]
    fn a_non_number_is_not_a_number() {
        for v in ["", "name", "0x", "0xzz", "1.2.3", "#motor"] {
            assert!(number(v).is_none(), "{v} read as a number");
        }
    }
}
