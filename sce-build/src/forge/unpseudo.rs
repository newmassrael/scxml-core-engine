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
    AlgorithmConst, AlgorithmConstType, AlgorithmModel, AlgorithmParam, AlgorithmSignature,
    AlgorithmStmt, AlgorithmValueType, BackpressurePolicy, BitSize, BoundedCollectionModel,
    BufferPoolModel, BufferPoolVariant, CachePolicy, CallArg, CodecField, CodecModel,
    CodecTestVector, CodecVariant, CountRef, DecodedField, DecodedFieldValue, DecodedValue, Endian,
    FlagDef, FlagInput, FoldBody, PeekByteSpec, PresentIfPredicate, PresentIfScope,
    ProcedureAssign, ProcedureDoneParam, ProcedureHelper, ProcedureModel, ProcedureSendAction,
    ProcedureState, ProcedureTransition, TestVector, TestVectorValue, TlvOverflowPolicy,
    TlvTerminateStrategy, VariantArm,
};
use crate::forge::model::{
    CapacitySource, CollectionOrdering, ConcurrencyMode, ConditionModel, Direction, EnumModel,
    EnumVariant, EventSchemaModel, FilterModel, FilterType, ForgeDocument, ForgeField, InboxConfig,
    InboxOrdering, InterpolationAxis, InterpolationMethod, InterpolationModel, LinkClass,
    LinkInboundEvent, LinkModel, LinkOutboundEvent, LookupEntry, LookupModel, MissPolicy,
    ObserverModel, OutOfBounds, OverflowPolicy, RangeRule, RateOfChangeRule, ReassemblyConfig,
    SceType, ThresholdMonitor, TimerModel, TransformModel, ValidatorModel, ValidatorRules,
    WorkerModel,
};
use crate::provenance::RequirementId;

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

/// Split a body into each top-level line and the lines nested under it.
///
/// The tree the nested kinds need, built once here rather than by each
/// parser counting indents for itself. A line's children are the run
/// that follows it at a greater depth; the run ends at the next line
/// back at its own depth or shallower.
fn group<'a>(body: &[&'a Line<'a>]) -> Vec<(&'a Line<'a>, Vec<&'a Line<'a>>)> {
    let mut out: Vec<(&Line<'_>, Vec<&Line<'_>>)> = Vec::new();
    let Some(top) = body.first().map(|l| l.depth) else {
        return out;
    };
    let mut i = 0;
    while i < body.len() {
        let head = body[i];
        let mut kids = Vec::new();
        i += 1;
        while i < body.len() && body[i].depth > top {
            kids.push(body[i]);
            i += 1;
        }
        out.push((head, kids));
    }
    out
}

/// Author text, with the renderer's escaping undone.
fn undo(s: &str, line: usize) -> Result<String, ParseError> {
    comment_text::decode(s).ok_or_else(|| ParseError {
        line,
        why: format!("`{s}` is not text this renderer could have written"),
    })
}

/// One word of a clause, as `crate::forge::pseudo::word` wrote it.
///
/// The escaped space is undone BEFORE `undo`, and it has to be that way
/// round: `comment_text::decode` refuses `\x20`, so running it first
/// would reject every value this layer encoded. The order is
/// unambiguous because a literal backslash in the author's text reaches
/// the output as `\x5C`, so the four characters `\x20` there can only
/// be the escape the renderer wrote.
fn undo_word(s: &str, line: usize) -> Result<String, ParseError> {
    undo(&s.replace(crate::forge::pseudo::WORD_SPACE, " "), line)
}

/// Read a rendering back into a document.
///
/// Covers the kinds the round-trip gate exercises today; a head this
/// module does not know is an error rather than a guess, for the same
/// reason the renderer refuses rather than abbreviates.
pub fn parse(input: &str) -> Result<ForgeDocument, ParseError> {
    let lines = lines_of(input);

    // A derived line came from a deployment, not from a document, so
    // there is no document to read this text back into. Refused here,
    // before any kind's parser runs, because the alternative — dropping
    // the lines and reading the rest — would let a round trip report
    // that it had verified a text part of which it never looked at.
    // `crate::forge::pseudo::DEPLOYMENT_SIGIL` is the one spelling.
    if let Some(l) = lines
        .iter()
        .find(|l| l.text.split_whitespace().next() == Some(crate::forge::pseudo::DEPLOYMENT_SIGIL))
    {
        return Err(ParseError {
            line: l.number,
            why: format!(
                "`{}` starts a line a deployment wrote, and a deployment is not \
                 part of any document — read back the rendering made without one",
                crate::forge::pseudo::DEPLOYMENT_SIGIL
            ),
        });
    }

    let head = lines.first().ok_or_else(|| ParseError {
        line: 0,
        why: "empty rendering".to_string(),
    })?;
    let keyword = head.text.split_whitespace().next().unwrap_or("");
    let body: Vec<&Line<'_>> = lines.iter().skip(1).collect();

    // Indentation carries the structure, so it is checked rather than
    // ignored: a line that drifted a level is a disagreement between
    // the two halves, and this is the only place that can see it.
    if head.depth != 0 {
        return Err(ParseError {
            line: head.number,
            why: "a head must sit at depth 0".to_string(),
        });
    }
    // A line may go one level deeper than the one before it and no
    // more. That is weaker than "every body line sits at depth 1" —
    // which was true while every kind read here was flat — and it is
    // the invariant that survives the nested kinds: a jump of two means
    // the two halves disagree about structure, and nothing else can see
    // it.
    let mut previous = 0usize;
    for l in &body {
        if l.depth > previous + 1 || l.depth == 0 {
            return Err(ParseError {
                line: l.number,
                why: format!("depth {} cannot follow depth {previous}", l.depth),
            });
        }
        previous = l.depth;
    }

    match keyword {
        "condition" => parse_condition(head, &body).map(ForgeDocument::Condition),
        "timer" => parse_timer(head).map(ForgeDocument::Timer),
        "enum" => parse_enum(head, &body).map(ForgeDocument::Enum),
        "transform" => parse_transform(head, &body).map(ForgeDocument::Transform),
        "event-schema" => parse_event_schema(head, &body).map(ForgeDocument::EventSchema),
        "lookup" => parse_lookup(head, &body).map(ForgeDocument::Lookup),
        "validator" => parse_validator(head, &body).map(ForgeDocument::Validator),
        "filter" => parse_filter(head, &body).map(ForgeDocument::Filter),
        "interpolation" => parse_interpolation(head, &body).map(ForgeDocument::Interpolation),
        "bounded-collection" => {
            parse_bounded_collection(head).map(ForgeDocument::BoundedCollection)
        }
        "worker" => parse_worker(head).map(ForgeDocument::Worker),
        "buffer-pool" => parse_buffer_pool(head, &body).map(ForgeDocument::BufferPool),
        "link" => parse_link(head, &body).map(ForgeDocument::Link),
        "observer" => parse_observer(head, &body).map(ForgeDocument::Observer),
        "algorithm" => parse_algorithm(head, &body).map(ForgeDocument::Algorithm),
        "procedure" => parse_procedure(head, &body).map(ForgeDocument::Procedure),
        "codec" => parse_codec(head, &body).map(ForgeDocument::Codec),
        "machine" => parse_statechart(head, &body).map(|m| ForgeDocument::Statechart(Box::new(m))),
        other => Err(ParseError {
            line: head.number,
            why: format!("`{other}` is not a kind this reader covers yet"),
        }),
    }
}

/// `transform <name>` with `in`/`out` fields, split by their keyword.
///
/// The direction is read from the line rather than from position,
/// which is the same choice the renderer made when it printed the
/// keyword instead of relying on which list the field came out of.
fn parse_transform(head: &Line<'_>, body: &[&Line<'_>]) -> Result<TransformModel, ParseError> {
    let mut m = TransformModel {
        name: head_name(head)?,
        inputs: Vec::new(),
        outputs: Vec::new(),
        source_location: None,
    };
    for l in body {
        let f = parse_field(l)?;
        match f.direction {
            Direction::Out => m.outputs.push(f),
            _ => m.inputs.push(f),
        }
    }
    Ok(m)
}

/// `event-schema <name> event <event>`
fn parse_event_schema(head: &Line<'_>, body: &[&Line<'_>]) -> Result<EventSchemaModel, ParseError> {
    let w: Vec<&str> = head.text.split_whitespace().collect();
    if w.get(2) != Some(&"event") {
        return Err(ParseError {
            line: head.number,
            why: "an event-schema head needs `event <name>`".to_string(),
        });
    }
    Ok(EventSchemaModel {
        name: undo(w.get(1).copied().unwrap_or(""), head.number)?,
        event_name: undo(w.get(3).copied().unwrap_or(""), head.number)?,
        fields: body
            .iter()
            .map(|l| parse_field(l))
            .collect::<Result<_, _>>()?,
        source_location: None,
    })
}

/// `lookup <name>` with the input field, the output field, the entries
/// and the miss policy.
fn parse_lookup(head: &Line<'_>, body: &[&Line<'_>]) -> Result<LookupModel, ParseError> {
    let mut fields: Vec<ForgeField> = Vec::new();
    let mut entries: Vec<LookupEntry> = Vec::new();
    let mut miss: Option<MissPolicy> = None;

    for l in body {
        if let Some(rest) = l.text.strip_prefix("miss ") {
            miss = Some(match rest.strip_prefix("default ") {
                Some(v) => MissPolicy::Default(undo(v, l.number)?),
                None if rest == "error" => MissPolicy::Error,
                None => {
                    return Err(ParseError {
                        line: l.number,
                        why: format!("`miss {rest}` is not a miss policy"),
                    })
                }
            });
        } else if let Some((key, rest)) = l.text.split_once(" -> ") {
            let (value, reqs) = match rest.split_once(" req ") {
                Some((v, r)) => (v, r.split_whitespace().collect::<Vec<_>>()),
                None => (rest, Vec::new()),
            };
            entries.push(LookupEntry {
                key: undo(key, l.number)?,
                value: undo(value, l.number)?,
                requirements: reqs
                    .into_iter()
                    .map(|r| undo(r, l.number).map(RequirementId))
                    .collect::<Result<_, _>>()?,
            });
        } else {
            fields.push(parse_field(l)?);
        }
    }

    if fields.len() != 2 {
        return Err(ParseError {
            line: head.number,
            why: format!(
                "a lookup needs an input and an output; found {}",
                fields.len()
            ),
        });
    }
    let miss_policy = miss.ok_or_else(|| ParseError {
        line: head.number,
        why: "a lookup needs a miss policy".to_string(),
    })?;
    let mut it = fields.into_iter();
    Ok(LookupModel {
        name: head_name(head)?,
        input: it.next().expect("two fields"),
        output: it.next().expect("two fields"),
        entries,
        miss_policy,
        source_location: None,
    })
}

