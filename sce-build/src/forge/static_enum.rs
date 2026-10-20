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
// four things — an `<alias>.<variant>`, a variable declared `enum:<alias>`, a
// conditional whose two branches are values of one enum, or the call of an
// imported algorithm that returns one — and what it may be used for is
// deliberately small: stored in a variable of its own enum, logged, compared
// with `==` or `!=` to a value of that same enum, and handed to an imported
// algorithm whose parameter takes that same enum. No ordering, no arithmetic,
// no argument to any other call.

use crate::forge::error::ExprError;
use crate::forge::expr::{dotted_path, parse_to_ast, BinOp, ExprKind, Refusal, TypedExpr};
use crate::forge::type_ctx::INDEXED_ELEMENT;
use crate::forge::types::TypeCtx;

/// The enum each slot of a call of an imported algorithm holds, by the alias the
/// statechart imports it under ([`crate::forge::type_ctx::StaticScope::call_enums`]):
/// what an argument of the call is held to, and what the call is a value of.
#[derive(Debug, Clone)]
pub(crate) struct CallEnums {
    /// The alias the statechart imports the algorithm under, which a refusal names.
    pub callee: String,
    /// One per parameter, in order: the enum it takes, or `None` for a parameter
    /// that is not an enum.
    pub params: Vec<Option<String>>,
    /// The enum it returns, if it returns one.
    pub ret: Option<String>,
}

/// What [`value_enum`] asks of the document it reads.
pub(crate) struct EnumReading<'r> {
    /// Which enum a variable or a field of the document is declared as, for the
    /// identifiers and paths that name one.
    pub var_alias: &'r dyn Fn(&str) -> Option<String>,
    /// The enums of the slots of the call a name is ([`CallEnums`]), for a name
    /// that is the call of an imported algorithm.
    pub calls: &'r dyn Fn(&str) -> Option<CallEnums>,
}

/// The enum alias `expr` is a value of, or `None` when it is not an enum
/// value — refused when it uses one for what an enum value is not for.
pub(crate) fn value_enum(
    expr: &str,
    ctx: &TypeCtx<'_>,
    reading: &EnumReading<'_>,
) -> Result<Option<String>, Refusal> {
    let ast = parse_to_ast(expr)?;
    class_of(&ast, ctx, reading)
}

/// A refusal placed on `node`.
fn refuse(node: &TypedExpr, what: &str) -> Refusal {
    ExprError::UnsupportedConstruct {
        construct: what.to_string(),
        observed: None,
    }
    .at(node.span.clone())
}

/// What an enum value used where none may stand is refused as.
const NOT_AN_OPERAND: &str = "an enum value used as an operand or an argument (an enum value is \
     compared with === and !== to one of its own enum, stored in a variable of it, logged, or \
     handed to an imported algorithm whose parameter takes that enum)";

fn class_of(
    node: &TypedExpr,
    ctx: &TypeCtx<'_>,
    reading: &EnumReading<'_>,
) -> Result<Option<String>, Refusal> {
    match &node.kind {
        // `<alias>.<variant>`: the variant itself was judged against the
        // enum's own set by the expression pass that ran before this one.
        ExprKind::Member { object, property } => {
            if let ExprKind::Ident(alias) = &object.kind {
                if ctx.lookup_enum(alias).is_some() {
                    return Ok(Some(alias.clone()));
                }
            }
            // `<record>.<field>` or `_event.data.<field>`: the path of a field of
            // a record, or of the payload, that holds an enum — whatever the
            // number of names it is spelled in.
            if let Some(held) = dotted_path(node).and_then(|path| (reading.var_alias)(&path)) {
                return Ok(Some(held));
            }
            // `<list>[<index>].<field>`: the field of the element a list of records
            // is indexed at, held under `<list>[].<field>` whatever the index is.
            if let ExprKind::Index {
                object: list,
                index,
            } = &object.kind
            {
                if let ExprKind::Ident(name) = &list.kind {
                    if let Some(held) =
                        (reading.var_alias)(&format!("{name}{INDEXED_ELEMENT}{property}"))
                    {
                        if class_of(index, ctx, reading)?.is_some() {
                            return Err(refuse(index, "an enum value as an index"));
                        }
                        return Ok(Some(held));
                    }
                }
            }
            nothing_but(node, ctx, reading)
        }
        ExprKind::Ident(name) => Ok((reading.var_alias)(name)),
        ExprKind::Conditional {
            condition,
            consequent,
            alternate,
        } => {
            if class_of(condition, ctx, reading)?.is_some() {
                return Err(refuse(condition, "an enum value as a condition"));
            }
            let then = class_of(consequent, ctx, reading)?;
            let otherwise = class_of(alternate, ctx, reading)?;
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
            let l = class_of(left, ctx, reading)?;
            let r = class_of(right, ctx, reading)?;
            match (l, r) {
                (None, None) => Ok(None),
                (Some(a), Some(b)) if a == b => Ok(None),
                _ => Err(refuse(
                    node,
                    "a comparison of an enum value with anything but a value of the same enum",
                )),
            }
        }
        // The call of an imported algorithm: each argument is held to the enum its
        // parameter takes, which the statechart and the algorithm import from one
        // document (docs/SCE_ACCEPTED_SUBSET.md §2.15), and the call is a value of
        // the enum it returns.
        ExprKind::Call { callee, args, .. } => {
            let slots = match &callee.kind {
                ExprKind::Ident(name) => (reading.calls)(name),
                _ => None,
            };
            let Some(slots) = slots else {
                return nothing_but(node, ctx, reading);
            };
            for (index, arg) in args.iter().enumerate() {
                let held = class_of(arg, ctx, reading)?;
                let wanted = slots.params.get(index).cloned().flatten();
                match (wanted, held) {
                    (None, None) => {}
                    (Some(wanted), Some(held)) if wanted == held => {}
                    (Some(wanted), Some(held)) => {
                        return Err(refuse(
                            arg,
                            &format!(
                                "a value of the enum `{held}` handed to `{}`, whose parameter \
                                 takes a value of the enum `{wanted}`",
                                slots.callee
                            ),
                        ))
                    }
                    (Some(wanted), None) => {
                        return Err(refuse(
                            arg,
                            &format!(
                                "a value that is not of the enum `{wanted}` handed to `{}`, \
                                 whose parameter takes one (write `{wanted}.<variant>`, or a \
                                 variable or a field of that enum)",
                                slots.callee
                            ),
                        ))
                    }
                    (None, Some(_)) => return Err(refuse(arg, NOT_AN_OPERAND)),
                }
            }
            Ok(slots.ret)
        }
        _ => nothing_but(node, ctx, reading),
    }
}

/// `node` is none of the shapes an enum value can take: no part of it may be
/// one either, since an operator or a call has no meaning for an enum value.
fn nothing_but(
    node: &TypedExpr,
    ctx: &TypeCtx<'_>,
    reading: &EnumReading<'_>,
) -> Result<Option<String>, Refusal> {
    for child in node.children() {
        if class_of(child, ctx, reading)?.is_some() {
            return Err(refuse(child, NOT_AN_OPERAND));
        }
    }
    Ok(None)
}
