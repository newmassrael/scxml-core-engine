// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The static data model, lowered for one backend (docs/SCE_ACCEPTED_SUBSET.md
// §2.15).
//
// [`crate::forge::static_datamodel`] judges a `sce-static` document; this
// rewrites the one a backend renders. Every expression goes through the forge
// expression lowerer against the same scope the judge used
// ([`crate::forge::type_ctx::static_statechart`]), and lands in a slot the
// backend's templates already render as native code: a condition in the
// native-condition fields a `kt:` guard fills, an `<assign>` or `<log>` value
// in the text the engine-free arm pastes. So a `sce-static` machine needs no
// template of its own beyond its variables' declarations — which this returns.

use std::collections::{BTreeSet, HashMap};

use crate::filters;
use crate::forge::error::GenerateError;
use crate::forge::event_schema_check::event_payload_paths;
use crate::forge::expr::{transpile_into, transpile_typed, ExprTarget, Refusal};
use crate::forge::type_ctx::{static_statechart, StaticEnum};
use crate::forge::types::InferredType;
use crate::model::{Action, Datamodel, SCXMLModel, Variable};

/// One variable of a `sce-static` machine as the backend declares it: its
/// document id, the field name generated code spells, the backend type, and
/// the lowered initial value.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StaticField {
    pub id: String,
    pub name: String,
    pub ty: String,
    pub init: String,
}

/// What lowering a `sce-static` machine for Kotlin produced beyond the
/// rewritten model.
#[derive(Debug, Clone, Default)]
pub struct KotlinStaticLowering {
    /// The machine's fields, in declaration order.
    pub fields: Vec<StaticField>,
    /// Events whose typed payload a lowered condition reads — the payload
    /// channel must carry them (see
    /// [`crate::forge::generator::build_kotlin_event_payload`]).
    pub payload_events: BTreeSet<String>,
}

