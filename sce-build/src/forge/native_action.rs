// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! §scxml-G-7 — `<sce:action>` Custom Action Element: native host-trait
//! dispatch.
//!
//! A `<sce:action name="op"><sce:arg expr="_event.data.field"/></sce:action>`
//! names a host operation that lowers to a direct call on a generated
//! `<Machine>Actions` trait method — **no runtime script engine**. This is
//! the engine-free symmetric counterpart, for *effects*, of the typed
//! `_event.data` guard lowering (see
//! [`crate::forge::event_schema_check::lower_typed_guard`]): the SCXML keeps
//! the operation symbolic (language-neutral SSOT), each argument flows
//! through the EventSchema typed-payload channel, and the host supplies the
//! behaviour by implementing the generated trait.
//!
//! v1 contract (enforced by [`validate`]):
//!
//! - **Placement.** A native action is a *direct child* of a `<transition>`,
//!   an `<onentry>`/`<onexit>` block, or initial executable content (an
//!   `<initial>` transition or a history state's default transition). The
//!   §scxml-G-7 example itself places a Custom Action Element directly in
//!   `<onentry>`. Nesting inside `<if>`/`<foreach>` is rejected — that call
//!   site is conditional or iterated, which v1 does not lower (the same
//!   limitation holds on a transition and in entry/exit alike).
//! - **Arguments.** Reading a typed argument needs the *triggering event's*
//!   payload in scope, which happens only on a `<transition>`. There, every
//!   `<sce:arg>` is a bare `_event.data.<field>` reference resolving to a
//!   declared, payload-eligible field on that event's imported EventSchema.
//!   `<onentry>`/`<onexit>`/initial content runs with no triggering event, so
//!   only a *no-argument* action is admissible there; it lowers to a bare
//!   host-trait call.
//! - **Consistency.** A `name` reused across call sites must carry the same
//!   argument types every time, so a single generated trait method serves
//!   them all.
//!
//! Anything outside this contract — a literal/derived argument, an unknown
//! field, an enum-typed schema, an arg-bearing action off a transition, or a
//! nested placement — is rejected at the validation stage rather than silently
//! routed through a script engine. The construct is engine-free *by
//! definition*, so it never degrades to a runtime fallback.

use crate::filters;
use crate::forge::error::{
    ForgeError, Located, RelatedRole, RelatedSite, SourceLocation, ValidationError,
};
use crate::forge::event_schema_check::schema_is_native_payload_eligible;
use crate::forge::generator::host_param_type;
use crate::forge::model::{EventSchemaModel, ForgeKind, SceType};
use crate::generator::Language;
use crate::model::{Action, Param, SCXMLModel};
use std::collections::{BTreeMap, BTreeSet};

const ACTION_TYPE: &str = "native_action";
const EVENT_DATA_PREFIX: &str = "_event.data.";

/// The symbolic name of the host operation that answers the host a child is
/// built with (docs/adr/0005, decision 6): `actions_for_<invoke>` for an
/// `<invoke type="scxml">`, and `actions_for_<invoke>_<stem>` for one candidate
/// of a hybrid `<invoke>`, each candidate being a different document with its
/// own acts. `field_suffix` is [`crate::model::InvokeBase::field_suffix`], the
/// identifier-safe spelling of the invoke's id. Spelled per language by
/// [`method_name`], as an act's name is, so one interface holds both.
fn host_operation(field_suffix: &str, candidate_stem: Option<&str>) -> String {
    match candidate_stem {
        None => format!("actions_for_{field_suffix}"),
        Some(stem) => format!("actions_for_{field_suffix}_{stem}"),
    }
}

/// The first `<invoke type="scxml">` of `model` whose child declares
/// `<sce:action>`s — or, of a hybrid one, a candidate that does — described for
/// a refusal, with where the `<invoke>` is.
///
/// A child's machine takes the host that performs its acts when it is built
/// (§scxml-6.4.1), so a parent that does not obtain one for it writes a call of
/// that constructor with the host left out, which does not compile in Rust,
/// Kotlin, Go or C++ and fails when the invoke starts in Python. Every generated
/// language obtains it from its parent's own host now; a `sce-static` target that
/// is not one — the Interpreter's lowering — asks this for itself, and refuses.
pub fn child_that_needs_a_host(model: &SCXMLModel) -> Option<(String, Option<SourceLocation>)> {
    model
        .states
        .values()
        .flat_map(|state| &state.invokes)
        .find_map(|invoke| match invoke {
            crate::model::Invoke::Scxml(info) if info.common.child_declares_host_acts => Some((
                format!(
                    "an <invoke id=\"{}\"> of a child that declares <sce:action>s",
                    info.common.base.invoke_id
                ),
                info.common.base.source_location.clone(),
            )),
            crate::model::Invoke::Hybrid(info) => info
                .candidates
                .iter()
                .find(|candidate| candidate.child_declares_host_acts)
                .map(|candidate| {
                    (
                        format!(
                            "an <invoke id=\"{}\"> that may start `{}`, a child that declares \
                             <sce:action>s",
                            info.common.base.invoke_id, candidate.stem
                        ),
                        info.common.base.source_location.clone(),
                    )
                }),
            _ => None,
        })
}

/// One child the parent obtains a host for: what [`host_operation`] names and
/// the `<invoke>` it belongs to.
struct ChildHostSite {
    operation: String,
    invoke_id: String,
    at: Option<SourceLocation>,
}

/// Every child of `model` that declares `<sce:action>`s: each
/// `<invoke type="scxml">` whose child does, and each candidate of a hybrid
/// `<invoke>` that does, in the order of the sorted states.
fn child_host_sites(model: &SCXMLModel) -> Vec<ChildHostSite> {
    let mut sites = Vec::new();
    for state in model.states.values() {
        for invoke in &state.invokes {
            match invoke {
                crate::model::Invoke::Scxml(info) if info.common.child_declares_host_acts => {
                    sites.push(ChildHostSite {
                        operation: host_operation(&info.common.base.field_suffix, None),
                        invoke_id: info.common.base.invoke_id.clone(),
                        at: info.common.base.source_location.clone(),
                    });
                }
                crate::model::Invoke::Hybrid(info) => {
                    for candidate in info
                        .candidates
                        .iter()
                        .filter(|c| c.child_declares_host_acts)
                    {
                        sites.push(ChildHostSite {
                            operation: host_operation(
                                &info.common.base.field_suffix,
                                Some(&candidate.stem),
                            ),
                            invoke_id: info.common.base.invoke_id.clone(),
                            at: info.common.base.source_location.clone(),
                        });
                    }
                }
                _ => {}
            }
        }
    }
    sites
}

/// If `expr` is exactly a `_event.data.<field>` reference, return `<field>`.
/// Any other shape (literal, arithmetic, datamodel id, nested path) returns
/// `None` — those are rejected by [`validate`].
fn arg_field(expr: &str) -> Option<&str> {
    let field = expr.trim().strip_prefix(EVENT_DATA_PREFIX)?;
    if !field.is_empty() && field.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        Some(field)
    } else {
        None
    }
}

fn is_native(action: &Action) -> bool {
    action.action_type == ACTION_TYPE
}

/// `true` iff `model` declares any `<sce:action>` anywhere (transitions,
/// entry/exit, or nested executable content). The non-Rust backends call
/// this to refuse the construct with a clear `generate/unsupported-feature`
/// diagnostic instead of failing on a missing per-language action template.
pub fn document_has_native_actions(model: &SCXMLModel) -> bool {
    model.states.values().any(|state| {
        state
            .transitions
            .iter()
            .flat_map(|t| t.actions.iter())
            .chain(state.on_entry_blocks.iter().flatten())
            .chain(state.on_exit_blocks.iter().flatten())
            .chain(state.initial_transition_actions.iter())
            .chain(state.initial_history_default_actions.iter())
            .any(|a| first_native(a).is_some())
    })
}

/// Find the first native `<sce:action>` at or below `action` (covering the
/// nested `<if>`/`<foreach>` executable-content trees). Used to reject
/// native actions in positions v1 does not support.
fn first_native(action: &Action) -> Option<&Action> {
    if is_native(action) {
        return Some(action);
    }
    action
        .nested_blocks()
        .into_iter()
        .flat_map(|block| block.actions.iter())
        .find_map(first_native)
}

fn located_on_action(
    action: &Action,
    diag_label: &str,
    err: ValidationError,
) -> Located<ForgeError> {
    located_at(action.source_location.as_ref(), diag_label, err)
}

/// A refusal at a recorded position, or file-scoped when none was
/// recorded.
fn located_at(
    at: Option<&SourceLocation>,
    diag_label: &str,
    err: ValidationError,
) -> Located<ForgeError> {
    let (line, col) = at.map_or((None, None), |l| (l.line, l.col));
    Located::new(ForgeError::Validation(Box::new(err)), diag_label, line, col)
}

fn placement_err(action: &Action, diag_label: &str, detail: &str) -> Located<ForgeError> {
    located_on_action(
        action,
        diag_label,
        ValidationError::NativeActionPlacement {
            name: action.native_action_name.clone(),
            detail: detail.to_string(),
        },
    )
}

/// A refusal of the action's arguments as a whole, on the action's row,
/// where its name is.
fn argument_err(action: &Action, diag_label: &str, detail: String) -> Located<ForgeError> {
    located_on_action(
        action,
        diag_label,
        ValidationError::NativeActionArgument {
            name: action.native_action_name.clone(),
            detail,
            observed: action.native_action_name.clone(),
        },
    )
}

/// A refusal of one argument, on its `<sce:arg>` row, where its `expr`
/// is — not on the action's, which does not hold it.
fn one_argument_err(
    action: &Action,
    arg: &Param,
    diag_label: &str,
    detail: String,
) -> Located<ForgeError> {
    located_at(
        arg.source_location
            .as_ref()
            .or(action.source_location.as_ref()),
        diag_label,
        ValidationError::NativeActionArgument {
            name: action.native_action_name.clone(),
            detail,
            observed: arg.expr.clone(),
        },
    )
}

/// Backend-neutral canonical name for a payload field type, used as the
/// signature-comparison key and in the conflict diagnostic. Enum is included
/// for exhaustiveness but never reached — enum-typed payloads are rejected by
/// the eligibility check before signatures are collected.
fn canonical_type(ty: &SceType) -> &'static str {
    match ty {
        SceType::Uint8 => "uint8",
        SceType::Uint16 => "uint16",
        SceType::Uint32 => "uint32",
        SceType::Uint64 => "uint64",
        SceType::Int8 => "int8",
        SceType::Int16 => "int16",
        SceType::Int32 => "int32",
        SceType::Int64 => "int64",
        SceType::Float32 => "float32",
        SceType::Float64 => "float64",
        SceType::Bool => "bool",
        SceType::String => "string",
        SceType::Bytes => "bytes",
        SceType::Enum(_) => "enum",
    }
}

/// The payload scope available to a `<sce:action>` at its host position.
///
/// A native action lowers to a host-trait call whose arguments, if any, are
/// read from the *triggering event's* typed payload. That payload is in scope
/// only on a `<transition>`; an `<onentry>`/`<onexit>` block or initial
/// executable content runs with no triggering event, so only a no-argument
/// action is admissible there.
enum PayloadScope<'a> {
    /// Direct `<transition>` child: the triggering event and its imported
    /// EventSchema (if any) are in scope, so arguments are permitted.
    Transition {
        event: &'a str,
        schema: Option<&'a EventSchemaModel>,
    },
    /// `<onentry>`/`<onexit>`/initial executable content: no triggering event,
    /// hence no typed payload. Only a no-argument action is admissible.
    Eventless,
}