/// `validator <name>` with fields and the three rule shapes.
fn parse_validator(head: &Line<'_>, body: &[&Line<'_>]) -> Result<ValidatorModel, ParseError> {
    let mut m = ValidatorModel {
        name: head_name(head)?,
        inputs: Vec::new(),
        rules: ValidatorRules {
            ranges: Vec::new(),
            rate_of_changes: Vec::new(),
            plausibility: None,
            plausibility_spelling: None,
        },
        source_location: None,
    };
    for l in body {
        let w: Vec<&str> = l.text.split_whitespace().collect();
        match w.first().copied() {
            Some("range") => {
                let id = undo(w.get(1).copied().unwrap_or(""), l.number)?;
                // A bound is read as the page spells it — the author's
                // spelling — and valued by its field's type, as the parser
                // values it, so `0x00` reads back as the model's `0` and
                // keeps `0x00` for the next rendering.
                let field_type = m
                    .inputs
                    .iter()
                    .find(|f| f.id == id)
                    .map(|f| f.sce_type.clone());
                let bound = |at: usize| -> Result<(String, String), ParseError> {
                    let spelling = undo(w.get(at).copied().unwrap_or(""), l.number)?;
                    let value = match &field_type {
                        Some(ty) => ty
                            .numeric_literal(&spelling)
                            .map(crate::forge::model::NumericLiteral::to_source)
                            .map_err(|rule| ParseError {
                                line: l.number,
                                why: format!("range bound `{spelling}` is not {rule}"),
                            })?,
                        None => spelling.clone(),
                    };
                    Ok((value, spelling))
                };
                let mut r = RangeRule {
                    id,
                    min: None,
                    max: None,
                    min_text: String::new(),
                    max_text: String::new(),
                };
                let mut i = 2;
                while i < w.len() {
                    match w[i] {
                        "min" => {
                            let (value, spelling) = bound(i + 1)?;
                            r.min = Some(value);
                            r.min_text = spelling;
                        }
                        "max" => {
                            let (value, spelling) = bound(i + 1)?;
                            r.max = Some(value);
                            r.max_text = spelling;
                        }
                        other => {
                            return Err(ParseError {
                                line: l.number,
                                why: format!("`{other}` is not a range clause"),
                            })
                        }
                    }
                    i += 2;
                }
                m.rules.ranges.push(r);
            }
            Some("rate") => {
                let interval = w
                    .get(5)
                    .and_then(|v| v.strip_suffix("ms"))
                    .and_then(|v| v.parse().ok())
                    .ok_or_else(|| ParseError {
                        line: l.number,
                        why: "a rate needs `interval <n>ms`".to_string(),
                    })?;
                m.rules.rate_of_changes.push(RateOfChangeRule {
                    id: undo(w.get(1).copied().unwrap_or(""), l.number)?,
                    max_delta: undo(w.get(3).copied().unwrap_or(""), l.number)?,
                    sample_interval_ms: interval,
                });
            }
            Some("plausibility") => {
                let rest = l.text.strip_prefix("plausibility ").unwrap_or("");
                m.rules.plausibility = Some(undo(rest, l.number)?);
            }
            _ => m.inputs.push(parse_field(l)?),
        }
    }
    Ok(m)
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

/// A rational as [`Rational`](crate::forge::quantity::Rational)'s own
/// `Display` writes it: `n` when the denominator is one, else `n/d`.
///
/// The fields are private, so the way back is `Rational::new`, which
/// also re-canonicalises — a value that reduces differently than it was
/// written comes back as the same rational, which is what the model
/// holds anyway.
fn rational(s: &str, line: usize) -> Result<crate::forge::quantity::Rational, ParseError> {
    let bad = || ParseError {
        line,
        why: format!("`{s}` is not a rational this renderer could have written"),
    };
    let (num, denom) = match s.split_once('/') {
        Some((n, d)) => (
            n.parse::<i64>().map_err(|_| bad())?,
            d.parse::<i64>().map_err(|_| bad())?,
        ),
        None => (s.parse::<i64>().map_err(|_| bad())?, 1),
    };
    crate::forge::quantity::Rational::new(num, denom).ok_or_else(bad)
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
        // A pseudocode line is not a row of the SCXML document.
        expr_spelling: None,
        expr_splices: None,
        quantity: None,
        max_size: None,
        default_covers: Vec::new(),
        retain: None,
        initial: None,
        initial_spelling: None,
    };

    // ⚠ The expression is taken from the LINE, not rebuilt from its
    // words. `rest[i + 1..].join(" ")` collapses every run of spaces
    // to one, and an expression is author text: a document aligning
    // the arms of a nested conditional came back with its alignment
    // gone, so the page was right and the read-back was not. The
    // clauses before `=` are all single words, and an id cannot hold a
    // space, so the first ` = ` in the line is the separator whatever
    // the expression contains after it.
    if let Some((_, expr)) = line.text.split_once(" = ") {
        f.expr = Some(undo(expr, line.number)?);
    }

    // Clauses, in the order the renderer writes them. Anything else is
    // an error: a clause silently skipped is exactly the loss the pair
    // exists to detect.
    let rest: Vec<&str> = words.collect();
    let mut i = 0;
    while i < rest.len() {
        match rest[i] {
            "=" => break,
            "max-size" => {
                f.max_size = rest.get(i + 1).and_then(|v| v.parse().ok());
                i += 2;
            }
            "quantity" => {
                let scale = rational(rest.get(i + 1).copied().unwrap_or(""), line.number)?;
                let offset = rational(rest.get(i + 2).copied().unwrap_or(""), line.number)?;
                let unit = crate::forge::quantity::UnitTag::intern(&undo_word(
                    rest.get(i + 3).copied().unwrap_or(""),
                    line.number,
                )?);
                f.quantity = Some(crate::forge::quantity::Quantity {
                    scale,
                    offset,
                    unit,
                });
                i += 4;
            }
            // `retain <scope>` and `initial <value>` — each attribute its
            // own clause, as the renderer writes them. A retained field
            // carries both (`retain nvm initial 7`); a field a transform
            // reads through `previous()` carries `initial` alone.
            //
            // ⚠ This reader used to take `initial` only as the second half
            // of `retain`, so the page of a document whose field declared
            // a first value for `previous()` could not be read back — its
            // own renderer's output, refused. Whether a pair is COMPLETE
            // (a retained field with no first value) is validation's to
            // say, and it does; a reader that decides it too refuses
            // pages rather than documents.
            "retain" => {
                f.retain = Some(undo_word(
                    rest.get(i + 1).copied().unwrap_or(""),
                    line.number,
                )?);
                i += 2;
            }
            "initial" => {
                f.initial = Some(undo_word(
                    rest.get(i + 1).copied().unwrap_or(""),
                    line.number,
                )?);
                i += 2;
            }
            // `default-covers <a> <b> ...` — variadic, so it runs to the
            // end of the word-shaped clauses. It is written last among
            // them for exactly that reason, and it stops at `=` because
            // the expression that may follow is not one of its members.
            "default-covers" => {
                let end = rest[i + 1..]
                    .iter()
                    .position(|w| *w == "=")
                    .map_or(rest.len(), |at| i + 1 + at);
                f.default_covers = rest[i + 1..end]
                    .iter()
                    .map(|c| undo_word(c, line.number))
                    .collect::<Result<_, _>>()?;
                i = end;
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
        expr_spelling: None,
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
        // ⚠ The spelling comes back too. The page carries the wire key
        // the way the author wrote it — hex, wherever the protocol's
        // own table is hex — so a reader that took only decimal would
        // fail on exactly the documents the spelling exists for.
        let spelling = w.get(3).copied().ok_or_else(|| ParseError {
            line: l.number,
            why: "a variant needs `= <n>`".to_string(),
        })?;
        // The parser's own reader, not a second one here: two answers
        // to "what is a variant value" is how the pair drifts apart.
        let value =
            crate::forge::parser::parse_variant_value(spelling).ok_or_else(|| ParseError {
                line: l.number,
                why: format!("`{spelling}` is not a value this renderer could have written"),
            })?;
        let source_line = match (w.get(4), w.get(5)) {
            (Some(&"@line"), Some(n)) => n.parse().ok(),
            _ => None,
        };
        m.variants.push(EnumVariant {
            name: undo(w.get(1).copied().unwrap_or(""), l.number)?,
            value,
            value_text: spelling.to_string(),
            source_line,
        });
    }
    Ok(m)
}

/// An `f64` as the renderer's `num` wrote it.
fn number(s: &str, line: usize) -> Result<f64, ParseError> {
    s.parse().map_err(|_| ParseError {
        line,
        why: format!("`{s}` is not a number this renderer could have written"),
    })
}

/// `filter <name> <type> [window <n>] [alpha <f>]` with two fields.
fn parse_filter(head: &Line<'_>, body: &[&Line<'_>]) -> Result<FilterModel, ParseError> {
    let w: Vec<&str> = head.text.split_whitespace().collect();
    let filter_type = match w.get(2).copied() {
        Some("moving-average") => FilterType::MovingAverage,
        Some("low-pass") => FilterType::LowPass,
        Some("debounce") => FilterType::Debounce,
        other => {
            return Err(ParseError {
                line: head.number,
                why: format!("`{}` is not a filter type", other.unwrap_or("")),
            })
        }
    };
    let mut window = None;
    let mut alpha = None;
    let mut i = 3;
    while i < w.len() {
        match w[i] {
            "window" => window = w.get(i + 1).and_then(|v| v.parse().ok()),
            "alpha" => alpha = Some(number(w.get(i + 1).copied().unwrap_or(""), head.number)?),
            other => {
                return Err(ParseError {
                    line: head.number,
                    why: format!("`{other}` is not a filter clause"),
                })
            }
        }
        i += 2;
    }
    let fields: Vec<ForgeField> = body
        .iter()
        .map(|l| parse_field(l))
        .collect::<Result<_, _>>()?;
    if fields.len() != 2 {
        return Err(ParseError {
            line: head.number,
            why: format!(
                "a filter needs an input and an output; found {}",
                fields.len()
            ),
        });
    }
    let mut it = fields.into_iter();
    Ok(FilterModel {
        name: undo(w.get(1).copied().unwrap_or(""), head.number)?,
        input: it.next().expect("two fields"),
        output: it.next().expect("two fields"),
        filter_type,
        window,
        alpha,
        source_location: None,
    })
}

/// `interpolation <name> method <m> out-of-bounds <o>` with fields,
/// axes and the value grid.
fn parse_interpolation(
    head: &Line<'_>,
    body: &[&Line<'_>],
) -> Result<InterpolationModel, ParseError> {
    let w: Vec<&str> = head.text.split_whitespace().collect();
    let method = match w.get(3).copied() {
        Some("linear") => InterpolationMethod::Linear,
        Some("bilinear") => InterpolationMethod::Bilinear,
        other => {
            return Err(ParseError {
                line: head.number,
                why: format!("`{}` is not an interpolation method", other.unwrap_or("")),
            })
        }
    };
    let out_of_bounds = match w.get(5).copied() {
        Some("clamp") => OutOfBounds::Clamp,
        Some("extrapolate") => OutOfBounds::Extrapolate,
        Some("error") => OutOfBounds::Error,
        other => {
            return Err(ParseError {
                line: head.number,
                why: format!("`{}` is not an out-of-bounds policy", other.unwrap_or("")),
            })
        }
    };

    let mut fields: Vec<ForgeField> = Vec::new();
    let mut axes: Vec<InterpolationAxis> = Vec::new();
    let mut values: Vec<f64> = Vec::new();
    for l in body {
        let lw: Vec<&str> = l.text.split_whitespace().collect();
        match lw.first().copied() {
            Some("axis") => {
                if lw.get(2) != Some(&"breakpoints") {
                    return Err(ParseError {
                        line: l.number,
                        why: "an axis needs `breakpoints`".to_string(),
                    });
                }
                axes.push(InterpolationAxis {
                    input_id: undo(lw.get(1).copied().unwrap_or(""), l.number)?,
                    breakpoints: lw[3..]
                        .iter()
                        .map(|v| number(v, l.number))
                        .collect::<Result<_, _>>()?,
                });
            }
            Some("values") => {
                values = lw[1..]
                    .iter()
                    .map(|v| number(v, l.number))
                    .collect::<Result<_, _>>()?;
            }
            _ => fields.push(parse_field(l)?),
        }
    }
    let output = fields.pop().ok_or_else(|| ParseError {
        line: head.number,
        why: "an interpolation needs an output field".to_string(),
    })?;
    Ok(InterpolationModel {
        name: undo(w.get(1).copied().unwrap_or(""), head.number)?,
        inputs: fields,
        output,
        method,
        out_of_bounds,
        axes,
        values,
        source_location: None,
    })
}

/// One line: `bounded-collection <name> of <t> capacity … overflow …
/// ordering … concurrency … [index-by <f>]`.
fn parse_bounded_collection(head: &Line<'_>) -> Result<BoundedCollectionModel, ParseError> {
    let w: Vec<&str> = head.text.split_whitespace().collect();
    let mut m = BoundedCollectionModel {
        name: undo(w.get(1).copied().unwrap_or(""), head.number)?,
        element_type: String::new(),
        capacity: CapacitySource::CompileConst { value: 0 },
        index_by: None,
        on_overflow: OverflowPolicy::Reject,
        ordering: CollectionOrdering::Insertion,
        concurrency: ConcurrencyMode::SingleWriter,
        source_location: None,
    };
    let mut i = 2;
    while i < w.len() {
        let value = w.get(i + 1).copied().unwrap_or("");
        match w[i] {
            "of" => m.element_type = undo(value, head.number)?,
            "capacity" => {
                m.capacity = match value {
                    "deploy-key" => CapacitySource::DeployKey {
                        key: undo(w.get(i + 2).copied().unwrap_or(""), head.number)?,
                    },
                    "const" => CapacitySource::CompileConst {
                        value: w.get(i + 2).and_then(|v| v.parse().ok()).unwrap_or(0),
                    },
                    other => {
                        return Err(ParseError {
                            line: head.number,
                            why: format!("`{other}` is not a capacity source"),
                        })
                    }
                };
                i += 1;
            }
            "overflow" => {
                m.on_overflow = match value {
                    "diagnostic-event" => OverflowPolicy::DiagnosticEvent,
                    "reject" => OverflowPolicy::Reject,
                    "oldest-wins" => OverflowPolicy::OldestWins,
                    other => {
                        return Err(ParseError {
                            line: head.number,
                            why: format!("`{other}` is not an overflow policy"),
                        })
                    }
                }
            }
            "ordering" => {
                m.ordering = match value {
                    "insertion" => CollectionOrdering::Insertion,
                    "sorted-by-index" => CollectionOrdering::SortedByIndex,
                    other => {
                        return Err(ParseError {
                            line: head.number,
                            why: format!("`{other}` is not an ordering"),
                        })
                    }
                }
            }
            "concurrency" => {
                m.concurrency = match value {
                    "single-writer" => ConcurrencyMode::SingleWriter,
                    "multi-writer" => ConcurrencyMode::MultiWriter,
                    other => {
                        return Err(ParseError {
                            line: head.number,
                            why: format!("`{other}` is not a concurrency mode"),
                        })
                    }
                }
            }
            "index-by" => m.index_by = Some(undo(value, head.number)?),
            other => {
                return Err(ParseError {
                    line: head.number,
                    why: format!("`{other}` is not a bounded-collection clause"),
                })
            }
        }
        i += 2;
    }
    Ok(m)
}

/// One line: `worker <name> link-rx <l> inbox depth <n> ordering <o>
/// [outbox <x>]`.
fn parse_worker(head: &Line<'_>) -> Result<WorkerModel, ParseError> {
    let w: Vec<&str> = head.text.split_whitespace().collect();
    let mut m = WorkerModel {
        name: undo(w.get(1).copied().unwrap_or(""), head.number)?,
        link_rx: String::new(),
        inbox: InboxConfig {
            depth: 0,
            ordering: InboxOrdering::Relaxed,
        },
        outbox: None,
        source_location: None,
    };
    let mut i = 2;
    while i < w.len() {
        let value = w.get(i + 1).copied().unwrap_or("");
        match w[i] {
            "link-rx" => m.link_rx = undo(value, head.number)?,
            "inbox" => {
                if value != "depth" {
                    return Err(ParseError {
                        line: head.number,
                        why: "inbox needs `depth <n>`".to_string(),
                    });
                }
                m.inbox.depth = w.get(i + 2).and_then(|v| v.parse().ok()).unwrap_or(0);
                i += 1;
            }
            "ordering" => {
                m.inbox.ordering = match value {
                    "acq_rel" => InboxOrdering::AcqRel,
                    "relaxed" => InboxOrdering::Relaxed,
                    other => {
                        return Err(ParseError {
                            line: head.number,
                            why: format!("`{other}` is not an inbox ordering"),
                        })
                    }
                }
            }
            "outbox" => m.outbox = Some(undo(value, head.number)?),
            other => {
                return Err(ParseError {
                    line: head.number,
                    why: format!("`{other}` is not a worker clause"),
                })
            }
        }
        i += 2;
    }
    Ok(m)
}

/// `buffer-pool <name> slots … size … section … align … cache …
/// [dma …]`, with an optional `reassembly` line under it.
fn parse_buffer_pool(head: &Line<'_>, body: &[&Line<'_>]) -> Result<BufferPoolModel, ParseError> {
    let w: Vec<&str> = head.text.split_whitespace().collect();
    let mut m = BufferPoolModel {
        name: undo(w.get(1).copied().unwrap_or(""), head.number)?,
        slot_count: 0,
        slot_size: 0,
        section: String::new(),
        alignment: 0,
        dma_channel: None,
        cache_policy: CachePolicy::None,
        variant: BufferPoolVariant::Default,
        source_location: None,
    };
    let mut i = 2;
    while i < w.len() {
        let value = w.get(i + 1).copied().unwrap_or("");
        match w[i] {
            "slots" => m.slot_count = value.parse().unwrap_or(0),
            "size" => m.slot_size = value.parse().unwrap_or(0),
            "section" => m.section = undo(value, head.number)?,
            "align" => m.alignment = value.parse().unwrap_or(0),
            "dma" => m.dma_channel = Some(undo(value, head.number)?),
            "cache" => {
                m.cache_policy = match value {
                    "maintain" => CachePolicy::Maintain,
                    "non-cacheable" => CachePolicy::NonCacheable,
                    "none" => CachePolicy::None,
                    other => {
                        return Err(ParseError {
                            line: head.number,
                            why: format!("`{other}` is not a cache policy"),
                        })
                    }
                }
            }
            other => {
                return Err(ParseError {
                    line: head.number,
                    why: format!("`{other}` is not a buffer-pool clause"),
                })
            }
        }
        i += 2;
    }
    for l in body {
        let lw: Vec<&str> = l.text.split_whitespace().collect();
        if lw.first() != Some(&"reassembly") {
            return Err(ParseError {
                line: l.number,
                why: format!("`{}` is not a buffer-pool body line", l.text),
            });
        }
        m.variant = BufferPoolVariant::Reassembly(ReassemblyConfig {
            max_fragments_per_message: lw.get(2).and_then(|v| v.parse().ok()).unwrap_or(0),
            reassembly_timeout_ms: lw
                .get(4)
                .and_then(|v| v.strip_suffix("ms"))
                .and_then(|v| v.parse().ok())
                .unwrap_or(0),
            per_peer_quota: lw.get(6).and_then(|v| v.parse().ok()).unwrap_or(0),
        });
    }
    Ok(m)
}

/// `link <name> class <c> framer <f> backpressure <p>
/// [accept-stage-copy-rate]` with pool and event lines under it.
fn parse_link(head: &Line<'_>, body: &[&Line<'_>]) -> Result<LinkModel, ParseError> {
    let w: Vec<&str> = head.text.split_whitespace().collect();
    let mut m = LinkModel {
        name: undo(w.get(1).copied().unwrap_or(""), head.number)?,
        class: LinkClass::Udp,
        framer: String::new(),
        backpressure: BackpressurePolicy::Drop,
        inbound: Vec::new(),
        outbound: Vec::new(),
        rx_pool: None,
        tx_pool: None,
        stage_pool: None,
        accept_stage_copy_rate: false,
        source_location: None,
    };
    let mut i = 2;
    while i < w.len() {
        let value = w.get(i + 1).copied().unwrap_or("");
        match w[i] {
            "class" => {
                m.class = match value {
                    "udp" => LinkClass::Udp,
                    "tcp" => LinkClass::Tcp,
                    "serial" => LinkClass::Serial,
                    "websocket" => LinkClass::Websocket,
                    "raw_eth" => LinkClass::RawEth,
                    other => {
                        return Err(ParseError {
                            line: head.number,
                            why: format!("`{other}` is not a link class"),
                        })
                    }
                }
            }
            "framer" => m.framer = undo(value, head.number)?,
            "backpressure" => {
                m.backpressure = match value {
                    "drop" => BackpressurePolicy::Drop,
                    "block" => BackpressurePolicy::Block,
                    "signal-event" => BackpressurePolicy::SignalEvent,
                    other => {
                        return Err(ParseError {
                            line: head.number,
                            why: format!("`{other}` is not a backpressure policy"),
                        })
                    }
                }
            }
            "accept-stage-copy-rate" => {
                m.accept_stage_copy_rate = true;
                i -= 1;
            }
            other => {
                return Err(ParseError {
                    line: head.number,
                    why: format!("`{other}` is not a link clause"),
                })
            }
        }
        i += 2;
    }

    for l in body {
        let lw: Vec<&str> = l.text.split_whitespace().collect();
        match lw.first().copied() {
            Some("rx-pool") => m.rx_pool = Some(undo(lw.get(1).copied().unwrap_or(""), l.number)?),
            Some("tx-pool") => m.tx_pool = Some(undo(lw.get(1).copied().unwrap_or(""), l.number)?),
            Some("stage-pool") => {
                m.stage_pool = Some(undo(lw.get(1).copied().unwrap_or(""), l.number)?)
            }
            Some("inbound") => {
                let rest = l.text.strip_prefix("inbound ").unwrap_or("");
                let (event, when) = match rest.split_once(" when ") {
                    Some((e, w)) => (e, Some(undo(w, l.number)?)),
                    None => (rest, None),
                };
                m.inbound.push(LinkInboundEvent {
                    event: undo(event, l.number)?,
                    when,
                });
            }
            Some("outbound") => {
                let rest = l.text.strip_prefix("outbound ").unwrap_or("");
                let (event, encode) = rest.split_once(" encode ").ok_or_else(|| ParseError {
                    line: l.number,
                    why: "an outbound needs `encode <e>`".to_string(),
                })?;
                m.outbound.push(LinkOutboundEvent {
                    event: undo(event, l.number)?,
                    encode: undo(encode, l.number)?,
                });
            }
            _ => {
                return Err(ParseError {
                    line: l.number,
                    why: format!("`{}` is not a link body line", l.text),
                })
            }
        }
    }
    Ok(m)
}

