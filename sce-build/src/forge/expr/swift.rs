// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Emitter — Swift.
//
// Swift is as strict as Kotlin about numeric types — no implicit conversion between
// integer widths, signedness or a real — and unlike Kotlin an integer literal is
// polymorphic: `5` is whatever integer or real type the place it lands in is. So a
// literal is left as written and every other operand is converted to the type its
// operator computes in, with an initialiser of that type.
//
// What differs from the other emitters, and why:
//
// * Swift TRAPS on an integer overflow and on a division by zero. Where the
//   integer arithmetic contract (SCE_FORGE.md §3.4.1) leaves an operation
//   unchecked the analysis has proved it cannot reach either, so the operator is
//   written as it is; every operation that can is a `Checked` helper
//   (`SceForgeRuntime`), which records the failure in `sceFailure` and yields 0, as
//   the Go and C++ ones do.
//
// * A conversion between integer widths is written `T(truncatingIfNeeded:)` where
//   it can narrow or change sign — the two's-complement wrap every other backend
//   makes — and `T(x)` where it widens, which never traps. A real is turned into
//   an integer only by `round` and `floor`, which `Real` of the forge runtime
//   saturates, so an out-of-range real is the limit and never a trap.
//
// * `String`'s `==` is canonical equivalence and its `<` the collation order, where
//   every other backend compares UTF-8 bytes (a bound counts them too), so a
//   comparison is made over `utf8`.
//
// * A comparison operator is non-associative in Swift (`a < b == c` is a
//   compile-time error), and the bitwise and shift operators sit at other levels
//   than ECMAScript places them, so a bitwise or shift operand, and a comparison in
//   a comparison, is always parenthesised rather than left to the precedence table.

use super::*;

/// The Swift spelling of an integer type.
pub(super) fn int_type(signed: bool, bits: u8) -> &'static str {
    match (signed, bits) {
        (true, 8) => "Int8",
        (true, 16) => "Int16",
        (true, 32) => "Int32",
        (true, _) => "Int64",
        (false, 8) => "UInt8",
        (false, 16) => "UInt16",
        (false, 32) => "UInt32",
        (false, _) => "UInt64",
    }
}

/// The unsigned type of the width of an integer type, which a logical shift is made
/// in.
fn unsigned_of(bits: u8) -> &'static str {
    int_type(false, bits)
}

/// The Swift spelling of a type a value can be named in, or `None` for one that has
/// no single spelling here (an untyped literal, an opaque value).
pub(crate) fn nameable_type(ty: InferredType) -> Option<String> {
    Some(match ty {
        InferredType::Int { signed, bits } => int_type(signed, bits).to_string(),
        InferredType::Float { bits: 32 } => "Float".to_string(),
        InferredType::Float { .. } => "Double".to_string(),
        InferredType::Bool => "Bool".to_string(),
        InferredType::Str => "String".to_string(),
        InferredType::Bytes => "[UInt8]".to_string(),
        _ => return None,
    })
}

/// A value of `ty` that stands where a checked operation that failed answers, the
/// zero its helpers yield — or `None` for a type with no zero to name.
pub(super) fn zero_of(ty: InferredType) -> Option<&'static str> {
    Some(match ty {
        InferredType::Int { .. } | InferredType::UntypedInt => "0",
        InferredType::Float { .. } | InferredType::UntypedFloat => "0.0",
        InferredType::Bool => "false",
        InferredType::Str => "\"\"",
        InferredType::Bytes => "[]",
        _ => return None,
    })
}

/// The precedence of an operator as the other emitters read it: ECMAScript's own
/// order, each class apart. The Swift table is not this one, which
/// [`swift_operand`] makes up for.
fn precedence(op: BinOp) -> u8 {
    match op {
        BinOp::Coalesce => COALESCE_PRECEDENCE,
        BinOp::Or => 1,
        BinOp::And => 2,
        BinOp::BitOr => 3,
        BinOp::BitXor => 4,
        BinOp::BitAnd => 5,
        BinOp::StrictEq | BinOp::StrictNeq => 6,
        BinOp::Lt | BinOp::Gt | BinOp::LtEq | BinOp::GtEq => 7,
        BinOp::Shl | BinOp::Shr | BinOp::UShr => 8,
        BinOp::Add | BinOp::Sub => 9,
        BinOp::Mul | BinOp::Div | BinOp::Mod => 10,
    }
}