/// Validate every `<sce:action>` in `scxml` against the v1 contract.
///
/// `imported_schemas` is the per-statechart `event → EventSchemaModel` view
/// (the same map [`crate::forge::event_schema_check::check`] consumes).
/// Returns the first failing diagnostic; `Ok(())` when there are no native
/// actions or all satisfy the contract.
pub fn validate(
    scxml: &SCXMLModel,
    imported_schemas: &BTreeMap<String, EventSchemaModel>,
    diag_label: &str,
) -> Result<(), Located<ForgeError>> {
    // Document-wide signature table: a `name` that recurs — on any transition
    // or in any entry/exit/initial block — must carry the same argument types
    // every time, so a single generated `Actions` trait method serves every
    // call site. Detecting a conflict here is fail-fast at SCE's own validation
    // stage rather than deferring it to a type error in the downstream compiler.
    let mut signatures = SignatureTable::new();
    // Under `datamodel="sce-static"` an argument is any typed expression over
    // the document's scope, not only a payload field (SCE Accepted Subset
    // §2.15), so the arguments are judged against that scope.
    let static_scope = crate::forge::type_ctx::StaticScope::of(scxml);

    for state in scxml.states.values() {
        // Eventless positions: <onentry>/<onexit> blocks, an <initial>
        // transition's executable content, and a history state's default
        // transition content. No triggering event is in scope, so only a
        // no-argument native action is admissible.
        let eventless = state
            .on_entry_blocks
            .iter()
            .chain(state.on_exit_blocks.iter())
            .flatten()
            .chain(state.initial_transition_actions.iter())
            .chain(state.initial_history_default_actions.iter());
        for action in eventless {
            check_placement(
                action,
                &PayloadScope::Eventless,
                static_scope.as_ref(),
                &mut signatures,
                diag_label,
            )?;
        }

        for transition in &state.transitions {
            let scope = PayloadScope::Transition {
                event: &transition.event,
                schema: imported_schemas.get(&transition.event),
            };
            for action in &transition.actions {
                check_placement(
                    action,
                    &scope,
                    static_scope.as_ref(),
                    &mut signatures,
                    diag_label,
                )?;
            }
        }
    }
    check_host_operations(scxml, &signatures, diag_label)
}

/// The host interface carries two kinds of operation in one namespace: the acts
/// the document names and, for each child that declares acts of its own, the
/// operation that answers the child's host ([`host_operation`]). A name both
/// kinds spell the same would be one method serving two meanings, so it is
/// refused at the construct that came second, as a conflict of signatures is.
fn check_host_operations(
    scxml: &SCXMLModel,
    signatures: &SignatureTable,
    diag_label: &str,
) -> Result<(), Located<ForgeError>> {
    let acts: BTreeMap<String, (&String, &Option<SourceLocation>)> = signatures
        .iter()
        .map(|(name, (_, at))| (filters::to_snake_case(name.clone()), (name, at)))
        .collect();
    let mut seen: BTreeMap<String, String> = BTreeMap::new();
    for site in child_host_sites(scxml) {
        let operation = filters::to_snake_case(site.operation.clone());
        if let Some((name, at)) = acts.get(&operation) {
            return Err(located_at(
                at.as_ref(),
                diag_label,
                ValidationError::NativeActionSignatureConflict {
                    name: (*name).clone(),
                    detail: format!(
                        "is also the host operation that answers the host of the child \
                         <invoke id=\"{}\"> starts; rename the act or the invoke",
                        site.invoke_id
                    ),
                },
            ));
        }
        if let Some(first) = seen.insert(operation.clone(), site.invoke_id.clone()) {
            return Err(located_at(
                site.at.as_ref(),
                diag_label,
                ValidationError::NativeActionSignatureConflict {
                    name: operation,
                    detail: format!(
                        "is the host operation of the children of both <invoke id=\"{first}\"> \
                         and <invoke id=\"{}\">; their ids differ only in spelling",
                        site.invoke_id
                    ),
                },
            ));
        }
    }
    Ok(())
}

/// Validate one executable-content `action` against the payload scope of its
/// host position, registering any direct `<sce:action>`'s signature in the
/// document-wide table. A native action nested inside `<if>`/`<foreach>` is
/// rejected: its call site is conditional or iterated, which v1 does not lower
/// (the same limitation applies on a transition and in entry/exit alike).
fn check_placement(
    action: &Action,
    scope: &PayloadScope,
    static_scope: Option<&crate::forge::type_ctx::StaticScope>,
    signatures: &mut SignatureTable,
    diag_label: &str,
) -> Result<(), Located<ForgeError>> {
    if is_native(action) {
        let sig = match static_scope {
            Some(static_scope) => static_signature(action, scope, static_scope, diag_label)?,
            None => signature_of(action, scope, diag_label)?,
        };
        register_signature(action, sig, signatures, diag_label)
    } else if let Some(na) = first_native(action) {
        Err(placement_err(
            na,
            diag_label,
            "supported only as a direct child of <transition>/<onentry>/<onexit>/\
             initial executable content (found nested inside <if>/<foreach>)",
        ))
    } else {
        Ok(())
    }
}

/// Resolve a direct `<sce:action>`'s signature against its payload scope.
/// On a transition the triggering event's payload is in scope, so arguments
/// are validated against its EventSchema; in an eventless position only a
/// no-argument action is admissible.
fn signature_of(
    action: &Action,
    scope: &PayloadScope,
    diag_label: &str,
) -> Result<Vec<String>, Located<ForgeError>> {
    match scope {
        PayloadScope::Transition { event, schema } => {
            validate_args(action, *schema, event, diag_label)
        }
        PayloadScope::Eventless if action.params.is_empty() => Ok(Vec::new()),
        PayloadScope::Eventless => Err(argument_err(
            action,
            diag_label,
            format!(
                "native action '{}' in <onentry>/<onexit>/initial executable content must \
                 take no arguments: no triggering event (hence no typed `_event.data` \
                 payload) is in scope there. Only a direct <transition> child reads payload.",
                action.native_action_name
            ),
        )),
    }
}

/// A direct `<sce:action>`'s signature under `datamodel="sce-static"`: each
/// argument judged against the document's typed scope — its variables, and
/// the triggering event's typed payload when the action sits on a transition
/// whose event carries a schema — and typed as the value it is
/// ([`static_argument_type`]).
///
/// An argument may read the datamodel in an eventless position: a variable
/// has a type with no event in sight. Reading `_event.data` there is refused,
/// because no payload is in scope to type it.
fn static_signature(
    action: &Action,
    scope: &PayloadScope,
    static_scope: &crate::forge::type_ctx::StaticScope,
    diag_label: &str,
) -> Result<Vec<String>, Located<ForgeError>> {
    let payload = match scope {
        PayloadScope::Transition { schema, .. } => *schema,
        PayloadScope::Eventless => None,
    };
    let paths = static_scope.paths(payload);
    let ctx = static_scope.ctx(&paths, &[]);
    let mut sig = Vec::with_capacity(action.params.len());
    for arg in &action.params {
        // A list is not a value a host method takes; refused as it is in
        // every other expression of the document, before its type is asked.
        if let Some(list) = static_scope.list_read_as_value(&arg.expr) {
            return Err(crate::forge::error::Located::in_file(
                crate::forge::expression_site::ExpressionSite::new(
                    &arg.expr,
                    arg.expr_spelling.as_ref(),
                )
                .place(crate::forge::static_datamodel::list_read_refusal(&list)),
                diag_label,
            ));
        }
        let ty = static_argument_type(&ctx, arg).map_err(|refusal| match refusal {
            ArgumentRefusal::Expression(refusal) => crate::forge::error::Located::in_file(
                crate::forge::expression_site::ExpressionSite::new(
                    &arg.expr,
                    arg.expr_spelling.as_ref(),
                )
                .place(refusal),
                diag_label,
            ),
            ArgumentRefusal::Untyped => one_argument_err(
                action,
                arg,
                diag_label,
                format!(
                    "argument '{}' has no type a host method could declare — \
                     `_event.data` is typed only on a transition whose event \
                     imports an EventSchema",
                    arg.expr
                ),
            ),
        })?;
        sig.push(canonical_type(&ty).to_string());
    }
    Ok(sig)
}

/// Why a static argument could not be typed.
pub(crate) enum ArgumentRefusal {
    /// The expression itself was refused by the expression layer.
    Expression(crate::forge::expr::Refusal),
    /// It was accepted, but its type is not one a host method can declare.
    Untyped,
}

/// The declared type a `<sce:action>` argument is passed as under the static
/// data model: the argument judged against `ctx` and carried as the value it
/// is ([`crate::forge::types::InferredType::to_sce_type`]). The one reading,
/// shared by validation and by each backend's lowering, so the method a host
/// implements and the call a machine makes cannot disagree.
pub(crate) fn static_argument_type(
    ctx: &crate::forge::types::TypeCtx<'_>,
    arg: &Param,
) -> Result<SceType, ArgumentRefusal> {
    let ty = crate::forge::expr::judge_into(
        &arg.expr,
        ctx,
        crate::forge::expr::Expected::Hint(crate::forge::types::InferredType::Unknown),
    )
    .map_err(ArgumentRefusal::Expression)?;
    ty.to_sce_type().ok_or(ArgumentRefusal::Untyped)
}

/// The first call of each native action name: the signature every later
/// call must match, and where that call is, so a conflict can name it.
type SignatureTable = BTreeMap<String, (Vec<String>, Option<SourceLocation>)>;

/// Record `sig` for `action.native_action_name` in the document-wide table,
/// rejecting a divergence from a prior occurrence — one generated trait method
/// must serve every call site of a given name, regardless of position.
fn register_signature(
    action: &Action,
    sig: Vec<String>,
    signatures: &mut SignatureTable,
    diag_label: &str,
) -> Result<(), Located<ForgeError>> {
    match signatures.get(&action.native_action_name) {
        Some((prev, first_at)) if *prev != sig => {
            let refusal = located_on_action(
                action,
                diag_label,
                ValidationError::NativeActionSignatureConflict {
                    name: action.native_action_name.clone(),
                    detail: format!(
                        "argument types ({}) here disagree with ({}) at another call site",
                        sig.join(", "),
                        prev.join(", "),
                    ),
                },
            );
            // The call that fixed the signature first, which this one
            // contradicts: the consumer reads it to decide which to change.
            Err(match first_at {
                Some(at) => refusal.related_to(RelatedSite {
                    role: RelatedRole::ConflictingUse,
                    location: SourceLocation {
                        file: diag_label.to_string(),
                        line: at.line,
                        col: at.col,
                    },
                    actual: Some(action.native_action_name.clone()),
                }),
                None => refusal,
            })
        }
        Some(_) => Ok(()),
        None => {
            signatures.insert(
                action.native_action_name.clone(),
                (sig, action.source_location.clone()),
            );
            Ok(())
        }
    }
}