/// `observer <name> [domain <d>]` with fields and monitor blocks.
///
/// The first kind read here that is not flat, and the first user of
/// [`group`]. A monitor is a block because two of its four values are
/// author expressions; see the renderer's note.
fn parse_observer(head: &Line<'_>, body: &[&Line<'_>]) -> Result<ObserverModel, ParseError> {
    let w: Vec<&str> = head.text.split_whitespace().collect();
    let event_domain = match (w.get(2), w.get(3)) {
        (Some(&"domain"), Some(d)) => Some(undo(d, head.number)?),
        (None, _) => None,
        (Some(other), _) => {
            return Err(ParseError {
                line: head.number,
                why: format!("`{other}` is not an observer clause"),
            })
        }
    };

    let mut m = ObserverModel {
        name: undo(w.get(1).copied().unwrap_or(""), head.number)?,
        inputs: Vec::new(),
        monitors: Vec::new(),
        event_domain,
        source_location: None,
    };

    for (line, kids) in group(body) {
        let Some(id) = line
            .text
            .strip_prefix("monitor ")
            .and_then(|r| r.strip_suffix(':'))
        else {
            m.inputs.push(parse_field(line)?);
            continue;
        };
        let mut mon = ThresholdMonitor {
            id: undo(id, line.number)?,
            enter_expr: String::new(),
            enter_spelling: None,
            leave_expr: None,
            leave_spelling: None,
            on_enter: String::new(),
            on_leave: None,
        };
        for k in kids {
            let (keyword, value) = k.text.split_once(' ').ok_or_else(|| ParseError {
                line: k.number,
                why: format!("`{}` is not a monitor clause", k.text),
            })?;
            match keyword {
                "on-enter" => mon.on_enter = undo(value, k.number)?,
                "on-leave" => mon.on_leave = Some(undo(value, k.number)?),
                "enter" => mon.enter_expr = undo(value, k.number)?,
                "leave" => mon.leave_expr = Some(undo(value, k.number)?),
                other => {
                    return Err(ParseError {
                        line: k.number,
                        why: format!("`{other}` is not a monitor clause"),
                    })
                }
            }
        }
        m.monitors.push(mon);
    }
    Ok(m)
}

/// `algorithm <name>(<p>: <t>, …) [-> <t> [returns-max <n>]]`.
///
/// The first kind whose body nests arbitrarily deep, so [`group`] is
/// applied recursively rather than once.
fn parse_algorithm(head: &Line<'_>, body: &[&Line<'_>]) -> Result<AlgorithmModel, ParseError> {
    let (name, rest) = head.text["algorithm ".len()..]
        .split_once('(')
        .ok_or_else(|| ParseError {
            line: head.number,
            why: "an algorithm head needs a parameter list".to_string(),
        })?;
    let (params_text, tail) = rest.rsplit_once(')').ok_or_else(|| ParseError {
        line: head.number,
        why: "an algorithm head needs a closing `)`".to_string(),
    })?;

    let mut params = Vec::new();
    for p in params_text
        .split(',')
        .map(str::trim)
        .filter(|p| !p.is_empty())
    {
        let (pname, ptype) = p.split_once(": ").ok_or_else(|| ParseError {
            line: head.number,
            why: format!("`{p}` is not `<name>: <type>`"),
        })?;
        params.push(AlgorithmParam {
            name: undo(pname, head.number)?,
            sce_type: AlgorithmValueType::from_attr(ptype).ok_or_else(|| ParseError {
                line: head.number,
                why: format!("`{ptype}` is not an sce:type"),
            })?,
            type_spelling: None,
        });
    }

    let tail: Vec<&str> = tail.split_whitespace().collect();
    let (return_type, returns_max_size) = match tail.first().copied() {
        None => (None, None),
        Some("->") => {
            let t = tail.get(1).copied().unwrap_or("");
            let rt = AlgorithmValueType::from_attr(t).ok_or_else(|| ParseError {
                line: head.number,
                why: format!("`{t}` is not an sce:type"),
            })?;
            let max = match tail.get(2).copied() {
                Some("returns-max") => tail.get(3).and_then(|v| v.parse().ok()),
                None => None,
                Some(other) => {
                    return Err(ParseError {
                        line: head.number,
                        why: format!("`{other}` is not a signature clause"),
                    })
                }
            };
            (Some(rt), max)
        }
        Some(other) => {
            return Err(ParseError {
                line: head.number,
                why: format!("`{other}` is not a signature clause"),
            })
        }
    };

    let mut m = AlgorithmModel {
        name: undo(name, head.number)?,
        signature: AlgorithmSignature {
            params,
            return_type,
            returns_max_size,
        },
        consts: Vec::new(),
        body: Vec::new(),
        test_vectors: Vec::new(),
        source_location: None,
    };

    // Consts and test vectors bracket the body, so they are pulled out
    // first and the rest goes through the one statement walker — which
    // is also what folds `else:` back into its `if`.
    let mut statements: Vec<&Line<'_>> = Vec::new();
    for (line, kids) in group(body) {
        if line.text.starts_with("const ") {
            m.consts.push(parse_const(line, &kids)?);
        } else if line.text.starts_with("test ") {
            m.test_vectors.push(parse_test_vector(line)?);
        } else {
            statements.push(line);
            statements.extend(kids);
        }
    }
    m.body = parse_stmt_list(&statements)?;
    Ok(m)
}

