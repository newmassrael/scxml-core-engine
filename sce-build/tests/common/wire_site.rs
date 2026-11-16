// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! Where a diagnostic's `actual` stands, read the way a consumer reads it.
//!
//! `SCE_ERROR_CONTRACT.md` §3.1.1: when a record carries `location.line`,
//! the value in `actual` occurs on that line, beginning at `location.col`
//! when the record carries that too ([`locate_at`]); when it does not, `actual`
//! occurs exactly once in the document, since a whole-file search is then
//! the only locating strategy the wire offers. A substitution is performed
//! on that same site. Each `related` entry that carries `actual` is held to
//! the same rule through [`locate_at`], its column included (§2.4).
//!
//! `diagnostic_fix_is_applicable` holds the tracked corpus to the rule, and
//! the suites that write their own documents hold theirs — refusals no
//! tracked document carries, which the corpus sweep therefore never meets.
//! Both ask here, so a suite cannot pass a record the gate would refuse.
//!
//! ⚠ Measured 2026-09-24: the rule was written out five times — the gate's
//! and four inline copies — and the ECMAScript method refusal had none of
//! them in front of it. Its suite asserted `actual == ".map()"` for a
//! document that wrote `.map(function(w) { … })`, a value that row does
//! not hold, and so pinned the defect instead of catching it.

// Every test binary that declares `mod common` compiles this file, and
// each reads the half it needs — the same reason `repository` carries it.
#![allow(dead_code)]

use std::fmt;

/// Why a consumer holding `actual` and `location.line` cannot find the
/// value where the record says it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Misplaced {
    /// `line` names no row of the document.
    NoSuchRow { line: usize, lines: usize },
    /// The row `line` names does not hold `actual`.
    NotOnTheRow { line: usize, row: String },
    /// The row holds `actual`, and not where `col` says it begins.
    NotAtTheColumn {
        line: usize,
        col: usize,
        row: String,
    },
    /// The record carries no `line`, and the document holds `actual`
    /// other than exactly once — a miss at zero, a coin flip above one.
    NotUnique { hits: usize },
}

impl fmt::Display for Misplaced {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Misplaced::NoSuchRow { line, lines } => {
                write!(f, "line {line} is not a row of a {lines}-line document")
            }
            Misplaced::NotOnTheRow { line, row } => {
                write!(f, "line {line} does not hold it: {:?}", row.trim())
            }
            Misplaced::NotAtTheColumn { line, col, row } => {
                write!(
                    f,
                    "line {line} holds it, and not at column {col}: {:?}",
                    row.trim()
                )
            }
            Misplaced::NotUnique { hits: 0 } => {
                write!(f, "no line is given and the document does not hold it")
            }
            Misplaced::NotUnique { hits } => {
                write!(f, "no line is given and the document holds it {hits} times")
            }
        }
    }
}

/// Can a consumer find `actual` in `text` — on `line` when the record gives
/// one, exactly once in the document when it does not?
pub fn locate(text: &str, line: Option<usize>, actual: &str) -> Result<(), Misplaced> {
    match line {
        Some(line) => {
            let lines: Vec<&str> = text.lines().collect();
            let Some(row) = line.checked_sub(1).and_then(|i| lines.get(i)) else {
                return Err(Misplaced::NoSuchRow {
                    line,
                    lines: lines.len(),
                });
            };
            if row.contains(actual) {
                Ok(())
            } else {
                Err(Misplaced::NotOnTheRow {
                    line,
                    row: (*row).to_string(),
                })
            }
        }
        None => match text.matches(actual).count() {
            1 => Ok(()),
            hits => Err(Misplaced::NotUnique { hits }),
        },
    }
}

/// [`locate`], and, when the record gives a column, `actual` begins at it:
/// `line[col-1..]` starts with `actual`, counting columns from 1 in
/// characters (SCE_ERROR_CONTRACT.md §2.2, §3.1.1).
pub fn locate_at(
    text: &str,
    line: Option<usize>,
    col: Option<usize>,
    actual: &str,
) -> Result<(), Misplaced> {
    locate(text, line, actual)?;
    let (Some(line), Some(col)) = (line, col) else {
        return Ok(());
    };
    let row = text.lines().nth(line - 1).unwrap_or_default();
    let from: String = row.chars().skip(col.saturating_sub(1)).collect();
    if from.starts_with(actual) {
        Ok(())
    } else {
        Err(Misplaced::NotAtTheColumn {
            line,
            col,
            row: row.to_string(),
        })
    }
}

/// §3.1.1 for one wire record against the text of the document it names:
/// `None` when the record carries no `actual`, when the value locates, or
/// when the rule exempts it; the reason otherwise.
///
/// The one exemption is §2.3's: a value a preprocessor assembled carries
/// `expanded_from`, and the row `location` names holds its parameterised
/// shape — `tick_{$n}` where the record reports `tick_1`. A record that
/// ALSO carries a `fix` is judged all the same, since a substitution needs
/// the value on that row and failing to find it is the finding — the rule
/// `diagnostic_fix_is_applicable` has held since before this module.
pub fn record_misplaced(record: &serde_json::Value, text: &str) -> Option<Misplaced> {
    let actual = record["actual"].as_str()?;
    if exempt_as_assembled(
        record.get("expanded_from").is_some(),
        record.get("fix").is_some(),
    ) {
        return None;
    }
    let line = record["location"]["line"].as_u64().map(|n| n as usize);
    let col = record["location"]["col"].as_u64().map(|n| n as usize);
    locate_at(text, line, col, actual).err()
}

/// Whether §2.3 exempts a record's own `actual` from §3.1.1: it names the
/// call site a preprocessor assembled the value from, and proposes no
/// substitution — see [`record_misplaced`].
pub fn exempt_as_assembled(expanded: bool, has_fix: bool) -> bool {
    expanded && !has_fix
}

/// Perform the substitution the way a consumer holding only the wire
/// record would: on the named line when the record gives one, otherwise on
/// the document's single occurrence.
pub fn apply_substitution(
    text: &str,
    actual: &str,
    replacement: &str,
    line: Option<usize>,
) -> String {
    match line {
        Some(n) => {
            let mut out: Vec<String> = Vec::new();
            for (i, l) in text.lines().enumerate() {
                if i + 1 == n {
                    out.push(l.replacen(actual, replacement, 1));
                } else {
                    out.push(l.to_string());
                }
            }
            let mut joined = out.join("\n");
            if text.ends_with('\n') {
                joined.push('\n');
            }
            joined
        }
        None => text.replacen(actual, replacement, 1),
    }
}
