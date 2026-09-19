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
    BackpressurePolicy, BoundedCollectionModel, BufferPoolModel, BufferPoolVariant, CachePolicy,
    CapacitySource, CollectionOrdering, ConcurrencyMode, ConditionModel, Direction, EnumModel,
    EnumVariant, EventSchemaModel, FilterModel, FilterType, ForgeDocument, ForgeField, InboxConfig,
    InboxOrdering, InterpolationAxis, InterpolationMethod, InterpolationModel, LinkClass,
    LinkInboundEvent, LinkModel, LinkOutboundEvent, LookupEntry, LookupModel, MissPolicy,
    OutOfBounds, OverflowPolicy, RangeRule, RateOfChangeRule, ReassemblyConfig, SceType,
    TimerModel, TransformModel, ValidatorModel, ValidatorRules, WorkerModel,
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
        },
        source_location: None,
    };
    for l in body {
        let w: Vec<&str> = l.text.split_whitespace().collect();
        match w.first().copied() {
            Some("range") => {
                let mut r = RangeRule {
                    id: undo(w.get(1).copied().unwrap_or(""), l.number)?,
                    min: None,
                    max: None,
                };
                let mut i = 2;
                while i < w.len() {
                    match w[i] {
                        "min" => r.min = Some(undo(w.get(i + 1).copied().unwrap_or(""), l.number)?),
                        "max" => r.max = Some(undo(w.get(i + 1).copied().unwrap_or(""), l.number)?),
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
            "quantity" => {
                let scale = rational(rest.get(i + 1).copied().unwrap_or(""), line.number)?;
                let offset = rational(rest.get(i + 2).copied().unwrap_or(""), line.number)?;
                let unit = crate::forge::quantity::UnitTag::intern(&undo(
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
];

/// This document's kind name when the reader covers it.
///
/// One `match` rather than a predicate plus a separate namer: the
/// round-trip gate needs the name to report which kinds it exercised,
/// and two spellings of "which kind is this" would let the gate count a
/// kind the reader does not actually take.
pub fn covered_kind(doc: &ForgeDocument) -> Option<&'static str> {
    let name = match doc {
        ForgeDocument::Condition(_) => "condition",
        ForgeDocument::Timer(_) => "timer",
        ForgeDocument::Enum(_) => "enum",
        ForgeDocument::Transform(_) => "transform",
        ForgeDocument::EventSchema(_) => "event-schema",
        ForgeDocument::Lookup(_) => "lookup",
        ForgeDocument::Validator(_) => "validator",
        ForgeDocument::Filter(_) => "filter",
        ForgeDocument::Interpolation(_) => "interpolation",
        ForgeDocument::BoundedCollection(_) => "bounded-collection",
        ForgeDocument::Worker(_) => "worker",
        ForgeDocument::BufferPool(_) => "buffer-pool",
        ForgeDocument::Link(_) => "link",
        _ => return None,
    };
    COVERED_KINDS.contains(&name).then_some(name)
}

/// Whether this document is one the reader covers.
pub fn covers(doc: &ForgeDocument) -> bool {
    covered_kind(doc).is_some()
}