/// `const <name>: <type> = <expr>` or the `fold` block form.
fn parse_const(line: &Line<'_>, kids: &[&Line<'_>]) -> Result<AlgorithmConst, ParseError> {
    let rest = &line.text["const ".len()..];
    let (name, after) = rest.split_once(": ").ok_or_else(|| ParseError {
        line: line.number,
        why: "a const needs `<name>: <type>`".to_string(),
    })?;
    let (type_text, value) = after.split_once(" = ").ok_or_else(|| ParseError {
        line: line.number,
        why: "a const needs `= <value>`".to_string(),
    })?;

    if let Some(spec) = type_text.strip_prefix("array<") {
        let (elem, len) =
            spec.trim_end_matches('>')
                .split_once(", ")
                .ok_or_else(|| ParseError {
                    line: line.number,
                    why: "an array const needs `array<<type>, <n>>`".to_string(),
                })?;
        let w: Vec<&str> = value.split_whitespace().collect();
        // `fold <var> in <a>..<b> -> <type>:`
        let (start, end) = w
            .get(3)
            .and_then(|r| r.split_once(".."))
            .ok_or_else(|| ParseError {
                line: line.number,
                why: "a fold needs `<a>..<b>`".to_string(),
            })?;
        let elem_type_word = w.get(5).copied().unwrap_or("").trim_end_matches(':');
        let mut fold = FoldBody {
            range_start: start.parse().unwrap_or(0),
            range_end: end.parse().unwrap_or(0),
            iter_var: undo(w.get(1).copied().unwrap_or(""), line.number)?,
            elem_type: SceType::from_attr(elem_type_word).ok_or_else(|| ParseError {
                line: line.number,
                why: format!("`{elem_type_word}` is not an sce:type"),
            })?,
            body: Vec::new(),
            yield_expr: String::new(),
            // A pseudo line is not an attribute of any SCXML document.
            yield_spelling: None,
        };
        for (l, k) in group(kids) {
            if let Some(y) = l.text.strip_prefix("yield ") {
                fold.yield_expr = undo(y, l.number)?;
            } else {
                fold.body.push(parse_stmt(l, &k)?);
            }
        }
        return Ok(AlgorithmConst {
            name: undo(name, line.number)?,
            sce_type: AlgorithmConstType::Array {
                elem: SceType::from_attr(elem).ok_or_else(|| ParseError {
                    line: line.number,
                    why: format!("`{elem}` is not an sce:type"),
                })?,
                len: len.parse().unwrap_or(0),
            },
            init: None,
            init_spelling: None,
            fold: Some(fold),
            compute_at_build: true,
            // A pseudo line is not a row of any SCXML document.
            line: None,
        });
    }

    Ok(AlgorithmConst {
        name: undo(name, line.number)?,
        sce_type: AlgorithmConstType::Scalar(SceType::from_attr(type_text).ok_or_else(|| {
            ParseError {
                line: line.number,
                why: format!("`{type_text}` is not an sce:type"),
            }
        })?),
        init: Some(undo(value, line.number)?),
        init_spelling: None,
        fold: None,
        compute_at_build: false,
        line: None,
    })
}

/// `test <hex> -> (bool|uint|int) <literal> @line <n>`
fn parse_test_vector(line: &Line<'_>) -> Result<TestVector, ParseError> {
    let w: Vec<&str> = line.text.split_whitespace().collect();
    let hex_text = w
        .get(1)
        .and_then(|h| h.strip_prefix("0x"))
        .ok_or_else(|| ParseError {
            line: line.number,
            why: "a test vector needs `0x<hex>`".to_string(),
        })?;
    let hex = (0..hex_text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex_text[i..i + 2], 16))
        .collect::<Result<Vec<u8>, _>>()
        .map_err(|_| ParseError {
            line: line.number,
            why: "a test vector's hex is not hex".to_string(),
        })?;
    let literal = w.get(4).copied().unwrap_or("");
    let bad = || ParseError {
        line: line.number,
        why: format!("`{literal}` is not a test-vector literal"),
    };
    // ⚠ Through `source_literal`: the renderer writes the author's
    // spelling, so a hex expected value arrives here as `0x29B1`.
    let value = match w.get(3).copied() {
        Some("bool") => TestVectorValue::Bool(literal == "true"),
        Some("uint") => {
            TestVectorValue::Uint(crate::source_literal::read_unsigned(literal).ok_or_else(bad)?)
        }
        Some("int") => {
            TestVectorValue::Int(crate::source_literal::read_signed(literal).ok_or_else(bad)?)
        }
        _ => return Err(bad()),
    };
    Ok(TestVector {
        hex,
        value_text: match w.get(3).copied() {
            Some("uint") | Some("int") => literal.to_string(),
            _ => String::new(),
        },
        value,
        source_line: w.get(6).and_then(|v| v.parse().ok()).unwrap_or(0),
    })
}

/// A run of statements, with `else:` folded into the `if` before it.
///
/// ⚠ The renderer writes `else:` as a SIBLING of its `if`, not as a
/// child, because that is how the block reads. So the one place that
/// walks a statement list is also the only place that can put the two
/// back together — doing it inside `parse_stmt` would need the previous
/// statement, which a per-line parser does not have.
fn parse_stmt_list(body: &[&Line<'_>]) -> Result<Vec<AlgorithmStmt>, ParseError> {
    let mut out: Vec<AlgorithmStmt> = Vec::new();
    for (line, kids) in group(body) {
        if line.text == "else:" {
            let Some(AlgorithmStmt::If { else_body, .. }) = out.last_mut() else {
                return Err(ParseError {
                    line: line.number,
                    why: "an `else:` with no `if` before it".to_string(),
                });
            };
            *else_body = Some(parse_stmt_list(&kids)?);
            continue;
        }
        out.push(parse_stmt(line, &kids)?);
    }
    Ok(out)
}

/// One statement, with the lines nested under it.
fn parse_stmt(line: &Line<'_>, kids: &[&Line<'_>]) -> Result<AlgorithmStmt, ParseError> {
    let t = line.text;
    let body_of = parse_stmt_list;

    if let Some(rest) = t.strip_prefix("var ") {
        let (name, after) = rest.split_once(": ").ok_or_else(|| ParseError {
            line: line.number,
            why: "a var needs `<name>: <type>`".to_string(),
        })?;
        // A bytes buffer is written without `= <expr>` — it starts empty —
        // and every other local with one, the split the parser enforces.
        let (decl, init) = match after.split_once(" = ") {
            Some((decl, init)) => (decl, Some(init)),
            None => (after, None),
        };
        let mut w = decl.split_whitespace();
        let type_word = w.next().unwrap_or("");
        let capacity = match (w.next(), w.next()) {
            (Some("cap"), Some(n)) => n.parse().ok(),
            _ => None,
        };
        let sce_type = AlgorithmValueType::from_attr(type_word).ok_or_else(|| ParseError {
            line: line.number,
            why: format!("`{type_word}` is not an sce:type"),
        })?;
        // A record local is its declaration plus one nested `field = expr`
        // line per field — the shape `pseudo` writes.
        if let Some(alias) = sce_type.record_alias() {
            if init.is_some() {
                return Err(ParseError {
                    line: line.number,
                    why: "a record var has no `= <expr>`; its fields are the lines under it"
                        .to_string(),
                });
            }
            let mut fields = Vec::new();
            for kid in kids {
                let (field, expr) = kid.text.split_once(" = ").ok_or_else(|| ParseError {
                    line: kid.number,
                    why: "a record field needs `<field> = <expr>`".to_string(),
                })?;
                fields.push(crate::forge::model::RecordFieldInit {
                    name: undo(field, kid.number)?,
                    name_spelling: None,
                    expr: undo(expr, kid.number)?,
                    expr_spelling: None,
                });
            }
            return Ok(AlgorithmStmt::RecordVar {
                name: undo(name, line.number)?,
                name_spelling: None,
                alias: alias.to_string(),
                type_spelling: None,
                fields,
            });
        }
        let init = match (init, sce_type.is_append_buffer()) {
            (Some(init), false) => Some(undo(init, line.number)?),
            (None, true) => None,
            (Some(_), true) => {
                return Err(ParseError {
                    line: line.number,
                    why: format!(
                        "a {} var starts empty and has no `= <expr>`",
                        sce_type.as_attr()
                    ),
                })
            }
            (None, false) => {
                return Err(ParseError {
                    line: line.number,
                    why: "a var needs `= <expr>`".to_string(),
                })
            }
        };
        // A pseudo line is not an attribute of any SCXML document, so no
        // statement it builds carries a spelling.
        return Ok(AlgorithmStmt::Var {
            name: undo(name, line.number)?,
            name_spelling: None,
            sce_type,
            init,
            init_spelling: None,
            capacity,
            capacity_spelling: None,
        });
    }
    if let Some(rest) = t.strip_prefix("append ") {
        let (target, expr) = rest.split_once(" <- ").ok_or_else(|| ParseError {
            line: line.number,
            why: "an append needs `<- <expr>`".to_string(),
        })?;
        return Ok(AlgorithmStmt::Append {
            target: undo(target, line.number)?,
            target_spelling: None,
            expr: undo(expr, line.number)?,
            expr_spelling: None,
        });
    }
    if let Some(rest) = t.strip_prefix("if ") {
        return Ok(AlgorithmStmt::If {
            cond: undo(rest.trim_end_matches(':'), line.number)?,
            cond_spelling: None,
            then_body: body_of(kids)?,
            // The renderer writes `else:` as a sibling line, so this
            // arm is filled by the caller's `else` handling below.
            else_body: None,
        });
    }
    if let Some(rest) = t.strip_prefix("while ") {
        let rest = rest.trim_end_matches(':');
        let (max_iter, cond) = match rest.strip_prefix("max ") {
            Some(after) => {
                let (n, c) = after.split_once(' ').ok_or_else(|| ParseError {
                    line: line.number,
                    why: "a while bound needs `max <n> <cond>`".to_string(),
                })?;
                (n.parse().ok(), c)
            }
            None => (None, rest),
        };
        return Ok(AlgorithmStmt::While {
            cond: undo(cond, line.number)?,
            cond_spelling: None,
            body: body_of(kids)?,
            max_iter,
        });
    }
    if let Some(rest) = t.strip_prefix("foreach ") {
        let (item, source) = rest
            .trim_end_matches(':')
            .split_once(" in ")
            .ok_or_else(|| ParseError {
                line: line.number,
                why: "a foreach needs `<item> in <source>`".to_string(),
            })?;
        return Ok(AlgorithmStmt::Foreach {
            item: undo(item, line.number)?,
            source: undo(source, line.number)?,
            source_spelling: None,
            body: body_of(kids)?,
        });
    }
    if t == "return" {
        return Ok(AlgorithmStmt::Return {
            expr: None,
            expr_spelling: None,
        });
    }
    if let Some(rest) = t.strip_prefix("return ") {
        return Ok(AlgorithmStmt::Return {
            expr: Some(undo(rest, line.number)?),
            expr_spelling: None,
        });
    }
    if let Some(rest) = t.strip_prefix("call ") {
        let target = rest.trim_end_matches(':');
        let mut args = Vec::new();
        for k in kids {
            let a = k.text.strip_prefix("arg ").ok_or_else(|| ParseError {
                line: k.number,
                why: format!("`{}` is not a call argument", k.text),
            })?;
            args.push(CallArg {
                expr: undo(a, k.number)?,
                spelling: None,
            });
        }
        return Ok(AlgorithmStmt::Call {
            target: undo(target, line.number)?,
            target_spelling: None,
            args,
            args_spelling: None,
        });
    }
    if let Some((target, expr)) = t.split_once(" = ") {
        return Ok(AlgorithmStmt::Assign {
            target: undo(target, line.number)?,
            target_spelling: None,
            expr: undo(expr, line.number)?,
            expr_spelling: None,
        });
    }
    Err(ParseError {
        line: line.number,
        why: format!("`{t}` is not a statement"),
    })
}

/// `procedure <name> initial <state>` with fields, helpers and states.
fn parse_procedure(head: &Line<'_>, body: &[&Line<'_>]) -> Result<ProcedureModel, ParseError> {
    let w: Vec<&str> = head.text.split_whitespace().collect();
    if w.get(2) != Some(&"initial") {
        return Err(ParseError {
            line: head.number,
            why: "a procedure head needs `initial <state>`".to_string(),
        });
    }
    let mut m = ProcedureModel {
        name: undo(w.get(1).copied().unwrap_or(""), head.number)?,
        inputs: Vec::new(),
        internals: Vec::new(),
        helpers: Vec::new(),
        initial: undo(w.get(3).copied().unwrap_or(""), head.number)?,
        states: Vec::new(),
        source_location: None,
    };

    for (line, kids) in group(body) {
        if let Some(rest) = line.text.strip_prefix("helper ") {
            m.helpers.push(parse_helper(rest, line.number)?);
        } else if line.text.starts_with("state ") || line.text.starts_with("final ") {
            m.states.push(parse_procedure_state(line, &kids)?);
        } else {
            let f = parse_field(line)?;
            match f.direction {
                Direction::Internal => m.internals.push(f),
                _ => m.inputs.push(f),
            }
        }
    }
    Ok(m)
}