/// Validate one native action's arguments and return its signature — the
/// ordered list of canonical argument type names — for the document-wide
/// consistency check. A no-argument action returns an empty signature.
fn validate_args(
    action: &Action,
    schema: Option<&EventSchemaModel>,
    event: &str,
    diag_label: &str,
) -> Result<Vec<String>, Located<ForgeError>> {
    // A no-argument native action (e.g. `reset_slot()`) needs no payload and
    // imposes no schema requirement — it lowers to a bare trait call.
    if action.params.is_empty() {
        return Ok(Vec::new());
    }

    let Some(schema) = schema else {
        return Err(argument_err(
            action,
            diag_label,
            format!(
                "has arguments but its triggering event '{event}' imports no EventSchema, \
                 so the argument types cannot be resolved"
            ),
        ));
    };

    // The generated payload struct emits every schema field; an enum-typed
    // field would name an out-of-scope alias. This is the same eligibility
    // rule the typed-guard channel enforces — keep them in lockstep.
    if !schema_is_native_payload_eligible(schema) {
        return Err(argument_err(
            action,
            diag_label,
            format!(
                "event '{event}' carries an enum-typed EventSchema field; native action \
                 arguments require an all-primitive payload schema"
            ),
        ));
    }

    let mut sig = Vec::with_capacity(action.params.len());
    for arg in &action.params {
        let Some(field_name) = arg_field(&arg.expr) else {
            return Err(one_argument_err(
                action,
                arg,
                diag_label,
                format!(
                    "argument '{}' must be a bare `_event.data.<field>` reference \
                     (literal and derived arguments are not supported)",
                    arg.expr
                ),
            ));
        };
        let Some(field) = schema.fields.iter().find(|f| f.id == field_name) else {
            let mut candidates: Vec<String> = schema.fields.iter().map(|f| f.id.clone()).collect();
            candidates.sort();
            candidates.dedup();
            // On the `<sce:arg>` that names the field, not on the action.
            return Err(located_at(
                arg.source_location
                    .as_ref()
                    .or(action.source_location.as_ref()),
                diag_label,
                ValidationError::CrossKindFieldNotFound {
                    importing_kind: ForgeKind::Statechart,
                    importing_name: action.native_action_name.clone(),
                    alias: "_event.data".to_string(),
                    field: field_name.to_string(),
                    imported_kind: ForgeKind::EventSchema,
                    imported_name: schema.name.clone(),
                    candidates,
                },
            ));
        };
        sig.push(canonical_type(&field.sce_type).to_string());
    }
    Ok(sig)
}

/// Per-backend artifacts produced by [`render`].
pub struct NativeActions {
    /// The full host-interface definition — a Rust `trait`, a Go or Kotlin
    /// `interface`, a C++ abstract struct, a Python `Protocol`, or a C11
    /// function-pointer vtable — or empty when the document declares no
    /// native actions.
    pub interface_def: String,
    /// The interface's name (`<Machine>Actions`, or `<machine>_actions_t` in
    /// C11), or empty when there are no native actions.
    pub interface_name: String,
    /// Events whose payload variant must exist because a native action reads
    /// one of their typed fields. Unioned into the payload-channel event set
    /// by every `build_*_event_payload` in [`crate::forge::generator`].
    pub payload_events: BTreeSet<String>,
    /// Whether any native action exists. Drives the host seam each backend
    /// opens for it: Rust's generic `Policy<A>`, the constructor parameter the
    /// other hosted backends take, the C11 `_init_with_actions` entry.
    pub any: bool,
    /// Every operation the interface declares, in the target language's
    /// spelling: the acts, then the operations that answer a child's host, each
    /// group in the order of its symbolic names.
    ///
    /// The five hosted backends get their "the host supplied it" guarantee
    /// from the type system — an unimplemented interface method does not
    /// compile. C has no such check, so `_init_with_actions` walks this list
    /// and refuses a vtable with a NULL member, which is the same guarantee
    /// moved to the one call a C host has to make. Emitted rather than
    /// re-derived in the template so the names cannot drift from the ones
    /// [`build_interface`] declared.
    pub operation_names: Vec<String>,
    /// Rust's `Recording<Interface>` ([`rust_recording_host`]), or empty for
    /// every other backend and for a document with no native action. Kept
    /// out of [`Self::interface_def`] because it holds its calls in a `Vec`:
    /// the template emits it only for a profile with `alloc`.
    pub recording_def: String,
}

/// One host operation's signature: the ordered `(parameter name, declared
/// type)` pairs its generated interface method takes.
///
/// The declared [`SceType`] is kept rather than a rendered per-language type
/// string, because one `SceType` does not always map to one parameter — C11
/// lowers `bytes` to a `const uint8_t *` plus its `size_t` length sibling, a
/// pair no single type string can express. The per-language expansion happens
/// once, at emit, in [`params_for`] and [`args_for`], which is also the one
/// place their arities are kept equal.
type Signature = Vec<(String, SceType)>;

/// Per-transition payload context for lowering an arg-bearing `<sce:action>`.
struct PayloadBinding<'a> {
    /// Triggering event name. Added to the payload-event union when the action
    /// reads a typed field, and spelled per backend at emit — the payload
    /// channel calls the same event `FragmentReceived` in Rust, Go, C++ and
    /// Kotlin and `fragment_received` in C11 and Python.
    event: &'a str,
    /// The triggering event's imported EventSchema (guaranteed present for an
    /// arg-bearing action by [`validate`]).
    schema: Option<&'a EventSchemaModel>,
}

/// The host-facing spelling of an operation name, in the target language's own
/// convention.
///
/// `<sce:action name="…">` keeps the operation SYMBOLIC and language-neutral —
/// that is the whole point of the construct — so each backend spells the same
/// symbol the way a host author writing in that language would. Go's arm is
/// not a style preference: a method the consumer package implements has to be
/// EXPORTED, so the identifier must be upper-camel there or the interface
/// cannot be implemented from outside the generated package at all.
fn method_name(lang: Language, name: &str) -> String {
    match lang {
        Language::Rust | Language::Python | Language::C11 => {
            filters::to_snake_case(name.to_string())
        }
        Language::Go => filters::to_pascal_case(name.to_string()),
        Language::Cpp | Language::Kotlin => filters::to_camel_case(name.to_string()),
    }
}

/// A parameter's identifier, in the target language's convention.
fn param_ident(lang: Language, name: &str) -> String {
    match lang {
        Language::Rust | Language::Python | Language::C11 => {
            filters::to_snake_case(name.to_string())
        }
        Language::Go | Language::Kotlin | Language::Cpp => filters::to_camel_case(name.to_string()),
    }
}

/// How the emitted code reaches the host object carrying the operations.
///
/// Every backend already runs its executable content from ONE scope — a policy
/// method in five of them, an `sm`-taking function in C11 — so this is that
/// scope's spelling of the interface member rather than a new concept. The C11
/// form is the struct member; its `user_data` companion is threaded separately
/// by [`call`], because a function pointer has no receiver to bind it to.
fn receiver(lang: Language) -> &'static str {
    match lang {
        Language::Rust => "self.actions.",
        Language::Go => "p.actions.",
        Language::Cpp => "actions_->",
        Language::Kotlin => "actions.",
        Language::Python => "self._actions.",
        Language::C11 => "sm->actions.",
    }
}

/// The statement calling host operation `name` with `args`, terminated the way
/// the target language terminates a statement.
///
/// C11 passes `user_data` first: a function pointer carries no receiver, so the
/// vtable hands the host its own state back on every call — C's answer to the
/// `&mut self` the other five bind for free.
fn call(lang: Language, name: &str, args: &[String]) -> String {
    let expression = call_expression(lang, name, args);
    match lang {
        Language::Rust | Language::Cpp | Language::C11 => format!("{expression};"),
        Language::Go | Language::Kotlin | Language::Python => expression,
    }
}

/// [`call`] without its terminator: the value a call answers, where a call
/// that answers one (a child's host) is an operand and not a statement.
fn call_expression(lang: Language, name: &str, args: &[String]) -> String {
    let method = method_name(lang, name);
    let recv = receiver(lang);
    let mut all: Vec<String> = Vec::new();
    if matches!(lang, Language::C11) {
        all.push("sm->actions.user_data".to_string());
    }
    all.extend(args.iter().cloned());
    format!("{recv}{method}({})", all.join(", "))
}

/// The parameter declarations one schema field contributes to a generated
/// interface method.
///
/// Usually one; C11's `bytes` contributes TWO — the pointer and the length that
/// make a byte run readable at all in C. Keeping that expansion here (and its
/// mirror in [`args_for`]) is what lets the signature table stay in declared
/// `SceType`s while each backend spells them its own way.
fn params_for(lang: Language, pname: &str, ty: &SceType) -> Vec<String> {
    let ident = param_ident(lang, pname);
    match (lang, ty) {
        (Language::C11, SceType::Bytes) => vec![
            format!("const uint8_t *{ident}"),
            format!("size_t {ident}_len"),
        ],
        (Language::Rust | Language::Kotlin | Language::Python, _) => {
            vec![format!("{ident}: {}", host_param_type(lang, ty))]
        }
        (Language::Go, _) => vec![format!("{ident} {}", host_param_type(lang, ty))],
        (Language::Cpp | Language::C11, _) => {
            vec![format!("{} {ident}", host_param_type(lang, ty))]
        }
    }
}

/// The call arguments one schema field contributes, read off `accessor` — the
/// backend's spelling of the bound typed payload. Mirrors [`params_for`]: the
/// two must expand each `SceType` to the same arity, or the emitted call does
/// not compile.
fn args_for(lang: Language, accessor: &str, field: &str, ty: &SceType) -> Vec<String> {
    match (lang, ty) {
        (Language::C11, SceType::Bytes) => vec![
            format!("{accessor}.{field}"),
            format!("{accessor}.{field}_len"),
        ],
        // Rust's owned payload fields (`SceBytes<CAP>` / `SceString`) deref to
        // `[u8]` / `str`, so a borrow is what the `&[u8]` / `&str` parameter
        // takes.
        (Language::Rust, SceType::String | SceType::Bytes) => vec![format!("&{accessor}.{field}")],
        _ => vec![format!("{accessor}.{field}")],
    }
}

/// The generated host interface's name: `<Machine>Actions` wherever a type is
/// PascalCase, the `_t`-suffixed snake form C uses for a typedef.
pub fn interface_name(lang: Language, machine_name: &str) -> String {
    match lang {
        Language::C11 => format!("{machine_name}_actions_t"),
        _ => format!("{machine_name}Actions"),
    }
}

/// Lower every `<sce:action>` on `model` to its `lang` call site, storing the
/// rendered code on `Action::native_action_rendered`, and return the host
/// interface definition + payload-event union.
///
/// Visits both `<transition>` actions and eventless executable content
/// (`<onentry>`/`<onexit>`/initial). Assumes [`validate`] already passed, so an
/// arg-bearing action is always a transition child with a resolved,
/// payload-eligible `_event.data.<field>` schema (the `expect`s in
/// [`CallRendering::lower`] are therefore total). `model` is the per-backend
/// codegen clone, never the parsed model.
///
/// `machine_name` is the caller's per-language machine token: the PascalCase
/// stem for the five hosted backends (matching what their
/// `build_*_event_payload` twins already use), the raw snake stem for C11.
pub fn render(model: &mut SCXMLModel, machine_name: &str, lang: Language) -> NativeActions {
    render_with_symbol_prefix(model, machine_name, lang, "")
}

/// [`render`] for a backend whose machine symbols carry a suite prefix that its
/// payload channel's types do not — C11's `--c-symbol-prefix`, which nests
/// `<prefix>_<machine>_raise_platform_error` and the `<PREFIX>_<MACHINE>_EVENT_*`
/// constants, while the channel's tag constants stay `<MACHINE>_PAYLOAD_*`. A
/// call site that raises `error.execution` names the first; `machine_name`
/// names the second. `""` for a backend with no such prefix.
pub fn render_with_symbol_prefix(
    model: &mut SCXMLModel,
    machine_name: &str,
    lang: Language,
    symbol_prefix: &str,
) -> NativeActions {
    render_with_options(model, machine_name, lang, symbol_prefix, "")
}

/// [`render`] for a backend that nests each machine's namespace under a suite
/// prefix — C++'s `SCE::Generated::<prefix>::<name>`. A parent's host interface
/// names the interface of a child's host, which lives in the child's namespace,
/// so the prefix is what spells that name (`namespace_prefix` is the
/// ready-to-prepend `<prefix>::` segment, `""` when unset).
pub fn render_in_namespace(
    model: &mut SCXMLModel,
    machine_name: &str,
    lang: Language,
    namespace_prefix: &str,
) -> NativeActions {
    render_with_options(model, machine_name, lang, "", namespace_prefix)
}

