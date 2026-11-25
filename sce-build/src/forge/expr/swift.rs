// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
//   (`SceForgeRuntime`), which throws its `AlgorithmError` and answers nothing, as
//   Rust's `?` does: a failure is not a value, and no value of any type stands in
//   for it (docs/adr/0099). A throwing call is the `try` over it
//   ([`SwiftExpr::tried`]), and a short-circuit whose right operand throws is the
//   runtime's own (`Checked.and`, `Checked.or`, `Checked.coalesce`), not the
//   standard library's `rethrows` operator.
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
// * The expression is built as a tree of the Swift expressions it is made of
//   ([`SwiftExpr`], `expr/swift_tree.rs`) and printed once, where it leaves this
//   layer: every parenthesis is the printer's, decided by the kinds of the nodes
//   around it and the standard library's precedence groups, in which a comparison
//   and a shift do not associate and the bitwise operators sit at other levels
//   than ECMAScript places them. No rule here groups anything or reads back what it
//   has written.

use super::swift_tree::{SwiftArg, SwiftBinOp, SwiftExpr, SwiftUnOp};
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

/// `call`, the text of a call that can throw that a writer outside this emitter
/// made, marked where it stands: the one spelling of a throwing call in Swift
/// (docs/adr/0099, section 3.2), printed as the text an expression leaves this layer
/// as, `(try call)`, so it stands as any operand where the writer splices it. The
/// mark is on the call itself, so a closure that holds one throws by its own body and
/// an expression that holds none has no `try` over nothing.
pub(crate) fn throwing(call: &str) -> String {
    SwiftExpr::atom(call).tried().to_string()
}

/// A closure of no argument whose body `body` can fail, its failure typed: Swift
/// infers a closure's thrown type as `any Error` even where the parameter it is
/// handed to names one (typed throws, SE-0413, leaves closure inference to a later
/// feature), so the signature is written out, with its result where it can be
/// named (`result`) and inferred from the one expression where it cannot.
fn failing_closure(body: SwiftExpr, result: Option<&str>) -> SwiftExpr {
    let result = result.map(|ty| format!(" -> {ty}")).unwrap_or_default();
    SwiftExpr::closure(format!("() throws(AlgorithmError){result}"), body)
}

/// How [`throwing`] opens a call it marks.
const THROWING_MARK: &str = "(try ";

/// Whether `text`, Swift a writer outside this emitter made of its own spellings,
/// holds a call that can throw: every such spelling marks one through [`throwing`],
/// so the mark is in the text exactly where one is. A construct that wraps text it
/// did not type (a closure over a statement's own expression) asks this to know
/// whether it throws in turn. A string literal that happens to hold the mark makes the
/// answer `true` for text that cannot throw, which costs a `try` over nothing (a
/// warning) and never a call left unmarked.
pub(crate) fn holds_throwing(text: &str) -> bool {
    text.contains(THROWING_MARK)
}

/// Whether evaluating `expr` can fail (SCE_FORGE.md §3.4.1): it is, or holds, a
/// checked operation or a call of a `may-fail` algorithm. Every other node is a
/// read, a literal or an operator that cannot fail, which the lowering admitted
/// only where the analysis proved it.
fn throws(expr: &TypedExpr) -> bool {
    match &expr.kind {
        ExprKind::Checked {
            discharge: Discharge::AtRun,
            ..
        } => true,
        ExprKind::Call { fails: true, .. } => true,
        _ => expr.children().into_iter().any(throws),
    }
}