/// Rewrite `model` — a clone the Kotlin backend renders — so every
/// expression of a `sce-static` document is native Kotlin. A document under
/// any other data model is left as it is.
///
/// An enum-typed variable is refused: its Kotlin type is the enum
/// document's, which a statechart does not yet import into its generated
/// unit.
pub fn lower_kotlin(
    model: &mut SCXMLModel,
    enums: &[StaticEnum],
) -> Result<KotlinStaticLowering, GenerateError> {
    if model.datamodel != Datamodel::SceStatic {
        return Ok(KotlinStaticLowering::default());
    }
    let variables: Vec<Variable> = model
        .variables
        .iter()
        .chain(model.states.values().flat_map(|s| s.datamodel.iter()))
        .cloned()
        .collect();
    let schemas = model.imported_event_schemas.clone();
    let names: Vec<(String, String)> = variables
        .iter()
        .map(|v| (v.id.clone(), filters::to_camel_case(v.id.clone())))
        .collect();

    let refused = |what: &str, text: &str, refusal: Refusal| {
        GenerateError::unsupported(format!(
            "{what} `{text}` has no Kotlin lowering: {}",
            refusal.error
        ))
    };

    // The fields, each initialised by its own lowered `expr`.
    let no_payload: Vec<(String, InferredType)> = Vec::new();
    let mut fields = Vec::new();
    {
        let ctx = static_statechart(variables.iter(), &no_payload, enums);
        let renames = renames(&names, None);
        for var in &variables {
            let Some(ty) = var
                .value_type
                .as_ref()
                .and_then(crate::forge::model::AlgorithmValueType::scalar)
            else {
                return Err(GenerateError::unsupported(format!(
                    "<data id=\"{}\"> has no scalar sce:type",
                    var.id
                )));
            };
            if matches!(ty, crate::forge::model::SceType::Enum(_)) {
                return Err(GenerateError::unsupported(format!(
                    "<data id=\"{}\" sce:type=\"{}\">: an enum-typed variable has no Kotlin \
                     lowering in a statechart yet",
                    var.id,
                    ty.as_attr()
                )));
            }
            let slot = InferredType::from_sce_type(ty);
            let init = transpile_into(&var.expr, ExprTarget::Kotlin, &ctx, &renames, slot)
                .map_err(|r| refused("the initial value", &var.expr, r))?;
            fields.push(StaticField {
                id: var.id.clone(),
                name: filters::to_camel_case(var.id.clone()),
                ty: crate::forge::generator::kotlin_type(ty).to_string(),
                init,
            });
        }
    }

    let mut payload_events = BTreeSet::new();
    for state in model.states.values_mut() {
        let plain_ctx = static_statechart(variables.iter(), &no_payload, enums);
        let plain_renames = renames(&names, None);
        for block in state
            .on_entry_blocks
            .iter_mut()
            .chain(state.on_exit_blocks.iter_mut())
        {
            lower_actions(block, &plain_ctx, &plain_renames)?;
        }
        lower_actions(
            &mut state.initial_transition_actions,
            &plain_ctx,
            &plain_renames,
        )?;
        lower_actions(
            &mut state.initial_history_default_actions,
            &plain_ctx,
            &plain_renames,
        )?;
        for transition in &mut state.transitions {
            let schema = schemas.get(&transition.event);
            let payload = schema.map(event_payload_paths).unwrap_or_default();
            let ctx = static_statechart(variables.iter(), &payload, enums);
            let field = payload_field(&transition.event);
            let accessor = format!("{field}!!");
            let renames = renames(&names, schema.map(|_| accessor.as_str()));
            if !transition.cond.trim().is_empty()
                && !transition.is_cpp_condition
                && !transition.is_kt_condition
            {
                let lowered = transpile_into(
                    &transition.cond,
                    ExprTarget::Kotlin,
                    &ctx,
                    &renames,
                    InferredType::Bool,
                )
                .map_err(|r| refused("the condition", &transition.cond, r))?;
                // A condition that reads the payload holds only while the
                // dequeued event carried one — the guard every typed
                // payload read in this backend takes.
                transition.cond_kt = if schema.is_some()
                    && crate::forge::expr::references_event_data_lexically(&transition.cond)
                {
                    payload_events.insert(transition.event.clone());
                    format!("{field} != null && ({lowered})")
                } else {
                    lowered
                };
                transition.is_kt_condition = true;
                transition.cond_constant = None;
            }
            lower_actions(&mut transition.actions, &ctx, &renames)?;
        }
    }
    for script in &mut model.global_scripts {
        let ctx = static_statechart(variables.iter(), &no_payload, enums);
        lower_action(script, &ctx, &renames(&names, None))?;
    }
    Ok(KotlinStaticLowering {
        fields,
        payload_events,
    })
}

/// A `<sce:action>` argument of a `sce-static` document, lowered for Kotlin.
#[derive(Debug, Clone)]
pub(crate) struct KotlinArgument {
    /// The argument as Kotlin, reading the machine's fields and, when
    /// [`Self::reads_payload`], the bound payload.
    pub text: String,
    /// The type the host method declares for it
    /// ([`crate::forge::native_action::static_argument_type`]).
    pub ty: crate::forge::model::SceType,
    /// Whether it reads the triggering event's typed payload, so the call
    /// must sit under the payload channel's guard.
    pub reads_payload: bool,
}

/// Lower one `<sce:action>` argument of a `sce-static` document for Kotlin:
/// judged against the scope validation judged it against — the document's
/// `variables`, and `schema`'s payload when the action sits on a transition
/// whose event carries one — and spelled with the renames every other
/// lowered expression takes. `None` for an argument validation refused,
/// which never reaches here.
pub(crate) fn lower_kotlin_argument(
    variables: &[Variable],
    event: Option<(&str, &crate::forge::model::EventSchemaModel)>,
    arg: &crate::model::Param,
) -> Option<KotlinArgument> {
    let payload = event
        .map(|(_, schema)| event_payload_paths(schema))
        .unwrap_or_default();
    let ctx = static_statechart(variables.iter(), &payload, &[]);
    let names: Vec<(String, String)> = variables
        .iter()
        .map(|v| (v.id.clone(), filters::to_camel_case(v.id.clone())))
        .collect();
    let accessor = event.map(|(event, _)| format!("{}!!", payload_field(event)));
    let renames = renames(&names, accessor.as_deref());
    let ty = crate::forge::native_action::static_argument_type(&ctx, arg).ok()?;
    let text = transpile_into(
        &arg.expr,
        ExprTarget::Kotlin,
        &ctx,
        &renames,
        InferredType::from_sce_type(&ty),
    )
    .ok()?;
    Some(KotlinArgument {
        text,
        ty,
        reads_payload: event.is_some()
            && crate::forge::expr::references_event_data_lexically(&arg.expr),
    })
}

