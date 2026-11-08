// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The bound of an expression's text: the most bytes of the string a value
//! denotes, computed beside type inference (docs/adr/0086, section 8.6 (c)).
//!
//! The analysis is abstract interpretation with a domain of its own, the
//! lengths, over the tree [`crate::forge::expr`] parses and the environment
//! ([`TypeCtx`]) its inference reads. The type stays what
//! [`InferredType`] says, and the length is a second fact about the same
//! value: two expressions of one type are of one type whatever their bounds,
//! so the type keeps an equality that says what it means.
//!
//! The rules follow the lattice of lengths, whose top is [`TextBound::Unbounded`]:
//! - a typed place takes the `B` of its `string[B]` (a variable's
//!   `sce:capacity`, a record's or a payload's field's `sce:max-size`, a loop
//!   item's field), read through the place's type or the bound the data model
//!   declares for its name;
//! - a string literal takes the bytes of the text it denotes, its escapes read;
//! - a concatenation takes the saturating sum of its operands', a conditional
//!   the larger of its branches', and `x ?? d` the larger of `x`'s present text
//!   and `d`'s;
//! - a number or a boolean, read as text, takes the longest text of its type
//!   ([`longest_text`], the one table);
//! - anything else has no bound, and says why, naming the subexpression that
//!   has none.
//!
//! One analysis for every lane. The C11 join of strings sizes its buffer by it
//! ([`crate::forge::expr`]), and the C11 size calculus measures the data a
//! machine writes by it ([`crate::forge::static_lowering`]).

use std::ops::Range;

use crate::forge::expr::{self, BinOp, ExprKind, Refusal, TypedExpr};
use crate::forge::types::{InferredType, TypeCtx};
use crate::forge::value_type::ValueType;

/// The most bytes of the text an expression denotes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextBound {
    /// At most this many bytes.
    Bounded(u64),
    /// No bound the build knows, and why.
    Unbounded(Unbounded),
}

/// Why an expression has no text bound: the subexpression that has none, as
/// the range of the text it was parsed from, and what it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unbounded {
    /// The range of the subexpression in the text the expression was parsed
    /// from; `None` for a node no text spells, which is reported as the whole.
    pub span: Option<Range<usize>>,
    /// What the subexpression is, as a refusal states it.
    pub why: String,
}

impl Unbounded {
    fn at(node: &TypedExpr, why: impl Into<String>) -> Self {
        Self {
            span: node.span.clone(),
            why: why.into(),
        }
    }

    /// The text of the subexpression that has no bound, in `source`, the text
    /// the expression was parsed from: the whole of it where the node records
    /// no range.
    pub fn subexpression<'s>(&self, source: &'s str) -> &'s str {
        self.span
            .clone()
            .and_then(|range| source.get(range))
            .unwrap_or(source)
            .trim()
    }

    /// The reason as a refusal states it: the subexpression of `source` and
    /// why it has no bound.
    pub fn describe(&self, source: &str) -> String {
        format!("`{}` {}", self.subexpression(source), self.why)
    }
}

impl TextBound {
    /// The bound, when there is one.
    pub fn bounded(&self) -> Option<u64> {
        match self {
            TextBound::Bounded(bytes) => Some(*bytes),
            TextBound::Unbounded(_) => None,
        }
    }

    /// The bound of a text made of this one and then `other`: the saturating
    /// sum, or the first of the two that has none.
    pub fn plus(self, other: TextBound) -> TextBound {
        match (self, other) {
            (TextBound::Bounded(a), TextBound::Bounded(b)) => {
                TextBound::Bounded(a.saturating_add(b))
            }
            (TextBound::Unbounded(why), _) | (_, TextBound::Unbounded(why)) => {
                TextBound::Unbounded(why)
            }
        }
    }

    /// The bound of a text that is this one or `other`: the larger, or the
    /// first of the two that has none.
    pub fn larger(self, other: TextBound) -> TextBound {
        match (self, other) {
            (TextBound::Bounded(a), TextBound::Bounded(b)) => TextBound::Bounded(a.max(b)),
            (TextBound::Unbounded(why), _) | (_, TextBound::Unbounded(why)) => {
                TextBound::Unbounded(why)
            }
        }
    }
}

/// The most bytes of the decimal text of an integer of type `ty`: the digits
/// of its largest magnitude and a minus sign for a signed one. A literal no
/// context typed takes the width of an `int64`, the widest it can be written
/// as.
pub fn integer_text_bound(ty: InferredType) -> u64 {
    // `bits * log10(2)` rounded up, in integers: 8 -> 3, 16 -> 5, 32 -> 10, 64 -> 20.
    let digits = |bits: u64| bits * 30103 / 100_000 + 1;
    match ty {
        InferredType::Int {
            signed: false,
            bits,
        } => digits(u64::from(bits)),
        InferredType::Int { signed: true, bits } => digits(u64::from(bits).saturating_sub(1)) + 1,
        _ => 20,
    }
}

