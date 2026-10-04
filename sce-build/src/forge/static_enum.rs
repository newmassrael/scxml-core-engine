// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// What an enum value may be used for in a `sce-static` document's expressions
// (docs/SCE_ACCEPTED_SUBSET.md §2.15).
//
// The expression typer declines to claim a type for an enum value
// ([`crate::forge::types::InferredType::from_sce_type`]): the integer an enum
// is carried as belongs to the enum document, which a statechart does not read
// the width of. That is the right answer for the layers that do arithmetic, and
// it leaves nothing to stop `mode == 3` or `mode + 1` or `mode = 7` from being
// accepted and written out, to fail in the generated code's own compiler where
// the author never sees the document.
//
// So this pass says it where the document is read. An enum value is one of
// three things — an `<alias>.<variant>`, a variable declared `enum:<alias>`,
// or a conditional whose two branches are values of one enum — and what it may
// be used for is deliberately small: stored in a variable of its own enum,
// logged, and compared with `==` or `!=` to a value of that same enum. No
// ordering, no arithmetic, no argument to a call.

use crate::forge::error::ExprError;
use crate::forge::expr::{dotted_path, parse_to_ast, BinOp, ExprKind, Refusal, TypedExpr};
use crate::forge::types::TypeCtx;

/// The enum alias `expr` is a value of, or `None` when it is not an enum
/// value — refused when it uses one for what an enum value is not for.
///
/// `var_alias` answers which enum a variable of the document is declared as,
/// for the identifiers that name one.
pub(crate) fn value_enum(
    expr: &str,
    ctx: &TypeCtx<'_>,
    var_alias: &dyn Fn(&str) -> Option<String>,
) -> Result<Option<String>, Refusal> {
    let ast = parse_to_ast(expr)?;
    class_of(&ast, ctx, var_alias)
}

/// A refusal placed on `node`.
fn refuse(node: &TypedExpr, what: &str) -> Refusal {
    ExprError::UnsupportedConstruct {
        construct: what.to_string(),
        observed: None,
    }
    .at(node.span.clone())
}

fn class_of(
    node: &TypedExpr,
    ctx: &TypeCtx<'_>,
    var_alias: &dyn Fn(&str) -> Option<String>,
) -> Result<Option<String>, Refusal> {
    match &node.kind {
        // `<alias>.<variant>`: the variant itself was judged against the
        // enum's own set by the expression pass that ran before this one.
        ExprKind::Member { object, .. } => {
            if let ExprKind::Ident(alias) = &object.kind {
                if ctx.lookup_enum(alias).is_some() {
                    return Ok(Some(alias.clone()));
                }
            }
            // `<record>.<field>` or `_event.data.<field>`: the path of a field of
            // a record, or of the payload, that holds an enum — whatever the
            // number of names it is spelled in.
            if let Some(held) = dotted_path(node).and_then(|path| var_alias(&path)) {
                return Ok(Some(held));
            }
            nothing_but(node, ctx, var_alias)
        }
        ExprKind::Ident(name) => Ok(var_alias(name)),
        ExprKind::Conditional {
            condition,
            consequent,
            alternate,
        } => {
            if class_of(condition, ctx, var_alias)?.is_some() {
                return Err(refuse(condition, "an enum value as a condition"));
            }
            let then = class_of(consequent, ctx, var_alias)?;
            let otherwise = class_of(alternate, ctx, var_alias)?;
            match (then, otherwise) {
                (None, None) => Ok(None),
                (Some(a), Some(b)) if a == b => Ok(Some(a)),
                _ => Err(refuse(
                    node,
                    "a conditional whose branches are not both values of one enum, or both \
                     not enum values",
                )),
            }
        }
        ExprKind::Binary {
            op: BinOp::StrictEq | BinOp::StrictNeq,
            left,
            right,
        } => {
            let l = class_of(left, ctx, var_alias)?;
            let r = class_of(right, ctx, var_alias)?;
            match (l, r) {
                (None, None) => Ok(None),
                (Some(a), Some(b)) if a == b => Ok(None),
                _ => Err(refuse(
                    node,
                    "a comparison of an enum value with anything but a value of the same enum",
                )),
            }
        }
        _ => nothing_but(node, ctx, var_alias),
    }
}

/// `node` is none of the shapes an enum value can take: no part of it may be
/// one either, since an operator or a call has no meaning for an enum value.
fn nothing_but(
    node: &TypedExpr,
    ctx: &TypeCtx<'_>,
    var_alias: &dyn Fn(&str) -> Option<String>,
) -> Result<Option<String>, Refusal> {
    for child in node.children() {
        if class_of(child, ctx, var_alias)?.is_some() {
            return Err(refuse(
                child,
                "an enum value used as an operand or an argument (an enum value is compared \
                 with === and !== to one of its own enum, stored in a variable of it, or logged)",
            ));
        }
    }
    Ok(None)
}