fn binop(op: BinOp) -> SwiftBinOp {
    match op {
        BinOp::Add => SwiftBinOp::Add,
        BinOp::Sub => SwiftBinOp::Sub,
        BinOp::Mul => SwiftBinOp::Mul,
        BinOp::Div => SwiftBinOp::Div,
        BinOp::Mod => SwiftBinOp::Rem,
        BinOp::StrictEq => SwiftBinOp::Eq,
        BinOp::StrictNeq => SwiftBinOp::Ne,
        BinOp::Lt => SwiftBinOp::Lt,
        BinOp::Gt => SwiftBinOp::Gt,
        BinOp::LtEq => SwiftBinOp::Le,
        BinOp::GtEq => SwiftBinOp::Ge,
        BinOp::And => SwiftBinOp::And,
        BinOp::Or => SwiftBinOp::Or,
        BinOp::BitAnd => SwiftBinOp::BitAnd,
        BinOp::BitOr => SwiftBinOp::BitOr,
        BinOp::BitXor => SwiftBinOp::BitXor,
        BinOp::Shl => SwiftBinOp::Shl,
        BinOp::Shr | BinOp::UShr => SwiftBinOp::Shr,
        // Swift's own `??`, the one a read with a default is written as
        // ([`swift_optional_form`]).
        BinOp::Coalesce => SwiftBinOp::Coalesce,
    }
}