/// The longest text a real is written as: ECMAScript's `Number::toString`,
/// which every engine spells a real by (ARCHITECTURE.md, "JSON Number Text"),
/// at most seventeen digits after `-0.00000`. The same number `J` of a real is
/// (docs/adr/0086, section 2.1), since the JSON of a real is that text.
const REAL_TEXT_MAX: u64 = 25;

/// The longest text of a boolean, `false`.
const BOOL_TEXT_MAX: u64 = 5;

/// The one table of the longest text a number or a boolean of type `ty` is
/// read as; `None` for a type that is neither.
pub fn longest_text(ty: InferredType) -> Option<u64> {
    match ty {
        InferredType::UntypedInt | InferredType::Int { .. } => Some(integer_text_bound(ty)),
        InferredType::UntypedFloat | InferredType::Float { .. } => Some(REAL_TEXT_MAX),
        InferredType::Bool => Some(BOOL_TEXT_MAX),
        _ => None,
    }
}

/// The bound of the text `node` denotes, its every node typed by
/// [`crate::forge::expr`]'s inference against `ctx`, which `ctx` the bounds
/// of its places are read from too.
pub(crate) fn text_bound(node: &TypedExpr, ctx: &TypeCtx<'_>) -> TextBound {
    // A number or a boolean is read as the text of its value, which is never
    // longer than the longest text of its type, whatever computes it.
    if let Some(longest) = longest_text(node.ty) {
        return TextBound::Bounded(longest);
    }
    let why = match node.ty {
        InferredType::Str => return text_of_a_string(node, ctx),
        InferredType::Unknown => {
            "is a value whose type is not known here, so it has no text bound".to_string()
        }
        other => format!("is {}, which is not read as a text", other.describe()),
    };
    TextBound::Unbounded(Unbounded::at(node, why))
}

/// [`text_bound`] of a node typed as a string.
fn text_of_a_string(node: &TypedExpr, ctx: &TypeCtx<'_>) -> TextBound {
    match &node.kind {
        // The UTF-8 bytes of the text the literal denotes, its escapes read.
        ExprKind::StringLit { value } => TextBound::Bounded(value.len() as u64),
        ExprKind::Ident(_) | ExprKind::Member { .. } | ExprKind::Index { .. } => {
            place_bound(node, ctx)
        }
        ExprKind::Conditional {
            consequent,
            alternate,
            ..
        } => text_bound(consequent, ctx).larger(text_bound(alternate, ctx)),
        ExprKind::Binary { op, left, right }
            if expr::is_string_concatenation(*op, left.ty, right.ty) =>
        {
            text_bound(left, ctx).plus(text_bound(right, ctx))
        }
        // `x ?? d` is a choice: the present value of `x`, or `d`.
        ExprKind::Binary {
            op: BinOp::Coalesce,
            left,
            right,
        } => present_text_bound(left, ctx).larger(text_bound(right, ctx)),
        ExprKind::Call { .. } => TextBound::Unbounded(Unbounded::at(
            node,
            "is a call, and the text it answers has no bound its signature declares",
        )),
        _ => TextBound::Unbounded(Unbounded::at(
            node,
            "is a text computed by an operation the bound of whose text is not known",
        )),
    }
}

/// The bound of the text the present value of `node` is, for the left side of
/// `node ?? d`: the `B` of the `optional<string[B]>` a place declares. A value
/// that is no optional one is its own present value.
fn present_text_bound(node: &TypedExpr, ctx: &TypeCtx<'_>) -> TextBound {
    if !matches!(node.ty, InferredType::Optional(_)) {
        return text_bound(node, ctx);
    }
    let held = expr::place_path(node).and_then(|path| ctx.value_of(&path));
    if let Some(place) = held {
        if let ValueType::Optional { elem } = &place.ty {
            if let ValueType::Text { max } = elem.as_ref() {
                return TextBound::Bounded(u64::from(max.max));
            }
        }
    }
    TextBound::Unbounded(Unbounded::at(
        node,
        "is an optional value whose present text no place's type bounds",
    ))
}