/// `<name>(<type>, …) -> <type> [returns-max <n>]`
fn parse_helper(rest: &str, line: usize) -> Result<ProcedureHelper, ParseError> {
    let (name, tail) = rest.split_once('(').ok_or_else(|| ParseError {
        line,
        why: "a helper needs an argument list".to_string(),
    })?;
    let (args_text, after) = tail.rsplit_once(')').ok_or_else(|| ParseError {
        line,
        why: "a helper needs a closing `)`".to_string(),
    })?;
    let args = args_text
        .split(',')
        .map(str::trim)
        .filter(|a| !a.is_empty())
        .map(|a| {
            SceType::from_attr(a).ok_or_else(|| ParseError {
                line,
                why: format!("`{a}` is not an sce:type"),
            })
        })
        .collect::<Result<_, _>>()?;
    let w: Vec<&str> = after.split_whitespace().collect();
    let returns_word = w.get(1).copied().unwrap_or("");
    Ok(ProcedureHelper {
        name: undo(name, line)?,
        args,
        returns: SceType::from_attr(returns_word).ok_or_else(|| ParseError {
            line,
            why: format!("`{returns_word}` is not an sce:type"),
        })?,
        returns_max_size: match w.get(2).copied() {
            Some("returns-max") => w.get(3).and_then(|v| v.parse().ok()),
            _ => None,
        },
    })
}

fn parse_procedure_state(
    line: &Line<'_>,
    kids: &[&Line<'_>],
) -> Result<ProcedureState, ParseError> {
    let (keyword, rest) = line.text.split_once(' ').ok_or_else(|| ParseError {
        line: line.number,
        why: "a state needs an id".to_string(),
    })?;
    let mut s = ProcedureState {
        id: undo(rest.trim_end_matches(':'), line.number)?,
        is_final: keyword == "final",
        transitions: Vec::new(),
        on_entry_sends: Vec::new(),
        done_params: Vec::new(),
        line: None,
    };

    for (l, sub) in group(kids) {
        if let Some(service) = l.text.strip_prefix("send ") {
            let mut send = ProcedureSendAction {
                service: undo(service.trim_end_matches(':'), l.number)?,
                subfunc: None,
                addr: None,
                addr_spelling: None,
                payload: None,
                payload_spelling: None,
                response_max_size: None,
            };
            for k in sub {
                let (keyword, value) = k.text.split_once(' ').ok_or_else(|| ParseError {
                    line: k.number,
                    why: format!("`{}` is not a send clause", k.text),
                })?;
                match keyword {
                    "subfunc" => send.subfunc = Some(undo(value, k.number)?),
                    "addr" => send.addr = Some(undo(value, k.number)?),
                    "payload" => send.payload = Some(undo(value, k.number)?),
                    "response-max" => send.response_max_size = value.parse().ok(),
                    other => {
                        return Err(ParseError {
                            line: k.number,
                            why: format!("`{other}` is not a send clause"),
                        })
                    }
                }
            }
            s.on_entry_sends.push(send);
        } else if let Some(rest) = l.text.strip_prefix("done ") {
            let (name, expr) = rest.split_once(" = ").ok_or_else(|| ParseError {
                line: l.number,
                why: "a done param needs `= <expr>`".to_string(),
            })?;
            s.done_params.push(ProcedureDoneParam {
                name: undo(name, l.number)?,
                expr: undo(expr, l.number)?,
                expr_spelling: None,
            });
        } else {
            s.transitions.push(parse_procedure_transition(l, &sub)?);
        }
    }
    Ok(s)
}

/// `[on <event> ]-> <target>[ when <cond>]` with assigns under it.
fn parse_procedure_transition(
    line: &Line<'_>,
    kids: &[&Line<'_>],
) -> Result<ProcedureTransition, ParseError> {
    let (event, rest) = match line.text.strip_prefix("on ") {
        Some(after) => {
            let (e, r) = after.split_once(" -> ").ok_or_else(|| ParseError {
                line: line.number,
                why: "a transition needs `-> <target>`".to_string(),
            })?;
            (Some(undo(e, line.number)?), r)
        }
        None => (
            None,
            line.text.strip_prefix("-> ").ok_or_else(|| ParseError {
                line: line.number,
                why: format!("`{}` is not a transition", line.text),
            })?,
        ),
    };
    // The guard is last, so the target is whatever precedes ` when `.
    let (target, cond) = match rest.split_once(" when ") {
        Some((t, c)) => (t, Some(undo(c, line.number)?)),
        None => (rest, None),
    };

    let mut assigns = Vec::new();
    for k in kids {
        let (location, expr) = k.text.split_once(" = ").ok_or_else(|| ParseError {
            line: k.number,
            why: format!("`{}` is not an assign", k.text),
        })?;
        assigns.push(ProcedureAssign {
            location: undo(location, k.number)?,
            location_spelling: None,
            expr: undo(expr, k.number)?,
            expr_spelling: None,
        });
    }

    Ok(ProcedureTransition {
        target: undo(target, line.number)?,
        cond,
        cond_spelling: None,
        event,
        assigns,
        line: None,
    })
}

fn endian_of(s: &str, line: usize) -> Result<Endian, ParseError> {
    match s {
        "big" => Ok(Endian::Big),
        "little" => Ok(Endian::Little),
        "native" => Ok(Endian::Native),
        other => Err(ParseError {
            line,
            why: format!("`{other}` is not an endianness"),
        }),
    }
}

/// `flag <name> bit <n> width <n> [value <v>]`, under either keyword.
fn parse_flag_def(w: &[&str], line: usize) -> Result<FlagDef, ParseError> {
    Ok(FlagDef {
        name: undo(w.get(1).copied().unwrap_or(""), line)?,
        bit: w.get(3).and_then(|v| v.parse().ok()).unwrap_or(0),
        width: w.get(5).and_then(|v| v.parse().ok()).unwrap_or(0),
        // ⚠ Read through `source_literal`, not `parse()`: the renderer
        // writes the author's spelling and a hex constant would
        // otherwise read as absent.
        value: match w.get(6).copied() {
            Some("value") => w
                .get(7)
                .and_then(|v| crate::source_literal::read_unsigned(v)),
            _ => None,
        },
        value_text: match w.get(6).copied() {
            Some("value") => undo(w.get(7).copied().unwrap_or(""), line)?,
            _ => String::new(),
        },
        // A pseudocode line is not a row of the SCXML document.
        line: None,
    })
}

/// `[not] (local|input):<field>.<flag> [or …]`
fn parse_present_if(s: &str, line: usize) -> Result<PresentIfPredicate, ParseError> {
    let (head, rest) = match s.split_once(" or ") {
        Some((h, r)) => (h, Some(r)),
        None => (s, None),
    };
    let (negate, head) = match head.strip_prefix("not ") {
        Some(h) => (true, h),
        None => (false, head),
    };
    let (scope_word, path) = head.split_once(':').ok_or_else(|| ParseError {
        line,
        why: "a present-if needs `<scope>:<field>.<flag>`".to_string(),
    })?;
    let (field_id, flag_name) = path.split_once('.').ok_or_else(|| ParseError {
        line,
        why: "a present-if needs `<field>.<flag>`".to_string(),
    })?;
    Ok(PresentIfPredicate {
        scope: match scope_word {
            "local" => PresentIfScope::Local,
            "input" => PresentIfScope::Input,
            other => {
                return Err(ParseError {
                    line,
                    why: format!("`{other}` is not a present-if scope"),
                })
            }
        },
        field_id: undo(field_id, line)?,
        flag_name: undo(flag_name, line)?,
        negate,
        or_with: match rest {
            Some(r) => Some(Box::new(parse_present_if(r, line)?)),
            None => None,
        },
    })
}

fn parse_bit_size(w: &[&str], line: usize) -> Result<BitSize, ParseError> {
    Ok(match w.first().copied() {
        Some("fixed") => BitSize::Fixed {
            bits: w.get(1).and_then(|v| v.parse().ok()).unwrap_or(0),
        },
        Some("tail") => BitSize::Tail,
        Some("length-ref") => BitSize::LengthRef,
        Some("embed") => BitSize::Embed,
        Some("vle") => BitSize::Vle {
            width_bits: w.get(1).and_then(|v| v.parse().ok()).unwrap_or(0),
        },
        Some("repeat") => BitSize::Repeat {
            count_ref: match w.get(1).copied() {
                Some("until-eof") => CountRef::UntilEof,
                Some("length-field") => {
                    CountRef::LengthField(undo(w.get(2).copied().unwrap_or(""), line)?)
                }
                other => {
                    return Err(ParseError {
                        line,
                        why: format!("`{}` is not a repeat count", other.unwrap_or("")),
                    })
                }
            },
        },
        Some("tlv-chain") => BitSize::TlvChain {
            max_depth: w.get(2).and_then(|v| v.parse().ok()).unwrap_or(0),
            on_overflow: match w.get(4).copied() {
                Some("reject") => TlvOverflowPolicy::Reject,
                Some("truncate") => TlvOverflowPolicy::Truncate,
                other => {
                    return Err(ParseError {
                        line,
                        why: format!("`{}` is not a TLV overflow policy", other.unwrap_or("")),
                    })
                }
            },
            terminate_on: match w.get(6).copied() {
                Some("exhaust-or-depth") => TlvTerminateStrategy::ExhaustOrDepth,
                Some("entry-flag") => TlvTerminateStrategy::EntryFlag {
                    flag_name: undo(w.get(7).copied().unwrap_or(""), line)?,
                },
                other => {
                    return Err(ParseError {
                        line,
                        why: format!("`{}` is not a TLV terminator", other.unwrap_or("")),
                    })
                }
            },
        },
        other => {
            return Err(ParseError {
                line,
                why: format!("`{}` is not a bit size", other.unwrap_or("")),
            })
        }
    })
}

/// `codec <name> endian <e> [input-length <n>]`.
fn parse_codec(head: &Line<'_>, body: &[&Line<'_>]) -> Result<CodecModel, ParseError> {
    let w: Vec<&str> = head.text.split_whitespace().collect();
    let mut m = CodecModel {
        name: undo(w.get(1).copied().unwrap_or(""), head.number)?,
        default_endian: endian_of(w.get(3).copied().unwrap_or(""), head.number)?,
        input_length: match w.get(4).copied() {
            Some("input-length") => w.get(5).and_then(|v| v.parse().ok()),
            _ => None,
        },
        fields: Vec::new(),
        variant: None,
        flag_inputs: Vec::new(),
        test_vectors: Vec::new(),
        source_location: None,
    };

    for (line, kids) in group(body) {
        let lw: Vec<&str> = line.text.split_whitespace().collect();
        match lw.first().copied() {
            Some("flag-input") => m.flag_inputs.push(FlagInput {
                name: undo(lw.get(1).copied().unwrap_or(""), line.number)?,
                width: lw.get(3).and_then(|v| v.parse().ok()).unwrap_or(0),
            }),
            Some("field") => m.fields.push(parse_codec_field(line, &kids)?),
            Some("variant") => m.variant = Some(parse_codec_variant(&lw, &kids, line.number)?),
            Some("test") => m
                .test_vectors
                .push(parse_codec_test(&lw, &kids, line.number)?),
            _ => {
                return Err(ParseError {
                    line: line.number,
                    why: format!("`{}` is not a codec body line", line.text),
                })
            }
        }
    }
    Ok(m)
}

fn parse_codec_field(line: &Line<'_>, kids: &[&Line<'_>]) -> Result<CodecField, ParseError> {
    let w: Vec<&str> = line.text.split_whitespace().collect();
    let id = w
        .get(1)
        .and_then(|v| v.strip_suffix(':'))
        .ok_or_else(|| ParseError {
            line: line.number,
            why: "a codec field needs `<id>:`".to_string(),
        })?;
    let type_word = w.get(2).copied().unwrap_or("");
    // `at byte <n> [bit <n>] size …`. Found by keyword rather than by
    // index, because `bit` is optional and counting past an optional
    // word is how the next clause silently reads the wrong token — the
    // `size` position below moves with it.
    let at = w
        .iter()
        .position(|t| *t == "at")
        .ok_or_else(|| ParseError {
            line: line.number,
            why: "a codec field needs `at byte <n>`".to_string(),
        })?;
    if w.get(at + 1).copied() != Some("byte") {
        return Err(ParseError {
            line: line.number,
            why: "a codec field's position starts `at byte <n>`".to_string(),
        });
    }
    let byte_offset = w
        .get(at + 2)
        .and_then(|v| crate::source_literal::read_unsigned(v))
        .ok_or_else(|| ParseError {
            line: line.number,
            why: "a codec field's byte offset is not a number".to_string(),
        })? as u32;
    let bit_offset = if w.get(at + 3).copied() == Some("bit") {
        Some(
            w.get(at + 4)
                .and_then(|v| crate::source_literal::read_unsigned(v))
                .ok_or_else(|| ParseError {
                    line: line.number,
                    why: "a codec field's bit offset is not a number".to_string(),
                })? as u32,
        )
    } else {
        None
    };
    let size_at = w
        .iter()
        .position(|t| *t == "size")
        .ok_or_else(|| ParseError {
            line: line.number,
            why: "a codec field needs `size <bit-size>`".to_string(),
        })?;

    let mut f = CodecField {
        id: undo(id, line.number)?,
        // A pseudocode line is not a row of the SCXML document.
        line: None,
        sce_type: SceType::from_attr(type_word).ok_or_else(|| ParseError {
            line: line.number,
            why: format!("`{type_word}` is not an sce:type"),
        })?,
        byte_offset,
        bit_offset,
        bit_size: parse_bit_size(&w[size_at + 1..], line.number)?,
        endian: None,
        max_size: None,
        length_field: None,
        flags: Vec::new(),
        present_if: None,
        repeat_body_alias: None,
        max_count: None,
        tlv_chain_body_alias: None,
        dma_burst_align: None,
        embed_body_alias: None,
        embed_length_from: None,
        length_arith: None,
        quantity: None,
    };

    for k in kids {
        let kw: Vec<&str> = k.text.split_whitespace().collect();
        let tail = |n: usize| k.text.splitn(n + 1, ' ').nth(n).unwrap_or("");
        match kw.first().copied() {
            Some("endian") => f.endian = Some(endian_of(kw[1], k.number)?),
            Some("max-size") => f.max_size = kw.get(1).and_then(|v| v.parse().ok()),
            Some("length-field") => f.length_field = Some(undo(tail(1), k.number)?),
            Some("length-arith") => f.length_arith = kw.get(1).and_then(|v| v.parse().ok()),
            Some("max-count") => f.max_count = kw.get(1).and_then(|v| v.parse().ok()),
            Some("repeat-body") => f.repeat_body_alias = Some(undo(tail(1), k.number)?),
            Some("tlv-body") => f.tlv_chain_body_alias = Some(undo(tail(1), k.number)?),
            Some("embed-body") => f.embed_body_alias = Some(undo(tail(1), k.number)?),
            Some("embed-length-from") => f.embed_length_from = Some(undo(tail(1), k.number)?),
            Some("dma-align") => f.dma_burst_align = kw.get(1).and_then(|v| v.parse().ok()),
            Some("quantity") => {
                f.quantity = Some(crate::forge::quantity::Quantity {
                    scale: rational(kw.get(1).copied().unwrap_or(""), k.number)?,
                    offset: rational(kw.get(2).copied().unwrap_or(""), k.number)?,
                    unit: crate::forge::quantity::UnitTag::intern(&undo(
                        kw.get(3).copied().unwrap_or(""),
                        k.number,
                    )?),
                })
            }
            Some("present-if") => f.present_if = Some(parse_present_if(tail(1), k.number)?),
            Some("flag") => f.flags.push(parse_flag_def(&kw, k.number)?),
            _ => {
                return Err(ParseError {
                    line: k.number,
                    why: format!("`{}` is not a codec field clause", k.text),
                })
            }
        }
    }
    Ok(f)
}

