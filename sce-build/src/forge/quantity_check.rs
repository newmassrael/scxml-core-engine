// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Physical-quantity unit-mismatch
// arithmetic validator.
//
// Walks every expression site in a parsed Forge document whose surface
// could combine two unit-tagged operands and emits
// `validation/cross-kind-type-mismatch` (typed via the new
// `ValidationError::QuantityUnitMismatch` variant — no new DiagnosticCode
// slot per the user-confirmed reuse decision) when the operator is
// arithmetic / bitwise / comparison and the two children carry
// `InferredType::Quantity` annotations on different `UnitTag` values.
//
// Surfaces covered (v1):
//   * Transform body expression (`<sce:expr>` per output field, plus
//     inline output-field `expr=` slots).
//   * Condition body expression.
//   * Validator predicate expression.
//   * Filter coefficient expressions are scalar-only and skip this walk.
//   * Procedure / Algorithm bodies are skipped for v1 — their
//     expression surfaces are richer (loops, assigns, helper calls) and
//     consumer demand for unit annotation there has not surfaced. The
//     walker contract is kind-agnostic so extending into Procedure /
//     Algorithm is a single match-arm addition once that consumer lands.
//
// The walker re-parses each expression text (parse_to_ast + infer_types
// via the type_ctx::<kind> builder) so it sees the same typed AST the
// codegen path would consume. This duplicates a small amount of work
// versus a single-pass design, but keeps the validator
// single-responsibility and removes the need to thread an
// error-collection sink through the inference recursion.

use crate::forge::error::{Located, ValidationError};
use crate::forge::expr::{infer_types, parse_to_ast, BinOp, ExprKind, TypedExpr};
use crate::forge::model::{
    ConditionModel, ForgeDocument, ForgeKind, ParsedForge, TransformModel, ValidatorModel,
};
use crate::forge::type_ctx;
use crate::forge::types::{InferredType, TypeCtx};

/// Entry point. Returns the first detected unit-mismatch site (callers
/// fail-fast — same convention as `cross_kind_check::check`).
pub fn check(
    parsed: &ParsedForge,
    label: &str,
) -> Result<(), Located<crate::forge::error::ForgeError>> {
    // ImportContext slices have type-context baked in; we don't need
    // their member surface for unit-mismatch detection (an imported
    // codec field referenced by `alias.field` carries its own
    // InferredType through type_ctx, which the walker sees just like
    // any local field). Pass an empty slice so the per-kind builders
    // populate only the local field types.
    let imports: &[crate::forge::generator::ImportContext] = &[];

    match &parsed.document {
        ForgeDocument::Transform(m) => {
            check_transform(m, imports, label)?;
        }
        ForgeDocument::Condition(m) => {
            check_condition(m, imports, label)?;
        }
        ForgeDocument::Validator(m) => {
            check_validator(m, imports, label)?;
        }
        // Other kinds either carry no expression slot (Lookup,
        // Interpolation, Timer, BufferPool) or their expression surface
        // is not implemented until a real consumer needs it (see
        // module-level scope comment).
        _ => {}
    }
    Ok(())
}

fn check_transform(
    m: &TransformModel,
    imports: &[crate::forge::generator::ImportContext],
    label: &str,
) -> Result<(), Located<crate::forge::error::ForgeError>> {
    let ctx = type_ctx::transform(m, imports);
    for out in &m.outputs {
        let Some(expr_src) = out.expr.as_ref() else {
            continue;
        };
        check_expression(expr_src, &ctx, ForgeKind::Transform, &m.name, label)?;
    }
    Ok(())
}

fn check_condition(
    m: &ConditionModel,
    imports: &[crate::forge::generator::ImportContext],
    label: &str,
) -> Result<(), Located<crate::forge::error::ForgeError>> {
    let ctx = type_ctx::condition(m, imports);
    // ConditionModel exposes the body expression on `m.expr`.
    if !m.expr.trim().is_empty() {
        check_expression(&m.expr, &ctx, ForgeKind::Condition, &m.name, label)?;
    }
    Ok(())
}