/// The bound of the text a place holds: the `B` of the `string[B]` its type
/// declares, or the bound the data model declares for its name.
fn place_bound(node: &TypedExpr, ctx: &TypeCtx<'_>) -> TextBound {
    let Some(path) = expr::place_path(node) else {
        return TextBound::Unbounded(Unbounded::at(node, "reads no place a type bounds"));
    };
    if let Some(place) = ctx.value_of(&path) {
        if let ValueType::Text { max } = &place.ty {
            return TextBound::Bounded(u64::from(max.max));
        }
    }
    match ctx.string_capacities.get(path.as_str()) {
        Some(bytes) => TextBound::Bounded(*bytes as u64),
        None => TextBound::Unbounded(Unbounded::at(
            node,
            "is a text whose place declares no bound (`sce:capacity`, `sce:max-size` or \
             `string[B]`)",
        )),
    }
}

/// The bound of the text the expression `written` denotes, read against
/// `ctx`: the expression parsed, its names resolved and its nodes typed as
/// every lowering reads them (`crate::forge::expr::resolve`). A refusal is
/// the expression's own, which the lowering that reads it raises too. The
/// range of an [`Unbounded`] is a range of `written.trim()`.
pub fn text_bound_of(written: &str, ctx: &TypeCtx<'_>) -> Result<TextBound, Refusal> {
    let tree = expr::resolve(written, ctx)?;
    Ok(text_bound(&tree, ctx))
}

/// The most bytes a literal text takes as a JSON string, its quotes included,
/// as either writer of a C11 machine spells it: the build's
/// ([`crate::filters::escape_json_string`]) and the runtime's
/// (`sce_forge_wire_put_text`), which writes a backspace and a form feed as
/// `\u00XX` where the build writes `\b` and `\f`. Each control is counted at
/// the longer of the two.
pub fn json_literal_len(text: &str) -> u64 {
    let body: u64 = text
        .chars()
        .map(|c| match c {
            '"' | '\\' | '\n' | '\r' | '\t' => 2,
            c if (c as u32) < 0x20 => 6,
            c => c.len_utf8() as u64,
        })
        .sum();
    body.saturating_add(2)
}

