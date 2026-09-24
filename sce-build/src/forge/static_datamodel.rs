// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The static data model's expressions, judged (docs/SCE_ACCEPTED_SUBSET.md
// §2.15).
//
// Under `datamodel="sce-static"` every expression is written in the forge
// expression language against a closed scope — the declared variables, the
// triggering event's typed payload, the imported enums, `In()` — built by
// [`crate::forge::type_ctx::static_statechart`]. This pass judges each one
// where it lands: a `<data>` initialiser and an `<assign>` against the
// variable's type, a condition as `bool`, a logged or sent value as whatever
// it is. A refusal is placed at the expression's own range.
//
// An expression attribute the model has no typed form for is refused rather
// than passed on: every backend's templates read one as script-engine text,
// so admitting it would evaluate part of a document in a language it never
// declared — the failure the `datamodel` attribute exists to prevent.

use crate::forge::error::{ForgeError, Located};
use crate::forge::event_schema_check::event_payload_paths;
use crate::forge::expr::{judge_into, Expected};
use crate::forge::expression_site::ExpressionSite;
use crate::forge::type_ctx::{static_statechart, StaticEnum};
use crate::forge::types::{InferredType, TypeCtx};
use crate::model::{Action, Datamodel, Invoke, SCXMLModel, Variable};
use crate::scxml_semantic::ScxmlSemanticError;

/// The attributes of executable content that carry an expression this model
/// does not type, with the element each belongs to. Refused where written.
const UNTYPED_ACTION_ATTRIBUTES: &[(&str, &str)] = &[
    ("send", "eventexpr"),
    ("send", "targetexpr"),
    ("send", "delayexpr"),
    ("send", "typeexpr"),
    ("send", "idlocation"),
    ("send", "namelist"),
    ("cancel", "sendidexpr"),
];

/// Judge every expression of a `sce-static` document; a document under any
/// other data model is not this pass's to judge.
///
/// `enums` are the document's imported enums
/// ([`StaticEnum::from_imports`]); empty for a document read with no
/// directory to resolve its imports against.
pub fn check(
    model: &SCXMLModel,
    enums: &[StaticEnum],
    diag_label: &str,
) -> Result<(), Located<ForgeError>> {
    if model.datamodel != Datamodel::SceStatic {
        return Ok(());
    }
    let variables: Vec<&Variable> = model
        .variables
        .iter()
        .chain(model.states.values().flat_map(|s| s.datamodel.iter()))
        .collect();
    let judge = Judge {
        variables: &variables,
        enums,
        diag_label,
    };
    let no_payload: Vec<(String, InferredType)> = Vec::new();
    let plain = judge.ctx(&no_payload);

    for var in &variables {
        if var.expr.trim().is_empty() {
            continue;
        }
        let slot = variable_type(var);
        judge.expr(
            &plain,
            &var.expr,
            var.expr_spelling.as_ref(),
            Expected::Slot(slot),
        )?;
    }
    judge.actions(&plain, &model.global_scripts, "")?;

    let mut states: Vec<_> = model.states.values().collect();
    states.sort_by_key(|s| s.document_order);
    for state in states {
        for block in state.on_entry_blocks.iter().chain(&state.on_exit_blocks) {
            judge.actions(&plain, block, &state.id)?;
        }
        judge.actions(&plain, &state.initial_transition_actions, &state.id)?;
        judge.actions(&plain, &state.initial_history_default_actions, &state.id)?;
        for transition in &state.transitions {
            // The payload is the triggering event's, so each transition
            // judges against its own.
            let payload = model
                .imported_event_schemas
                .get(&transition.event)
                .map(event_payload_paths)
                .unwrap_or_default();
            let ctx = judge.ctx(&payload);
            if !transition.cond.trim().is_empty()
                && !transition.is_cpp_condition
                && !transition.is_kt_condition
            {
                judge.expr(
                    &ctx,
                    &transition.cond,
                    transition.cond_spelling.as_ref(),
                    Expected::Slot(InferredType::Bool),
                )?;
            }
            judge.actions(&ctx, &transition.actions, &state.id)?;
        }
        for invoke in &state.invokes {
            judge.invoke(&plain, invoke, &state.id)?;
        }
        if let Some(done) = &state.donedata {
            for param in &done.params {
                if let Some(expr) = &param.expr {
                    judge.expr(
                        &plain,
                        expr,
                        param.expr_spelling.as_ref(),
                        Expected::Hint(InferredType::Unknown),
                    )?;
                }
            }
            if let crate::model::DoneDataContent::Expression(expr) = &done.content {
                return Err(judge.untyped(
                    format!("<content expr=\"{expr}\">"),
                    done.content_spelling.as_ref(),
                    &state.id,
                    expr,
                ));
            }
        }
    }
    Ok(())
}

/// A variable's declared type; `Unknown` for an enum, whose width the
/// inference layer declines to claim ([`InferredType::from_sce_type`]).
fn variable_type(var: &Variable) -> InferredType {
    var.value_type
        .as_ref()
        .and_then(crate::forge::model::AlgorithmValueType::scalar)
        .map_or(InferredType::Unknown, InferredType::from_sce_type)
}

struct Judge<'a> {
    variables: &'a [&'a Variable],
    enums: &'a [StaticEnum],
    diag_label: &'a str,
}