fn render_with_options(
    model: &mut SCXMLModel,
    machine_name: &str,
    lang: Language,
    symbol_prefix: &str,
    namespace_prefix: &str,
) -> NativeActions {
    let schemas = model.imported_event_schemas.clone();
    // Under `datamodel="sce-static"` an argument is a typed expression over the
    // document's scope, lowered by the static lowering (SCE Accepted
    // Subset §2.15). Only a backend that lowers the model reaches here with
    // such a document; the others refused it before rendering.
    let static_scope = crate::forge::type_ctx::StaticScope::of(model);
    // Read once, before the walk borrows `model` mutably. Every other raise
    // site in this generator is written under the same condition — a document
    // that declares no `error.execution` has no enum variant to name, and an
    // event nothing can match would be discarded on arrival anyway.
    let raises_error = model.events.contains("error.execution");

    // A child that declares acts is built with a host its parent's own host
    // answers (docs/adr/0005, decision 6), so the parent has a host to take one
    // from even when it declares no act itself. Read before the walk below
    // borrows `model` for the actions, and written on each invoke so the
    // template that starts the child reads the call and not a name.
    let child_hosts = lower_child_hosts(
        model,
        lang,
        namespace_prefix,
        &interface_name(lang, machine_name),
    );
    let mut calls = CallRendering {
        lang,
        machine_name,
        symbol_prefix,
        raises_error,
        static_scope: static_scope.as_ref(),
        sigs: BTreeMap::new(),
        payload_events: BTreeSet::new(),
    };
    let mut any = !child_hosts.is_empty();

    for state in model.states.values_mut() {
        // Eventless positions: every native action here is no-argument
        // (enforced by `validate`), so it lowers to a bare host call.
        let eventless = state
            .on_entry_blocks
            .iter_mut()
            .flatten()
            .chain(state.on_exit_blocks.iter_mut().flatten())
            .chain(state.initial_transition_actions.iter_mut())
            .chain(state.initial_history_default_actions.iter_mut());
        for action in eventless {
            if !is_native(action) {
                continue;
            }
            calls.lower(action, None);
            any = true;
        }

        for transition in &mut state.transitions {
            let event = transition.event.clone();
            let binding = PayloadBinding {
                event: &event,
                schema: schemas.get(&event),
            };
            for action in &mut transition.actions {
                if !is_native(action) {
                    continue;
                }
                calls.lower(action, Some(&binding));
                any = true;
            }
        }
    }

    let CallRendering {
        sigs,
        payload_events,
        ..
    } = calls;
    let name = interface_name(lang, machine_name);
    let operation_names: Vec<String> = sigs
        .keys()
        .map(|n| method_name(lang, n))
        .chain(child_hosts.iter().map(|h| method_name(lang, &h.operation)))
        .collect();
    let (interface_def, interface_name) = if any {
        (build_interface(lang, &name, &sigs, &child_hosts), name)
    } else {
        (String::new(), String::new())
    };
    // Kotlin's recording host rides inside its interface definition; Rust's is
    // apart, because it holds its calls in a `Vec` and the template emits it
    // only where `alloc` is.
    let recording_def = if any && lang == Language::Rust {
        rust_recording_host(&interface_name, &sigs, &child_hosts)
    } else {
        String::new()
    };

    NativeActions {
        interface_def,
        interface_name,
        payload_events,
        any,
        operation_names,
        recording_def,
    }
}

/// One operation of a parent's host interface that answers the host of a child:
/// its symbolic name ([`host_operation`]) and the interface of the child it
/// answers one for.
struct ChildHost {
    operation: String,
    child_interface: String,
}

impl ChildHost {
    /// The associated type a Rust trait declares for the host this operation
    /// answers (`ActionsForWorker`), which is also the name of the operation in
    /// the casing a type has.
    fn associated_type(&self) -> String {
        filters::to_pascal_case(self.operation.clone())
    }
}

/// The machine token a generated child's symbols carry in `lang` — the one
/// [`interface_name`] is built from, as [`render`] is handed it for the child's
/// own document: PascalCase for the five hosted backends, the raw name for C11.
fn machine_token(lang: Language, child_name: &str) -> String {
    match lang {
        Language::C11 => child_name.to_string(),
        _ => filters::to_pascal_case(child_name.to_string()),
    }
}

/// The name of the interface of the host a child is built with, as the parent's
/// interface spells it: the child's own interface name, and — where a machine
/// lives in a namespace of its own, as C++'s does — qualified by that namespace,
/// which a parent in another one cannot reach by the bare name.
fn child_interface_type(lang: Language, child_name: &str, namespace_prefix: &str) -> String {
    let bare = interface_name(lang, &machine_token(lang, child_name));
    match lang {
        Language::Cpp => format!("::SCE::Generated::{namespace_prefix}{child_name}::{bare}"),
        // A sibling module of the parent's, as the machine names the child it
        // starts (`super::<child>_sm::<Child>Policy`).
        Language::Rust => format!("super::{child_name}_sm::{bare}"),
        _ => bare,
    }
}

/// Write on every invoke of `model` whose child declares acts the call that
/// answers the child's host ([`InvokeSessionCommon::child_host_call`]), and
/// return the operations the parent's interface declares for them, sorted and
/// one per name (a name collision is refused earlier, by [`validate`]).
///
/// [`InvokeSessionCommon::child_host_call`]: crate::model::InvokeSessionCommon::child_host_call
fn lower_child_hosts(
    model: &mut SCXMLModel,
    lang: Language,
    namespace_prefix: &str,
    parent_interface: &str,
) -> Vec<ChildHost> {
    let mut hosts: BTreeMap<String, ChildHost> = BTreeMap::new();
    // The call, and the type it answers where the child is generic over its host.
    let mut declare = |operation: String, child_name: &str| -> (String, String) {
        let call = call_expression(lang, &operation, &[]);
        let host = ChildHost {
            child_interface: child_interface_type(lang, child_name, namespace_prefix),
            operation: operation.clone(),
        };
        let host_type = match lang {
            Language::Rust => format!("<A as {parent_interface}>::{}", host.associated_type()),
            _ => String::new(),
        };
        hosts.entry(operation).or_insert(host);
        (call, host_type)
    };
    for state in model.states.values_mut() {
        for invoke in &mut state.invokes {
            match invoke {
                crate::model::Invoke::Scxml(info) if info.common.child_declares_host_acts => {
                    let operation = host_operation(&info.common.base.field_suffix, None);
                    (info.common.child_host_call, info.common.child_host_type) =
                        declare(operation, &info.common.child_name);
                }
                crate::model::Invoke::Hybrid(info) => {
                    let suffix = info.common.base.field_suffix.clone();
                    for candidate in info
                        .candidates
                        .iter_mut()
                        .filter(|c| c.child_declares_host_acts)
                    {
                        let operation = host_operation(&suffix, Some(&candidate.stem));
                        (candidate.child_host_call, candidate.child_host_type) =
                            declare(operation, &candidate.stem);
                    }
                }
                _ => {}
            }
        }
    }
    // Templates read an invoke from two places: the state that holds it, which was
    // written above, and the document's own list of them ([`SCXMLModel::invokes`]),
    // which the field declarations of a machine walk. The second is a view rebuilt
    // from the first, so it is rebuilt.
    model.refresh_invokes_view();
    hosts.into_values().collect()
}

/// One [`render`] pass over a document's native actions: what every call site
/// reads — the backend, its machine token, whether the document can raise
/// `error.execution`, and the `sce-static` scope when there is one — and the
/// two tables every call site adds to.
struct CallRendering<'r> {
    lang: Language,
    machine_name: &'r str,
    /// The prefix the machine's own symbols carry beyond [`Self::machine_name`]
    /// ([`render_with_symbol_prefix`]); empty for every backend but C11's.
    symbol_prefix: &'r str,
    /// Reaches [`guard_payload`] unchanged — it decides whether the arm an
    /// untyped delivery takes says so with `error.execution` or stays empty.
    raises_error: bool,
    /// The document's scope under `datamodel="sce-static"`; `None` under
    /// any other data model.
    static_scope: Option<&'r crate::forge::type_ctx::StaticScope>,
    /// Signatures keyed by action name; the first occurrence defines the
    /// signature and `validate` has already proven every later occurrence
    /// agrees, so a single interface method serves every call site. A
    /// no-argument action (the only kind admissible in an eventless position
    /// outside `sce-static`) registers an empty signature, which still emits
    /// its method.
    sigs: BTreeMap<String, Signature>,
    /// The events whose typed payload some call site reads.
    payload_events: BTreeSet<String>,
}

impl CallRendering<'_> {
    /// Lower one `<sce:action>` to its call site (stored on
    /// `action.native_action_rendered`) and fold its signature into
    /// [`Self::sigs`].
    ///
    /// `binding` is `Some` only for a `<transition>` child, where the
    /// triggering event's typed payload is in scope; `None` for an eventless
    /// position (`<onentry>`/`<onexit>`/initial), where the action is
    /// necessarily no-argument outside `sce-static`. A no-argument action
    /// lowers to a bare host call in either case; an arg-bearing one reads its
    /// values from the event's typed payload and is wrapped in that backend's
    /// tag check.
    fn lower(&mut self, action: &mut Action, binding: Option<&PayloadBinding>) {
        let lang = self.lang;
        let name = action.native_action_name.clone();

        if action.params.is_empty() {
            self.sigs.entry(name.clone()).or_default();
            action.native_action_rendered = call(lang, &name, &[]);
            return;
        }

        // `datamodel="sce-static"`: each argument is a typed expression,
        // lowered by the static lowering and typed by the value it is. Only
        // the backends that lower the model reach here; every other one
        // refused the document before rendering.
        if let Some(static_scope) = self.static_scope {
            let event = binding.and_then(|b| b.schema.map(|schema| (b.event, schema)));
            let mut call_args = Vec::new();
            let mut params: Signature = Vec::new();
            let mut reads_payload = false;
            let mut can_fail = false;
            for (i, arg) in action.params.iter().enumerate() {
                let lowered = crate::forge::static_lowering::lower_static_argument(
                    static_scope,
                    event,
                    arg,
                    lang,
                )
                .expect("validated: a sce-static argument is typed against this scope");
                let pname = if arg.name.is_empty() {
                    format!("arg{}", i + 1)
                } else {
                    arg.name.clone()
                };
                reads_payload |= lowered.reads_payload;
                can_fail |= lowered.can_fail;
                call_args.push(lowered.text);
                params.push((pname, lowered.ty));
            }
            // The signature keeps the first call site's; the types the arguments
            // are held in while a failing one is received are this site's own.
            let typed_args: Vec<(String, SceType)> = call_args
                .iter()
                .cloned()
                .zip(params.iter().map(|(_, ty)| ty.clone()))
                .collect();
            self.sigs.entry(name.clone()).or_insert(params);
            let stmt = call(lang, &name, &call_args);
            // E12 D5: an argument that can fail stops the call before it is
            // made, and `error.execution` names it.
            let stmt = if can_fail {
                crate::forge::static_lowering::receive_static_statement(
                    lang,
                    &stmt,
                    &format!("{}{}", receiver(lang), method_name(lang, &name)),
                    &typed_args,
                    &format!("{}{}", self.symbol_prefix, self.machine_name),
                    self.raises_error,
                    &format!("<sce:action name='{name}'>"),
                )
                .expect("the backend lowers sce-static: it lowered the arguments")
            } else {
                stmt
            };
            action.native_action_rendered = match (reads_payload, binding) {
                (true, Some(binding)) => {
                    self.payload_events.insert(binding.event.to_string());
                    guard_payload(
                        lang,
                        self.machine_name,
                        self.symbol_prefix,
                        binding.event,
                        &name,
                        &stmt,
                        self.raises_error,
                    )
                }
                _ => stmt,
            };
            return;
        }

        // Arg-bearing: `validate` guarantees a transition binding with a
        // resolved, payload-eligible schema, so the lookups below are total.
        let binding =
            binding.expect("validated: arg-bearing native action is a <transition> child");
        let schema = binding
            .schema
            .expect("validated: arg-bearing native action has a schema");

        let accessor = payload_accessor(lang, binding.event);
        let mut call_args: Vec<String> = Vec::new();
        let mut params: Signature = Vec::new();
        for arg in &action.params {
            let field = arg_field(&arg.expr).expect("validated: bare _event.data field");
            let f = schema
                .fields
                .iter()
                .find(|f| f.id == field)
                .expect("validated: field exists on schema");
            let pname = if arg.name.is_empty() {
                field.to_string()
            } else {
                arg.name.clone()
            };
            call_args.extend(args_for(lang, &accessor, field, &f.sce_type));
            params.push((pname, f.sce_type.clone()));
        }

        self.payload_events.insert(binding.event.to_string());
        self.sigs.entry(name.clone()).or_insert(params);

        let stmt = call(lang, &name, &call_args);
        action.native_action_rendered = guard_payload(
            lang,
            self.machine_name,
            self.symbol_prefix,
            binding.event,
            &name,
            &stmt,
            self.raises_error,
        );
    }
}