/// The most bytes of the JSON object a list of `<param>`s is written as, each
/// pair its name and the bound of the JSON of its value: the braces, each
/// name as a quoted key with its colon, and a comma between each two
/// members. Pairs are taken by name, as the writers take them (ascending by
/// name, ARCHITECTURE.md "JSON Object Key Order"): one value is the member's
/// value, and more than one under one name an array of them, with its
/// brackets and its commas, since every value of a repeated name is
/// delivered.
pub fn params_json_bound<'p>(pairs: impl IntoIterator<Item = (&'p str, TextBound)>) -> TextBound {
    let mut by_name: std::collections::BTreeMap<&str, Vec<TextBound>> =
        std::collections::BTreeMap::new();
    for (name, value) in pairs {
        by_name.entry(name).or_default().push(value);
    }
    let commas = (by_name.len() as u64).saturating_sub(1);
    let mut total = TextBound::Bounded(2u64.saturating_add(commas));
    for (name, values) in by_name {
        let key = json_literal_len(name).saturating_add(1);
        let count = values.len() as u64;
        let value = if count == 1 {
            values
                .into_iter()
                .fold(TextBound::Bounded(0), TextBound::plus)
        } else {
            values
                .into_iter()
                .fold(TextBound::Bounded(2 + (count - 1)), TextBound::plus)
        };
        total = total.plus(TextBound::Bounded(key)).plus(value);
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::value_path::Place;
    use crate::forge::value_type::Bound;

    /// A context that declares `wait` a `uint32`, `level` an `int8`, `on` a
    /// boolean, `ratio` a `float32`, `name` a text of eight bytes by its
    /// capacity, `note` a text of twelve by its type, the payload's `label` a
    /// text of six by its schema, and `loose` a text nothing bounds.
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
        ctx.insert_var("on", InferredType::Bool);
        ctx.insert_var("ratio", InferredType::Float { bits: 32 });
        ctx.insert_var("name", InferredType::Str);
        ctx.insert_string_capacity("name", 8);
        ctx.insert_value("note", Place::of(ValueType::Text { max: Bound::of(12) }));
        ctx.insert_value(
            "_event.data.label",
            Place::of(ValueType::Text { max: Bound::of(6) }),
        );
        ctx.insert_var("loose", InferredType::Str);
        ctx
    }

    fn bound(written: &str) -> TextBound {
        text_bound_of(written, &ctx()).expect("the expression resolves")
    }

    #[test]
    fn a_literal_takes_the_bytes_of_the_text_it_denotes() {
        assert_eq!(bound("'abc'"), TextBound::Bounded(3));
        // An escape is read: `\n` is one byte of the text.
        assert_eq!(bound(r"'a\nb'"), TextBound::Bounded(3));
        // A character past ASCII is its UTF-8 bytes.
        assert_eq!(bound("'\u{e9}'"), TextBound::Bounded(2));
    }

    #[test]
    fn a_typed_place_takes_the_bound_of_its_type() {
        // By the type the place holds.
        assert_eq!(bound("note"), TextBound::Bounded(12));
        // By the capacity the data model declares for the name.
        assert_eq!(bound("name"), TextBound::Bounded(8));
    }

    #[test]
    fn a_payload_field_takes_its_schema_bound() {
        assert_eq!(bound("_event.data.label"), TextBound::Bounded(6));
    }

    #[test]
    fn a_concatenation_takes_the_sum_of_its_operands() {
        // 8 + 2.
        assert_eq!(bound("name + 'ms'"), TextBound::Bounded(10));
        // 1 + 10 + 1: a number is its longest text.
        assert_eq!(bound("'a' + wait + 'b'"), TextBound::Bounded(12));
        // 12 + 6 + 8.
        assert_eq!(
            bound("note + _event.data.label + name"),
            TextBound::Bounded(26)
        );
    }

    #[test]
    fn a_conditional_takes_the_larger_of_its_branches() {
        assert_eq!(bound("on ? name : 'x'"), TextBound::Bounded(8));
        assert_eq!(bound("on ? 'x' : note"), TextBound::Bounded(12));
    }

    #[test]
    fn a_coalescing_choice_takes_the_larger_of_the_present_text_and_the_default() {
        let mut ctx = ctx();
        ctx.insert_value(
            "maybe",
            Place::of(ValueType::Optional {
                elem: Box::new(ValueType::Text { max: Bound::of(9) }),
            }),
        );
        let at = |written: &str| text_bound_of(written, &ctx).expect("the expression resolves");
        assert_eq!(at("maybe ?? 'x'"), TextBound::Bounded(9));
        assert_eq!(at("maybe ?? note"), TextBound::Bounded(12));
    }

    #[test]
    fn a_number_or_a_boolean_takes_the_longest_text_of_its_type() {
        assert_eq!(bound("wait"), TextBound::Bounded(10));
        assert_eq!(bound("level"), TextBound::Bounded(4));
        assert_eq!(bound("on"), TextBound::Bounded(5));
        assert_eq!(bound("ratio"), TextBound::Bounded(25));
        // A literal no context typed is as wide as an `int64`.
        assert_eq!(bound("1000"), TextBound::Bounded(20));
        assert_eq!(
            longest_text(InferredType::Int {
                signed: true,
                bits: 64
            }),
            Some(20)
        );
        assert_eq!(
            longest_text(InferredType::Int {
                signed: false,
                bits: 8
            }),
            Some(3)
        );
        assert_eq!(longest_text(InferredType::Str), None);
    }

    #[test]
    fn anything_else_has_no_bound_and_names_the_subexpression_that_has_none() {
        let written = "name + loose";
        let TextBound::Unbounded(why) = bound(written) else {
            panic!("a text nothing bounds has no bound");
        };
        assert_eq!(why.subexpression(written), "loose");
        assert!(why
            .describe(written)
            .starts_with("`loose` is a text whose place"));
        // An unbounded operand leaves the whole without a bound, whichever
        // side it stands on.
        assert!(bound("loose + 'x'").bounded().is_none());
        assert!(bound("on ? 'x' : loose").bounded().is_none());
    }

    #[test]
    fn json_of_params_takes_each_name_and_groups_a_repeated_one() {
        // `{"a":<3>}`: braces 2, key `"a":` 4, value 3.
        assert_eq!(
            params_json_bound([("a", TextBound::Bounded(3))]),
            TextBound::Bounded(9)
        );
        // `{"a":[<3>,<4>],"b":<1>}`: 2 + 1 comma, `"a":` 4 + `[` `]` 2 + 3 + 1
        // comma + 4, `"b":` 4 + 1.
        assert_eq!(
            params_json_bound([
                ("b", TextBound::Bounded(1)),
                ("a", TextBound::Bounded(3)),
                ("a", TextBound::Bounded(4)),
            ]),
            TextBound::Bounded(3 + 4 + 2 + 3 + 1 + 4 + 4 + 1)
        );
        assert!(params_json_bound([(
            "a",
            TextBound::Unbounded(Unbounded {
                span: None,
                why: String::new()
            })
        )])
        .bounded()
        .is_none());
        // A quote and a control each at their longest escape.
        assert_eq!(json_literal_len("a\"\u{8}"), 2 + 1 + 2 + 6);
    }
}
