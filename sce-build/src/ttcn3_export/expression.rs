// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A `sce-static` expression of a test component written as a TTCN-3
//! expression (docs/SCE_TEST_PROFILE.md §9.5).
//!
//! The expression is not read again from its text. It is the tree the data
//! model's judge reads ([`crate::forge::expr::resolve`]), typed against the
//! scope every backend's lowering reads ([`crate::forge::type_ctx::StaticScope`]),
//! and this module prints that tree. What the tree holds that TTCN-3 has no
//! form for is refused by name, never approximated:
//!
//! - integer arithmetic, bitwise and shift operators. `sce-static` computes
//!   each at the width of its operands and fails the statement that holds an
//!   overflow (`error.execution`), and a TTCN-3 `integer` has no width, so
//!   the two would compute different values where one fails;
//! - a conditional expression, which TTCN-3 does not have;
//! - a call, an index, a value that may be absent, `null` and `??`, which
//!   name values a TTCN-3 component of the profile does not hold;
//! - a float, which the profile leaves out (§8.1).
//!
//! What is written: a variable or a module parameter by its name; a field of
//! the received message as a field of the variable the receive stores it in
//! (-4 §9.37.3, the value redirect); a variant of an enum by its name; an
//! integer as its decimal digits, `true` and `false`, a text as a character
//! string (the filter every template writes one with), a byte string as
//! `'…'O`; `===` and `!==` as `==` and `!=`, the four relational operators
//! as they are, `&&`, `||` and `!` as `and`, `or` and `not`. Each operand
//! that is itself an operation is parenthesised, so the tree is the one
//! TTCN-3 reads whatever its operators' precedence (-1 §7.1.0, table 4).

use std::collections::BTreeSet;

use crate::forge::expr::{BinOp, ExprKind, TypedExpr, UnaryOp};
use crate::forge::model::EnumModel;
use crate::forge::types::InferredType;

/// What an expression is read against.
pub(super) struct Reading<'r> {
    /// The variables and module parameters of the component, by id: the
    /// names an expression may read.
    pub(super) names: &'r BTreeSet<String>,
    /// The enums the component imports, by alias.
    pub(super) enums: &'r std::collections::BTreeMap<String, EnumModel>,
    /// The variable a receive branch stores its message in, where the
    /// expression stands in the content of one that reads it.
    pub(super) message: Option<&'r str>,
}

/// An expression written: its text, and whether it reads the received
/// message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Printed {
    pub(super) text: String,
    pub(super) reads_message: bool,
}

/// `expr` as TTCN-3, or why it has none.
pub(super) fn print(expr: &TypedExpr, reading: &Reading<'_>) -> Result<Printed, String> {
    let mut reads_message = false;
    let text = node(expr, reading, &mut reads_message)?;
    Ok(Printed {
        text,
        reads_message,
    })
}

/// The decimal digits of an integer literal written in any base the
/// expression language reads, or `None` for a literal that is no integer.
pub(super) fn integer_literal(written: &str) -> Option<i128> {
    let digits: String = written.chars().filter(|c| *c != '_').collect();
    let (body, radix) = match digits.get(..2) {
        Some("0x") | Some("0X") => (&digits[2..], 16),
        Some("0b") | Some("0B") => (&digits[2..], 2),
        Some("0o") | Some("0O") => (&digits[2..], 8),
        _ => (digits.as_str(), 10),
    };
    if body.is_empty() || !body.chars().all(|c| c.is_digit(radix)) {
        return None;
    }
    i128::from_str_radix(body, radix).ok()
}

/// A byte string as TTCN-3 writes an `octetstring`: its hex digits, in upper
/// case, between `'` and `'O`.
pub(super) fn octetstring(bytes: &[u8]) -> String {
    let hex: String = bytes.iter().map(|b| format!("{b:02X}")).collect();
    format!("'{hex}'O")
}

/// The path below `_event.data` that `expr` reads, when it reads one.
fn message_field(expr: &TypedExpr) -> Option<String> {
    let path = crate::forge::expr::dotted_path(expr)?;
    path.strip_prefix(crate::forge::event_schema_check::EVENT_DATA_PATH)
        .map(|rest| rest.trim_start_matches('.').to_string())
}

