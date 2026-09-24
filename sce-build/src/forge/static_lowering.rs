// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The static data model, lowered for one backend (docs/SCE_ACCEPTED_SUBSET.md
// §2.15).
//
// [`crate::forge::static_datamodel`] judges a `sce-static` document; this
// rewrites the one a backend renders. Every expression goes through the forge
// expression lowerer against the same scope the judge used
// ([`crate::forge::type_ctx::StaticScope`]), and lands in a slot the
// backend's templates already render as native code: a condition in the
// native-condition fields a `kt:` guard fills, an `<assign>` or `<log>` value
// in the text the engine-free arm pastes. So a `sce-static` machine needs no
// template of its own beyond its variables' declarations — which this returns.

use std::collections::{BTreeSet, HashMap};

use crate::filters;
use crate::forge::error::GenerateError;
use crate::forge::expr::{transpile_into, transpile_typed, ExprTarget, Refusal};
use crate::forge::type_ctx::{StaticEnum, StaticScope};
use crate::forge::types::InferredType;
use crate::model::{Action, SCXMLModel};

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
    /// One top-level data class per event-schema a `record:<alias>`
    /// variable names, declared in the machine's own file the way its event
    /// payload classes are — so the machine imports no other unit.
    pub record_defs: Vec<String>,
}

/// The Kotlin class a `record:<alias>` variable of `machine` is held in:
/// declared in the machine's own file, one per alias.
fn record_class(machine: &str, alias: &str) -> String {
    format!(
        "{machine}{}Record",
        filters::to_pascal_case(alias.to_string())
    )
}