/// The backend's spelling of the bound typed payload a native action reads its
/// arguments from.
///
/// Each one is the SAME field the backend's `build_*_event_payload` twin
/// already fills on the populate seam and reads in a native transition guard.
/// The two channels read one payload, so a native action can never see a value
/// a guard on the same event could not.
fn payload_accessor(lang: Language, event: &str) -> String {
    let variant = filters::to_event_variant(event.to_string());
    match lang {
        // Bound by the payload-sum `match` arm emitted around the call.
        Language::Rust => "ev".to_string(),
        Language::Go => format!("p.pending{variant}Payload"),
        Language::Cpp => format!("pending{variant}Payload_"),
        // Bound by the `?.let` / walrus binding emitted around the call.
        Language::Kotlin => "it".to_string(),
        Language::Python => "_p".to_string(),
        Language::C11 => format!(
            "sm->pending_payload.as.{}",
            event.replace(['.', '-'], "_").to_lowercase()
        ),
    }
}

/// Wrap `stmt` in the check that proves the bound typed payload is the one
/// this action reads from, and say so when it is not there.
///
/// An event that arrives without its typed payload cannot supply the
/// arguments. Two things can produce one, and only one of them is a host
/// mistake: a host reaching for `raise_<event>_by_name` instead of the
/// generated typed inject, and the DOCUMENT'S OWN `<raise event="…"/>` of a
/// payload-typed event — legal SCXML that this generator accepts. So the
/// answer cannot be "blame the caller". The spec already names one, cited in
/// the body below: the failure is signalled as `error.execution` on the
/// internal event queue, which the document can answer with a transition of
/// its own.
///
/// That is what every backend emits here, in its own convention — one
/// behaviour rather than six. It replaces two wrong answers measured on
/// 2026-08-24 against one document: five backends skipped in silence, and Rust
/// alone aborted the process through a `debug_assert!` that a release build
/// compiled away, so the same legal document either killed a development build
/// or did nothing at all depending on the profile.
///
/// `raises_error` is `false` for a document that declares no `error.execution`
/// event; there is then no enum variant to name and nothing could match the
/// event anyway, so the arm stays empty. That is the same
/// `'error.execution' in model.events` condition every other raise site in
/// this generator is written under.
fn guard_payload(
    lang: Language,
    machine_name: &str,
    symbol_prefix: &str,
    event: &str,
    action_name: &str,
    stmt: &str,
    raises_error: bool,
) -> String {
    let variant = filters::to_event_variant(event.to_string());
    // §scxml-3.12.2: `error.execution` is the processor's own signal for errors
    // "internal to the execution of the document, such as those arising from
    // expression evaluation", and it MUST go on the internal event queue — where
    // a transition can answer it, or nothing can and it is discarded. An
    // argument the delivery cannot supply is exactly such an error, which is why
    // neither silence nor a process abort is available here. Cited in the body
    // rather than the doc comment because the ledger's Rust resolver binds a
    // citation to the symbol enclosing it, and a `///` line encloses nothing.
    //
    // One message for all six. A per-backend wording is how a single contract
    // turns back into six, which is the drift this lowering exists to prevent.
    let msg = format!(
        "<sce:action name='{action_name}'> needs the typed payload of \
         '{event}', which this delivery did not carry"
    );
    match lang {
        Language::Rust => {
            let enum_name = format!("{machine_name}Payload");
            let otherwise = if raises_error {
                format!(
                    "engine.raise(sce_rust_runtime::EventWithMetadata::platform_error(\
                     {machine_name}Event::ErrorExecution, {msg:?}));"
                )
            } else {
                String::new()
            };
            format!(
                "match &self.pending_payload {{\n            \
                 {enum_name}::{variant}(ev) => {{ {stmt} }}\n            \
                 _ => {{ {otherwise} }}\n        }}"
            )
        }
        Language::Go => {
            let otherwise = if raises_error {
                format!(
                    " else {{\n\t\tengine.Raise(sce.NewPlatformError(\
                     {machine_name}EventErrorExecution, \"{msg}\"))\n\t}}"
                )
            } else {
                String::new()
            };
            format!(
                "if p.pendingPayloadTag == {machine_name}PayloadTag{variant} \
                 {{\n\t\t{stmt}\n\t}}{otherwise}"
            )
        }
        Language::Cpp => {
            let otherwise = if raises_error {
                format!(
                    " else {{\n    engine.raise(typename Engine::EventWithMetadata(\
                     Event::Error_execution, \"{msg}\"));\n}}"
                )
            } else {
                String::new()
            };
            format!(
                "if (pendingPayloadTag_ == {machine_name}PayloadTag::{variant}) \
                 {{\n    {stmt}\n}}{otherwise}"
            )
        }
        Language::Kotlin => {
            let otherwise = if raises_error {
                format!(
                    " ?: run {{ raiseInternal({machine_name}Event.Error.Execution, \
                     EventMetadata(data = \"{msg}\", type = \"platform\")) }}"
                )
            } else {
                String::new()
            };
            format!("pending{variant}Payload?.let {{ {stmt} }}{otherwise}")
        }
        Language::Python => {
            let snake = filters::to_snake_case(event.to_string());
            // A statement of more than one line — the `try` block that receives
            // a failing argument of a `sce-static` machine's call — cannot sit
            // in the one-line forms below, so it is a block of its own. Its
            // arguments read the payload through the attribute the machine's
            // lowering named, not through the walrus binding, so none is made.
            if stmt.contains('\n') {
                let body = stmt
                    .lines()
                    .map(|line| format!("    {line}"))
                    .collect::<Vec<_>>()
                    .join("\n");
                let otherwise = if raises_error {
                    format!("\nelse:\n    self._raise_error_execution(engine, \"{msg}\")")
                } else {
                    String::new()
                };
                return format!("if self._pending_{snake}_payload is not None:\n{body}{otherwise}");
            }
            // ONE line, because this backend's dispatcher hands the call site a
            // literal indent prefix that only reaches the first one. The
            // conditional EXPRESSION keeps both arms on it, and evaluates its
            // condition first, so the walrus still binds before `stmt` runs.
            if raises_error {
                // `_raise_error_execution` is this backend's single raise site
                // (emitted unconditionally); calling it is what keeps
                // `_event.type` and `_event.data` filled the same way here as
                // everywhere else Python signals a platform error.
                format!(
                    "{stmt} if (_p := self._pending_{snake}_payload) is not None \
                     else self._raise_error_execution(engine, \"{msg}\")"
                )
            } else {
                format!("if (_p := self._pending_{snake}_payload) is not None: {stmt}")
            }
        }
        Language::C11 => {
            let upper = machine_name.to_uppercase();
            let token = event.replace(['.', '-'], "_").to_uppercase();
            // `_raise_platform_error` is emitted unconditionally by this
            // backend's header and its own comment calls itself "the single way
            // generated code raises a platform error" — hand-rolling the
            // carrier here is exactly the drift it exists to absorb, and would
            // leave `_event.type` and `_event.data` empty.
            //
            // The machine's own symbols carry the suite prefix (the function
            // and the event constant), the payload channel's tag does not.
            let otherwise = if raises_error {
                let symbol_upper = symbol_prefix.to_uppercase();
                format!(
                    " else {{\n    {symbol_prefix}{machine_name}_raise_platform_error(\
                     sm, {symbol_upper}{upper}_EVENT_ERROR_EXECUTION, \"{msg}\");\n}}"
                )
            } else {
                String::new()
            };
            format!(
                "if (sm->pending_payload.tag == {upper}_PAYLOAD_{token}) \
                 {{\n    {stmt}\n}}{otherwise}"
            )
        }
    }
}