/// `text`, the operand `child` of `parent`, parenthesised where Swift would bind it
/// to something else than the document does.
///
/// The precedence table decides what it can; the rest is the two places Swift's
/// differs. A bitwise or shift operator is at a level of its own, so one that is an
/// operand or has one is parenthesised whatever sits beside it, and a comparison
/// beside a comparison is, which Swift will not associate.
fn swift_operand(child: &TypedExpr, parent: BinOp, is_left: bool, text: String) -> String {
    let wrap = child_needs_parens(child, parent, is_left, precedence)
        || match &child.kind {
            ExprKind::Binary { op, .. } => {
                (op.is_comparison() && parent.is_comparison())
                    || (op.is_bitwise() && !parent.is_logical())
                    || (parent.is_bitwise() && !op.is_logical())
            }
            _ => false,
        };
    if wrap {
        format!("({text})")
    } else {
        text
    }
}

fn binop(op: BinOp) -> &'static str {
    match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Mod => "%",
        BinOp::StrictEq => "==",
        BinOp::StrictNeq => "!=",
        BinOp::Lt => "<",
        BinOp::Gt => ">",
        BinOp::LtEq => "<=",
        BinOp::GtEq => ">=",
        BinOp::And => "&&",
        BinOp::Or => "||",
        BinOp::BitAnd => "&",
        BinOp::BitOr => "|",
        BinOp::BitXor => "^",
        BinOp::Shl => "<<",
        BinOp::Shr | BinOp::UShr => ">>",
        BinOp::Coalesce => unreachable!("{COALESCE_NOT_SPELLED}"),
    }
}

/// Raised where a construct has no Swift spelling yet, placed at the node.
fn unsupported(construct: impl Into<String>, at: &TypedExpr) -> Refusal {
    ExprError::UnsupportedConstruct {
        construct: construct.into(),
        observed: None,
    }
    .at(at.span.clone())
}

pub(super) fn emit(expr: &TypedExpr, expected: InferredType) -> Result<String, Refusal> {
    // A quantity is emitted as its numeric base (see `rust_coerce`).
    let expected = expected.strip_quantity();
    // Push-down: see emit_rust for rationale.
    if let ExprKind::Binary { op, left, right } = &expr.kind {
        if op.is_arith()
            && !is_string_concatenation(*op, left.ty, right.ty)
            && matches!(expected, InferredType::Float { .. })
        {
            let l = swift_operand(left, *op, true, emit(left, expected)?);
            let r = swift_operand(right, *op, false, emit(right, expected)?);
            return Ok(format!("{l} {} {r}", binop(*op)));
        }
    }
    // Push-down: `-40` in a real context is `-40.0`, as the literal's own `.0` is
    // written in `coerce`.
    if let ExprKind::Unary {
        op: op @ (UnaryOp::Neg | UnaryOp::Pos),
        operand,
    } = &expr.kind
    {
        if matches!(expected, InferredType::Float { .. }) {
            let inner = emit(operand, expected)?;
            return Ok(prefixed(op, operand, inner));
        }
    }
    if let ExprKind::Call { callee, args, .. } = &expr.kind {
        // `len(x)` is Swift's `Int`, which is none of the integer types a document
        // declares: in a sized-integer context it is converted to that one, as
        // Go's and Rust's are.
        if is_len_builtin(callee, args) {
            if let InferredType::Int { signed, bits } = expected {
                return Ok(format!(
                    "{}(truncatingIfNeeded: {})",
                    int_type(signed, bits),
                    length_of(&args[0])?
                ));
            }
        }
        // The context decides the width, not a constant (see `real_to_int_builtin`).
        if let Some(op) = real_to_int_builtin(callee, args) {
            if let InferredType::Int { signed, bits } = expected {
                return Ok(real_to_int(
                    op,
                    emit(&args[0], InferredType::Float { bits: 64 })?,
                    int_type(signed, bits),
                ));
            }
        }
    }
    let raw = emit_node(expr)?;
    Ok(coerce(raw, expr.ty, expected, expr))
}