impl<'a> Judge<'a> {
    fn ctx<'c>(&self, payload: &'c [(String, InferredType)]) -> TypeCtx<'c>
    where
        'a: 'c,
    {
        static_statechart(self.variables.iter().copied(), payload, self.enums)
    }

    /// `expr` judged against `expected`, refused at its own range.
    fn expr(
        &self,
        ctx: &TypeCtx<'_>,
        expr: &str,
        spelling: Option<&crate::attribute_spelling::AttributeSpelling>,
        expected: Expected,
    ) -> Result<InferredType, Located<ForgeError>> {
        judge_into(expr, ctx, expected).map_err(|refusal| {
            Located::in_file(
                ExpressionSite::new(expr, spelling).place(refusal),
                self.diag_label,
            )
        })
    }

    /// The refusal of an expression attribute this model has no typed form
    /// for, placed on the attribute.
    fn untyped(
        &self,
        construct: String,
        spelling: Option<&crate::attribute_spelling::AttributeSpelling>,
        state: &str,
        value: &str,
    ) -> Located<ForgeError> {
        Located::new(
            ScxmlSemanticError::StaticDatamodelRule {
                construct,
                datamodel: Datamodel::SceStatic.as_str().to_string(),
                rule: "this data model types the expressions of <data>, a condition, \
                       <assign>, <log> and <param>; this attribute has no typed form"
                    .to_string(),
                state: state.to_string(),
                observed: (!value.is_empty()).then(|| value.to_string()),
            }
            .into(),
            self.diag_label,
            spelling.map(|s| s.row()),
            spelling.map(|s| s.col()),
        )
    }

    fn actions(
        &self,
        ctx: &TypeCtx<'_>,
        actions: &[Action],
        state: &str,
    ) -> Result<(), Located<ForgeError>> {
        for action in actions {
            self.action(ctx, action, state)?;
        }
        Ok(())
    }

    fn action(
        &self,
        ctx: &TypeCtx<'_>,
        action: &Action,
        state: &str,
    ) -> Result<(), Located<ForgeError>> {
        let kind = action.action_type.as_str();
        for (element, attr) in UNTYPED_ACTION_ATTRIBUTES {
            if kind != *element {
                continue;
            }
            let value = action_attribute(action, attr);
            if !value.is_empty() {
                return Err(self.untyped(
                    format!("{attr}=\"{value}\""),
                    action.spellings.get(attr),
                    state,
                    value,
                ));
            }
        }
        match kind {
            "assign" => {
                // The location is a declared variable; its type is the slot.
                let slot = self.expr(
                    ctx,
                    &action.location,
                    action.spellings.get("location"),
                    Expected::Hint(InferredType::Unknown),
                )?;
                self.expr(
                    ctx,
                    &action.expr,
                    action.spellings.get("expr"),
                    Expected::Slot(slot),
                )?;
            }
            "if" => {
                self.expr(
                    ctx,
                    &action.cond,
                    action.spellings.get("cond"),
                    Expected::Slot(InferredType::Bool),
                )?;
                self.actions(ctx, &action.then_actions, state)?;
                for branch in &action.elseif_branches {
                    self.expr(
                        ctx,
                        &branch.cond,
                        branch.cond_spelling.as_ref(),
                        Expected::Slot(InferredType::Bool),
                    )?;
                    self.actions(ctx, &branch.actions, state)?;
                }
                self.actions(ctx, &action.else_actions, state)?;
            }
            "log" if !action.expr.trim().is_empty() => {
                self.expr(
                    ctx,
                    &action.expr,
                    action.spellings.get("expr"),
                    Expected::Hint(InferredType::Unknown),
                )?;
            }
            "send" => {
                for param in &action.params {
                    if !param.expr.trim().is_empty() {
                        self.expr(
                            ctx,
                            &param.expr,
                            param.expr_spelling.as_ref(),
                            Expected::Hint(InferredType::Unknown),
                        )?;
                    }
                }
                if !action.contentexpr.is_empty() {
                    return Err(self.untyped(
                        format!("<content expr=\"{}\">", action.contentexpr),
                        action.contentexpr_spelling.as_ref(),
                        state,
                        &action.contentexpr,
                    ));
                }
            }
            "foreach" => {
                return Err(self.untyped(
                    format!("array=\"{}\"", action.array),
                    action.spellings.get("array"),
                    state,
                    &action.array,
                ));
            }
            _ => {}
        }
        Ok(())
    }

    fn invoke(
        &self,
        ctx: &TypeCtx<'_>,
        invoke: &Invoke,
        state: &str,
    ) -> Result<(), Located<ForgeError>> {
        // A hybrid invoke names its child by an expression evaluated at run
        // time — its `srcexpr` or its `<content expr>`.
        if let Invoke::Hybrid(info) = invoke {
            let (attr, value, spelling) = if info.srcexpr.is_empty() {
                (
                    "contentexpr",
                    &info.contentexpr,
                    info.contentexpr_spelling.as_ref(),
                )
            } else {
                ("srcexpr", &info.srcexpr, info.srcexpr_spelling.as_ref())
            };
            return Err(self.untyped(format!("{attr}=\"{value}\""), spelling, state, value));
        }
        let base = invoke.base();
        if !base.idlocation.is_empty() {
            return Err(self.untyped(
                format!("idlocation=\"{}\"", base.idlocation),
                None,
                state,
                &base.idlocation,
            ));
        }
        for param in &base.params {
            if !param.expr.trim().is_empty() {
                self.expr(
                    ctx,
                    &param.expr,
                    param.expr_spelling.as_ref(),
                    Expected::Hint(InferredType::Unknown),
                )?;
            }
        }
        Ok(())
    }
}

/// The value of one of [`UNTYPED_ACTION_ATTRIBUTES`] on `action`.
fn action_attribute<'a>(action: &'a Action, attr: &str) -> &'a str {
    match attr {
        "eventexpr" => &action.eventexpr,
        "targetexpr" => &action.targetexpr,
        "delayexpr" => &action.delayexpr,
        "typeexpr" => &action.typeexpr,
        "idlocation" => &action.idlocation,
        "namelist" => &action.namelist,
        "sendidexpr" => &action.sendidexpr,
        _ => "",
    }
}