/// Emit the host interface every `<sce:action>` in the document dispatches
/// through, in the target language's own expression of "an interface".
///
/// The construct is engine-free BY DEFINITION, so each of these is a direct
/// call surface rather than a registry lookup: a Rust trait bound on the
/// policy, a Go or Kotlin interface, a C++ abstract base, a Python `Protocol`,
/// a C11 struct of function pointers. Rust's is the only one that makes "the
/// host supplied it" a compile-time fact for free; the other five take the
/// interface where the machine is constructed, which puts the same guarantee
/// at the one call every host has to make anyway.
///
/// After the acts come the operations that answer a child's host
/// ([`ChildHost`]), each returning the interface of the child it answers for.
/// They are declared beside the acts because the host a parent already has is
/// where it obtains the host of a child (docs/adr/0005, decision 6).
fn build_interface(
    lang: Language,
    interface_name: &str,
    sigs: &BTreeMap<String, Signature>,
    children: &[ChildHost],
) -> String {
    const DOC: [&str; 3] = [
        "W3C SCXML G.7: host operations dispatched by `<sce:action>`.",
        "The host supplies the side effects while the statechart keeps each",
        "operation symbolic. No runtime script engine is involved.",
    ];
    let doc = |prefix: &str| -> String {
        DOC.iter()
            .map(|l| format!("{prefix}{l}\n"))
            .collect::<Vec<_>>()
            .join("")
    };
    let plist = |sig: &Signature| -> Vec<String> {
        sig.iter()
            .flat_map(|(n, t)| params_for(lang, n, t))
            .collect()
    };

    let mut methods = String::new();
    match lang {
        Language::Rust => {
            for (name, sig) in sigs {
                let params = plist(sig).join(", ");
                let sep = if params.is_empty() { "" } else { ", " };
                methods.push_str(&format!(
                    "    fn {}(&mut self{sep}{params});\n",
                    method_name(lang, name)
                ));
            }
            // The host of a child is a type the parent's host chooses, since the child's
            // machine is generic over it (`ChildPolicy<A>`): an associated type bounded
            // by the child's own trait, and an operation that answers a value of it.
            // The bound is `'static` because `Engine` stores the policy and erases its
            // lifetime.
            let mut associated = String::new();
            for child in children {
                associated.push_str(&format!(
                    "    type {}: {} + 'static;\n",
                    child.associated_type(),
                    child.child_interface
                ));
                methods.push_str(&format!(
                    "    fn {}(&mut self) -> Self::{};\n",
                    method_name(lang, &child.operation),
                    child.associated_type()
                ));
            }
            format!(
                "{}pub trait {interface_name} {{\n{associated}{methods}}}\n",
                doc("/// ")
            )
        }
        Language::Go => {
            for (name, sig) in sigs {
                methods.push_str(&format!(
                    "\t{}({})\n",
                    method_name(lang, name),
                    plist(sig).join(", ")
                ));
            }
            // The child's interface is declared beside its machine, in the one package
            // the parent and the children it invokes are generated into.
            for child in children {
                methods.push_str(&format!(
                    "\t{}() {}\n",
                    method_name(lang, &child.operation),
                    child.child_interface
                ));
            }
            format!(
                "{}type {interface_name} interface {{\n{methods}}}\n",
                doc("// ")
            )
        }
        Language::Kotlin => {
            for (name, sig) in sigs {
                methods.push_str(&format!(
                    "    fun {}({})\n",
                    method_name(lang, name),
                    plist(sig).join(", ")
                ));
            }
            for child in children {
                methods.push_str(&format!(
                    "    fun {}(): {}\n",
                    method_name(lang, &child.operation),
                    child.child_interface
                ));
            }
            format!(
                "/**\n{} */\ninterface {interface_name} {{\n{methods}}}\n\n{}",
                doc(" * "),
                kotlin_recording_host(interface_name, sigs, children)
            )
        }
        Language::Python => {
            for (name, sig) in sigs {
                let params = plist(sig).join(", ");
                let sep = if params.is_empty() { "" } else { ", " };
                methods.push_str(&format!(
                    "    def {}(self{sep}{params}) -> None:\n        ...\n\n",
                    method_name(lang, name)
                ));
            }
            // The child's interface is declared by the child's own module, which the
            // machine imports where it starts the child, so the annotation names it
            // as text and nothing here has to import a sibling at load.
            for child in children {
                methods.push_str(&format!(
                    "    def {}(self) -> \"{}\":\n        \"\"\"The host the child is built with, asked each time it starts.\"\"\"\n        ...\n\n",
                    method_name(lang, &child.operation),
                    child.child_interface
                ));
            }
            format!(
                "class {interface_name}(Protocol):\n    \"\"\"\n{}    \"\"\"\n\n{methods}",
                doc("    ")
            )
        }
        Language::Cpp => {
            for (name, sig) in sigs {
                methods.push_str(&format!(
                    "    virtual void {}({}) = 0;\n",
                    method_name(lang, name),
                    plist(sig).join(", ")
                ));
            }
            // A reference to a host the answering host owns, as the machine's own host
            // is: the machine does not own it and its owner outlives the child, which
            // is built with it and never keeps it past its own end.
            for child in children {
                methods.push_str(&format!(
                    "    virtual {}& {}() = 0;\n",
                    child.child_interface,
                    method_name(lang, &child.operation)
                ));
            }
            format!(
                "{}struct {interface_name} {{\n    virtual ~{interface_name}() = default;\n\
                 {methods}}};\n",
                doc("// ")
            )
        }
        Language::C11 => {
            // A struct of function pointers plus the `user_data` C needs to
            // hand a host its own state back — the same shape the runtime's
            // host-processor registry already uses, so a C host meets one
            // convention rather than two.
            for (name, sig) in sigs {
                let mut params = vec!["void *user_data".to_string()];
                params.extend(plist(sig));
                methods.push_str(&format!(
                    "    void (*{})({});\n",
                    method_name(lang, name),
                    params.join(", ")
                ));
            }
            // The table of the host a child is built with, which `_invoked_begin`
            // copies as `_init_with_actions` copies the machine's own: a pointer to
            // a table the answering host owns for the length of the call, NULL
            // for a host that has none to give (the start is then refused).
            for child in children {
                methods.push_str(&format!(
                    "    const {} *(*{})(void *user_data);\n",
                    child.child_interface,
                    method_name(lang, &child.operation)
                ));
            }
            let tag = interface_name.trim_end_matches("_t");
            format!(
                "/*\n{}\n   Every member must be non-NULL before the machine runs: an unset\n   \
                 operation is an act the document declared and nobody performs,\n   \
                 which `_init_with_actions` refuses rather than discovers at the\n   \
                 first entry action. */\ntypedef struct {tag} {{\n{methods}    \
                 /* Handed back unchanged on every call — C's answer to the\n       \
                 receiver the other backends bind. */\n    void *user_data;\n}} \
                 {interface_name};\n",
                doc("   ")
            )
        }
    }
}

/// `Recording<Interface>`: the Kotlin host interface's implementation that
/// performs nothing and records every call, in order, as a value — so a test
/// drives the machine with no host at all: send an event, read what the
/// machine asked the host to do, answer with an event, check the snapshot.
///
/// Generated from the same signatures as the interface, so it cannot fall out
/// of step with it: a new `<sce:action>` is a new recorded call, and a changed
/// argument a changed field. Each call is a `data class` (a `data object` for
/// a call with no argument), so a test compares whole calls by value.
///
/// A host that answers a child's host cannot invent one, so the recording host
/// of such a parent takes one source per child, a function the test gives, and
/// records each question beside the acts. The test keeps what the source
/// answers, which is where it reads what the child was asked to do.
fn kotlin_recording_host(
    interface_name: &str,
    sigs: &BTreeMap<String, Signature>,
    children: &[ChildHost],
) -> String {
    let lang = Language::Kotlin;
    let mut variants = String::new();
    let mut overrides = String::new();
    let mut sources = String::new();
    for child in children {
        let method = method_name(lang, &child.operation);
        let variant = filters::to_pascal_case(method.clone());
        let source = format!("{method}Source");
        sources.push_str(&format!(
            "    /** Answers the host [{}] is asked for; the test keeps what it answers. */\n    \
             private val {source}: () -> {},\n",
            child.child_interface, child.child_interface
        ));
        variants.push_str(&format!("        data object {variant} : Call\n"));
        overrides.push_str(&format!(
            "    override fun {method}(): {} {{\n        recorded += Call.{variant}\n        \
             return {source}()\n    }}\n",
            child.child_interface
        ));
    }
    let constructor = if sources.is_empty() {
        String::new()
    } else {
        format!("(\n{sources})")
    };
    for (name, sig) in sigs {
        let method = method_name(lang, name);
        let variant = filters::to_pascal_case(method.clone());
        let params: Vec<String> = sig
            .iter()
            .flat_map(|(n, t)| params_for(lang, n, t))
            .collect();
        // A `bytes` argument arrives as a `ByteArray`, which a data class
        // compares by reference; it is recorded as a copy in a `List<Byte>`,
        // which compares by value, so the promise above holds for it too.
        let fields: Vec<String> = sig
            .iter()
            .map(|(n, t)| match t {
                SceType::Bytes => format!("val {}: List<Byte>", param_ident(lang, n)),
                _ => format!("val {}: {}", param_ident(lang, n), host_param_type(lang, t)),
            })
            .collect();
        let args: Vec<String> = sig
            .iter()
            .map(|(n, t)| match t {
                SceType::Bytes => format!("{}.toList()", param_ident(lang, n)),
                _ => param_ident(lang, n),
            })
            .collect();
        if params.is_empty() {
            variants.push_str(&format!("        data object {variant} : Call\n"));
            overrides.push_str(&format!(
                "    override fun {method}() {{\n        recorded += Call.{variant}\n    }}\n"
            ));
        } else {
            variants.push_str(&format!(
                "        data class {variant}({}) : Call\n",
                fields.join(", ")
            ));
            overrides.push_str(&format!(
                "    override fun {method}({}) {{\n        recorded += Call.{variant}({})\n    }}\n",
                params.join(", "),
                args.join(", ")
            ));
        }
    }
    format!(
        "/**\n * [{interface_name}] that performs nothing and records every call in order —\n \
         * the host a test drives the machine with. Read [calls] after the machine\n \
         * has run; each call is compared by value.\n */\n\
         class Recording{interface_name}{constructor} : {interface_name} {{\n    \
         /** One recorded host call. */\n    sealed interface Call {{\n{variants}    }}\n\n    \
         private val recorded = mutableListOf<Call>()\n\n    \
         /** Every call so far, oldest first. */\n    \
         val calls: List<Call>\n        get() = recorded.toList()\n\n    \
         /** Forget the calls recorded so far. */\n    \
         fun clear() {{\n        recorded.clear()\n    }}\n\n{overrides}}}\n"
    )
}