fn node(expr: &TypedExpr, reading: &Reading<'_>, reads: &mut bool) -> Result<String, String> {
    if matches!(expr.ty, InferredType::Optional(_) | InferredType::Null) {
        return Err(
            "it reads a value that may be absent, which no variable of a TTCN-3 component of \
             the profile holds"
                .to_string(),
        );
    }
    match &expr.kind {
        ExprKind::NumberLit(written) => {
            if matches!(
                expr.ty,
                InferredType::UntypedFloat | InferredType::Float { .. }
            ) {
                return Err(format!(
                    "{written} is a float, which the test profile leaves out \
                     (docs/SCE_TEST_PROFILE.md §8.1)"
                ));
            }
            integer_literal(written)
                .map(|n| n.to_string())
                .ok_or_else(|| format!("{written} is no integer"))
        }
        ExprKind::StringLit { value } => Ok(crate::filters::ttcn3_charstring(value.clone())),
        ExprKind::BytesLit { bytes } => Ok(octetstring(bytes)),
        ExprKind::BoolLit(value) => Ok(value.to_string()),
        ExprKind::Ident(name) if reading.names.contains(name) => Ok(name.clone()),
        ExprKind::Ident(name) => Err(format!(
            "`{name}` is no variable or module parameter of the component"
        )),
        ExprKind::Member { object, property } => {
            if let Some(field) = message_field(expr) {
                let Some(message) = reading.message else {
                    return Err(format!(
                        "`_event.data.{field}` reads a message where no receive stores one"
                    ));
                };
                // A segment `value` reads the present value of an optional
                // record, and no field is named so: `value` is a reserved
                // word, which the types module refuses as a field's name.
                if field.is_empty() || field.split('.').any(|segment| segment == "value") {
                    return Err(format!(
                        "`_event.data.{field}` reads the message whole, or a value that may \
                         be absent, which a TTCN-3 component of the profile does not hold"
                    ));
                }
                *reads = true;
                return Ok(format!("{message}.{field}"));
            }
            if let ExprKind::Ident(alias) = &object.kind {
                if let Some(model) = reading.enums.get(alias) {
                    if model.variants.iter().any(|v| &v.name == property) {
                        return Ok(property.clone());
                    }
                    return Err(format!("`{alias}.{property}` is no variant of the enum"));
                }
            }
            Err(format!(
                "`.{property}` reads a field of a value that is no received message, which a \
                 TTCN-3 component of the profile does not hold"
            ))
        }
        ExprKind::Binary { op, left, right } => {
            let spelled = match op {
                BinOp::StrictEq => "==",
                BinOp::StrictNeq => "!=",
                BinOp::Lt => "<",
                BinOp::Gt => ">",
                BinOp::LtEq => "<=",
                BinOp::GtEq => ">=",
                BinOp::And => "and",
                BinOp::Or => "or",
                other => {
                    return Err(format!(
                        "`{}` computes at the width of its operands and fails on an overflow, \
                         and a TTCN-3 integer has no width; or it reads a value that may be \
                         absent",
                        other.token()
                    ))
                }
            };
            if matches!(op, BinOp::Lt | BinOp::Gt | BinOp::LtEq | BinOp::GtEq)
                && [left, right].iter().any(|side| {
                    matches!(
                        side.ty,
                        InferredType::Str | InferredType::Bytes | InferredType::Bool
                    )
                })
            {
                return Err(format!(
                    "`{spelled}` orders texts, byte strings or bools, and TTCN-3 orders integers \
                     and enumerated values alone (ES 201 873-1 §7.1.3)"
                ));
            }
            Ok(format!(
                "{} {spelled} {}",
                operand(left, reading, reads)?,
                operand(right, reading, reads)?
            ))
        }
        ExprKind::Unary {
            op: UnaryOp::Not,
            operand: inner,
        } => Ok(format!("not {}", operand(inner, reading, reads)?)),
        ExprKind::Unary {
            op: UnaryOp::Neg,
            operand: inner,
        } if matches!(inner.kind, ExprKind::NumberLit(_)) => {
            Ok(format!("-{}", node(inner, reading, reads)?))
        }
        ExprKind::Unary { .. } => Err(
            "a unary operator other than `!` computes at the width of its operand, and a \
             TTCN-3 integer has no width"
                .to_string(),
        ),
        ExprKind::Conditional { .. } => {
            Err("a conditional expression has no TTCN-3 form".to_string())
        }
        ExprKind::Call { .. } | ExprKind::Total { .. } | ExprKind::Checked { .. } => Err(
            "a call, or an operation of a stated width, has no TTCN-3 form the export writes"
                .to_string(),
        ),
        ExprKind::Index { .. } | ExprKind::BytesView { .. } => Err(
            "it reads an element of a list, which no TTCN-3 component of the profile holds"
                .to_string(),
        ),
        ExprKind::NullLit | ExprKind::Raw(_) => {
            Err("it names a value that may be absent".to_string())
        }
    }
}

/// An operand of an operator: parenthesised when it is itself an operation.
fn operand(expr: &TypedExpr, reading: &Reading<'_>, reads: &mut bool) -> Result<String, String> {
    let text = node(expr, reading, reads)?;
    Ok(match expr.kind {
        ExprKind::Binary { .. } | ExprKind::Unary { .. } => format!("({text})"),
        _ => text,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_integer_literal_is_its_decimal_digits_in_any_base() {
        assert_eq!(integer_literal("42"), Some(42));
        assert_eq!(integer_literal("0x1F"), Some(31));
        assert_eq!(integer_literal("0b101"), Some(5));
        assert_eq!(integer_literal("0o17"), Some(15));
        assert_eq!(integer_literal("1_000"), Some(1000));
        assert_eq!(integer_literal("1.5"), None);
        assert_eq!(integer_literal("0x"), None);
    }

    #[test]
    fn a_byte_string_is_written_in_upper_case_hex() {
        assert_eq!(octetstring(&[0x0a, 0xff]), "'0AFF'O");
        assert_eq!(octetstring(&[]), "''O");
    }
}