/// `round(real)` or `floor(real)` as an integer of `target`.
fn real_to_int(op: RealToInt, real: String, target: &str) -> String {
    let helper = match op {
        RealToInt::Round => "round",
        RealToInt::Floor => "floor",
    };
    format!("Real.{helper}({real}, to: {target}.self)")
}

/// A unary `op` over `operand`, written `inner`, parenthesised where it would
/// otherwise run into the operator (`- -x` is not `--x`, a Swift operator of its own).
fn prefixed(op: &UnaryOp, operand: &TypedExpr, inner: String) -> String {
    let prefix = match op {
        UnaryOp::Neg => "-",
        UnaryOp::Pos => "+",
        UnaryOp::Not => "!",
        UnaryOp::BitNot => "~",
    };
    let wrap = matches!(
        &operand.kind,
        ExprKind::Binary { .. } | ExprKind::Conditional { .. } | ExprKind::Unary { .. }
    );
    if wrap {
        format!("{prefix}({inner})")
    } else {
        format!("{prefix}{inner}")
    }
}

/// The number of elements, or of UTF-8 bytes of a string, of `measured` as Swift's
/// `Int`.
fn length_of(measured: &TypedExpr) -> Result<String, Refusal> {
    let text = wrap_postfix(measured, emit(measured, InferredType::Unknown)?);
    Ok(if measured.ty == InferredType::Str {
        format!("{text}.utf8.count")
    } else {
        format!("{text}.count")
    })
}