/// The data class [`record_class`] names: one `val` per schema field, in the
/// schema's order, typed as the payload classes type them. An enum field is
/// refused, as an enum variable is.
fn record_def(
    class: &str,
    alias: &str,
    schema: &crate::forge::model::EventSchemaModel,
) -> Result<String, GenerateError> {
    let mut params = Vec::with_capacity(schema.fields.len());
    for field in &schema.fields {
        if matches!(field.sce_type, crate::forge::model::SceType::Enum(_)) {
            return Err(GenerateError::unsupported(format!(
                "record:{alias} has the enum-typed field `{}`, which has no Kotlin \
                 lowering in a statechart yet",
                field.id
            )));
        }
        params.push(format!(
            "val {}: {}",
            crate::forge::generator::event_schema_field_ident(
                &field.id,
                crate::generator::Language::Kotlin
            ),
            crate::forge::generator::kotlin_type(&field.sce_type)
        ));
    }
    Ok(format!(
        "/** SCE Accepted Subset §2.15: a `record:{alias}` datamodel value. */\ndata class {class}({})",
        params.join(", ")
    ))
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
    machine: &str,
    enums: &[StaticEnum],
) -> Result<KotlinStaticLowering, GenerateError> {
    let Some(scope) = StaticScope::of(model) else {
        return Ok(KotlinStaticLowering::default());
    };
    let variables = &scope.variables;
    let schemas = model.imported_event_schemas.clone();
    let records = model.imported_records.clone();
    let names: Vec<(String, String)> = variables
        .iter()
        .map(|v| (v.id.clone(), filters::to_camel_case(v.id.clone())))
        .collect();
    // The record variables, each with the schema its alias names — what a
    // field assignment is rewritten against.
    let record_vars: RecordVars = variables
        .iter()
        .filter_map(|v| {
            let alias = v.value_type.as_ref()?.record_alias()?;
            Some((v.id.clone(), records.get(alias)?.clone()))
        })
        .collect();
    // The list variables, each with its element and bound — what an
    // `<sce:append>` is rewritten against.
    let list_vars: ListVars = variables
        .iter()
        .filter_map(|v| {
            let elem = v.value_type.as_ref()?.list_elem()?.clone();
            Some((v.id.clone(), (elem, v.capacity?)))
        })
        .collect();
    let rewrites = Rewrites {
        records: record_vars,
        lists: list_vars,
        machine,
        raises_error: model.events.contains("error.execution"),
    };

    let refused = |what: &str, text: &str, refusal: Refusal| {
        GenerateError::unsupported(format!(
            "{what} `{text}` has no Kotlin lowering: {}",
            refusal.error
        ))
    };

    // Every record variable's fields, in every scope below.
    let no_payload = scope.paths(None);
    let mut fields = Vec::new();
    let mut record_defs = Vec::new();
    let mut declared_classes = BTreeSet::new();
    {
        let ctx = scope.ctx(&no_payload, enums);
        let renames = renames(&names, None);
        for var in variables {
            // A record variable is built whole from its `<sce:set>`s, in
            // the schema's order — the rule the judge already held it to.
            if let Some(alias) = var.value_type.as_ref().and_then(|t| t.record_alias()) {
                let schema = records.get(alias).ok_or_else(|| {
                    GenerateError::unsupported(format!(
                        "<data id=\"{}\">: record:{alias} names no event-schema this build \
                         read",
                        var.id
                    ))
                })?;
                let class = record_class(machine, alias);
                if declared_classes.insert(class.clone()) {
                    record_defs.push(record_def(&class, alias, schema)?);
                }
                let mut args = Vec::with_capacity(schema.fields.len());
                for field in &schema.fields {
                    let init = var
                        .record_fields
                        .iter()
                        .find(|f| f.name == field.id)
                        .ok_or_else(|| {
                            GenerateError::unsupported(format!(
                                "<data id=\"{}\">: no <sce:set> gives `{}`",
                                var.id, field.id
                            ))
                        })?;
                    let value = transpile_into(
                        &init.expr,
                        ExprTarget::Kotlin,
                        &ctx,
                        &renames,
                        InferredType::from_sce_type(&field.sce_type),
                    )
                    .map_err(|r| refused("the field value", &init.expr, r))?;
                    args.push(format!(
                        "{} = {value}",
                        crate::forge::generator::event_schema_field_ident(
                            &field.id,
                            crate::generator::Language::Kotlin
                        )
                    ));
                }
                fields.push(StaticField {
                    id: var.id.clone(),
                    name: filters::to_camel_case(var.id.clone()),
                    init: format!("{class}({})", args.join(", ")),
                    ty: class,
                });
                continue;
            }
            // A list starts empty. It is immutable, so a snapshot holding it
            // keeps what it saw however the machine appends afterwards.
            if let Some(elem) = var.value_type.as_ref().and_then(|t| t.list_elem()) {
                fields.push(StaticField {
                    id: var.id.clone(),
                    name: filters::to_camel_case(var.id.clone()),
                    ty: format!("List<{}>", crate::forge::generator::kotlin_type(elem)),
                    init: "emptyList()".to_string(),
                });
                continue;
            }
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
        let plain_ctx = scope.ctx(&no_payload, enums);
        let plain_renames = renames(&names, None);
        for block in state
            .on_entry_blocks
            .iter_mut()
            .chain(state.on_exit_blocks.iter_mut())
        {
            lower_actions(block, &plain_ctx, &plain_renames, &rewrites)?;
        }
        lower_actions(
            &mut state.initial_transition_actions,
            &plain_ctx,
            &plain_renames,
            &rewrites,
        )?;
        lower_actions(
            &mut state.initial_history_default_actions,
            &plain_ctx,
            &plain_renames,
            &rewrites,
        )?;
        for transition in &mut state.transitions {
            let schema = schemas.get(&transition.event);
            let paths = scope.paths(schema);
            let ctx = scope.ctx(&paths, enums);
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
            // Content that reads the payload cannot run for a delivery that
            // did not carry one; the template opens it with the check that
            // says so ([`crate::model::Transition::content_reads_payload`]).
            if lower_actions(&mut transition.actions, &ctx, &renames, &rewrites)?
                && schema.is_some()
            {
                payload_events.insert(transition.event.clone());
                transition.content_reads_payload = true;
            }
        }
    }
    for script in &mut model.global_scripts {
        let ctx = scope.ctx(&no_payload, enums);
        lower_action(script, &ctx, &renames(&names, None), &rewrites)?;
    }
    Ok(KotlinStaticLowering {
        fields,
        payload_events,
        record_defs,
    })
}

/// A `sce-static` document's record variables, each with the schema its
/// alias names.
type RecordVars = std::collections::BTreeMap<String, crate::forge::model::EventSchemaModel>;

/// A `sce-static` document's list variables, each with its element type and
/// its declared capacity.
type ListVars = std::collections::BTreeMap<String, (crate::forge::model::SceType, u32)>;

/// What rewriting an action needs beyond its expressions: the record and
/// list variables a write to one is rewritten against, the machine name the
/// generated event type is spelled from, and whether the document declares
/// `error.execution` — without it there is no variant to raise, and nothing
/// could match one.
struct Rewrites<'m> {
    records: RecordVars,
    lists: ListVars,
    machine: &'m str,
    raises_error: bool,
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
/// `scope`, and the event's payload when the action sits on a transition
/// whose event carries one — and spelled with the renames every other
/// lowered expression takes. `None` for an argument validation refused,
/// which never reaches here.
pub(crate) fn lower_kotlin_argument(
    scope: &StaticScope,
    event: Option<(&str, &crate::forge::model::EventSchemaModel)>,
    arg: &crate::model::Param,
) -> Option<KotlinArgument> {
    let paths = scope.paths(event.map(|(_, schema)| schema));
    let ctx = scope.ctx(&paths, &[]);
    let names: Vec<(String, String)> = scope
        .variables
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

/// Lower every action of `actions` in place. `true` when any expression
/// lowered reads the triggering event's payload.
fn lower_actions(
    actions: &mut [Action],
    ctx: &crate::forge::types::TypeCtx<'_>,
    renames: &HashMap<&str, &str>,
    rewrites: &Rewrites<'_>,
) -> Result<bool, GenerateError> {
    let mut reads_payload = false;
    for action in actions {
        reads_payload |= lower_action(action, ctx, renames, rewrites)?;
    }
    Ok(reads_payload)
}

/// Lower one action and what it nests, in place. `true` when any expression
/// lowered reads the triggering event's payload.
fn lower_action(
    action: &mut Action,
    ctx: &crate::forge::types::TypeCtx<'_>,
    renames: &HashMap<&str, &str>,
    rewrites: &Rewrites<'_>,
) -> Result<bool, GenerateError> {
    let lower = |text: &str, slot: InferredType| {
        transpile_into(text, ExprTarget::Kotlin, ctx, renames, slot).map_err(|r| {
            GenerateError::unsupported(format!("`{text}` has no Kotlin lowering: {}", r.error))
        })
    };
    let reads = crate::forge::expr::references_event_data_lexically;
    let mut reads_payload = false;
    match action.action_type.as_str() {
        "assign" => {
            reads_payload = reads(&action.expr);
            let slot = crate::forge::expr::infer_expr_type(&action.location, ctx)
                .unwrap_or(InferredType::Unknown);
            let value = lower(&action.expr, slot)?;
            // A record's field is a `val` of an immutable data class, so the
            // assignment builds the next value with that field replaced —
            // the lowering an algorithm's record local takes (E9).
            let location = action.location.trim().to_string();
            match location
                .split_once('.')
                .filter(|(var, _)| rewrites.records.contains_key(*var))
            {
                Some((var, field)) => {
                    let name = renames.get(var).copied().unwrap_or(var);
                    action.expr = format!(
                        "{name}.copy({} = {value})",
                        crate::forge::generator::event_schema_field_ident(
                            field,
                            crate::generator::Language::Kotlin
                        )
                    );
                    action.location = var.to_string();
                }
                None => action.expr = value,
            }
        }
        "if" if !action.is_cpp_condition && !action.is_kt_condition => {
            reads_payload = reads(&action.cond);
            action.cond_kt = lower(&action.cond, InferredType::Bool)?;
            action.is_kt_condition = true;
            action.cond_constant = None;
        }
        "log" if !action.expr.trim().is_empty() => {
            reads_payload = reads(&action.expr);
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
        // A list is an immutable `List<T>` field, so an append builds the
        // next list, and does so only while the list is under its bound — on
        // every backend, so a machine holds the same list wherever it runs.
        // Past the bound nothing is appended and `error.execution` says so
        // (W3C SCXML 3.12.2), as every other execution error of this backend
        // is reported.
        "sce_append" => {
            reads_payload = reads(&action.expr);
            let target = action.location.trim();
            let (elem, capacity) = rewrites.lists.get(target).ok_or_else(|| {
                GenerateError::unsupported(format!(
                    "<sce:append target=\"{target}\"> names no list variable"
                ))
            })?;
            let value = lower(&action.expr, InferredType::from_sce_type(elem))?;
            let name = renames.get(target).copied().unwrap_or(target);
            let otherwise = if rewrites.raises_error {
                format!(
                    " else {{ raisePlatformError({}Event.Error.Execution, \
                     \"<sce:append target='{}'>: the list already holds its capacity of \
                     {capacity}\") }}",
                    rewrites.machine,
                    filters::escape_kotlin(target.to_string()),
                )
            } else {
                String::new()
            };
            action.content_kt = format!(
                "if ({name}.size < {capacity}) {{ {name} = {name} + ({value}) }}{otherwise}"
            );
        }
        "sce_clear" => {
            let target = action.location.trim();
            let name = renames.get(target).copied().unwrap_or(target);
            action.content_kt = format!("{name} = emptyList()");
        }
        _ => {}
    }
    Ok(lower_nested(action, ctx, renames, rewrites)? || reads_payload)
}

/// Every `<elseif>` condition and every block nested inside `action`,
/// through the model's own accessors for them. `true` when any expression
/// lowered reads the triggering event's payload.
fn lower_nested(
    action: &mut Action,
    ctx: &crate::forge::types::TypeCtx<'_>,
    renames: &HashMap<&str, &str>,
    rewrites: &Rewrites<'_>,
) -> Result<bool, GenerateError> {
    let mut reads_payload = false;
    for branch in action.branch_conditions_mut() {
        if branch.is_cpp_condition || branch.is_kt_condition || branch.cond.trim().is_empty() {
            continue;
        }
        reads_payload |= crate::forge::expr::references_event_data_lexically(&branch.cond);
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
        reads_payload |= lower_actions(block, ctx, renames, rewrites)?;
    }
    Ok(reads_payload)
}
