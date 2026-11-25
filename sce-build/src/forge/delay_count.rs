// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A `<send>`'s `delayexpr` that joins an integer to the unit it counts
//! (`wait + 'ms'`), read as the count and its unit (docs/adr/0134, decision 1).
//!
//! The text such a join spells is the integer's decimal digits followed by the
//! unit, and the one CSS2 time reader (ARCHITECTURE.md, "Durations (Single
//! Source of Truth)") reads it as the count times the unit's milliseconds when
//! the count is not negative and the product fits the largest delay, and as no
//! time otherwise. A backend whose machine computes the milliseconds itself
//! ([`crate::forge::static_lowering::StaticTarget::computes_delay_ms`]) has
//! the same answers from the integer with checked arithmetic and no text in
//! between, which the Rust runtime holds equal to the text's over every value of
//! every integer type (`helpers::send::delay_of_count`).
//!
//! [`DelayUnit`] is also the unit table the build's own reader of a written
//! `delay` scales by ([`crate::parser::parse_delay_to_ms`]), so the build has
//! one.

use crate::forge::expr::{self, BinOp, ExprKind};
use crate::forge::types::{InferredType, TypeCtx};

/// The unit a CSS2 time is written in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DelayUnit {
    /// `ms`, either case.
    Milliseconds,
    /// `s`, either case.
    Seconds,
}

impl DelayUnit {
    /// The milliseconds one of this unit is.
    pub const fn milliseconds(self) -> i64 {
        match self {
            Self::Milliseconds => 1,
            Self::Seconds => 1000,
        }
    }

    /// The unit `literal` names when it is joined straight after a count's
    /// decimal digits: `ms` or `s`, either case, with nothing before it and
    /// only ASCII whitespace after it, which the reader drops from the end of a
    /// time. `None` for any other literal, whose join the reader reads as
    /// something else than the count's time (a space before the unit, digits,
    /// a fraction, another word); `tests/durations/count_unit.json` holds the
    /// cases.
    pub fn after_count(literal: &str) -> Option<Self> {
        let unit = literal.trim_end_matches(|c: char| c.is_ascii_whitespace());
        if unit.eq_ignore_ascii_case("ms") {
            Some(Self::Milliseconds)
        } else if unit.eq_ignore_ascii_case("s") {
            Some(Self::Seconds)
        } else {
            None
        }
    }
}

/// A `delayexpr` read as an integer and the unit it counts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DelayCount {
    /// The integer's expression, as the document writes it.
    pub count: String,
    /// The integer's type, as inference gives it.
    pub ty: InferredType,
    /// The unit the integer counts.
    pub unit: DelayUnit,
}

impl DelayCount {
    /// The type the count is lowered at: its own, or an `int64` for a literal
    /// no context typed, the widest an integer literal is written as.
    pub fn slot(&self) -> InferredType {
        match self.ty {
            InferredType::UntypedInt => InferredType::Int {
                signed: true,
                bits: 64,
            },
            ty => ty,
        }
    }
}

/// `written`, a `delayexpr` resolved against `ctx`, read as a count and its
/// unit when it is `<integer> + '<unit>'` ([`DelayUnit::after_count`]); `None`
/// for every other expression, which is read as the text it computes. An
/// expression this reading cannot resolve is none of the form either, and its
/// own lowering judges it.
pub fn delay_count(written: &str, ctx: &TypeCtx<'_>) -> Option<DelayCount> {
    let source = written.trim();
    let tree = expr::resolve(source, ctx).ok()?;
    let ExprKind::Binary {
        op: BinOp::Add,
        left,
        right,
    } = &tree.kind
    else {
        return None;
    };
    if !matches!(left.ty, InferredType::Int { .. } | InferredType::UntypedInt) {
        return None;
    }
    let ExprKind::StringLit { value } = &right.kind else {
        return None;
    };
    let unit = DelayUnit::after_count(value)?;
    let count = source.get(left.span.clone()?)?.trim();
    (!count.is_empty()).then(|| DelayCount {
        count: count.to_string(),
        ty: left.ty,
        unit,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// tests/durations/count_unit.json: the build reads each literal as the
    /// unit the table gives it, or as none.
    #[test]
    fn a_literal_is_read_as_the_unit_the_shared_table_gives_it() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../tests/durations/count_unit.json"
        );
        let table: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(path).expect("the shared count-unit table"),
        )
        .expect("the table is JSON");
        let cases = table["cases"].as_array().expect("the table has cases");
        assert!(cases.len() >= 10, "the table lost cases: {}", cases.len());
        let mut units = 0;
        for case in cases {
            let literal = case["literal"].as_str().expect("a case's literal is text");
            let want = match case["unit"].as_str() {
                Some("ms") => Some(DelayUnit::Milliseconds),
                Some("s") => Some(DelayUnit::Seconds),
                None if case["unit"].is_null() => None,
                other => panic!("{literal:?}: unit is \"ms\", \"s\" or null, got {other:?}"),
            };
            units += usize::from(want.is_some());
            assert_eq!(DelayUnit::after_count(literal), want, "{literal:?}");
        }
        assert!(
            units > 0 && units < cases.len(),
            "the table names both kinds"
        );
    }

    /// A context that declares `wait` a `uint32`, `level` an `int8`, `ratio`
    /// a `float32` and `name` a text of eight bytes.
    fn ctx() -> TypeCtx<'static> {
        let mut ctx = TypeCtx::new();
        ctx.insert_var(
            "wait",
            InferredType::Int {
                signed: false,
                bits: 32,
            },
        );
        ctx.insert_var(
            "level",
            InferredType::Int {
                signed: true,
                bits: 8,
            },
        );
        ctx.insert_var("ratio", InferredType::Float { bits: 32 });
        ctx.insert_var("name", InferredType::Str);
        ctx.insert_string_capacity("name", 8);
        ctx
    }

    /// docs/adr/0134, decision 1: an integer joined to a unit is read as the
    /// count and its unit, the count as written and typed; anything else is
    /// read as the text it computes.
    #[test]
    fn an_integer_joined_to_a_unit_is_a_count_and_anything_else_a_text() {
        let ctx = ctx();
        let count = |written: &str| delay_count(written, &ctx);
        let uint32 = InferredType::Int {
            signed: false,
            bits: 32,
        };
        assert_eq!(
            count("wait + 'ms'"),
            Some(DelayCount {
                count: "wait".to_string(),
                ty: uint32,
                unit: DelayUnit::Milliseconds,
            })
        );
        assert_eq!(
            count("  level + 'S'  ").map(|c| (c.count, c.unit)),
            Some(("level".to_string(), DelayUnit::Seconds))
        );
        // The integer is the whole left operand, an operation included.
        assert_eq!(
            count("wait * 2 + 'ms '").map(|c| (c.count, c.ty)),
            Some(("wait * 2".to_string(), uint32))
        );
        // A literal no context typed is lowered as the widest integer.
        let literal = count("5 + 's'").expect("a literal count");
        assert_eq!(literal.count, "5");
        assert_eq!(
            literal.slot(),
            InferredType::Int {
                signed: true,
                bits: 64
            }
        );
        for text in [
            "'5s'",
            "name + 'ms'",
            "ratio + 'ms'",
            "'ms' + wait",
            "wait + ' ms'",
            "wait + '0ms'",
            "wait + 'ms' + ''",
            "wait",
            "level > 0 ? wait + 'ms' : '1s'",
        ] {
            assert_eq!(count(text), None, "{text}");
        }
    }
}