fn emit_node(expr: &TypedExpr) -> Result<String, Refusal> {
    if optional_form(expr).is_some() {
        return Err(unsupported(
            "an optional value has no Swift lowering yet (docs/adr/0055)",
            expr,
        ));
    }
    Ok(match &expr.kind {
        ExprKind::NumberLit(n) => portable_number_literal(n),
        ExprKind::StringLit { value, .. } => format!("\"{value}\""),
        ExprKind::BytesLit { bytes } => {
            format!("Array(\"{}\".utf8)", bytes_as_quoted_ascii(bytes))
        }
        ExprKind::BoolLit(b) => if *b { "true" } else { "false" }.to_string(),
        ExprKind::NullLit => "nil".to_string(),
        // The spelling the function DECLARED the name with: one function spells the
        // parameter, the variable and every read ([`ExprTarget::local_spelling`]).
        ExprKind::Ident(s) => ExprTarget::Swift.local_spelling(s),
        ExprKind::Raw(s) => s.clone(),
        ExprKind::Binary { op, left, right } if is_string_concatenation(*op, left.ty, right.ty) => {
            // The digits of an integer are `String(n)`; ECMA-262 13.15.3, and the
            // same text in every backend.
            let operand = |side: &TypedExpr, is_left: bool| -> Result<String, Refusal> {
                let text = emit(side, side.ty)?;
                match concat_operand(side.ty) {
                    ConcatOperand::Text => {
                        Ok(if concat_text_needs_parens(side, is_left, precedence) {
                            format!("({text})")
                        } else {
                            text
                        })
                    }
                    ConcatOperand::Decimal => Ok(format!("String({text})")),
                    ConcatOperand::Refused(what) => Err(concat_refusal(what, text).into()),
                }
            };
            format!("{} + {}", operand(left, true)?, operand(right, false)?)
        }
        ExprKind::Binary { op, left, right } => binary(*op, left, right)?,
        ExprKind::Unary { op, operand } => {
            let inner = emit(operand, expr.ty)?;
            prefixed(op, operand, inner)
        }
        ExprKind::Conditional {
            condition,
            consequent,
            alternate,
        } => format!(
            "{} ? {} : {}",
            wrap_conditional_part(condition, emit(condition, InferredType::Bool)?),
            wrap_conditional_part(consequent, emit(consequent, expr.ty)?),
            wrap_conditional_part(alternate, emit(alternate, expr.ty)?),
        ),
        ExprKind::Member { object, property } => format!(
            "{}.{property}",
            wrap_postfix(object, emit(object, InferredType::Unknown)?)
        ),
        ExprKind::Index { object, index } => format!(
            "{}[{}]",
            wrap_postfix(object, emit(object, InferredType::Unknown)?),
            subscript(index)?,
        ),
        ExprKind::Call {
            callee,
            args,
            params,
            fails,
        } => {
            if is_len_builtin(callee, args) {
                let length = length_of(&args[0])?;
                return Ok(match expr.ty {
                    InferredType::Int { signed, bits } => {
                        format!("{}(truncatingIfNeeded: {length})", int_type(signed, bits))
                    }
                    _ => length,
                });
            }
            if let Some(op) = real_to_int_builtin(callee, args) {
                return Ok(real_to_int(
                    op,
                    emit(&args[0], InferredType::Float { bits: 64 })?,
                    "Int64",
                ));
            }
            let mut a = Vec::with_capacity(args.len());
            for (i, arg) in args.iter().enumerate() {
                a.push(emit(arg, argument_type(params, i))?);
            }
            let symbol = wrap_postfix(callee, emit(callee, InferredType::Unknown)?);
            let call = format!("{symbol}({})", a.join(", "));
            if *fails {
                pass_failure_on(ExprTarget::Swift, &symbol, &call, expr.ty)
                    .map_err(|e| e.at(expr.span.clone()))?
            } else {
                call
            }
        }
        // A bounded string read as the `bytes` an algorithm takes: its UTF-8.
        ExprKind::BytesView { source, .. } => {
            format!(
                "Array({}.utf8)",
                wrap_postfix(source, emit(source, InferredType::Unknown)?)
            )
        }
        // SCE_FORGE.md §3.4.1: an index read through `Checked.at`, which answers
        // `fallback` and records an out-of-range failure rather than trap.
        ExprKind::Checked {
            op: CheckedOp::Index,
            left,
            right: Some(index),
        } => {
            let Some(zero) = zero_of(expr.ty) else {
                return Err(unsupported(
                    format!("a checked index read of {}", expr.ty.describe()),
                    expr,
                ));
            };
            format!(
                "Checked.at(sceFailure, {}, {}, fallback: {zero})",
                emit(left, InferredType::Unknown)?,
                emit(index, InferredType::Unknown)?,
            )
        }
        ExprKind::Checked {
            op: CheckedOp::Narrow,
            left,
            ..
        } => {
            let InferredType::Int { signed, bits } = expr.ty else {
                return Err(unsupported(
                    format!("a checked narrowing to {}", expr.ty.describe()),
                    expr,
                ));
            };
            format!(
                "Checked.narrow(sceFailure, {}, to: {}.self)",
                emit(left, left.ty)?,
                int_type(signed, bits)
            )
        }
        // SCE_FORGE.md §3.4.1: the helper of the operation, generic in the width it
        // is called at, so both operands are spelled at the operation's own type.
        ExprKind::Checked { op, left, right } => {
            let mut first = emit(left, expr.ty)?;
            // The helper is generic in the width, which its arguments give it, and
            // literals alone give it `Int`: one of them is named at the type.
            let literal = |node: &TypedExpr| matches!(node.kind, ExprKind::NumberLit(_));
            if literal(left) && right.as_deref().is_none_or(literal) {
                if let InferredType::Int { signed, bits } = expr.ty {
                    first = format!("{}({first})", int_type(signed, bits));
                }
            }
            let mut operands = vec!["sceFailure".to_string(), first];
            if let Some(right) = right {
                operands.push(emit(right, expr.ty)?);
            }
            format!("Checked.{}({})", op.helper(), operands.join(", "))
        }
    })
}