/// The nullable field the Kotlin payload channel binds `event`'s typed
/// payload to — the spelling [`crate::forge::generator::build_kotlin_event_payload`]
/// declares.
fn payload_field(event: &str) -> String {
    format!(
        "pending{}Payload",
        filters::to_event_variant(event.to_string())
    )
}

/// The renames a lowered expression takes: each variable to its field, `In`
/// to the machine's active-state test (the function a pure `In()` guard
/// lowers to), and `_event.data` to the payload accessor when there is one.
fn renames<'a>(
    names: &'a [(String, String)],
    payload: Option<&'a str>,
) -> HashMap<&'a str, &'a str> {
    let mut map: HashMap<&str, &str> = names
        .iter()
        .map(|(id, name)| (id.as_str(), name.as_str()))
        .collect();
    map.insert("In", "isStateActive");
    if let Some(accessor) = payload {
        map.insert("_event.data", accessor);
    }
    map
}

fn lower_actions(
    actions: &mut [Action],
    ctx: &crate::forge::types::TypeCtx<'_>,
    renames: &HashMap<&str, &str>,
) -> Result<(), GenerateError> {
    for action in actions {
        lower_action(action, ctx, renames)?;
    }
    Ok(())
}

fn lower_action(
    action: &mut Action,
    ctx: &crate::forge::types::TypeCtx<'_>,
    renames: &HashMap<&str, &str>,
) -> Result<(), GenerateError> {
    let lower = |text: &str, slot: InferredType| {
        transpile_into(text, ExprTarget::Kotlin, ctx, renames, slot).map_err(|r| {
            GenerateError::unsupported(format!("`{text}` has no Kotlin lowering: {}", r.error))
        })
    };
    match action.action_type.as_str() {
        "assign" => {
            let slot = crate::forge::expr::infer_expr_type(&action.location, ctx)
                .unwrap_or(InferredType::Unknown);
            action.expr = lower(&action.expr, slot)?;
        }
        "if" if !action.is_cpp_condition && !action.is_kt_condition => {
            action.cond_kt = lower(&action.cond, InferredType::Bool)?;
            action.is_kt_condition = true;
            action.cond_constant = None;
        }
        "log" if !action.expr.trim().is_empty() => {
            action.expr = transpile_typed(
                &action.expr,
                ExprTarget::Kotlin,
                ctx,
                renames,
                InferredType::Unknown,
            )
            .map_err(|r| {
                GenerateError::unsupported(format!(
                    "`{}` has no Kotlin lowering: {}",
                    action.expr, r.error
                ))
            })?;
        }
        _ => {}
    }
    lower_nested(action, ctx, renames)
}

/// Every `<elseif>` condition and every block nested inside `action`,
/// through the model's own accessors for them.
fn lower_nested(
    action: &mut Action,
    ctx: &crate::forge::types::TypeCtx<'_>,
    renames: &HashMap<&str, &str>,
) -> Result<(), GenerateError> {
    for branch in action.branch_conditions_mut() {
        if branch.is_cpp_condition || branch.is_kt_condition || branch.cond.trim().is_empty() {
            continue;
        }
        branch.cond_kt = transpile_into(
            &branch.cond,
            ExprTarget::Kotlin,
            ctx,
            renames,
            InferredType::Bool,
        )
        .map_err(|r| {
            GenerateError::unsupported(format!(
                "`{}` has no Kotlin lowering: {}",
                branch.cond, r.error
            ))
        })?;
        branch.is_kt_condition = true;
        branch.cond_constant = None;
    }
    for block in action.nested_blocks_mut() {
        lower_actions(block, ctx, renames)?;
    }
    Ok(())
}