fn check_validator(
    m: &ValidatorModel,
    imports: &[crate::forge::generator::ImportContext],
    label: &str,
) -> Result<(), Located<crate::forge::error::ForgeError>> {
    let ctx = type_ctx::validator(m, imports);
    if let Some(expr) = m.rules.plausibility.as_ref() {
        if !expr.trim().is_empty() {
            check_expression(expr, &ctx, ForgeKind::Validator, &m.name, label)?;
        }
    }
    Ok(())
}

/// Parse + infer the expression, then walk the typed AST looking for
/// binary-op nodes whose two operands carry `InferredType::Quantity`
/// annotations on different unit tags.
fn check_expression(
    expr_src: &str,
    ctx: &TypeCtx<'_>,
    kind: ForgeKind,
    name: &str,
    label: &str,
) -> Result<(), Located<crate::forge::error::ForgeError>> {
    let trimmed = expr_src.trim();
    if trimmed.is_empty() {
        return Ok(());
    }
    // If parsing fails the expression-stage pipeline will surface its
    // own diagnostic later. The cross-kind validator stayed silent on
    // unparseable expressions on the same principle; we mirror that
    // here so the same expression doesn't double-emit.
    let Ok(mut ast) = parse_to_ast(trimmed) else {
        return Ok(());
    };
    infer_types(&mut ast, ctx);

    if let Some(mismatch) = find_unit_mismatch(&ast) {
        let err = crate::forge::error::ForgeError::Validation(Box::new(
            ValidationError::QuantityUnitMismatch {
                kind,
                name: name.to_owned(),
                op: mismatch.op.to_owned(),
                left_unit: mismatch.left_unit.to_owned(),
                right_unit: mismatch.right_unit.to_owned(),
                expr: trimmed.to_owned(),
            },
        ));
        return Err(Located::new(err, label, None, None));
    }
    Ok(())
}

/// First detected unit mismatch in a typed AST. Bottom-up traversal: the
/// deepest violation surfaces first so the diagnostic points at the
/// innermost culprit instead of the surrounding folded subtree.
fn find_unit_mismatch(ast: &TypedExpr) -> Option<UnitMismatch> {
    match &ast.kind {
        ExprKind::Binary { op, left, right } => {
            if let Some(child) = find_unit_mismatch(left) {
                return Some(child);
            }
            if let Some(child) = find_unit_mismatch(right) {
                return Some(child);
            }
            if let (
                InferredType::Quantity { unit: u_l, .. },
                InferredType::Quantity { unit: u_r, .. },
            ) = (left.ty, right.ty)
            {
                if u_l != u_r {
                    return Some(UnitMismatch {
                        op: binop_token(*op),
                        left_unit: u_l.as_str(),
                        right_unit: u_r.as_str(),
                    });
                }
            }
            None
        }
        ExprKind::Unary { operand, .. } => find_unit_mismatch(operand),
        ExprKind::Conditional {
            condition,
            consequent,
            alternate,
        } => find_unit_mismatch(condition)
            .or_else(|| find_unit_mismatch(consequent))
            .or_else(|| find_unit_mismatch(alternate)),
        ExprKind::Member { object, .. } => find_unit_mismatch(object),
        ExprKind::Index { object, index } => {
            find_unit_mismatch(object).or_else(|| find_unit_mismatch(index))
        }
        ExprKind::Call { callee, args } => {
            if let Some(child) = find_unit_mismatch(callee) {
                return Some(child);
            }
            for a in args {
                if let Some(child) = find_unit_mismatch(a) {
                    return Some(child);
                }
            }
            None
        }
        // RFC c7-wildcard W-project: algorithm-kind-only projection node;
        // recurse into its source (unreachable on the unit-mismatch path,
        // which runs over typed numeric expressions).
        ExprKind::BytesView { source, .. } => find_unit_mismatch(source),
        // Leaves carry no nested arithmetic.
        ExprKind::NumberLit(_)
        | ExprKind::StringLit { .. }
        | ExprKind::BytesLit { .. }
        | ExprKind::BoolLit(_)
        | ExprKind::NullLit
        | ExprKind::Ident(_)
        | ExprKind::Raw(_) => None,
    }
}