/// `text` as a part of a conditional, parenthesised when it is one itself: Swift
/// reads `a ? b : c ? d : e` to the right, and a condition that is a conditional
/// would swallow what follows it.
fn wrap_conditional_part(part: &TypedExpr, text: String) -> String {
    if matches!(&part.kind, ExprKind::Conditional { .. }) {
        format!("({text})")
    } else {
        text
    }
}

/// An index as a Swift subscript takes it: an `Int`. A literal is one already, and
/// every other integer is converted — an index the analysis admits is in range, so
/// the conversion does not trap.
fn subscript(index: &TypedExpr) -> Result<String, Refusal> {
    let text = emit(index, InferredType::Unknown)?;
    Ok(match index.ty {
        InferredType::Int { .. } => format!("Int({text})"),
        _ => text,
    })
}

fn binary(op: BinOp, left: &TypedExpr, right: &TypedExpr) -> Result<String, Refusal> {
    let equality = matches!(op, BinOp::StrictEq | BinOp::StrictNeq);
    let negated = |call: String| {
        if op == BinOp::StrictNeq {
            format!("!{call}")
        } else {
            call
        }
    };
    // A byte string is an `[UInt8]`, whose `==` compares the elements.
    //
    // A string is compared over its UTF-8 bytes. `==` on a `String` is canonical
    // equivalence (`"\u{e9}"` equals `"e\u{301}"`), and the bound a variable holds
    // counts bytes: two values that are one here would be two on every other backend.
    if equality && left.ty == InferredType::Str && right.ty == InferredType::Str {
        let l = wrap_postfix(left, emit(left, InferredType::Unknown)?);
        let r = wrap_postfix(right, emit(right, InferredType::Unknown)?);
        return Ok(negated(format!("{l}.utf8.elementsEqual({r}.utf8)")));
    }
    if op.is_comparison()
        && !equality
        && left.ty == InferredType::Str
        && right.ty == InferredType::Str
    {
        // The order of the UTF-8 bytes, which is the order of the code points and
        // the order every other backend compares a string in.
        let l = wrap_postfix(left, emit(left, InferredType::Unknown)?);
        let r = wrap_postfix(right, emit(right, InferredType::Unknown)?);
        let before = |a: &str, b: &str| format!("{a}.utf8.lexicographicallyPrecedes({b}.utf8)");
        return Ok(match op {
            BinOp::Lt => before(&l, &r),
            BinOp::Gt => before(&r, &l),
            BinOp::LtEq => format!("!{}", before(&r, &l)),
            _ => format!("!{}", before(&l, &r)),
        });
    }
    let operand_ty = binary_operand_type(op, left.ty, right.ty);
    // A shift's count is any integer in Swift, so it is not converted to the width
    // of what is shifted.
    let count_ty = if matches!(op, BinOp::Shl | BinOp::Shr | BinOp::UShr) {
        InferredType::Unknown
    } else {
        operand_ty
    };
    let l_raw = emit(left, operand_ty)?;
    let r_raw = emit(right, count_ty)?;
    // `>>>` is a logical shift: made in the unsigned type of the width, which is
    // where Swift's own `>>` is one.
    if op == BinOp::UShr {
        if let InferredType::Int { signed: true, bits } = operand_ty {
            let unsigned = unsigned_of(bits);
            let signed = int_type(true, bits);
            return Ok(format!(
                "{signed}(truncatingIfNeeded: {unsigned}(truncatingIfNeeded: {l_raw}) >> {r_raw})"
            ));
        }
    }
    let l = swift_operand(left, op, true, l_raw);
    let r = swift_operand(right, op, false, r_raw);
    Ok(format!("{l} {} {r}", binop(op)))
}

