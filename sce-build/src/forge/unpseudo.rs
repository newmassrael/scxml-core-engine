// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! Reading a pseudocode rendering back into the IR.
//!
//! [`crate::forge::pseudo`] writes it; this reads it. The pair exists
//! to answer one question — *is the text a reviewer approved the same
//! document the compiler consumes* — and nothing else consumes this
//! module's output.
//!
//! # Why this produces a MODEL and not SCXML
//!
//! `rfc-pseudocode-review-surface.md` §13 left this open: whether the
//! reverse direction must write `b.scxml` or may stop at the IR. It
//! stops at the IR, and the cheaper answer is also the more correct
//! one.
//!
//! What has to be established is that the pseudocode and the document
//! say the same thing. Comparing `model(pseudo)` against
//! `model(scxml)` establishes exactly that. Writing `b.scxml` in
//! between adds a second artefact to the chain whose own fidelity would
//! then need its own proof — a longer chain proving the same claim. The
//! consumers that genuinely want SCXML text (the visualizer's own
//! `DOMParser` walk, the XSD, the W3C suite) read the AUTHORED
//! document, never a regenerated one, so none of them is served by it
//! either.
//!
//! # ⚠ This parser only has to accept what the renderer emits
//!
//! Not text a human typed, not a typo, not an unusual indent. That
//! relaxation is what makes the direction affordable at all: there are
//! no diagnostics to design, no recovery, and no stable public grammar
//! to keep. A line it does not recognise is an error, and the error is
//! for the pair's own test to read, not for an author.
//!
//! The grammar is the module comment of [`crate::forge::pseudo`]. Where
//! the two disagree, that one is right and this one is the defect.

use crate::comment_text;
use crate::forge::model::{
    ConditionModel, Direction, EnumModel, EnumVariant, ForgeDocument, ForgeField, SceType,
    TimerModel,
};

/// Why a rendering could not be read back.
///
/// Carries the line so a failing round trip names where the two halves
/// disagree; nothing else reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    /// 1-based line number in the rendering.
    pub line: usize,
    /// What was wrong, in the pair's own terms.
    pub why: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}: {}", self.line, self.why)
    }
}

/// One rendered line: how deep it sits and what it says.
struct Line<'a> {
    number: usize,
    depth: usize,
    text: &'a str,
}

fn lines_of(input: &str) -> Vec<Line<'_>> {
    input
        .lines()
        .enumerate()
        .filter(|(_, l)| !l.trim().is_empty())
        .map(|(i, l)| {
            let indent = l.len() - l.trim_start().len();
            Line {
                number: i + 1,
                // Two spaces per level, which is what `Out::line`
                // writes. An odd indent cannot occur in the renderer's
                // output, so it is not a case this parser handles.
                depth: indent / 2,
                text: l.trim_start(),
            }
        })
        .collect()
}

/// Author text, with the renderer's escaping undone.
fn undo(s: &str, line: usize) -> Result<String, ParseError> {
    comment_text::decode(s).ok_or_else(|| ParseError {
        line,
        why: format!("`{s}` is not text this renderer could have written"),
    })
}

/// Read a rendering back into a document.
///
/// Covers the kinds the round-trip gate exercises today; a head this
/// module does not know is an error rather than a guess, for the same
/// reason the renderer refuses rather than abbreviates.
pub fn parse(input: &str) -> Result<ForgeDocument, ParseError> {
    let lines = lines_of(input);
    let head = lines.first().ok_or_else(|| ParseError {
        line: 0,
        why: "empty rendering".to_string(),
    })?;
    let keyword = head.text.split_whitespace().next().unwrap_or("");
    let body: Vec<&Line<'_>> = lines.iter().skip(1).collect();

    // The three kinds read here are flat: the renderer puts the head at
    // depth 0 and every body line at depth 1. Checking it rather than
    // ignoring it is what makes the indentation load-bearing — a line
    // that drifted a level is a disagreement between the two halves,
    // and this is the only place that can see it.
    if head.depth != 0 {
        return Err(ParseError {
            line: head.number,
            why: "a head must sit at depth 0".to_string(),
        });
    }
    if let Some(bad) = body.iter().find(|l| l.depth != 1) {
        return Err(ParseError {
            line: bad.number,
            why: format!("expected depth 1, found depth {}", bad.depth),
        });
    }

    match keyword {
        "condition" => parse_condition(head, &body).map(ForgeDocument::Condition),
        "timer" => parse_timer(head).map(ForgeDocument::Timer),
        "enum" => parse_enum(head, &body).map(ForgeDocument::Enum),
        other => Err(ParseError {
            line: head.number,
            why: format!("`{other}` is not a kind this reader covers yet"),
        }),
    }
}