struct UnitMismatch {
    op: &'static str,
    left_unit: &'static str,
    right_unit: &'static str,
}

fn binop_token(op: BinOp) -> &'static str {
    match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Mod => "%",
        BinOp::StrictEq => "===",
        BinOp::StrictNeq => "!==",
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
        BinOp::Shr => ">>",
        BinOp::UShr => ">>>",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::quantity::{NumericBaseType, Rational, UnitTag};
    use crate::forge::types::TypeCtx;

    fn celsius_ctx() -> TypeCtx<'static> {
        let mut ctx = TypeCtx::new();
        let q_celsius = InferredType::Quantity {
            base: NumericBaseType::Int {
                signed: true,
                bits: 8,
            },
            scale: Rational::parse("0.5").unwrap(),
            offset: Rational::from_int(-40),
            unit: UnitTag::intern("celsius-qcheck-test"),
        };
        let q_kelvin = InferredType::Quantity {
            base: NumericBaseType::Int {
                signed: true,
                bits: 8,
            },
            scale: Rational::parse("0.5").unwrap(),
            offset: Rational::zero(),
            unit: UnitTag::intern("kelvin-qcheck-test"),
        };
        ctx.insert_var("celsius", q_celsius);
        ctx.insert_var("kelvin", q_kelvin);
        ctx
    }

    #[test]
    fn same_unit_arithmetic_passes() {
        let mut ast = parse_to_ast("celsius + celsius").unwrap();
        infer_types(&mut ast, &celsius_ctx());
        assert!(find_unit_mismatch(&ast).is_none());
    }

    #[test]
    fn mixed_unit_arithmetic_is_detected() {
        let mut ast = parse_to_ast("celsius + kelvin").unwrap();
        infer_types(&mut ast, &celsius_ctx());
        let m = find_unit_mismatch(&ast).expect("unit mismatch should surface");
        assert_eq!(m.op, "+");
        assert_eq!(m.left_unit, "celsius-qcheck-test");
        assert_eq!(m.right_unit, "kelvin-qcheck-test");
    }

    #[test]
    fn quantity_with_literal_passes() {
        let mut ast = parse_to_ast("celsius * 2").unwrap();
        infer_types(&mut ast, &celsius_ctx());
        assert!(find_unit_mismatch(&ast).is_none());
    }

    #[test]
    fn deepest_mismatch_surfaces_first() {
        // Outer add is celsius vs Unknown (because inner is Unknown after
        // inner unit mismatch); the inner subtraction is the real culprit.
        let mut ast = parse_to_ast("(celsius - kelvin) + celsius").unwrap();
        infer_types(&mut ast, &celsius_ctx());
        let m = find_unit_mismatch(&ast).expect("inner mismatch should surface");
        assert_eq!(m.op, "-");
    }

    #[test]
    fn comparison_between_different_units_is_detected() {
        let mut ast = parse_to_ast("celsius < kelvin").unwrap();
        infer_types(&mut ast, &celsius_ctx());
        let m = find_unit_mismatch(&ast).expect("comparison mismatch should surface");
        assert_eq!(m.op, "<");
    }

    #[test]
    fn unparseable_expression_returns_ok() {
        // Mirror cross_kind_check::check_expression — silent on
        // syntax errors so we don't double-emit.
        let result = check_expression(
            "((",
            &celsius_ctx(),
            ForgeKind::Transform,
            "test_fn",
            "test.scxml",
        );
        assert!(result.is_ok());
    }
}