/// `raw`, of `from`, as a value of `to`.
fn coerce(raw: String, from: InferredType, to: InferredType, node: &TypedExpr) -> String {
    use InferredType::*;
    // A quantity is emitted as its numeric base (see `rust_coerce`): Swift has no
    // implicit conversion between an integer and a real.
    let (from, to) = (from.strip_quantity(), to.strip_quantity());
    if from == to || matches!(to, Unknown) || matches!(from, Unknown) {
        return raw;
    }
    match (from, to) {
        // An untyped decimal integer literal in a real context is written as a real,
        // so `9 / 5` is not an integer division: `9.0 / 5.0`.
        (UntypedInt, Float { .. }) => match &node.kind {
            ExprKind::NumberLit(text) if is_decimal_integer_literal(text) => format!("{raw}.0"),
            _ => raw,
        },
        // An untyped literal takes the type of the place it lands in.
        (UntypedInt, _) | (UntypedFloat, _) => raw,
        (Int { .. }, Float { bits: 32 }) => format!("Float({raw})"),
        (Int { .. }, Float { .. }) => format!("Double({raw})"),
        (Float { bits: 32 }, Float { bits: 64 }) => format!("Double({raw})"),
        (Float { bits: 64 }, Float { bits: 32 }) => format!("Float({raw})"),
        (
            Int {
                signed: from_signed,
                bits: from_bits,
            },
            Int {
                signed: to_signed,
                bits: to_bits,
            },
        ) if (from_signed, from_bits) != (to_signed, to_bits) => {
            let target = int_type(to_signed, to_bits);
            // A conversion that cannot lose the value is the plain initialiser, which
            // does not trap for one; the others are the wrap every backend makes.
            let widens = to_bits > from_bits && (to_signed || !from_signed);
            if widens {
                format!("{target}({raw})")
            } else {
                format!("{target}(truncatingIfNeeded: {raw})")
            }
        }
        _ => raw,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::types::TypeCtx;

    fn int(signed: bool, bits: u8) -> InferredType {
        InferredType::Int { signed, bits }
    }

    fn emit_in(expr: &str, ctx: &TypeCtx<'_>, expected: InferredType) -> Result<String, Refusal> {
        transpile_typed(expr, ExprTarget::Swift, ctx, &HashMap::new(), expected)
    }

    fn spell(expr: &str, ctx: &TypeCtx<'_>) -> String {
        emit_in(expr, ctx, InferredType::Unknown)
            .unwrap_or_else(|e| panic!("{expr} lowers for Swift: {e:?}"))
    }

    /// An integer literal takes the type of the place it lands in, so it is written
    /// as it is; a real context makes it a real, so `9 / 5` is not an integer
    /// division.
    #[test]
    fn a_literal_is_left_to_the_place_it_lands_in_and_a_real_context_makes_it_a_real() {
        let mut ctx = TypeCtx::new();
        ctx.insert_var("celsius", InferredType::Float { bits: 64 });
        ctx.insert_var("count", int(false, 32));
        assert_eq!(
            emit_in(
                "celsius * 9 / 5 + 32",
                &ctx,
                InferredType::Float { bits: 64 }
            )
            .unwrap(),
            "celsius * 9.0 / 5.0 + 32.0"
        );
        assert_eq!(spell("count === 5", &ctx), "count == 5");
    }

    /// A conversion that cannot lose the value is the plain initialiser, and the rest
    /// are the two's-complement wrap every backend makes: Swift's own would trap.
    #[test]
    fn an_integer_conversion_wraps_where_it_can_narrow() {
        let mut ctx = TypeCtx::new();
        ctx.insert_var("small", int(false, 8));
        ctx.insert_var("wide", int(true, 64));
        assert_eq!(
            emit_in("small", &ctx, int(false, 32)).unwrap(),
            "UInt32(small)"
        );
        assert_eq!(
            emit_in("wide", &ctx, int(false, 16)).unwrap(),
            "UInt16(truncatingIfNeeded: wide)"
        );
        assert_eq!(
            emit_in("small", &ctx, InferredType::Float { bits: 32 }).unwrap(),
            "Float(small)"
        );
    }

    /// `==` on a `String` is canonical equivalence and `<` the collation order, where
    /// every other backend compares UTF-8 bytes, so a string is compared over them.
    #[test]
    fn a_string_is_compared_over_its_utf8_bytes() {
        let mut ctx = TypeCtx::new();
        ctx.insert_var("name", InferredType::Str);
        ctx.insert_var("other", InferredType::Str);
        assert_eq!(
            spell("name === 'ack'", &ctx),
            "name.utf8.elementsEqual(\"ack\".utf8)"
        );
        assert_eq!(
            spell("name !== other", &ctx),
            "!name.utf8.elementsEqual(other.utf8)"
        );
        assert_eq!(
            spell("name < other", &ctx),
            "name.utf8.lexicographicallyPrecedes(other.utf8)"
        );
        assert_eq!(
            spell("name >= other", &ctx),
            "!name.utf8.lexicographicallyPrecedes(other.utf8)"
        );
    }

    /// A bitwise or shift operator sits at a level of Swift's own, and a comparison
    /// does not associate: both are parenthesised rather than left to precedence.
    #[test]
    fn a_bitwise_operand_and_a_comparison_in_a_comparison_are_parenthesised() {
        let mut ctx = TypeCtx::new();
        ctx.insert_var("raw", int(false, 8));
        ctx.insert_var("a", int(false, 8));
        ctx.insert_var("b", int(false, 8));
        ctx.insert_var("c", InferredType::Bool);
        assert_eq!(spell("(raw >> 4) & 0x0F", &ctx), "(raw >> 4) & 0x0F");
        assert_eq!(spell("raw + 1 | 2", &ctx), "(raw + 1) | 2");
        assert_eq!(spell("(a < b) === c", &ctx), "(a < b) == c");
    }

    /// `>>>` shifts the bits of the unsigned type of the width, which is what Swift's
    /// own `>>` is on one.
    #[test]
    fn a_logical_shift_is_made_in_the_unsigned_type_of_the_width() {
        let mut ctx = TypeCtx::new();
        ctx.insert_var("word", int(true, 32));
        ctx.insert_var("bits", int(false, 32));
        assert_eq!(
            spell("word >>> 3", &ctx),
            "Int32(truncatingIfNeeded: UInt32(truncatingIfNeeded: word) >> 3)"
        );
        assert_eq!(spell("bits >>> 3", &ctx), "bits >> 3");
    }

    /// An integer operation of a body that receives failures is a `Checked` helper
    /// that records the failure instead of trapping, at the width of the operation.
    #[test]
    fn a_checked_operation_records_its_failure_in_the_body() {
        let mut ctx = TypeCtx::new();
        ctx.insert_var("count", int(false, 32));
        ctx.insert_var("step", int(false, 32));
        ctx.receives_failures = true;
        assert_eq!(
            emit_in("count + step", &ctx, int(false, 32)).unwrap(),
            "Checked.add(sceFailure, count, step)"
        );
        assert_eq!(
            emit_in("count - 1", &ctx, int(false, 32)).unwrap(),
            "Checked.sub(sceFailure, count, 1)"
        );
    }

    /// A conditional is Swift's own; one that is an operand is parenthesised.
    #[test]
    fn a_conditional_is_swifts_own() {
        let mut ctx = TypeCtx::new();
        ctx.insert_var("flag", InferredType::Bool);
        ctx.insert_var("a", int(true, 32));
        assert_eq!(spell("flag ? a : 0", &ctx), "flag ? a : 0");
        assert_eq!(spell("(flag ? a : 0) + 1", &ctx), "(flag ? a : 0) + 1");
    }

    /// A negation of a negation is not the `--` operator.
    #[test]
    fn a_prefix_beside_a_prefix_is_parenthesised() {
        let mut ctx = TypeCtx::new();
        ctx.insert_var("n", int(true, 32));
        ctx.insert_var("flag", InferredType::Bool);
        assert_eq!(spell("-(-n)", &ctx), "-(-n)");
        assert_eq!(spell("!(!flag)", &ctx), "!(!flag)");
    }
}
