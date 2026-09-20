// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A number as the author wrote it, beside the number it denotes.
//!
//! # What was wrong
//!
//! `<sce:arm value="0x1a"/>` reaches the model as `26`, and the model
//! is all the pseudocode review surface has. So the page says `26`, and
//! a reviewer holding it against the UDS table they wrote — which says
//! `0x1A` — does not find it. Measured 2026-09-20 across the corpus:
//! **43 numbers on 6 carriers** are respelled that way, and no gate in
//! the tree could see it, because every comparison SCE makes is at
//! model level and the model never held the text.
//!
//! The round trip cannot catch this either, and that is worth stating
//! rather than fixing by making the round trip stricter: it compares a
//! model against a model, both of which hold `26`. Only the SOURCE
//! knows the author wrote `0x1a`.
//!
//! # The shape
//!
//! Each carrier keeps a companion `*_text` holding the source spelling,
//! `#[serde(skip)]` so the `--emit-ast` wire format does not move — the
//! same shape `VariantArm::source_line` already uses for a value that
//! belongs to diagnostics rather than to consumers.
//!
//! ⚠ The companion can be EMPTY, and that is not a defect. A model
//! built by the pseudocode reader, by a test, or by any path that never
//! saw a source document has no spelling to carry, and
//! [`as_written`] then prints the number. What the field promises is
//! "the author's spelling if there was one", never "a spelling".
//!
//! ⚠⚠ Two fields for one fact is a shape this repository distrusts, and
//! the reason it is right here is that they are not one fact: `value`
//! is what the machine computes with and `*_text` is what the author
//! typed. A single carrier would have to be the text, and then every
//! consumer would re-parse it — which is the same lossy step moved to a
//! worse place. `an_authored_spelling_parses_to_the_value_beside_it`
//! holds the two together.

use std::fmt::Display;

/// The author's spelling of a number, or the number when the model
/// never held one.
pub fn as_written(text: &str, value: impl Display) -> String {
    if text.is_empty() {
        value.to_string()
    } else {
        text.to_string()
    }
}

/// An unsigned integer written the way the forge grammar accepts one.
///
/// Decimal, `0x` hex or `0b` binary, with optional surrounding space.
/// The grammar's own three forms — a reader that took only decimal
/// would turn `0x1a` into a parse failure the moment the renderer
/// started preserving it, which is precisely what the pseudocode reader
/// did before this existed.
pub fn read_unsigned(text: &str) -> Option<u64> {
    let t = text.trim();
    if let Some(rest) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        return u64::from_str_radix(rest, 16).ok();
    }
    if let Some(rest) = t.strip_prefix("0b").or_else(|| t.strip_prefix("0B")) {
        return u64::from_str_radix(rest, 2).ok();
    }
    t.parse::<u64>().ok()
}

/// A signed integer written the way the forge grammar accepts one.
pub fn read_signed(text: &str) -> Option<i64> {
    let t = text.trim();
    let (negative, body) = match t.strip_prefix('-') {
        Some(rest) => (true, rest.trim_start()),
        None => (false, t.strip_prefix('+').unwrap_or(t)),
    };
    let magnitude = read_unsigned(body)?;
    let signed = i64::try_from(magnitude).ok()?;
    Some(if negative { -signed } else { signed })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_spelling_falls_back_to_the_number() {
        assert_eq!(as_written("", 26u64), "26");
        assert_eq!(as_written("", -1i64), "-1");
    }

    #[test]
    fn a_spelling_is_printed_verbatim() {
        assert_eq!(as_written("0x1a", 26u64), "0x1a");
        // Including one that does not look like the value: the field is
        // the author's, and a renderer that second-guessed it would be
        // respelling again.
        assert_eq!(as_written("0X1A", 26u64), "0X1A");
    }

    #[test]
    fn every_form_the_grammar_accepts_reads_back() {
        assert_eq!(read_unsigned("26"), Some(26));
        assert_eq!(read_unsigned("0x1a"), Some(26));
        assert_eq!(read_unsigned("0X1A"), Some(26));
        assert_eq!(read_unsigned("0b11010"), Some(26));
        assert_eq!(read_unsigned("  26  "), Some(26));
    }

    #[test]
    fn a_non_number_is_refused_rather_than_defaulted() {
        for v in ["", "0x", "0xzz", "twenty-six", "26.5", "-1"] {
            assert!(read_unsigned(v).is_none(), "{v} read as unsigned");
        }
    }

    #[test]
    fn a_sign_reads_on_every_radix() {
        assert_eq!(read_signed("-26"), Some(-26));
        assert_eq!(read_signed("-0x1a"), Some(-26));
        assert_eq!(read_signed("+26"), Some(26));
        assert_eq!(read_signed("26"), Some(26));
    }

    /// The pair a carrier keeps must agree, which is the invariant the
    /// whole shape rests on.
    #[test]
    fn a_spelling_parses_to_the_number_it_sits_beside() {
        for (text, value) in [("0x1a", 26u64), ("26", 26), ("0b11010", 26), ("0", 0)] {
            assert_eq!(
                read_unsigned(&as_written(text, value)),
                Some(value),
                "`{text}` does not read back as {value}"
            );
        }
    }
}