fn parse_codec_variant(
    w: &[&str],
    kids: &[&Line<'_>],
    line: usize,
) -> Result<CodecVariant, ParseError> {
    let mut v = CodecVariant {
        tag_field: None,
        tag_flag: None,
        arms: Vec::new(),
        default_arm: None,
        peek_byte: None,
    };
    let mut peek_id: Option<String> = None;
    let mut i = 1;
    while i < w.len() {
        let value = w.get(i + 1).copied().unwrap_or("");
        match w[i] {
            "tag-field" => v.tag_field = Some(undo(value, line)?),
            "tag-flag" => v.tag_flag = Some(undo(value, line)?),
            "peek-byte" => peek_id = Some(undo(value, line)?),
            other => {
                return Err(ParseError {
                    line,
                    why: format!("`{other}` is not a variant clause"),
                })
            }
        }
        i += 2;
    }

    let mut peek_flags = Vec::new();
    for k in kids {
        let kw: Vec<&str> = k.text.split_whitespace().collect();
        match kw.first().copied() {
            Some("peek-flag") => peek_flags.push(parse_flag_def(&kw, k.number)?),
            Some("arm") | Some("default-arm") => {
                let (value, rest) = k
                    .text
                    .split_once(" -> ")
                    .and_then(|(l, r)| l.split_whitespace().nth(1).map(|n| (n, r)))
                    .ok_or_else(|| ParseError {
                        line: k.number,
                        why: "an arm needs `<n> -> <alias>`".to_string(),
                    })?;
                let is_default = rest.ends_with(" default");
                let alias = rest.strip_suffix(" default").unwrap_or(rest);
                // ⚠ `crate::source_literal::read_unsigned`, not
                // `parse()`. The renderer now writes the author's
                // spelling, so a hex discriminator reaches this line as
                // `0x1a` — and `"0x1a".parse::<u64>()` fails, which the
                // old `unwrap_or(0)` turned into a silent zero. A
                // rendering that reads back as a different document is
                // the one thing this module may not do.
                let spelling = undo(value, k.number)?;
                let arm = VariantArm {
                    value: crate::source_literal::read_unsigned(&spelling).ok_or_else(|| {
                        ParseError {
                            line: k.number,
                            why: format!("`{spelling}` is not a discriminator value"),
                        }
                    })?,
                    value_text: spelling,
                    body_alias: undo(alias, k.number)?,
                    is_default,
                    // A pseudocode line is not a row of the SCXML document.
                    line: None,
                };
                if kw[0] == "arm" {
                    v.arms.push(arm);
                } else {
                    v.default_arm = Some(arm);
                }
            }
            _ => {
                return Err(ParseError {
                    line: k.number,
                    why: format!("`{}` is not a variant body line", k.text),
                })
            }
        }
    }
    if let Some(id) = peek_id {
        v.peek_byte = Some(PeekByteSpec {
            id,
            flags: peek_flags,
        });
    }
    Ok(v)
}

fn parse_codec_test(
    w: &[&str],
    kids: &[&Line<'_>],
    line: usize,
) -> Result<CodecTestVector, ParseError> {
    // ⚠ The `0x` is OPTIONAL here, because the renderer now writes the
    // author's spelling and a payload is written bare throughout this
    // tree. Requiring the prefix would make the reader refuse the very
    // renderings it exists to read back.
    let written = w.get(1).copied().ok_or_else(|| ParseError {
        line,
        why: "a test vector needs its wire bytes".to_string(),
    })?;
    let digits = written.strip_prefix("0x").unwrap_or(written);
    if !digits.len().is_multiple_of(2) {
        return Err(ParseError {
            line,
            why: format!("`{written}` is not a whole number of bytes"),
        });
    }
    let hex = (0..digits.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&digits[i..i + 2], 16))
        .collect::<Result<Vec<u8>, _>>()
        .map_err(|_| ParseError {
            line,
            why: "a test vector's hex is not hex".to_string(),
        })?;

    let mut fields = Vec::new();
    for k in kids {
        let (name, rest) = k.text.split_once(" = ").ok_or_else(|| ParseError {
            line: k.number,
            why: "a decoded field needs `= <value>`".to_string(),
        })?;
        let (kind, literal) = rest.split_once(' ').ok_or_else(|| ParseError {
            line: k.number,
            why: "a decoded value needs a type and a literal".to_string(),
        })?;
        let bad = || ParseError {
            line: k.number,
            why: format!("`{literal}` is not a {kind} literal"),
        };
        let value = match kind {
            "bool" => DecodedFieldValue::Bool(literal == "true"),
            // ⚠ Through `source_literal`: the renderer writes the
            // author's spelling, so a hex expected-value arrives here
            // as `0xCAFEBABE` and `parse()` would refuse it.
            "uint" => DecodedFieldValue::Uint(
                crate::source_literal::read_unsigned(literal).ok_or_else(bad)?,
            ),
            "int" => {
                DecodedFieldValue::Int(crate::source_literal::read_signed(literal).ok_or_else(bad)?)
            }
            "string" => DecodedFieldValue::String(undo(literal, k.number)?),
            "bytes" => {
                let h = literal.strip_prefix("0x").ok_or_else(bad)?;
                DecodedFieldValue::Bytes(
                    (0..h.len())
                        .step_by(2)
                        .map(|i| u8::from_str_radix(&h[i..i + 2], 16))
                        .collect::<Result<_, _>>()
                        .map_err(|_| bad())?,
                )
            }
            _ => return Err(bad()),
        };
        fields.push(DecodedField {
            name: undo(name, k.number)?,
            value_text: match kind {
                "uint" | "int" => literal.to_string(),
                _ => String::new(),
            },
            value,
        });
    }

    Ok(CodecTestVector {
        hex,
        hex_text: written.to_string(),
        decoded: DecodedValue::Plain { fields },
        source_line: w.get(3).and_then(|v| v.parse().ok()).unwrap_or(0),
    })
}

// ── Statechart ─────────────────────────────────────────────────
//
// ⚠ The statechart model is NOT the document: of its 92 fields, 39 are
// written by the analyzer and many more are computed during parsing.
// The renderer deliberately writes only the authored core, so a model
// read back from a rendering is missing everything derived — and that
// is why the round-trip law for this kind compares RENDERINGS rather
// than models. See the gate's own note; the split is stated there.

/// `machine <name> (datamodel: <d>, initial: <s>[, binding: <b>]…)`.
fn parse_statechart(
    head: &Line<'_>,
    body: &[&Line<'_>],
) -> Result<crate::model::SCXMLModel, ParseError> {
    let (name, clauses) = head.text["machine ".len()..]
        .split_once(" (")
        .ok_or_else(|| ParseError {
            line: head.number,
            why: "a machine head needs its clause list".to_string(),
        })?;

    let mut m = crate::model::SCXMLModel {
        name: undo(name, head.number)?,
        ..Default::default()
    };
    for clause in clauses.trim_end_matches(')').split(", ") {
        let (key, value) = clause.split_once(": ").ok_or_else(|| ParseError {
            line: head.number,
            why: format!("`{clause}` is not `<key>: <value>`"),
        })?;
        match key {
            "datamodel" => {
                m.datamodel =
                    crate::model::Datamodel::from_attr(value).ok_or_else(|| ParseError {
                        line: head.number,
                        why: format!("`{value}` is not a datamodel"),
                    })?
            }
            "initial" => m.initial = undo(value, head.number)?,
            "binding" => m.binding = undo(value, head.number)?,
            "queue" => m.event_queue_capacity = value.parse().ok(),
            other => {
                return Err(ParseError {
                    line: head.number,
                    why: format!("`{other}` is not a machine clause"),
                })
            }
        }
    }

    let mut order = 0u32;
    let mut driver_order = 0u32;
    for (line, kids) in group(body) {
        let w: Vec<&str> = line.text.split_whitespace().collect();
        match w.first().copied() {
            // `document_order` is this loop's own position, not
            // something the page carries — the renderer prints the href
            // alone, because the rest of a `DriverRef` is derived.
            // Counting here reproduces what the parser counted there.
            Some("driver") => {
                m.driver_refs.push(crate::model::DriverRef {
                    href: undo(w.get(1).copied().unwrap_or(""), line.number)?,
                    resolved_path: None,
                    document_order: driver_order,
                    source_location: None,
                });
                driver_order += 1;
            }
            Some("context") => {
                let mut c = crate::model::ContextObject {
                    id: undo(w.get(1).copied().unwrap_or(""), line.number)?,
                    cpp_type: String::new(),
                    cpp_include: String::new(),
                    kt_type: String::new(),
                };
                let mut i = 2;
                while i < w.len() {
                    let v = undo(w.get(i + 1).copied().unwrap_or(""), line.number)?;
                    match w[i] {
                        "cpp-type" => c.cpp_type = v,
                        "cpp-include" => c.cpp_include = v,
                        "kt-type" => c.kt_type = v,
                        other => {
                            return Err(ParseError {
                                line: line.number,
                                why: format!("`{other}` is not a context clause"),
                            })
                        }
                    }
                    i += 2;
                }
                m.context_object_ids.insert(c.id.clone());
                m.context_objects.push(c);
            }
            Some("data") => m.variables.push(parse_variable(line)?),
            Some("state") | Some("parallel") | Some("final") => {
                let mut s = parse_scxml_state(line, &kids)?;
                s.document_order = order;
                order += 1;
                m.invokes.extend(s.invokes.iter().cloned());
                m.states.insert(s.id.clone(), s);
            }
            _ => m.global_scripts.push(parse_scxml_action(line, &kids)?),
        }
    }
    Ok(m)
}

/// `data <id>[: <type>] [sce-type <t>] [src <s>] [= <expr>] [content <c>]`
fn parse_variable(line: &Line<'_>) -> Result<crate::model::Variable, ParseError> {
    let rest = &line.text["data ".len()..];
    let (id, mut tail) = match rest.split_once(' ') {
        Some((a, b)) => (a, b),
        None => (rest, ""),
    };
    let (id, var_type) = match id.split_once(':') {
        Some((i, _)) => {
            // The type sits after the colon and before the first space,
            // so it is the head of the tail when the id ended in `:`.
            let (t, after) = match tail.split_once(' ') {
                Some((t, a)) => (t, a),
                None => (tail, ""),
            };
            tail = after;
            (i, t.to_string())
        }
        None => (id, String::new()),
    };

    let mut v = crate::model::Variable {
        id: undo(id, line.number)?,
        expr: String::new(),
        // A rendering holds no attribute, so nothing places a refusal here.
        expr_spelling: None,
        src: String::new(),
        content: String::new(),
        source_location: None,
        var_type: undo(&var_type, line.number)?,
        value_type: None,
        value_type_spelling: None,
    };
    // Clause order is the renderer's: `sce-type` and `src` first, then the
    // free-text `= <expr>` and `content`, each of which closes the line.
    if let Some(after) = tail.strip_prefix("sce-type ") {
        let (t, more) = match after.split_once(' ') {
            Some((t, m)) => (t, m),
            None => (after, ""),
        };
        let t = undo(t, line.number)?;
        v.value_type = Some(
            crate::forge::model::AlgorithmValueType::from_attr(&t).ok_or_else(|| ParseError {
                line: line.number,
                why: format!("`{t}` is not a value type"),
            })?,
        );
        tail = more;
    }
    if let Some(after) = tail.strip_prefix("src ") {
        let (s, more) = match after.split_once(' ') {
            Some((s, m)) => (s, m),
            None => (after, ""),
        };
        v.src = undo(s, line.number)?;
        tail = more;
    }
    if let Some(after) = tail.strip_prefix("= ") {
        match after.split_once(" content ") {
            Some((e, c)) => {
                v.expr = undo(e, line.number)?;
                v.content = undo(c, line.number)?;
            }
            None => v.expr = undo(after, line.number)?,
        }
    } else if let Some(c) = tail.strip_prefix("content ") {
        v.content = undo(c, line.number)?;
    }
    Ok(v)
}

fn parse_scxml_state(
    line: &Line<'_>,
    kids: &[&Line<'_>],
) -> Result<crate::model::State, ParseError> {
    let w: Vec<&str> = line.text.trim_end_matches(':').split_whitespace().collect();
    let mut s = crate::model::State {
        id: undo(w.get(1).copied().unwrap_or(""), line.number)?,
        is_final: w[0] == "final",
        is_parallel: w[0] == "parallel",
        ..Default::default()
    };
    let mut i = 2;
    while i < w.len() {
        match w[i] {
            "initial" => {
                s.initial = undo(w.get(i + 1).copied().unwrap_or(""), line.number)?;
                i += 2;
            }
            "history" => {
                s.initial_history_id = undo(w.get(i + 1).copied().unwrap_or(""), line.number)?;
                s.initial_history_default_target =
                    undo(w.get(i + 3).copied().unwrap_or(""), line.number)?;
                i += 4;
            }
            "initial-children" | "unhandled" => {
                let keyword = w[i];
                let mut items = Vec::new();
                i += 1;
                while i < w.len() && !matches!(w[i], "initial" | "history" | "unhandled") {
                    items.push(undo(w[i], line.number)?);
                    i += 1;
                }
                if keyword == "unhandled" {
                    s.unhandled = items;
                } else {
                    s.initial_children = items;
                }
            }
            other => {
                return Err(ParseError {
                    line: line.number,
                    why: format!("`{other}` is not a state clause"),
                })
            }
        }
    }

    for (l, sub) in group(kids) {
        // A head clause the head line could not delimit is written
        // here instead, same keyword, its value running to the end of
        // the line. See `head_inlinable` in the renderer.
        if let Some(v) = l.text.strip_prefix("initial ") {
            s.initial = undo(v, l.number)?;
        } else if let Some(v) = l.text.strip_prefix("initial-children ") {
            s.initial_children = undo_each(v, l.number)?;
        } else if let Some(v) = l.text.strip_prefix("unhandled ") {
            s.unhandled = undo_each(v, l.number)?;
        } else if let Some(id) = l.text.strip_prefix("req ") {
            s.req.push(RequirementId(undo(id, l.number)?));
        } else if l.text.starts_with("data ") {
            s.datamodel.push(parse_variable(l)?);
        } else if l.text.starts_with("invoke") {
            s.invokes.push(parse_scxml_invoke(l, &sub)?);
        } else if let Some(rest) = l.text.strip_prefix("on sample ") {
            let sw: Vec<&str> = rest.split_whitespace().collect();
            s.on_sample_blocks.push(crate::model::OnSampleNode {
                link: undo(sw.first().copied().unwrap_or(""), l.number)?,
                event: undo(sw.get(2).copied().unwrap_or(""), l.number)?,
                callback: match sw.get(3).copied() {
                    Some("callback") => Some(undo(sw.get(4).copied().unwrap_or(""), l.number)?),
                    _ => None,
                },
                document_order: s.on_sample_blocks.len() as u32,
            });
        } else if l.text == "on entry:" {
            s.on_entry_blocks.push(parse_action_list(&sub)?);
        } else if l.text == "on exit:" {
            s.on_exit_blocks.push(parse_action_list(&sub)?);
        } else if l.text == "on initial:" {
            s.initial_transition_actions = parse_action_list(&sub)?;
        } else if l.text == "on history-default:" {
            s.initial_history_default_actions = parse_action_list(&sub)?;
        } else if l.text == "done:" {
            s.donedata = Some(parse_scxml_donedata(&sub)?);
        } else {
            s.transitions.push(parse_scxml_transition(l, &sub)?);
        }
    }
    for (i, t) in s.transitions.iter_mut().enumerate() {
        t.transition_index = i;
    }
    Ok(s)
}

fn parse_scxml_donedata(kids: &[&Line<'_>]) -> Result<crate::model::DoneData, ParseError> {
    let mut d = crate::model::DoneData {
        params: Vec::new(),
        content: crate::model::DoneDataContent::None,
        content_location: None,
        content_spelling: None,
    };
    for (k, sub) in group(kids) {
        if let Some(rest) = k.text.strip_prefix("param ") {
            d.params.push(parse_donedata_param(rest, &sub, k.number)?);
        } else if let Some(rest) = k.text.strip_prefix("content ") {
            let (kind, value) = rest.split_once(' ').ok_or_else(|| ParseError {
                line: k.number,
                why: "done content needs a kind and a value".to_string(),
            })?;
            d.content = match kind {
                "expr" => crate::model::DoneDataContent::Expression(undo(value, k.number)?),
                "text" => crate::model::DoneDataContent::InlineText(undo(value, k.number)?),
                "literal" => crate::model::DoneDataContent::Literal(undo(value, k.number)?),
                other => {
                    return Err(ParseError {
                        line: k.number,
                        why: format!("`{other}` is not a done content kind"),
                    })
                }
            };
        } else {
            return Err(ParseError {
                line: k.number,
                why: format!("`{}` is not a done line", k.text),
            });
        }
    }
    Ok(d)
}

/// `[on <event> ]-> <target> [<type>] [when <cond>] [native-guard <g>]`
fn parse_scxml_transition(
    line: &Line<'_>,
    kids: &[&Line<'_>],
) -> Result<crate::model::Transition, ParseError> {
    let (event, rest) = match line.text.strip_prefix("on ") {
        Some(after) => {
            let (e, r) = after.split_once(" -> ").ok_or_else(|| ParseError {
                line: line.number,
                why: "a transition needs `-> <target>`".to_string(),
            })?;
            (undo(e, line.number)?, r)
        }
        None => (
            String::new(),
            line.text.strip_prefix("-> ").ok_or_else(|| ParseError {
                line: line.number,
                why: format!("`{}` is not a transition", line.text),
            })?,
        ),
    };
    // The two guards close the line, so they come off the end first.
    let (rest, native_payload_guard) = match rest.split_once(" native-guard ") {
        Some((r, g)) => (r, undo(g, line.number)?),
        None => (rest, String::new()),
    };
    let (rest, cond) = match rest.split_once(" when ") {
        Some((r, c)) => (r, undo(c, line.number)?),
        None => (rest, String::new()),
    };
    let (target, transition_type) = match rest.split_once(" [") {
        Some((t, ty)) => (t, undo(ty.trim_end_matches(']'), line.number)?),
        None => (rest, String::new()),
    };

    // ⚠ A transition's requirement ids come first in its body, above
    // the actions — the renderer has always written them there and
    // this reader handed the whole body to the action list, so `req
    // REQ_TRANS_GO` was reported as "not an action". The state body
    // already reads `req` this way; the transition body did not.
    let top = kids.first().map(|l| l.depth);
    let (req, actions): (Vec<&Line<'_>>, Vec<&Line<'_>>) = kids
        .iter()
        .partition(|l| Some(l.depth) == top && l.text.starts_with("req "));
    let mut requirements = Vec::new();
    for l in &req {
        let id = l.text.strip_prefix("req ").unwrap_or_default();
        requirements.push(RequirementId(undo(id, l.number)?));
    }

    Ok(crate::model::Transition {
        event,
        target: if target == "(no target)" {
            String::new()
        } else {
            undo(target, line.number)?
        },
        cond,
        transition_type,
        native_payload_guard,
        req: requirements,
        actions: parse_action_list(&actions)?,
        ..Default::default()
    })
}

fn parse_scxml_invoke(
    line: &Line<'_>,
    kids: &[&Line<'_>],
) -> Result<crate::model::Invoke, ParseError> {
    use crate::model::*;

    let id = line
        .text
        .trim_end_matches(':')
        .strip_prefix("invoke")
        .unwrap_or("")
        .trim();
    let mut base = InvokeBase {
        invoke_id: undo(id, line.number)?,
        ..Default::default()
    };
    let mut kind = String::new();
    let mut autoforward = false;
    let mut scxml = ScxmlInvokeInfo::default();
    let mut hybrid = HybridInvokeInfo::default();
    let mut mesh_target: Option<MeshRpcTarget> = None;
    let mut mesh_event = String::new();
    let mut deadline_ms = None;
    let mut unsupported_src = String::new();
    let mut host_served = false;

    for (k, sub) in group(kids) {
        let (keyword, value) = match k.text.split_once(' ') {
            Some((a, b)) => (a, b),
            // A clause that opens a block has no value and ends in a
            // colon — `child:` is the one. Split on the space alone and
            // the keyword keeps the colon and matches nothing.
            None => (k.text.trim_end_matches(':'), ""),
        };
        match keyword {
            "id-into" => base.idlocation = undo(value, k.number)?,
            "param" => base.params.push(parse_param(value, &sub, k.number)?),
            "req" => base.req.push(RequirementId(undo(value, k.number)?)),
            "type" => kind = value.to_string(),
            "autoforward" => autoforward = true,
            "src" => {
                scxml.src = undo(value, k.number)?;
                unsupported_src = scxml.src.clone();
            }
            "namelist" => scxml.namelist = undo(value, k.number)?,
            "finalize" => scxml.finalize_content = undo(value, k.number)?,
            "mesh-target" => scxml.remote_mesh_target = Some(undo(value, k.number)?),
            "mesh-transport" => scxml.remote_mesh_transport = Some(undo(value, k.number)?),
            "srcexpr" => hybrid.srcexpr = undo(value, k.number)?,
            "contentexpr" => hybrid.contentexpr = undo(value, k.number)?,
            "candidates" => {
                let written = undo(value, k.number)?;
                hybrid.candidates = Vec::new();
                for tok in written.split_whitespace() {
                    // The stem comes from the same derivation the parser
                    // uses, never from the rendered text: it is the
                    // identity a value is matched against, so a model
                    // rebuilt here must name the same document the one
                    // read from XML would.
                    hybrid.candidates.push(
                        crate::model::InvokeCandidate::from_path(tok).ok_or_else(|| {
                            ParseError {
                                line: k.number,
                                why: format!("candidate `{tok}` names no document"),
                            }
                        })?,
                    );
                }
            }
            "target" => {
                let (how, what) = value.split_once(' ').ok_or_else(|| ParseError {
                    line: k.number,
                    why: "a mesh target needs `src` or `srcexpr`".to_string(),
                })?;
                mesh_target = Some(match how {
                    "src" => MeshRpcTarget::Src {
                        src: undo(what, k.number)?,
                    },
                    _ => MeshRpcTarget::SrcExpr {
                        srcexpr: undo(what, k.number)?,
                    },
                });
            }
            "event" => mesh_event = undo(value, k.number)?,
            "deadline" => deadline_ms = value.trim_end_matches("ms").parse().ok(),
            "host-served" => host_served = true,
            "child" => {
                scxml.inline_child = Some(Box::new(parse_statechart(
                    sub.first().ok_or_else(|| ParseError {
                        line: k.number,
                        why: "a child block needs a machine".to_string(),
                    })?,
                    &sub[1..],
                )?));
            }
            other => {
                return Err(ParseError {
                    line: k.number,
                    why: format!("`{other}` is not an invoke clause"),
                })
            }
        }
    }

    Ok(match kind.as_str() {
        "scxml" => {
            scxml.common = InvokeSessionCommon {
                base,
                autoforward,
                ..Default::default()
            };
            Invoke::Scxml(scxml)
        }
        "hybrid" => {
            hybrid.common = InvokeSessionCommon {
                base,
                autoforward,
                ..Default::default()
            };
            Invoke::Hybrid(hybrid)
        }
        "mesh-rpc" => Invoke::MeshRpc(MeshRpcInvokeInfo {
            base,
            target: mesh_target.ok_or_else(|| ParseError {
                line: line.number,
                why: "a mesh-rpc invoke needs a target".to_string(),
            })?,
            mesh_event,
            deadline_ms,
        }),
        // Anything else is the `Unsupported` arm, whose `invoke_type`
        // IS the word the `type` clause carried — the renderer writes
        // the raw type there rather than a keyword of its own.
        other => Invoke::Unsupported(UnsupportedInvokeInfo {
            base,
            invoke_type: other.to_string(),
            src: unsupported_src,
            host_served,
        }),
    })
}

/// `<name>`, `<name> = <expr>`, `<name> from <loc>`, or `<name>:` with
/// an `expr` / `from` block under it.
///
/// ⚠ The FIRST word after the name selects the clause, and everything
/// after that word is the value, verbatim to the end of the line.
/// Searching the line for a separator instead is what lost four
/// locations: ` from ` does not occur in `first from a`, so the clause
/// was read as absent rather than as a location, and the page said
/// something the reader could not recover. A value that itself
/// contains ` from ` or ` = ` cannot move this boundary, because the
/// boundary is a first word and not a search.
fn parse_param(
    s: &str,
    kids: &[&Line<'_>],
    line: usize,
) -> Result<crate::model::Param, ParseError> {
    let mut p = crate::model::Param::default();
    if let Some(head) = s.strip_suffix(':') {
        p.name = undo(head, line)?;
        for k in kids {
            let (keyword, value) = split_clause(k.text);
            match keyword {
                "expr" => p.expr = undo(value, k.number)?,
                "from" => p.location = undo(value, k.number)?,
                other => {
                    return Err(ParseError {
                        line: k.number,
                        why: format!("`{other}` is not a param clause"),
                    })
                }
            }
        }
        return Ok(p);
    }
    let (name, rest) = match s.split_once(' ') {
        Some((n, r)) => (n, r),
        None => (s, ""),
    };
    p.name = undo(name, line)?;
    if !rest.is_empty() {
        match split_clause(rest) {
            ("=", value) => p.expr = undo(value, line)?,
            ("from", value) => p.location = undo(value, line)?,
            (other, _) => {
                return Err(ParseError {
                    line,
                    why: format!("`{other}` is not a param clause"),
                })
            }
        }
    }
    Ok(p)
}

/// The same grammar for `<donedata>`'s param, whose clauses are
/// optional rather than empty-when-absent.
///
/// ⚠ A bare keyword is the EMPTY value and no clause line is the
/// absent one. `<param name="Var3" location=""/>` is a real document
/// here (W3C test298 points at a location that does not exist, on
/// purpose), so the two may not collapse into each other.
fn parse_donedata_param(
    s: &str,
    kids: &[&Line<'_>],
    line: usize,
) -> Result<crate::model::DoneDataParam, ParseError> {
    let mut p = crate::model::DoneDataParam {
        name: String::new(),
        expr: None,
        location: None,
        source_location: None,
        expr_spelling: None,
        location_spelling: None,
    };
    if let Some(head) = s.strip_suffix(':') {
        p.name = undo(head, line)?;
        for k in kids {
            match split_clause(k.text) {
                ("expr", value) => p.expr = Some(undo(value, k.number)?),
                ("from", value) => p.location = Some(undo(value, k.number)?),
                (other, _) => {
                    return Err(ParseError {
                        line: k.number,
                        why: format!("`{other}` is not a param clause"),
                    })
                }
            }
        }
        return Ok(p);
    }
    let (name, rest) = match s.split_once(' ') {
        Some((n, r)) => (n, r),
        None => (s, ""),
    };
    p.name = undo(name, line)?;
    if !rest.is_empty() {
        match split_clause(rest) {
            ("=", value) => p.expr = Some(undo(value, line)?),
            ("from", value) => p.location = Some(undo(value, line)?),
            (other, _) => {
                return Err(ParseError {
                    line,
                    why: format!("`{other}` is not a param clause"),
                })
            }
        }
    }
    Ok(p)
}

/// A whitespace-separated list of ids, each decoded.
///
/// An id cannot hold whitespace — the renderer refuses a document
/// whose id list would need it — so splitting on whitespace is exact
/// rather than a guess about where an entry ends.
fn undo_each(s: &str, line: usize) -> Result<Vec<String>, ParseError> {
    s.split_whitespace().map(|w| undo(w, line)).collect()
}

/// A keyword-led clause split into its keyword and the rest of the
/// line, verbatim.
///
/// ⚠ Exactly one separator space is removed, never `trim()`. The
/// values on these lines are author text, and a trailing space in one
/// is the author's: `<log label="_event ">` is a real W3C document,
/// and trimming it made the round trip lose a character the page had
/// shown correctly. A bare keyword is the empty value.
fn split_clause(text: &str) -> (&str, &str) {
    match text.split_once(' ') {
        Some((keyword, value)) => (keyword, value),
        None => (text, ""),
    }
}

fn parse_action_list(body: &[&Line<'_>]) -> Result<Vec<crate::model::Action>, ParseError> {
    let mut out: Vec<crate::model::Action> = Vec::new();
    for (line, kids) in group(body) {
        if line.text == "else:" {
            let Some(last) = out.last_mut() else {
                return Err(ParseError {
                    line: line.number,
                    why: "an `else:` with no `if` before it".to_string(),
                });
            };
            last.set_else(parse_action_list(&kids)?);
            continue;
        }
        if let Some(cond) = line.text.strip_prefix("elif ") {
            let Some(last) = out.last_mut() else {
                return Err(ParseError {
                    line: line.number,
                    why: "an `elif` with no `if` before it".to_string(),
                });
            };
            last.push_elseif(
                undo(cond.trim_end_matches(':'), line.number)?,
                parse_action_list(&kids)?,
            );
            continue;
        }
        out.push(parse_scxml_action(line, &kids)?);
    }
    Ok(out)
}

fn parse_scxml_action(
    line: &Line<'_>,
    kids: &[&Line<'_>],
) -> Result<crate::model::Action, ParseError> {
    let t = line.text;
    let mut a = crate::model::Action::default();

    if let Some(rest) = t.strip_prefix("raise ") {
        a.action_type = "raise".to_string();
        a.event = undo(rest, line.number)?;
    } else if let Some(rest) = t.strip_prefix("script ") {
        a.action_type = "script".to_string();
        a.content = undo(rest, line.number)?;
    } else if t == "cancel" || t.starts_with("cancel ") {
        a.action_type = "cancel".to_string();
        let rest = t.strip_prefix("cancel").unwrap_or("");
        let rest = rest.strip_prefix(' ').unwrap_or(rest);
        match rest.split_once(" expr ") {
            Some((id, e)) => {
                a.sendid = undo(id, line.number)?;
                a.sendidexpr = undo(e, line.number)?;
            }
            None if rest.starts_with("expr ") => {
                a.sendidexpr = undo(&rest["expr ".len()..], line.number)?
            }
            None => a.sendid = undo(rest, line.number)?,
        }
    } else if t == "log" || t == "log:" || t.starts_with("log ") || t.starts_with("log: ") {
        a.action_type = "log".to_string();
        // ⚠ No splitting and no `trim()`: each form carries at most
        // one value and it runs to the end of the line. Splitting at
        // `": "` read `<log label="…is: " expr="Var1"/>` as a
        // different label and a different expression, and trimming
        // dropped the trailing space of `<log label="_event ">`. Both
        // are real W3C documents and both were author text.
        if let Some(expr) = t.strip_prefix("log: ") {
            a.expr = undo(expr, line.number)?;
        } else if let Some(label) = t.strip_prefix("log ") {
            a.label = undo(label, line.number)?;
        } else if t == "log:" {
            for k in kids {
                match split_clause(k.text) {
                    ("label", v) => a.label = undo(v, k.number)?,
                    ("expr", v) => a.expr = undo(v, k.number)?,
                    (other, _) => {
                        return Err(ParseError {
                            line: k.number,
                            why: format!("`{other}` is not a log clause"),
                        })
                    }
                }
            }
        }
    } else if let Some(rest) = t.strip_prefix("call ") {
        a.action_type = "native_action".to_string();
        a.native_action_name = undo(rest.trim_end_matches(':'), line.number)?;
        for (k, sub) in group(kids) {
            let arg = k.text.strip_prefix("arg ").ok_or_else(|| ParseError {
                line: k.number,
                why: format!("`{}` is not a call argument", k.text),
            })?;
            a.params.push(parse_param(arg, &sub, k.number)?);
        }
    } else if t == "assign:" || t.starts_with("assign ") {
        a.action_type = "assign".to_string();
        let rest = t.strip_prefix("assign").unwrap_or("");
        let rest = rest.strip_prefix(' ').unwrap_or(rest);
        a.location = undo(rest.trim_end_matches(':'), line.number)?;
        for k in kids {
            let (keyword, value) = k.text.split_once(' ').ok_or_else(|| ParseError {
                line: k.number,
                why: format!("`{}` is not an assign clause", k.text),
            })?;
            match keyword {
                "expr" => a.expr = undo(value, k.number)?,
                "content" => a.content = undo(value, k.number)?,
                other => {
                    return Err(ParseError {
                        line: k.number,
                        why: format!("`{other}` is not an assign clause"),
                    })
                }
            }
        }
    } else if let Some(rest) = t.strip_prefix("if ") {
        // ⚠ Built rather than filled in, because the model owns what an
        // `<if>` is made of. Replacing `a` wholesale is safe HERE and would
        // not be everywhere: the arms are exclusive and nothing stamps this
        // action before the chain or after it -- the function ends at
        // `Ok(a)`. A site that did stamp first would lose the stamp, which
        // is how a top-level `<script>` lost its annotations once already.
        a = crate::model::Action::if_then(
            undo(rest.trim_end_matches(':'), line.number)?,
            parse_action_list(kids)?,
        );
    } else if let Some(rest) = t.strip_prefix("foreach ") {
        a.action_type = "foreach".to_string();
        let (head, array) = rest
            .trim_end_matches(':')
            .rsplit_once(" in ")
            .ok_or_else(|| ParseError {
                line: line.number,
                why: "a foreach needs `in <array>`".to_string(),
            })?;
        a.array = undo(array, line.number)?;
        match head.split_once(" index ") {
            Some((item, index)) => {
                a.item = undo(item, line.number)?;
                a.index = undo(index, line.number)?;
            }
            None => a.item = undo(head, line.number)?,
        }
        a.actions = parse_action_list(kids)?;
    // ⚠ `send:` — a block whose event name is absent because the
    // document computed it (`<send eventexpr="Var1"/>`). The renderer
    // has always written that shape and this guard did not admit it,
    // so five W3C documents rendered to a page nobody could read back.
    } else if t == "send" || t == "send:" || t.starts_with("send ") {
        a.action_type = "send".to_string();
        let rest = t.strip_prefix("send").unwrap_or("");
        let rest = rest.strip_prefix(' ').unwrap_or(rest);
        a.event = undo(rest.trim_end_matches(':'), line.number)?;
        for (k, sub) in group(kids) {
            // The value keeps any trailing colon: that is how `param
            // <name>:` tells `parse_param` its clauses are below.
            let (keyword, value) = split_clause(k.text);
            let v = undo(value, k.number)?;
            match keyword {
                "eventexpr" => a.eventexpr = v,
                "to" => a.target = v,
                "to-expr" => a.targetexpr = v,
                "type" => a.send_type = v,
                "type-expr" => a.typeexpr = v,
                "after" => a.delay = v,
                "after-expr" => a.delayexpr = v,
                "id" => a.id = v,
                "id-into" => a.idlocation = v,
                "namelist" => a.namelist = v,
                "content" => a.content = v,
                "content-expr" => a.contentexpr = v,
                "param" => a.params.push(parse_param(value, &sub, k.number)?),
                other => {
                    return Err(ParseError {
                        line: k.number,
                        why: format!("`{other}` is not a send clause"),
                    })
                }
            }
        }
    } else if let Some((location, expr)) = t.split_once(" = ") {
        a.action_type = "assign".to_string();
        a.location = undo(location, line.number)?;
        a.expr = undo(expr, line.number)?;
    } else {
        return Err(ParseError {
            line: line.number,
            why: format!("`{t}` is not an action"),
        });
    }
    Ok(a)
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
pub const COVERED_KINDS: &[&str] = &[
    "condition",
    "timer",
    "enum",
    "transform",
    "event-schema",
    "lookup",
    "validator",
    "filter",
    "interpolation",
    "bounded-collection",
    "worker",
    "buffer-pool",
    "link",
    "observer",
    "algorithm",
    "procedure",
    "codec",
    "statechart",
];

/// This document's kind name when the reader covers it.
///
/// The name comes from
/// [`ForgeKind::as_attr`](crate::forge::model::ForgeKind::as_attr), not
/// from a `match` here: which kind a document is is one question, and
/// this module used to answer it a second time. Two spellings would let
/// this report a kind the reader does not actually take. What belongs
/// here is only the second half — whether that name is in
/// [`COVERED_KINDS`].
pub fn covered_kind(doc: &ForgeDocument) -> Option<&'static str> {
    let name = doc.kind().as_attr();
    COVERED_KINDS.contains(&name).then_some(name)
}

/// Whether this document is one the reader covers.
pub fn covers(doc: &ForgeDocument) -> bool {
    covered_kind(doc).is_some()
}