/// `Recording<Interface>`: the Rust host trait's implementation that performs
/// nothing and records every call, in order, as a value — the twin of
/// [`kotlin_recording_host`], for the same reason: a test drives the machine
/// with no host at all, and reads back what the machine asked of it.
///
/// Generated from the same signatures as the trait, so it cannot fall out of
/// step with it. Each call is a variant of `<Interface>Call`, which derives
/// `PartialEq`, so a test compares whole calls by value; a borrowed argument
/// (`&str`, `&[u8]`) is recorded as its owned copy. The machine owns its host,
/// so a test reads the calls back through `Policy::actions()`.
///
/// A host that answers a child's host cannot invent one, so the recording host of
/// such a parent is generic over what each child answers and takes, for each, the
/// function that answers it, and records each question beside the acts — the twin
/// of what [`kotlin_recording_host`] does. It then has no `Default` and no `Debug`,
/// neither of which a function can give.
fn rust_recording_host(
    interface_name: &str,
    sigs: &BTreeMap<String, Signature>,
    children: &[ChildHost],
) -> String {
    let lang = Language::Rust;
    let owned = |t: &SceType| -> String {
        match t {
            SceType::String => "String".to_string(),
            SceType::Bytes => "Vec<u8>".to_string(),
            _ => host_param_type(lang, t),
        }
    };
    let mut variants = String::new();
    let mut methods = String::new();
    // What a parent with children adds: one type parameter and one source function
    // per child host, and the operation that records the question and answers.
    let mut type_params = Vec::new();
    let mut bounds = Vec::new();
    let mut fields = String::new();
    let mut ctor_params = Vec::new();
    let mut ctor_fields = Vec::new();
    let mut associated = String::new();
    for child in children {
        let method = method_name(lang, &child.operation);
        let variant = filters::to_pascal_case(method.clone());
        let assoc = child.associated_type();
        let param = format!("{assoc}Host");
        type_params.push(param.clone());
        bounds.push(format!("{param}: {} + 'static", child.child_interface));
        fields.push_str(&format!("    {method}: Box<dyn FnMut() -> {param}>,\n"));
        ctor_params.push(format!("{method}: impl FnMut() -> {param} + 'static"));
        ctor_fields.push(format!("{method}: Box::new({method})"));
        associated.push_str(&format!("    type {assoc} = {param};\n"));
        variants.push_str(&format!("    {variant},\n"));
        methods.push_str(&format!(
            "    fn {method}(&mut self) -> {param} {{\n        self.calls.push({interface_name}Call::{variant});\n        (self.{method})()\n    }}\n"
        ));
    }
    for (name, sig) in sigs {
        let method = method_name(lang, name);
        let variant = filters::to_pascal_case(method.clone());
        let params: Vec<String> = sig
            .iter()
            .flat_map(|(n, t)| params_for(lang, n, t))
            .collect();
        let sep = if params.is_empty() { "" } else { ", " };
        if sig.is_empty() {
            variants.push_str(&format!("    {variant},\n"));
            methods.push_str(&format!(
                "    fn {method}(&mut self) {{\n        self.calls.push({interface_name}Call::{variant});\n    }}\n"
            ));
            continue;
        }
        let fields: Vec<String> = sig
            .iter()
            .map(|(n, t)| format!("{}: {}", param_ident(lang, n), owned(t)))
            .collect();
        let values: Vec<String> = sig
            .iter()
            .map(|(n, t)| {
                let id = param_ident(lang, n);
                match t {
                    SceType::String => format!("{id}: {id}.to_string()"),
                    SceType::Bytes => format!("{id}: {id}.to_vec()"),
                    _ => id,
                }
            })
            .collect();
        variants.push_str(&format!("    {variant} {{ {} }},\n", fields.join(", ")));
        methods.push_str(&format!(
            "    fn {method}(&mut self{sep}{}) {{\n        self.calls.push({interface_name}Call::{variant} {{ {} }});\n    }}\n",
            params.join(", "),
            values.join(", ")
        ));
    }
    let generics = if type_params.is_empty() {
        String::new()
    } else {
        format!("<{}>", type_params.join(", "))
    };
    let bounded = if bounds.is_empty() {
        String::new()
    } else {
        format!("<{}>", bounds.join(", "))
    };
    // A function is neither `Default` nor `Debug`, so a recorder that holds one is
    // built with `new` and derives neither.
    let derive = if children.is_empty() {
        "#[derive(Debug, Default)]\n"
    } else {
        ""
    };
    let constructor = if children.is_empty() {
        String::new()
    } else {
        format!(
            "    /// A recorder that answers each child's host with what its function\n    \
             /// returns, asked each time the machine starts the child.\n    \
             pub fn new({}) -> Self {{\n        Self {{\n            calls: Vec::new(),\n            {},\n        }}\n    }}\n\n",
            ctor_params.join(", "),
            ctor_fields.join(",\n            ")
        )
    };
    format!(
        "/// One call the machine made of its host, recorded by\n\
         /// [`Recording{interface_name}`].\n\
         #[derive(Debug, Clone, PartialEq)]\n\
         pub enum {interface_name}Call {{\n{variants}}}\n\n\
         /// [`{interface_name}`] that performs nothing and records every call in\n\
         /// order — the host a test drives the machine with. Read\n\
         /// [`calls`](Self::calls) after the machine has run.\n\
         {derive}\
         pub struct Recording{interface_name}{bounded} {{\n    calls: Vec<{interface_name}Call>,\n{host_fields}}}\n\n\
         impl{bounded} Recording{interface_name}{generics} {{\n\
         {constructor}    \
         /// Every call so far, oldest first.\n    \
         pub fn calls(&self) -> &[{interface_name}Call] {{\n        &self.calls\n    }}\n\n    \
         /// Forget the calls recorded so far.\n    \
         pub fn clear(&mut self) {{\n        self.calls.clear();\n    }}\n}}\n\n\
         impl{bounded} {interface_name} for Recording{interface_name}{generics} {{\n{associated}{methods}}}\n",
        host_fields = fields,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::model::{Direction, ForgeField};
    use crate::parser::SCXMLParser;

    fn parse(scxml: &str) -> SCXMLModel {
        SCXMLParser::new().parse_string(scxml, "t").unwrap()
    }

    /// One schema with a bytes + uint32 payload, keyed by event name — the
    /// shape `validate` resolves arguments against.
    fn fragment_schema() -> BTreeMap<String, EventSchemaModel> {
        let schema = EventSchemaModel {
            name: "FragmentSchema".to_string(),
            event_name: "fragment.received".to_string(),
            fields: vec![
                ForgeField {
                    id: "payload".to_string(),
                    sce_type: SceType::Bytes,
                    direction: Direction::In,
                    expr: None,
                    expr_spelling: None,
                    expr_splices: None,
                    quantity: None,
                    max_size: Some(64),
                    default_covers: Vec::new(),
                    retain: None,
                    initial: None,
                    initial_spelling: None,
                },
                ForgeField {
                    id: "offset".to_string(),
                    sce_type: SceType::Uint32,
                    direction: Direction::In,
                    expr: None,
                    expr_spelling: None,
                    expr_splices: None,
                    quantity: None,
                    max_size: None,
                    default_covers: Vec::new(),
                    retain: None,
                    initial: None,
                    initial_spelling: None,
                },
            ],
            payload_none: false,
            datamodel_markers: Vec::new(),
            source_location: None,
        };
        let mut m = BTreeMap::new();
        m.insert("fragment.received".to_string(), schema);
        m
    }

    #[test]
    fn sce_action_parses_onto_transition() {
        let model = parse(
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" initial="s">
                <state id="s"><transition event="e" target="s">
                    <sce:action name="do_effect"><sce:arg expr="_event.data.x"/></sce:action>
                </transition></state>
            </scxml>"#,
        );
        let tr = &model.states.get("s").unwrap().transitions[0];
        assert_eq!(tr.actions.len(), 1);
        assert_eq!(tr.actions[0].action_type, ACTION_TYPE);
        assert_eq!(tr.actions[0].native_action_name, "do_effect");
        assert_eq!(tr.actions[0].params.len(), 1);
        assert_eq!(tr.actions[0].params[0].expr, "_event.data.x");
    }

    #[test]
    fn valid_typed_and_noarg_actions_accepted() {
        let model = parse(
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" initial="idle">
                <state id="idle"><transition event="fragment.received" target="a">
                    <sce:action name="append"><sce:arg expr="_event.data.payload"/><sce:arg expr="_event.data.offset"/></sce:action>
                </transition></state>
                <state id="a"><transition event="reset" target="idle">
                    <sce:action name="reset_slot"/>
                </transition></state>
            </scxml>"#,
        );
        assert!(validate(&model, &fragment_schema(), "t").is_ok());
    }

    #[test]
    fn unknown_field_rejected() {
        let model = parse(
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" initial="idle">
                <state id="idle"><transition event="fragment.received" target="idle">
                    <sce:action name="op"><sce:arg expr="_event.data.nope"/></sce:action>
                </transition></state>
            </scxml>"#,
        );
        assert!(validate(&model, &fragment_schema(), "t").is_err());
    }

    #[test]
    fn conflicting_signature_rejected() {
        // Same action name `f`, but `payload` is bytes on one transition and
        // `offset` is uint32 on another — one trait method cannot serve both.
        let model = parse(
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" initial="idle">
                <state id="idle">
                    <transition event="fragment.received" target="idle">
                        <sce:action name="f"><sce:arg expr="_event.data.payload"/></sce:action>
                    </transition>
                    <transition event="fragment.received" target="idle">
                        <sce:action name="f"><sce:arg expr="_event.data.offset"/></sce:action>
                    </transition>
                </state>
            </scxml>"#,
        );
        let refusal = validate(&model, &fragment_schema(), "t").expect_err("the signatures differ");
        // The record names the call that contradicts; `related` names the
        // call that fixed the signature first, where the same name is.
        assert_eq!(refusal.location.line, Some(7), "{refusal:?}");
        assert_eq!(
            refusal.related(),
            [RelatedSite {
                role: RelatedRole::ConflictingUse,
                location: SourceLocation {
                    file: "t".to_string(),
                    line: Some(4),
                    col: refusal.related()[0].location.col,
                },
                actual: Some("f".to_string()),
            }],
        );
    }

    #[test]
    fn consistent_signature_across_transitions_accepted() {
        // Same action name `f` with the same field (hence same type) on two
        // transitions is fine — one trait method serves both.
        let model = parse(
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" initial="idle">
                <state id="idle">
                    <transition event="fragment.received" target="idle">
                        <sce:action name="f"><sce:arg expr="_event.data.offset"/></sce:action>
                    </transition>
                    <transition event="fragment.received" target="idle">
                        <sce:action name="f"><sce:arg expr="_event.data.offset"/></sce:action>
                    </transition>
                </state>
            </scxml>"#,
        );
        assert!(validate(&model, &fragment_schema(), "t").is_ok());
    }

    #[test]
    fn literal_argument_rejected() {
        let model = parse(
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" initial="idle">
                <state id="idle"><transition event="fragment.received" target="idle">
                    <sce:action name="op"><sce:arg expr="42"/></sce:action>
                </transition></state>
            </scxml>"#,
        );
        assert!(validate(&model, &fragment_schema(), "t").is_err());
    }

    #[test]
    fn args_without_schema_rejected() {
        let model = parse(
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" initial="idle">
                <state id="idle"><transition event="unschemed" target="idle">
                    <sce:action name="op"><sce:arg expr="_event.data.payload"/></sce:action>
                </transition></state>
            </scxml>"#,
        );
        assert!(validate(&model, &fragment_schema(), "t").is_err());
    }

    #[test]
    fn noarg_action_in_onentry_accepted() {
        // The §scxml-G-7 example itself places a Custom Action Element in
        // <onentry>; a no-argument native action needs no payload, so it is
        // valid there and lowers to a bare host-trait call.
        let model = parse(
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" initial="idle">
                <state id="idle"><onentry>
                    <sce:action name="on_idle_entry"/>
                </onentry></state>
            </scxml>"#,
        );
        assert!(validate(&model, &fragment_schema(), "t").is_ok());
    }

    #[test]
    fn noarg_action_in_onexit_accepted() {
        let model = parse(
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" initial="idle">
                <state id="idle"><onexit>
                    <sce:action name="on_idle_exit"/>
                </onexit></state>
            </scxml>"#,
        );
        assert!(validate(&model, &fragment_schema(), "t").is_ok());
    }

    #[test]
    fn noarg_action_in_initial_transition_accepted() {
        // Initial executable content also runs with no triggering event in
        // scope, so a no-argument native action is admissible there too — the
        // same rule, with no carve-out for entry/exit alone.
        let model = parse(
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" initial="root">
                <state id="root">
                    <initial><transition target="child"><sce:action name="on_init"/></transition></initial>
                    <state id="child"><transition event="e" target="child"/></state>
                </state>
            </scxml>"#,
        );
        assert!(validate(&model, &fragment_schema(), "t").is_ok());
    }

    #[test]
    fn argbearing_action_in_onentry_rejected() {
        // No triggering event is in scope in <onentry>, so an arg-bearing
        // native action (one that would read `_event.data`) is rejected.
        let model = parse(
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" initial="idle">
                <state id="idle"><onentry>
                    <sce:action name="op"><sce:arg expr="_event.data.payload"/></sce:action>
                </onentry></state>
            </scxml>"#,
        );
        assert!(validate(&model, &fragment_schema(), "t").is_err());
    }

    #[test]
    fn argbearing_action_in_onexit_rejected() {
        let model = parse(
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" initial="idle">
                <state id="idle"><onexit>
                    <sce:action name="op"><sce:arg expr="_event.data.payload"/></sce:action>
                </onexit></state>
            </scxml>"#,
        );
        assert!(validate(&model, &fragment_schema(), "t").is_err());
    }

    #[test]
    fn native_action_nested_in_if_rejected() {
        // A native action inside <if>/<foreach> has a conditional/iterated call
        // site, which v1 does not lower — rejected on a transition just as it
        // is in entry/exit.
        let model = parse(
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" initial="idle">
                <state id="idle"><transition event="e" target="idle">
                    <if cond="true"><sce:action name="op"/></if>
                </transition></state>
            </scxml>"#,
        );
        assert!(validate(&model, &fragment_schema(), "t").is_err());
    }

    #[test]
    fn signature_conflict_across_positions_rejected() {
        // The document-wide signature table spans positions: a no-argument `f`
        // in <onentry> and an arg-bearing `f` on a transition cannot both be
        // served by one trait method.
        let model = parse(
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" initial="idle">
                <state id="idle">
                    <onentry><sce:action name="f"/></onentry>
                    <transition event="fragment.received" target="idle">
                        <sce:action name="f"><sce:arg expr="_event.data.offset"/></sce:action>
                    </transition>
                </state>
            </scxml>"#,
        );
        assert!(validate(&model, &fragment_schema(), "t").is_err());
    }

    #[test]
    fn noarg_name_reused_across_positions_accepted() {
        // The same no-argument action name in <onentry>, <onexit>, and on a
        // transition is consistent (all empty signatures) — one trait method.
        let model = parse(
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" initial="idle">
                <state id="idle">
                    <onentry><sce:action name="reset_slot"/></onentry>
                    <onexit><sce:action name="reset_slot"/></onexit>
                    <transition event="reset" target="idle"><sce:action name="reset_slot"/></transition>
                </state>
            </scxml>"#,
        );
        assert!(validate(&model, &fragment_schema(), "t").is_ok());
    }

    #[test]
    fn missing_name_rejected_at_parse() {
        let res = SCXMLParser::new().parse_string(
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" initial="s">
                <state id="s"><transition event="e" target="s"><sce:action/></transition></state>
            </scxml>"#,
            "t",
        );
        assert!(res.is_err(), "<sce:action> without name must fail at parse");
    }

    #[test]
    fn the_kotlin_recording_host_records_every_call_by_value() {
        let mut sigs: BTreeMap<String, Signature> = BTreeMap::new();
        sigs.insert(
            "append_fragment".to_string(),
            vec![
                ("payload".to_string(), SceType::Bytes),
                ("offset".to_string(), SceType::Uint32),
            ],
        );
        sigs.insert("reset_slot".to_string(), Vec::new());
        let out = build_interface(Language::Kotlin, "MActions", &sigs, &[]);
        assert!(
            out.contains("class RecordingMActions : MActions {"),
            "the recorder implements the interface it is generated from:\n{out}"
        );
        // A byte array compares by reference inside a data class, so a bytes
        // argument is recorded as a list that compares by value.
        assert!(
            out.contains(
                "data class AppendFragment(val payload: List<Byte>, val offset: UInt) : Call"
            ),
            "{out}"
        );
        assert!(
            out.contains("recorded += Call.AppendFragment(payload.toList(), offset)"),
            "{out}"
        );
        assert!(out.contains("data object ResetSlot : Call"), "{out}");
        assert!(out.contains("override fun resetSlot() {"), "{out}");
    }

    /// A parent's host answers the host of each child that declares acts, so the
    /// interface declares one operation per child, returning the child's own
    /// interface, and the recording host asks its test for what to answer.
    #[test]
    fn the_kotlin_interface_of_a_parent_answers_the_host_of_each_child() {
        let children = [ChildHost {
            operation: "actions_for_worker".to_string(),
            child_interface: "WorkerActions".to_string(),
        }];
        let mut sigs: BTreeMap<String, Signature> = BTreeMap::new();
        sigs.insert("reset_slot".to_string(), Vec::new());
        let out = build_interface(Language::Kotlin, "MActions", &sigs, &children);
        assert!(
            out.contains("    fun resetSlot()\n    fun actionsForWorker(): WorkerActions\n}"),
            "the acts, then the operation that answers a child's host:\n{out}"
        );
        assert!(
            out.contains(
                "class RecordingMActions(\n    /** Answers the host [WorkerActions] is asked for; \
                 the test keeps what it answers. */\n    \
                 private val actionsForWorkerSource: () -> WorkerActions,\n) : MActions {"
            ),
            "the recorder takes what to answer from its test:\n{out}"
        );
        assert!(
            out.contains(
                "override fun actionsForWorker(): WorkerActions {\n        \
                 recorded += Call.ActionsForWorker\n        return actionsForWorkerSource()\n    }"
            ),
            "the question is recorded, then answered:\n{out}"
        );
        // A parent with no child to answer for is as it was.
        let plain = build_interface(Language::Kotlin, "MActions", &sigs, &[]);
        assert!(!plain.contains("Source"), "{plain}");
    }

    /// The Python interface answers a child's host as the Kotlin one does, after
    /// the acts, and names the child's interface as text: the child's module
    /// declares it, and the machine imports that module where it starts the child.
    #[test]
    fn the_python_interface_of_a_parent_answers_the_host_of_each_child() {
        let children = [ChildHost {
            operation: "actions_for_worker".to_string(),
            child_interface: "WorkerActions".to_string(),
        }];
        let mut sigs: BTreeMap<String, Signature> = BTreeMap::new();
        sigs.insert("reset_slot".to_string(), Vec::new());
        let out = build_interface(Language::Python, "MActions", &sigs, &children);
        let act = out.find("def reset_slot(self) -> None:").expect(&out);
        let factory = out
            .find("def actions_for_worker(self) -> \"WorkerActions\":")
            .expect(&out);
        assert!(
            act < factory,
            "the acts, then the operation that answers a child's host:\n{out}"
        );
        let plain = build_interface(Language::Python, "MActions", &sigs, &[]);
        assert!(!plain.contains("actions_for"), "{plain}");
    }

    /// The C++ interface answers a child's host as a reference to the interface
    /// the child's own namespace declares, qualified by the suite prefix where
    /// there is one: a parent in another namespace cannot reach it by its bare
    /// name.
    #[test]
    fn the_cpp_interface_of_a_parent_answers_the_host_of_each_child() {
        assert_eq!(
            child_interface_type(Language::Cpp, "worker", ""),
            "::SCE::Generated::worker::WorkerActions"
        );
        assert_eq!(
            child_interface_type(Language::Cpp, "worker", "suite::"),
            "::SCE::Generated::suite::worker::WorkerActions"
        );
        let children = [ChildHost {
            operation: "actions_for_worker".to_string(),
            child_interface: child_interface_type(Language::Cpp, "worker", ""),
        }];
        let mut sigs: BTreeMap<String, Signature> = BTreeMap::new();
        sigs.insert("reset_slot".to_string(), Vec::new());
        let out = build_interface(Language::Cpp, "MActions", &sigs, &children);
        let act = out.find("virtual void resetSlot() = 0;").expect(&out);
        let factory = out
            .find("virtual ::SCE::Generated::worker::WorkerActions& actionsForWorker() = 0;")
            .expect(&out);
        assert!(
            act < factory,
            "the acts, then the operation that answers a child's host:\n{out}"
        );
        let plain = build_interface(Language::Cpp, "MActions", &sigs, &[]);
        assert!(!plain.contains("actionsFor"), "{plain}");
    }

    /// The C11 table answers the child's host as a pointer to the child's own
    /// table, after the acts, and the operation is among the members `_init`
    /// refuses a table for lacking — it is one of [`NativeActions::operation_names`].
    #[test]
    fn the_c11_table_of_a_parent_answers_the_host_of_each_child() {
        let children = [ChildHost {
            operation: "actions_for_worker".to_string(),
            child_interface: child_interface_type(Language::C11, "worker", ""),
        }];
        assert_eq!(children[0].child_interface, "worker_actions_t");
        let mut sigs: BTreeMap<String, Signature> = BTreeMap::new();
        sigs.insert("reset_slot".to_string(), Vec::new());
        let out = build_interface(Language::C11, "m_actions_t", &sigs, &children);
        let act = out
            .find("void (*reset_slot)(void *user_data);")
            .expect(&out);
        let factory = out
            .find("const worker_actions_t *(*actions_for_worker)(void *user_data);")
            .expect(&out);
        assert!(
            act < factory,
            "the acts, then the operation that answers a child's host:\n{out}"
        );
        let plain = build_interface(Language::C11, "m_actions_t", &sigs, &[]);
        assert!(!plain.contains("actions_for"), "{plain}");
    }

    /// A document in which an act and a child's host would be one method is
    /// refused where the second is written, and so is one in which two
    /// children's hosts would be.
    #[test]
    fn an_act_named_for_the_host_of_a_child_is_refused() {
        let child = |id: &str| {
            format!(
                r#"<invoke type="scxml" id="{id}"><content>
                     <scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
                            version="1.0" initial="b" datamodel="sce-static">
                       <state id="b"><onentry><sce:action name="hello"/></onentry></state>
                     </scxml>
                   </content></invoke>"#
            )
        };
        let document = |body: String| {
            parse(&format!(
                r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
                          version="1.0" initial="s" datamodel="sce-static">
                     <state id="s">{body}</state>
                   </scxml>"#
            ))
        };
        let clash = document(format!(
            r#"{} <onentry><sce:action name="actions_for_worker"/></onentry>"#,
            child("worker")
        ));
        let refusal =
            validate(&clash, &fragment_schema(), "t").expect_err("one method, two meanings");
        assert!(
            format!("{refusal:?}").contains("actions_for_worker")
                && format!("{refusal:?}").contains("host operation that answers the host"),
            "{refusal:?}"
        );
        // Spelled as another language spells it, it is the same method.
        let camel = document(format!(
            r#"{} <onentry><sce:action name="actionsForWorker"/></onentry>"#,
            child("worker")
        ));
        let refusal =
            validate(&camel, &fragment_schema(), "t").expect_err("one method, spelled two ways");
        assert!(
            format!("{refusal:?}").contains("actionsForWorker"),
            "{refusal:?}"
        );
        let twins = document(format!("{}{}", child("a_b"), child("aB")));
        let refusal =
            validate(&twins, &fragment_schema(), "t").expect_err("one method, two children");
        assert!(
            format!("{refusal:?}").contains("their ids differ only in spelling"),
            "{refusal:?}"
        );
        let apart = document(format!("{}{}", child("one"), child("two")));
        assert!(validate(&apart, &fragment_schema(), "t").is_ok());
    }

    #[test]
    fn the_rust_recording_host_records_every_call_by_value() {
        let mut sigs: BTreeMap<String, Signature> = BTreeMap::new();
        sigs.insert(
            "append_fragment".to_string(),
            vec![
                ("payload".to_string(), SceType::Bytes),
                ("offset".to_string(), SceType::Uint32),
            ],
        );
        sigs.insert("reset_slot".to_string(), Vec::new());
        let out = rust_recording_host("MActions", &sigs, &[]);
        assert!(
            out.contains("impl MActions for RecordingMActions {"),
            "the recorder implements the trait it is generated from:\n{out}"
        );
        assert!(
            out.contains("#[derive(Debug, Default)]\npub struct RecordingMActions {"),
            "a parent with no child to answer for is as it was:\n{out}"
        );
        // A borrowed argument is recorded as its owned copy, so a recorded
        // call outlives the dispatch and compares by value.
        assert!(
            out.contains("AppendFragment { payload: Vec<u8>, offset: u32 },"),
            "{out}"
        );
        assert!(
            out.contains(
                "self.calls.push(MActionsCall::AppendFragment { payload: payload.to_vec(), offset });"
            ),
            "{out}"
        );
        assert!(out.contains("    ResetSlot,\n"), "{out}");
        assert!(out.contains("fn reset_slot(&mut self) {"), "{out}");
    }

    /// The Rust trait of a parent declares, for each child, an associated type
    /// bounded by the child's own trait and an operation that answers one, and
    /// its recording host is generic over what each answers: it takes the function
    /// that answers, records the question beside the acts, and derives neither
    /// `Default` nor `Debug`, which a function cannot give.
    #[test]
    fn the_rust_trait_of_a_parent_answers_the_host_of_each_child() {
        let children = [ChildHost {
            operation: "actions_for_worker".to_string(),
            child_interface: child_interface_type(Language::Rust, "worker", ""),
        }];
        assert_eq!(
            children[0].child_interface,
            "super::worker_sm::WorkerActions"
        );
        let mut sigs: BTreeMap<String, Signature> = BTreeMap::new();
        sigs.insert("reset_slot".to_string(), Vec::new());
        let out = build_interface(Language::Rust, "MActions", &sigs, &children);
        assert!(
            out.contains(
                "pub trait MActions {\n    type ActionsForWorker: super::worker_sm::WorkerActions + 'static;\n"
            ),
            "the associated type leads the trait:\n{out}"
        );
        let act = out.find("fn reset_slot(&mut self);").expect(&out);
        let factory = out
            .find("fn actions_for_worker(&mut self) -> Self::ActionsForWorker;")
            .expect(&out);
        assert!(
            act < factory,
            "the acts, then the operation that answers a child's host:\n{out}"
        );

        let recording = rust_recording_host("MActions", &sigs, &children);
        assert!(
            recording.contains(
                "pub struct RecordingMActions<ActionsForWorkerHost: super::worker_sm::WorkerActions + 'static> {"
            ),
            "{recording}"
        );
        assert!(!recording.contains("derive(Debug, Default)"), "{recording}");
        assert!(
            recording.contains(
                "pub fn new(actions_for_worker: impl FnMut() -> ActionsForWorkerHost + 'static) -> Self {"
            ),
            "{recording}"
        );
        assert!(
            recording.contains(
                "fn actions_for_worker(&mut self) -> ActionsForWorkerHost {\n        self.calls.push(MActionsCall::ActionsForWorker);\n        (self.actions_for_worker)()\n    }"
            ),
            "the question is recorded, then answered:\n{recording}"
        );
        let plain = build_interface(Language::Rust, "MActions", &sigs, &[]);
        assert!(!plain.contains("type "), "{plain}");
    }
}
