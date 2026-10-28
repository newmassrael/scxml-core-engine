// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! Which schema an optional record is, where an algorithm hands one to another
//! algorithm (docs/adr/0055, step 9).
//!
//! A record is a type of its schema: two schemas are two structs on every backend,
//! even when their fields agree (SCE_FORGE.md §4.12), and the expression layer holds
//! a record to the one it is declared as by the identity its type carries
//! ([`crate::forge::types::InferredType::Record`]). An optional record's type carries
//! none ([`crate::forge::types::OptionalElem::Record`]), so a slot of
//! `optional<record:X>` would take an optional record of any schema, and the
//! generated code of the backend would be the first to say so, in its own compiler.
//!
//! This pass says it where the document is read, once, before any backend renders the
//! body. A parameter that is an optional record takes `null`, a record or an optional
//! record of its own schema; a call that answers one lands in a place of that schema.
//! It refuses only what it can tell is another schema: an argument it cannot name (a
//! number, say) is the expression layer's refusal, not this pass's.

use crate::forge::error::{ExprError, ForgeError};
use crate::forge::expr::{parse_to_ast, ExprKind, Refusal, TypedExpr};
use crate::forge::expression_site::ExpressionSite;
use crate::forge::generator::{AlgorithmSlots, AlgorithmTypes, CalleeSlot, ImportContext};
use crate::forge::model::{AlgorithmModel, AlgorithmStmt, OptionalElemType};
use crate::forge::types::RecordId;

/// Hold every call of an imported algorithm with an optional record slot, in the
/// body of `m`, to the schema of that slot.
pub(crate) fn check_algorithm(
    m: &AlgorithmModel,
    imports: &[ImportContext],
    options: &crate::ForgeCompileOptions,
) -> Result<(), ForgeError> {
    let takes_one = imports.iter().any(|import| {
        import.kind == "algorithm"
            && import.record_slots.as_ref().is_some_and(|slots| {
                slots
                    .params
                    .iter()
                    .chain(slots.ret.iter())
                    .any(|slot| matches!(slot, CalleeSlot::OptionalRecord { .. }))
            })
    });
    if !takes_one {
        return Ok(());
    }
    let types = AlgorithmTypes::collect(m, imports, options)?;
    // The schema this body's own return is declared as, when it is an optional
    // record: what a `<sce:return>` of a call answers into.
    let returned = m
        .signature
        .return_type
        .as_ref()
        .and_then(|ty| ty.optional_elem())
        .and_then(OptionalElemType::record_alias)
        .and_then(|alias| {
            imports
                .iter()
                .find(|import| import.alias == alias)
                .and_then(|import| import.record.as_ref())
        })
        .map(|record| record.id);
    let walk = Walk {
        types: &types,
        imports,
        returned,
    };
    walk.block(&m.body)
}

/// The walk of one algorithm's body ([`check_algorithm`]).
struct Walk<'w> {
    types: &'w AlgorithmTypes<'w>,
    imports: &'w [ImportContext],
    returned: Option<RecordId>,
}

