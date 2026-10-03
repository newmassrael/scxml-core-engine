// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! How wide a label is, in the face the print diagram sets it in.
//!
//! Whether a figure fits its page at the minimum font size is decided from
//! these widths, so they are read from the font file
//! (`tools/diagram/gen_font_metrics.py` writes [`table`]) rather than
//! estimated — a 1 em guess for Hangul over-measures Korean text by 8.7 %,
//! and would refold figures that fit.
//!
//! A character the table does not cover is REFUSED ([`Unmeasured`]), never
//! given a width: a guessed width is a guessed fit.

#[path = "metrics_table.rs"]
mod table;

pub use table::FONT_VERSION;

/// The face a run of text is set in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Face {
    /// Names, events and prose.
    Proportional,
    /// Conditions and actions — code, set so its columns line up.
    Mono,
}

/// A character the table has no advance for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unmeasured {
    pub character: char,
    pub text: String,
}

impl std::fmt::Display for Unmeasured {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "U+{:04X} ({:?}) in {:?} has no measured width in the diagram's font table",
            self.character as u32, self.character, self.text
        )
    }
}

impl Face {
    /// The font family this face's widths were measured in — the one a
    /// renderer must name, or its text runs wider than the fit assumed.
    pub fn family(self) -> &'static str {
        match self {
            Face::Proportional => table::PROPORTIONAL_FAMILY,
            Face::Mono => table::MONO_FAMILY,
        }
    }

    /// How far below the top of a line's em box its baseline sits, as a
    /// fraction of the point size.
    pub fn ascent(self) -> f64 {
        f64::from(match self {
            Face::Proportional => table::PROPORTIONAL_ASCENT,
            Face::Mono => table::MONO_ASCENT,
        }) / 1000.0
    }
}

/// The advance of `c` in `face`, in thousandths of an em.
pub fn advance(face: Face, c: char) -> Option<u16> {
    let runs = match face {
        Face::Proportional => table::PROPORTIONAL,
        Face::Mono => table::MONO,
    };
    let cp = c as u32;
    let i = runs.partition_point(|&(_, last, _)| last < cp);
    runs.get(i)
        .filter(|&&(first, _, _)| first <= cp)
        .map(|&(_, _, width)| width)
}

/// The width of `text` set in `face` at `size_pt` points, in points.
pub fn width_pt(face: Face, text: &str, size_pt: f64) -> Result<f64, Unmeasured> {
    let mut thousandths: u64 = 0;
    for c in text.chars() {
        let a = advance(face, c).ok_or_else(|| Unmeasured {
            character: c,
            text: text.to_string(),
        })?;
        thousandths += u64::from(a);
    }
    Ok(thousandths as f64 * size_pt / 1000.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The table is the font it says it is. A regeneration from another
    /// revision changes this string, and every figure's fit with it — so it
    /// is a decision to see in review, not a drift to discover in print.
    #[test]
    fn the_table_is_pinned_to_one_font_revision() {
        assert_eq!(
            FONT_VERSION,
            "Version 2.004;hotconv 1.0.118;makeotfexe 2.5.65603"
        );
    }

    /// The values the design rests on, read back through the lookup.
    #[test]
    fn measured_advances_come_back_through_the_lookup() {
        // Hangul written as escapes: the probes measure the script, and an
        // escape keeps this file readable to every reader and out of the
        // non-Latin-prose registry (the way that gate writes its own).
        assert_eq!(advance(Face::Proportional, '\u{AC00}'), Some(920));
        assert_eq!(advance(Face::Proportional, '\u{D7A3}'), Some(920));
        assert_eq!(advance(Face::Proportional, 'A'), Some(608));
        assert_eq!(advance(Face::Proportional, 'W'), Some(878));
        assert_eq!(advance(Face::Mono, 'A'), Some(500));
        assert_eq!(advance(Face::Mono, '\u{AC00}'), Some(920));
        // 10 pt: two syllables, a space and `A`.
        let w = width_pt(Face::Proportional, "\u{C7A0}\u{AE08} A", 10.0).expect("measured");
        assert!(
            (w - (920.0 * 2.0 + 224.0 + 608.0) / 100.0).abs() < 1e-9,
            "{w}"
        );
    }

    /// What a specification's values hold is measured in both faces: a unit
    /// (Ω, μ), an operator (≥, ≠), a mark (✓), a Hanja term, a kana. Written as
    /// escapes, like the Hangul probes. Before the table covered them a
    /// document with one in a value was refused whole.
    #[test]
    fn the_symbols_a_specification_holds_are_measured() {
        for (name, c) in [
            ("ohm", '\u{03A9}'),
            ("micro (Greek)", '\u{03BC}'),
            ("micro (Latin-1)", '\u{00B5}'),
            ("greater-or-equal", '\u{2265}'),
            ("not-equal", '\u{2260}'),
            ("infinity", '\u{221E}'),
            ("check mark", '\u{2713}'),
            ("degree Celsius", '\u{2103}'),
            ("Hanja", '\u{6F22}'),
            ("hiragana", '\u{304B}'),
            ("superscript two", '\u{00B2}'),
            ("won sign", '\u{20A9}'),
        ] {
            for face in [Face::Proportional, Face::Mono] {
                let advance = advance(face, c).unwrap_or_else(|| panic!("{name} in {face:?}"));
                assert!(
                    (1..=2000).contains(&advance),
                    "{name} in {face:?}: {advance}"
                );
            }
        }
    }

    /// What the font has no glyph for stays unmeasured, and so stays refused by
    /// name: a colour emoji is not in Noto Sans CJK, and a width for it would
    /// be a guess.
    #[test]
    fn a_character_the_font_does_not_carry_is_still_refused() {
        assert_eq!(advance(Face::Proportional, '\u{1F600}'), None);
        let e = width_pt(Face::Proportional, "ok \u{1F600}", 9.0).unwrap_err();
        assert_eq!(e.character, '\u{1F600}');
    }

    /// A character outside the table is refused, not guessed.
    #[test]
    fn an_unmeasured_character_is_refused() {
        // Cyrillic is outside the ranges the generator reads.
        let e = width_pt(Face::Proportional, "go \u{0416}", 9.0).unwrap_err();
        assert_eq!(e.character, '\u{0416}');
        assert_eq!(advance(Face::Proportional, '\u{0416}'), None);
    }
}