/// `<keyword> <name>` — the shape every head starts with.
fn head_name(head: &Line<'_>) -> Result<String, ParseError> {
    let rest = head
        .text
        .split_once(' ')
        .map(|(_, r)| r)
        .ok_or_else(|| ParseError {
            line: head.number,
            why: "a head with no name".to_string(),
        })?;
    undo(rest.split_whitespace().next().unwrap_or(""), head.number)
}

/// `(in|out|internal) <id>: <type> <clause>...`
fn parse_field(line: &Line<'_>) -> Result<ForgeField, ParseError> {
    let mut words = line.text.split_whitespace();
    let dir = match words.next() {
        Some("in") => Direction::In,
        Some("out") => Direction::Out,
        Some("internal") => Direction::Internal,
        _ => {
            return Err(ParseError {
                line: line.number,
                why: "a field must start with in / out / internal".to_string(),
            })
        }
    };
    let id = words
        .next()
        .and_then(|w| w.strip_suffix(':'))
        .ok_or_else(|| ParseError {
            line: line.number,
            why: "a field needs `<id>:`".to_string(),
        })?;
    let type_word = words.next().ok_or_else(|| ParseError {
        line: line.number,
        why: "a field needs a type".to_string(),
    })?;
    let sce_type = SceType::from_attr(type_word).ok_or_else(|| ParseError {
        line: line.number,
        why: format!("`{type_word}` is not an sce:type"),
    })?;

    let mut f = ForgeField {
        id: undo(id, line.number)?,
        sce_type,
        direction: dir,
        expr: None,
        quantity: None,
        max_size: None,
        default_covers: Vec::new(),
        retain: None,
    };

    // Clauses, in the order the renderer writes them. Anything else is
    // an error: a clause silently skipped is exactly the loss the pair
    // exists to detect.
    let rest: Vec<&str> = words.collect();
    let mut i = 0;
    while i < rest.len() {
        match rest[i] {
            "=" => {
                let v = rest[i + 1..].join(" ");
                f.expr = Some(undo(&v, line.number)?);
                break;
            }
            "max-size" => {
                f.max_size = rest.get(i + 1).and_then(|v| v.parse().ok());
                i += 2;
            }
            other => {
                return Err(ParseError {
                    line: line.number,
                    why: format!("`{other}` is not a field clause this reader covers"),
                })
            }
        }
    }
    Ok(f)
}

fn parse_condition(head: &Line<'_>, body: &[&Line<'_>]) -> Result<ConditionModel, ParseError> {
    let mut m = ConditionModel {
        name: head_name(head)?,
        inputs: Vec::new(),
        expr: String::new(),
        source_location: None,
    };
    for l in body {
        if let Some(rest) = l.text.strip_prefix("when ") {
            m.expr = undo(rest, l.number)?;
        } else {
            m.inputs.push(parse_field(l)?);
        }
    }
    Ok(m)
}