impl Walk<'_> {
    fn block(&self, body: &[AlgorithmStmt]) -> Result<(), ForgeError> {
        body.iter().try_for_each(|stmt| self.statement(stmt))
    }

    fn statement(&self, stmt: &AlgorithmStmt) -> Result<(), ForgeError> {
        match stmt {
            AlgorithmStmt::Var {
                name,
                init: Some(init),
                init_spelling,
                ..
            } => self.expression(
                init,
                init_spelling.as_ref(),
                self.optional_record_named(name),
            ),
            AlgorithmStmt::Var { init: None, .. } => Ok(()),
            AlgorithmStmt::RecordVar { fields, .. } => fields.iter().try_for_each(|field| {
                self.expression(&field.expr, field.expr_spelling.as_ref(), None)
            }),
            AlgorithmStmt::RecordFromCall {
                init,
                init_spelling,
                ..
            } => self.expression(init, init_spelling.as_ref(), None),
            AlgorithmStmt::Assign {
                target,
                expr,
                expr_spelling,
                ..
            } => self.expression(
                expr,
                expr_spelling.as_ref(),
                self.optional_record_named(target.trim()),
            ),
            AlgorithmStmt::Append {
                expr,
                expr_spelling,
                ..
            } => self.expression(expr, expr_spelling.as_ref(), None),
            AlgorithmStmt::If {
                cond,
                cond_spelling,
                then_body,
                else_body,
            } => {
                self.expression(cond, cond_spelling.as_ref(), None)?;
                self.block(then_body)?;
                else_body.as_deref().map_or(Ok(()), |body| self.block(body))
            }
            AlgorithmStmt::While {
                cond,
                cond_spelling,
                body,
                ..
            } => {
                self.expression(cond, cond_spelling.as_ref(), None)?;
                self.block(body)
            }
            AlgorithmStmt::Foreach { body, .. } => self.block(body),
            AlgorithmStmt::Return {
                expr: Some(expr),
                expr_spelling,
            } => self.expression(expr, expr_spelling.as_ref(), self.returned),
            AlgorithmStmt::Return { expr: None, .. } => Ok(()),
            AlgorithmStmt::Call { target, args, .. } => {
                let slots = self.slots(target.trim());
                for (index, arg) in args.iter().enumerate() {
                    self.expression(&arg.expr, arg.spelling.as_ref(), None)?;
                    let wanted = slots
                        .and_then(|slots| slots.params.get(index))
                        .and_then(optional_record_id);
                    if let Some(wanted) = wanted {
                        let site = ExpressionSite::new(&arg.expr, arg.spelling.as_ref());
                        if let Ok(ast) = parse_to_ast(&arg.expr) {
                            self.argument(target.trim(), &ast, wanted)
                                .map_err(|refusal| site.place(refusal))?;
                        }
                    }
                }
                Ok(())
            }
            AlgorithmStmt::Require {
                cond,
                cond_spelling,
            } => self.expression(cond, cond_spelling.as_ref(), None),
        }
    }

    /// The expression `expr`, written at `spelling`, in a place that holds the
    /// optional record of the schema `place` when it is one. An expression this pass
    /// cannot read is left to the passes that report it.
    fn expression(
        &self,
        expr: &str,
        spelling: Option<&crate::attribute_spelling::AttributeSpelling>,
        place: Option<RecordId>,
    ) -> Result<(), ForgeError> {
        let Ok(ast) = parse_to_ast(expr) else {
            return Ok(());
        };
        let site = ExpressionSite::new(expr, spelling);
        self.node(&ast, place)
            .map_err(|refusal| site.place(refusal))
    }

    /// `node` and every call below it. `place` is the schema of the optional record
    /// `node` as a whole is written into, if it is the whole of such a statement.
    fn node(&self, node: &TypedExpr, place: Option<RecordId>) -> Result<(), Refusal> {
        if let ExprKind::Call { callee, args, .. } = &node.kind {
            if let ExprKind::Ident(name) = &callee.kind {
                if let Some(slots) = self.slots(name) {
                    for (arg, slot) in args.iter().zip(&slots.params) {
                        if let Some(wanted) = optional_record_id(slot) {
                            self.argument(name, arg, wanted)?;
                        }
                    }
                    if let (Some(answered), Some(place)) =
                        (slots.ret.as_ref().and_then(optional_record_id), place)
                    {
                        if answered != place {
                            return Err(refuse(
                                node,
                                &format!(
                                    "the call of `{name}`, which answers an optional {}, put \
                                     into a place that holds an optional {}",
                                    answered.describe(),
                                    place.describe()
                                ),
                            ));
                        }
                    }
                }
            }
        }
        node.children()
            .into_iter()
            .try_for_each(|child| self.node(child, None))
    }

    /// The argument `arg` of a call of `callee`, whose parameter takes an optional
    /// record of the schema `wanted`: `null` stands, and so does a record or an
    /// optional record, or a call that answers one, of that very schema.
    fn argument(&self, callee: &str, arg: &TypedExpr, wanted: RecordId) -> Result<(), Refusal> {
        match &arg.kind {
            ExprKind::Ident(name) => match self.record_named(name) {
                Some((given, optional)) if given != wanted => Err(refuse(
                    arg,
                    &format!(
                        "`{name}`, {} {}, handed to `{callee}`, whose parameter takes an \
                         optional {}",
                        if optional { "an optional" } else { "a" },
                        given.describe(),
                        wanted.describe()
                    ),
                )),
                _ => Ok(()),
            },
            // The nested call's own arguments are judged where it is walked
            // ([`Self::node`]).
            ExprKind::Call { callee: inner, .. } => {
                let answered = match &inner.kind {
                    ExprKind::Ident(name) => self.slots(name).and_then(|slots| match &slots.ret {
                        Some(CalleeSlot::OptionalRecord { id } | CalleeSlot::Record { id, .. }) => {
                            Some((name, *id))
                        }
                        _ => None,
                    }),
                    _ => None,
                };
                match answered {
                    Some((name, given)) if given != wanted => Err(refuse(
                        arg,
                        &format!(
                            "the call of `{name}`, which answers {}, handed to `{callee}`, \
                             whose parameter takes an optional {}",
                            given.describe(),
                            wanted.describe()
                        ),
                    )),
                    _ => Ok(()),
                }
            }
            ExprKind::Conditional {
                consequent,
                alternate,
                ..
            } => {
                self.argument(callee, consequent, wanted)?;
                self.argument(callee, alternate, wanted)
            }
            _ => Ok(()),
        }
    }

    /// The whole signature of the imported algorithm `name` names, its alias or the
    /// `<alias>.<name>` spelling a `<sce:call>` may write, when it has a record slot
    /// or an optional one.
    fn slots(&self, name: &str) -> Option<&AlgorithmSlots> {
        let alias = name.split_once('.').map_or(name, |(alias, _)| alias);
        self.imports
            .iter()
            .find(|import| import.alias == alias && import.kind == "algorithm")?
            .record_slots
            .as_ref()
    }

    /// The schema of the record or optional record this body names `name`, and
    /// whether it is the optional one.
    fn record_named(&self, name: &str) -> Option<(RecordId, bool)> {
        self.types
            .records
            .iter()
            .find(|(held, _, _)| *held == name)
            .map(|(_, _, record)| (record.id, false))
            .or_else(|| {
                self.types
                    .optional_records
                    .iter()
                    .find(|(held, _, _)| *held == name)
                    .map(|(_, _, record)| (record.id, true))
            })
    }

    /// The schema of the optional record this body names `name`, if it is one.
    fn optional_record_named(&self, name: &str) -> Option<RecordId> {
        self.record_named(name)
            .filter(|(_, optional)| *optional)
            .map(|(id, _)| id)
    }
}

/// The schema of the optional record `slot` is, if it is one.
fn optional_record_id(slot: &CalleeSlot) -> Option<RecordId> {
    match slot {
        CalleeSlot::OptionalRecord { id } => Some(*id),
        CalleeSlot::Scalar(_) | CalleeSlot::Record { .. } | CalleeSlot::Optional(_) => None,
    }
}

/// A refusal placed on `node`.
fn refuse(node: &TypedExpr, what: &str) -> Refusal {
    ExprError::UnsupportedConstruct {
        construct: what.to_string(),
        observed: None,
    }
    .at(node.span.clone())
}