fn unop(op: UnaryOp) -> SwiftUnOp {
    match op {
        UnaryOp::Neg => SwiftUnOp::Neg,
        UnaryOp::Pos => SwiftUnOp::Pos,
        UnaryOp::Not => SwiftUnOp::Not,
        UnaryOp::BitNot => SwiftUnOp::BitNot,
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

/// The Swift expression `expr` is where a value of `expected` is wanted.
pub(super) fn emit(expr: &TypedExpr, expected: InferredType) -> Result<SwiftExpr, Refusal> {
    // A quantity is emitted as its numeric base (see `rust_coerce`).
    let expected = expected.strip_quantity();
    // Push-down: see emit_rust for rationale.
    if let ExprKind::Binary { op, left, right } = &expr.kind {
        if op.is_arith()
            && !is_string_concatenation(*op, left.ty, right.ty)
            && matches!(expected, InferredType::Float { .. })
        {
            return Ok(SwiftExpr::binary(
                binop(*op),
                emit(left, expected)?,
                emit(right, expected)?,
            ));
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
            return Ok(emit(operand, expected)?.prefixed(unop(*op)));
        }
    }
    if let ExprKind::Call { callee, args, .. } = &expr.kind {
        // `len(x)` is Swift's `Int`, which is none of the integer types a document
        // declares: in a sized-integer context it is converted to that one, as
        // Go's and Rust's are.
        if is_len_builtin(callee, args) {
            if let InferredType::Int { signed, bits } = expected {
                return Ok(truncating(int_type(signed, bits), length_of(&args[0])?));
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
    let raw = match typed_integer_literal(ExprTarget::Swift, expr) {
        Some(literal) => SwiftExpr::atom(literal),
        None => emit_node(expr)?,
    };
    Ok(coerce(raw, expr.ty, expected, expr))
}

/// `value` converted to the integer type `target` by the two's-complement wrap:
/// `T(truncatingIfNeeded: value)`.
fn truncating(target: &str, value: SwiftExpr) -> SwiftExpr {
    SwiftExpr::call(
        SwiftExpr::atom(target),
        vec![SwiftArg::labeled("truncatingIfNeeded", value)],
    )
}

/// `round(real)` or `floor(real)` as an integer of `target`.
fn real_to_int(op: RealToInt, real: SwiftExpr, target: &str) -> SwiftExpr {
    let helper = match op {
        RealToInt::Round => "round",
        RealToInt::Floor => "floor",
    };
    SwiftExpr::call(
        SwiftExpr::atom(format!("Real.{helper}")),
        vec![
            SwiftArg::bare(real),
            SwiftArg::labeled("to", SwiftExpr::atom(format!("{target}.self"))),
        ],
    )
}

/// The number of elements, or of UTF-8 bytes of a string, of `measured` as Swift's
/// `Int`.
fn length_of(measured: &TypedExpr) -> Result<SwiftExpr, Refusal> {
    let value = emit(measured, InferredType::Unknown)?;
    Ok(if measured.ty == InferredType::Str {
        value.member("utf8").member("count")
    } else {
        value.member("count")
    })
}

/// The bytes of `text`, a `String`, as the `[UInt8]` of its UTF-8 encoding.
fn utf8_array(text: SwiftExpr) -> SwiftExpr {
    SwiftExpr::path_call("Array", vec![text.member("utf8")])
}

/// A construct of an optional value (docs/adr/0055, section 2) as Swift spells it over
/// `T?`: a test is `== nil` and `!= nil`, a read of a value proven present is
/// `SceOptional.proven(x)` (which fails loudly where the proof was wrong, as it is
/// never expected to be), a read with a default is `??`, and equality is `==` over the
/// two optional values, a plain one lifted by Swift where it is a number, a `bool` or a
/// value of an enum. A text is compared by the bytes it holds (a `String`'s own `==` is
/// canonical equivalence, which no other backend means), so an equality of optional
/// texts is refused by name until it is spelled over `utf8`.
fn swift_optional_form(form: OptionalForm<'_>, whole: &TypedExpr) -> Result<SwiftExpr, Refusal> {
    Ok(match form {
        OptionalForm::Test { subject, present } => SwiftExpr::binary(
            if present {
                SwiftBinOp::Ne
            } else {
                SwiftBinOp::Eq
            },
            emit(subject, InferredType::Unknown)?,
            SwiftExpr::atom("nil"),
        ),
        OptionalForm::Proven { subject } => SwiftExpr::path_call(
            "SceOptional.proven",
            vec![emit(subject, InferredType::Unknown)?],
        ),
        OptionalForm::Otherwise { value, fallback } => {
            let read = emit(value, InferredType::Unknown)?;
            let default = emit(fallback, whole.ty)?;
            // A fallback that can fail is evaluated only where the value is absent,
            // through the runtime's coalesce, which keeps the failure's type.
            if throws(fallback) {
                let result = nameable_type(whole.ty);
                return Ok(SwiftExpr::path_call(
                    "Checked.coalesce",
                    vec![read, failing_closure(default, result.as_deref())],
                )
                .tried());
            }
            SwiftExpr::binary(SwiftBinOp::Coalesce, read, default)
        }
        OptionalForm::Equality { left, right, equal } => {
            let present_of = |ty: InferredType| match ty {
                InferredType::Optional(elem) => elem.present(),
                other => other,
            };
            if present_of(left.ty) == InferredType::Str || present_of(right.ty) == InferredType::Str
            {
                return Err(unsupported(
                    "an equality of optional texts (compared by the bytes they hold)",
                    whole,
                ));
            }
            SwiftExpr::binary(
                if equal {
                    SwiftBinOp::Eq
                } else {
                    SwiftBinOp::Ne
                },
                emit(left, InferredType::Unknown)?,
                emit(right, InferredType::Unknown)?,
            )
        }
        OptionalForm::Constant(answer) => SwiftExpr::atom(answer.to_string()),
    })
}

fn emit_node(expr: &TypedExpr) -> Result<SwiftExpr, Refusal> {
    if let Some(form) = optional_form(expr) {
        return swift_optional_form(form, expr);
    }
    Ok(match &expr.kind {
        // docs/adr/0063: the runtime's helper of the operation, generic in the
        // width. Typing refuses an operation of literals alone, so a declared
        // operand gives the helper its type.
        ExprKind::Total { op, rule, .. } => {
            let (_, _, operands) = total_parts(expr, "Swift", emit)?;
            SwiftExpr::path_call(
                format!(
                    "{}.{}",
                    crate::filters::capitalize_first(rule.helper()),
                    op.helper()
                ),
                operands,
            )
        }
        ExprKind::NumberLit(n) => SwiftExpr::atom(portable_number_literal(n)),
        ExprKind::StringLit { value } => SwiftExpr::atom(format!(
            "\"{}\"",
            crate::filters::escape_swift(value.clone())
        )),
        ExprKind::BytesLit { bytes } => utf8_array(SwiftExpr::atom(format!(
            "\"{}\"",
            bytes_as_quoted_ascii(bytes)
        ))),
        ExprKind::BoolLit(b) => SwiftExpr::atom(if *b { "true" } else { "false" }),
        ExprKind::NullLit => SwiftExpr::atom("nil"),
        // The spelling the function DECLARED the name with: one function spells the
        // parameter, the variable and every read ([`ExprTarget::local_spelling`]).
        ExprKind::Ident(s) => SwiftExpr::atom(ExprTarget::Swift.local_spelling(s)),
        // A fragment another pass wrote whole, which stands as one operand.
        ExprKind::Raw(s) => SwiftExpr::atom(s.clone()),
        ExprKind::Binary { op, left, right } if is_string_concatenation(*op, left.ty, right.ty) => {
            // The digits of an integer are `String(n)`; ECMA-262 13.15.3, and the
            // same text in every backend.
            let operand = |side: &TypedExpr| -> Result<SwiftExpr, Refusal> {
                let value = emit(side, side.ty)?;
                match concat_operand(side.ty) {
                    ConcatOperand::Text => Ok(value),
                    ConcatOperand::Decimal => Ok(SwiftExpr::path_call("String", vec![value])),
                    ConcatOperand::Refused(what) => {
                        Err(concat_refusal(what, value.to_string()).into())
                    }
                }
            };
            SwiftExpr::binary(SwiftBinOp::Add, operand(left)?, operand(right)?)
        }
        ExprKind::Binary { op, left, right } => binary(*op, left, right)?,
        ExprKind::Unary { op, operand } => emit(operand, expr.ty)?.prefixed(unop(*op)),
        ExprKind::Conditional {
            condition,
            consequent,
            alternate,
        } => {
            let condition = emit(condition, InferredType::Bool)?;
            let consequent = emit(consequent, expr.ty)?;
            SwiftExpr::conditional(condition, consequent, emit(alternate, expr.ty)?)
        }
        ExprKind::Member { object, property } => {
            emit(object, InferredType::Unknown)?.member(property.as_str())
        }
        ExprKind::Index { object, index } => {
            let object = emit(object, InferredType::Unknown)?;
            object.indexed(subscript(index)?)
        }
        ExprKind::Call {
            callee,
            args,
            params,
            fails,
            ..
        } => {
            if is_len_builtin(callee, args) {
                let length = length_of(&args[0])?;
                return Ok(match expr.ty {
                    InferredType::Int { signed, bits } => {
                        truncating(int_type(signed, bits), length)
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
            let call = SwiftExpr::call(
                emit(callee, InferredType::Unknown)?,
                a.into_iter().map(SwiftArg::bare).collect(),
            );
            // A call of a `may-fail` function throws its failure, which is the
            // caller's by `try` alone, as [`pass_failure_on`] spells it for Swift.
            if *fails {
                call.tried()
            } else {
                call
            }
        }
        // A bounded string read as the `bytes` an algorithm takes: its UTF-8.
        ExprKind::BytesView { source, .. } => utf8_array(emit(source, InferredType::Unknown)?),
        // SCE_FORGE.md §3.4.1: an index read through `Checked.at`, which throws an
        // out-of-range failure rather than trap, and answers nothing in its place, so
        // an element of any type is read alike (docs/adr/0099, section 5).
        // An operation proven at build time is printed apart from its operator by
        // the Rust emitter alone, and is never handed here, but for a shift.
        //
        // SCE_FORGE.md §3.4.1, "The width of a shift": a shift whose count is not
        // known below its width ([`settle_shifts`]).
        ExprKind::Checked {
            op,
            left,
            right: Some(count),
            discharge,
        } if op.shift().is_some() => settled_shift(expr, *op, left, count, *discharge)?,
        ExprKind::Checked {
            discharge: Discharge::AtBuild,
            ..
        } => {
            return Err(unsupported(
                "an integer operation proven at build time, which only Rust prints apart \
                 from its operator"
                    .to_string(),
                expr,
            ))
        }
        ExprKind::Checked {
            op: CheckedOp::Index,
            left,
            right: Some(index),
            ..
        } => SwiftExpr::path_call(
            "Checked.at",
            vec![
                emit(left, InferredType::Unknown)?,
                emit(index, InferredType::Unknown)?,
            ],
        )
        .tried(),
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
            SwiftExpr::call(
                SwiftExpr::atom("Checked.narrow"),
                vec![
                    SwiftArg::bare(emit(left, left.ty)?),
                    SwiftArg::labeled(
                        "to",
                        SwiftExpr::atom(format!("{}.self", int_type(signed, bits))),
                    ),
                ],
            )
            .tried()
        }
        // SCE_FORGE.md §3.4.1: the helper of the operation, generic in the width it
        // is called at, so both operands are spelled at the operation's own type.
        // The helper takes the width from its arguments, and an operation of
        // literals alone has its literals named at its type already: its context
        // gave them that type (`land_integers`), which `typed_integer_literal`
        // writes.
        ExprKind::Checked {
            op, left, right, ..
        } => {
            let mut operands = vec![emit(left, expr.ty)?];
            if let Some(right) = right {
                operands.push(emit(right, expr.ty)?);
            }
            SwiftExpr::path_call(format!("Checked.{}", op.helper()), operands).tried()
        }
    })
}

/// A shift the build settled ([`settle_shifts`]), in Swift.
///
/// Swift's own `<<` and `>>` are the total shift already: a "smart shift" of a
/// fixed-width integer by a count of any integer type at or past the width
/// shifts every bit out, leaving 0, or the sign for a signed `>>` (the Swift
/// standard library, `BinaryInteger.<<` and `>>`). The total shift is the
/// operator, as [`binary`] writes it (a `>>>` of a signed value in the unsigned
/// type of its width). A smart shift by a count below 0 shifts the other way; such
/// a count reaches a total shift only outside the bodies the contract covers, where
/// it is the caller's to rule out, as a divisor of 0 is.
///
/// The checked shift is the runtime's `Checked.shl`, `shr` or `ushr`, generic in
/// the width and the count, which throws `outOfRange` for a count below 0.
fn settled_shift(
    expr: &TypedExpr,
    op: CheckedOp,
    left: &TypedExpr,
    count: &TypedExpr,
    discharge: Discharge,
) -> Result<SwiftExpr, Refusal> {
    let Some(operator) = op.shift() else {
        unreachable!("only a shift is settled as one");
    };
    Ok(match discharge {
        Discharge::AtBuild | Discharge::Total => binary(operator, left, count)?,
        Discharge::AtRun => SwiftExpr::path_call(
            format!("Checked.{}", op.helper()),
            vec![emit(left, expr.ty)?, emit(count, InferredType::Unknown)?],
        )
        .tried(),
    })
}

/// An index as a Swift subscript takes it: an `Int`. A literal is one already, and
/// every other integer is converted — an index the analysis admits is in range, so
/// the conversion does not trap.
fn subscript(index: &TypedExpr) -> Result<SwiftExpr, Refusal> {
    let value = emit(index, InferredType::Unknown)?;
    Ok(match index.ty {
        InferredType::Int { .. } => SwiftExpr::path_call("Int", vec![value]),
        _ => value,
    })
}

fn binary(op: BinOp, left: &TypedExpr, right: &TypedExpr) -> Result<SwiftExpr, Refusal> {
    let equality = matches!(op, BinOp::StrictEq | BinOp::StrictNeq);
    // A byte string is an `[UInt8]`, whose `==` compares the elements.
    //
    // A string is compared over its UTF-8 bytes. `==` on a `String` is canonical
    // equivalence (`"\u{e9}"` equals `"e\u{301}"`), and the bound a variable holds
    // counts bytes: two values that are one here would be two on every other backend.
    if equality && left.ty == InferredType::Str && right.ty == InferredType::Str {
        let l = emit(left, InferredType::Unknown)?.member("utf8");
        let r = emit(right, InferredType::Unknown)?.member("utf8");
        let equal = l.method("elementsEqual", vec![r]);
        return Ok(if op == BinOp::StrictNeq {
            equal.prefixed(SwiftUnOp::Not)
        } else {
            equal
        });
    }
    if op.is_comparison()
        && !equality
        && left.ty == InferredType::Str
        && right.ty == InferredType::Str
    {
        // The order of the UTF-8 bytes, which is the order of the code points and
        // the order every other backend compares a string in.
        let l = emit(left, InferredType::Unknown)?.member("utf8");
        let r = emit(right, InferredType::Unknown)?.member("utf8");
        let before = |a: SwiftExpr, b: SwiftExpr| a.method("lexicographicallyPrecedes", vec![b]);
        return Ok(match op {
            BinOp::Lt => before(l, r),
            BinOp::Gt => before(r, l),
            BinOp::LtEq => before(r, l).prefixed(SwiftUnOp::Not),
            _ => before(l, r).prefixed(SwiftUnOp::Not),
        });
    }
    let operand_ty = binary_operand_type(op, left.ty, right.ty);
    // A shift's count is any integer in Swift, so it is not converted to the width
    // of what is shifted.
    let l = emit(left, operand_ty)?;
    let r = emit(right, right_operand_type(op, operand_ty))?;
    // `>>>` is a logical shift: made in the unsigned type of the width, which is
    // where Swift's own `>>` is one.
    if op == BinOp::UShr {
        if let InferredType::Int { signed: true, bits } = operand_ty {
            let shifted = SwiftExpr::binary(SwiftBinOp::Shr, truncating(unsigned_of(bits), l), r);
            return Ok(truncating(int_type(true, bits), shifted));
        }
    }
    // A right operand that can fail is evaluated only where the left one decides
    // nothing, through the runtime's short-circuit, which keeps the failure's type
    // where the standard library's `rethrows` operator need not.
    if matches!(op, BinOp::And | BinOp::Or) && throws(right) {
        let helper = if op == BinOp::And { "and" } else { "or" };
        return Ok(SwiftExpr::path_call(
            format!("Checked.{helper}"),
            vec![l, failing_closure(r, Some("Bool"))],
        )
        .tried());
    }
    Ok(SwiftExpr::binary(binop(op), l, r))
}

/// `raw`, of `from`, as a value of `to`.
fn coerce(raw: SwiftExpr, from: InferredType, to: InferredType, node: &TypedExpr) -> SwiftExpr {
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
            ExprKind::NumberLit(text) if is_decimal_integer_literal(text) => {
                SwiftExpr::atom(format!("{}.0", portable_number_literal(text)))
            }
            _ => raw,
        },
        // An untyped literal takes the type of the place it lands in.
        (UntypedInt, _) | (UntypedFloat, _) => raw,
        (Int { .. }, Float { bits: 32 }) => SwiftExpr::path_call("Float", vec![raw]),
        (Int { .. }, Float { .. }) => SwiftExpr::path_call("Double", vec![raw]),
        (Float { bits: 32 }, Float { bits: 64 }) => SwiftExpr::path_call("Double", vec![raw]),
        (Float { bits: 64 }, Float { bits: 32 }) => SwiftExpr::path_call("Float", vec![raw]),
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
                SwiftExpr::path_call(target, vec![raw])
            } else {
                truncating(target, raw)
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

    /// A bitwise or shift operator sits in the precedence group the standard
    /// library declares for it (a shift above a multiplication, `&` a
    /// multiplication, `|` an addition), and a comparison does not associate, so
    /// one under another is parenthesised.
    #[test]
    fn a_bitwise_operand_binds_by_its_group_and_a_comparison_in_a_comparison_is_grouped() {
        let mut ctx = TypeCtx::new();
        ctx.insert_var("raw", int(false, 8));
        ctx.insert_var("a", int(false, 8));
        ctx.insert_var("b", int(false, 8));
        ctx.insert_var("c", InferredType::Bool);
        assert_eq!(spell("(raw >> 4) & 0x0F", &ctx), "raw >> 4 & 0x0F");
        assert_eq!(spell("raw + 1 | 2", &ctx), "raw + 1 | 2");
        assert_eq!(spell("(raw | 1) * 2", &ctx), "(raw | 1) * 2");
        assert_eq!(spell("(a < b) === c", &ctx), "(a < b) == c");
    }

    /// `>>>` shifts the bits of the unsigned type of the width, which is what Swift's
    /// own `>>` is on one. A `>>>` of a signed value is settled as a shift by a count
    /// not known below the width is ([`settle_shifts`]), and written as that shift.
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

    /// SCE_FORGE.md §3.4.1, "The width of a shift": Swift's own shift is the total
    /// one, 0 or the sign for a count at or past the width, so a count at least 0
    /// is the operator whatever it is known to be; a count that may be below 0,
    /// where failures are received, is the runtime's `Checked` shift, which throws
    /// `outOfRange` there.
    #[test]
    fn a_count_not_known_below_the_width_is_the_operator_or_the_checked_shift() {
        let mut ctx = TypeCtx::new();
        ctx.insert_var("word", int(true, 32));
        ctx.insert_var("n", int(false, 8));
        ctx.insert_var("k", int(true, 32));
        assert_eq!(spell("word << n", &ctx), "word << n");
        assert_eq!(spell("word >> 40", &ctx), "word >> 40");
        ctx.receives_failures = true;
        assert_eq!(spell("word >> n", &ctx), "word >> n");
        assert_eq!(spell("word >>> k", &ctx), "(try Checked.ushr(word, k))");
    }

    /// An integer operation of a body that receives failures is a `Checked` helper
    /// that throws its failure instead of trapping, at the width of the operation,
    /// marked `try` where it stands (docs/adr/0099): grouped where the expression
    /// leaves, bare as an argument.
    #[test]
    fn a_checked_operation_throws_its_failure_where_it_stands() {
        let mut ctx = TypeCtx::new();
        ctx.insert_var("count", int(false, 32));
        ctx.insert_var("step", int(false, 32));
        ctx.receives_failures = true;
        assert_eq!(
            emit_in("count + step", &ctx, int(false, 32)).unwrap(),
            "(try Checked.add(count, step))"
        );
        assert_eq!(
            emit_in("count - 1", &ctx, int(false, 32)).unwrap(),
            "(try Checked.sub(count, 1))"
        );
        assert_eq!(
            emit_in("(count + step) * 2", &ctx, int(false, 32)).unwrap(),
            "(try Checked.mul(try Checked.add(count, step), 2))"
        );
    }

    /// The right operand of `&&` and `||` that can fail is evaluated through the
    /// runtime's short-circuit; one that cannot keeps Swift's own operator, so no
    /// `try` covers nothing.
    #[test]
    fn a_short_circuit_whose_right_operand_can_fail_is_the_runtimes() {
        let mut ctx = TypeCtx::new();
        ctx.insert_var("count", int(false, 32));
        ctx.insert_var("flag", InferredType::Bool);
        ctx.receives_failures = true;
        assert_eq!(
            spell("flag && count + 1 > 3", &ctx),
            "(try Checked.and(flag, { () throws(AlgorithmError) -> Bool in (try \
             Checked.add(count, 1)) > 3 }))"
        );
        assert_eq!(
            spell("flag || count + 1 > 3", &ctx),
            "(try Checked.or(flag, { () throws(AlgorithmError) -> Bool in (try \
             Checked.add(count, 1)) > 3 }))"
        );
        assert_eq!(
            spell("count + 1 > 3 && flag", &ctx),
            "(try Checked.add(count, 1)) > 3 && flag"
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