/// `timer <name> period <n>us fire <e> [reset-on <e>] [cancel-on-exit <s>]`
fn parse_timer(head: &Line<'_>) -> Result<TimerModel, ParseError> {
    let w: Vec<&str> = head.text.split_whitespace().collect();
    let mut m = TimerModel {
        name: undo(w.get(1).copied().unwrap_or(""), head.number)?,
        period_us: 0,
        reset_on_event: None,
        cancel_on_state_exit: None,
        fire_event: String::new(),
        source_location: None,
    };
    let mut i = 2;
    while i < w.len() {
        match w[i] {
            "period" => {
                let v = w.get(i + 1).and_then(|v| v.strip_suffix("us"));
                m.period_us = v.and_then(|v| v.parse().ok()).ok_or_else(|| ParseError {
                    line: head.number,
                    why: "period must be `<n>us`".to_string(),
                })?;
                i += 2;
            }
            "fire" => {
                m.fire_event = undo(w.get(i + 1).copied().unwrap_or(""), head.number)?;
                i += 2;
            }
            "reset-on" => {
                m.reset_on_event = Some(undo(w.get(i + 1).copied().unwrap_or(""), head.number)?);
                i += 2;
            }
            "cancel-on-exit" => {
                m.cancel_on_state_exit =
                    Some(undo(w.get(i + 1).copied().unwrap_or(""), head.number)?);
                i += 2;
            }
            other => {
                return Err(ParseError {
                    line: head.number,
                    why: format!("`{other}` is not a timer clause"),
                })
            }
        }
    }
    Ok(m)
}

/// `enum <name>: <type> [strict]` with `variant <name> = <n> [@line <n>]`
fn parse_enum(head: &Line<'_>, body: &[&Line<'_>]) -> Result<EnumModel, ParseError> {
    let w: Vec<&str> = head.text.split_whitespace().collect();
    let name = w
        .get(1)
        .and_then(|n| n.strip_suffix(':'))
        .ok_or_else(|| ParseError {
            line: head.number,
            why: "an enum head needs `<name>:`".to_string(),
        })?;
    let type_word = w.get(2).copied().unwrap_or("");
    let underlying_type = SceType::from_attr(type_word).ok_or_else(|| ParseError {
        line: head.number,
        why: format!("`{type_word}` is not an sce:type"),
    })?;

    let mut m = EnumModel {
        name: undo(name, head.number)?,
        underlying_type,
        variants: Vec::new(),
        strict_variants: w.get(3) == Some(&"strict"),
        source_location: None,
    };

    for l in body {
        let w: Vec<&str> = l.text.split_whitespace().collect();
        if w.first() != Some(&"variant") {
            return Err(ParseError {
                line: l.number,
                why: format!("`{}` is not an enum body line", l.text),
            });
        }
        let value = w
            .get(3)
            .and_then(|v| v.parse().ok())
            .ok_or_else(|| ParseError {
                line: l.number,
                why: "a variant needs `= <n>`".to_string(),
            })?;
        let source_line = match (w.get(4), w.get(5)) {
            (Some(&"@line"), Some(n)) => n.parse().ok(),
            _ => None,
        };
        m.variants.push(EnumVariant {
            name: undo(w.get(1).copied().unwrap_or(""), l.number)?,
            value,
            source_line,
        });
    }
    Ok(m)
}

/// Serialised comparison of two documents, with the one key the law
/// excludes stripped wherever it appears.
///
/// ⚠ `source_location` is the ONLY exclusion, and it is stripped by key
/// name at every depth rather than by a list of the places it occurs.
/// A hand-kept list of excluded paths is how a real difference gets
/// filed as an expected one.
pub fn ir_for_comparison(doc: &ForgeDocument) -> Result<serde_json::Value, serde_json::Error> {
    let mut v = serde_json::to_value(doc)?;
    strip(&mut v, "source_location");
    Ok(v)
}

fn strip(v: &mut serde_json::Value, key: &str) {
    match v {
        serde_json::Value::Object(map) => {
            map.remove(key);
            for x in map.values_mut() {
                strip(x, key);
            }
        }
        serde_json::Value::Array(items) => {
            for x in items {
                strip(x, key);
            }
        }
        _ => {}
    }
}

/// The kinds [`parse`] reads back today.
///
/// Published so the round-trip gate derives its worklist instead of
/// keeping a second list beside this one — the two would drift, and the
/// drift would look like coverage.
pub const COVERED_KINDS: &[&str] = &["condition", "timer", "enum"];

/// Whether this document is one the reader covers.
pub fn covers(doc: &ForgeDocument) -> bool {
    let name = match doc {
        ForgeDocument::Condition(_) => "condition",
        ForgeDocument::Timer(_) => "timer",
        ForgeDocument::Enum(_) => "enum",
        _ => return false,
    };
    COVERED_KINDS.contains(&name)
}
