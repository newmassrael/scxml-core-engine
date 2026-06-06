// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
use crate::forge::error::{ForgeError, Located, ValidationError};
use crate::forge::event_schema_check::schema_is_native_payload_eligible;
use crate::forge::generator::rust_param_type;
use crate::forge::model::{EventSchemaModel, ForgeKind, SceType};
use crate::model::{Action, SCXMLModel};
use std::collections::{BTreeMap, BTreeSet};

const ACTION_TYPE: &str = "native_action";
const EVENT_DATA_PREFIX: &str = "_event.data.";

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
        .then_actions
        .iter()
        .chain(action.else_actions.iter())
        .chain(action.actions.iter())
        .chain(action.elseif_branches.iter().flat_map(|b| b.actions.iter()))
        .find_map(first_native)
}

fn located_on_action(
    action: &Action,
    diag_label: &str,
    err: ValidationError,
) -> Located<ForgeError> {
    let (line, col) = action
        .source_location
        .as_ref()
        .map_or((None, None), |l| (l.line, l.col));
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

fn argument_err(action: &Action, diag_label: &str, detail: String) -> Located<ForgeError> {
    located_on_action(
        action,
        diag_label,
        ValidationError::NativeActionArgument {
            name: action.native_action_name.clone(),
            detail,
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
    let mut signatures: BTreeMap<String, Vec<String>> = BTreeMap::new();

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
                check_placement(action, &scope, &mut signatures, diag_label)?;
            }
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
    signatures: &mut BTreeMap<String, Vec<String>>,
    diag_label: &str,
) -> Result<(), Located<ForgeError>> {
    if is_native(action) {
        let sig = signature_of(action, scope, diag_label)?;
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

/// Record `sig` for `action.native_action_name` in the document-wide table,
/// rejecting a divergence from a prior occurrence — one generated trait method
/// must serve every call site of a given name, regardless of position.
fn register_signature(
    action: &Action,
    sig: Vec<String>,
    signatures: &mut BTreeMap<String, Vec<String>>,
    diag_label: &str,
) -> Result<(), Located<ForgeError>> {
    match signatures.get(&action.native_action_name) {
        Some(prev) if *prev != sig => Err(located_on_action(
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
        )),
        Some(_) => Ok(()),
        None => {
            signatures.insert(action.native_action_name.clone(), sig);
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
            return Err(argument_err(
                action,
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
            return Err(located_on_action(
                action,
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

/// Rust-backend artifacts produced by [`render_rust`].
pub struct RustNativeActions {
    /// The full `pub trait <Machine>Actions { … }` definition, or empty
    /// when the document declares no native actions.
    pub trait_def: String,
    /// `<Machine>Actions`, or empty when there are no native actions.
    pub trait_name: String,
    /// Events whose payload variant must exist because a native action reads
    /// one of their typed fields. Unioned into the payload-channel event set
    /// by [`crate::forge::generator::build_rust_event_payload`].
    pub payload_events: BTreeSet<String>,
    /// Whether any native action exists (drives the generic `Policy<A>`).
    pub any: bool,
}

/// Per-transition payload context for lowering an arg-bearing `<sce:action>`.
struct PayloadBinding<'a> {
    /// Triggering event name (added to the payload-event union when the action
    /// reads a typed field).
    event: &'a str,
    /// `to_event_variant(event)` — the payload-enum variant carrying the typed
    /// fields the action reads.
    variant: &'a str,
    /// The triggering event's imported EventSchema (guaranteed present for an
    /// arg-bearing action by [`validate`]).
    schema: Option<&'a EventSchemaModel>,
}

/// Lower every `<sce:action>` on `model` to its Rust call site, storing the
/// rendered code on `Action::native_action_rendered`, and return the trait
/// definition + payload-event union.
///
/// Visits both `<transition>` actions and eventless executable content
/// (`<onentry>`/`<onexit>`/initial). Assumes [`validate`] already passed, so an
/// arg-bearing action is always a transition child with a resolved,
/// payload-eligible `_event.data.<field>` schema (the `expect`s in
/// [`lower_native_call`] are therefore total). `model` is the per-backend
/// codegen clone, never the parsed model.
pub fn render_rust(model: &mut SCXMLModel, machine_name: &str) -> RustNativeActions {
    let enum_name = format!("{machine_name}Payload");
    let trait_name = format!("{machine_name}Actions");
    let schemas = model.imported_event_schemas.clone();

    let mut payload_events: BTreeSet<String> = BTreeSet::new();
    // Method signatures keyed by action name; the first occurrence defines the
    // signature and `validate` has already proven every later occurrence agrees,
    // so a single trait method serves every call site. A no-argument action (the
    // only kind admissible in an eventless position) registers an empty
    // signature, which still emits its `fn <name>(&mut self);` method.
    let mut sigs: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    let mut any = false;

    for state in model.states.values_mut() {
        // Eventless positions: every native action here is no-argument
        // (enforced by `validate`), so it lowers to a bare host-trait call.
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
            lower_native_call(action, None, &enum_name, &mut sigs, &mut payload_events);
            any = true;
        }

        for transition in &mut state.transitions {
            let event = transition.event.clone();
            let variant = filters::to_event_variant(event.clone());
            let binding = PayloadBinding {
                event: &event,
                variant: &variant,
                schema: schemas.get(&event),
            };
            for action in &mut transition.actions {
                if !is_native(action) {
                    continue;
                }
                lower_native_call(
                    action,
                    Some(&binding),
                    &enum_name,
                    &mut sigs,
                    &mut payload_events,
                );
                any = true;
            }
        }
    }

    let (trait_def, trait_name) = if any {
        (build_trait(&trait_name, &sigs), trait_name)
    } else {
        (String::new(), String::new())
    };

    RustNativeActions {
        trait_def,
        trait_name,
        payload_events,
        any,
    }
}

/// Lower one `<sce:action>` to its Rust call site (stored on
/// `action.native_action_rendered`) and fold its signature into `sigs`.
///
/// `binding` is `Some` only for a `<transition>` child, where the triggering
/// event's typed payload is in scope; `None` for an eventless position
/// (`<onentry>`/`<onexit>`/initial), where the action is necessarily
/// no-argument. A no-argument action lowers to a bare `self.actions.<name>();`
/// in either case; an arg-bearing one reads its values from the event's payload
/// variant and is wrapped in a payload-match arm.
fn lower_native_call(
    action: &mut Action,
    binding: Option<&PayloadBinding>,
    enum_name: &str,
    sigs: &mut BTreeMap<String, Vec<(String, String)>>,
    payload_events: &mut BTreeSet<String>,
) {
    let name = action.native_action_name.clone();

    if action.params.is_empty() {
        sigs.entry(name.clone()).or_default();
        action.native_action_rendered = format!("self.actions.{name}();");
        return;
    }

    // Arg-bearing: `validate` guarantees a transition binding with a resolved,
    // payload-eligible schema, so the lookups below are total.
    let binding = binding.expect("validated: arg-bearing native action is a <transition> child");
    let schema = binding
        .schema
        .expect("validated: arg-bearing native action has a schema");

    let mut call_args: Vec<String> = Vec::new();
    let mut params: Vec<(String, String)> = Vec::new();
    for arg in &action.params {
        let field = arg_field(&arg.expr).expect("validated: bare _event.data field");
        let f = schema
            .fields
            .iter()
            .find(|f| f.id == field)
            .expect("validated: field exists on schema");
        let by_ref = matches!(f.sce_type, SceType::String | SceType::Bytes);
        let pname = if arg.name.is_empty() {
            field.to_string()
        } else {
            arg.name.clone()
        };
        call_args.push(if by_ref {
            format!("&ev.{field}")
        } else {
            format!("ev.{field}")
        });
        params.push((pname, rust_param_type(&f.sce_type)));
    }

    payload_events.insert(binding.event.to_string());
    sigs.entry(name.clone()).or_insert(params);

    // The typed arguments are read from the event's payload variant. An event
    // raised by name (not via the generated typed inject) carries no payload
    // variant and cannot supply them — a contract violation, NOT a silent skip:
    // the `_` arm `debug_assert!`s so a debug/test build fails loudly, while a
    // release build compiles it away (no MCU cost). This mirrors the typed-guard
    // channel's documented default-payload contract.
    let call = format!("self.actions.{}({});", name, call_args.join(", "));
    let msg = format!(
        "native action '{name}' requires the typed payload of its \
         triggering event; raise the event via its generated typed inject"
    );
    let variant = binding.variant;
    action.native_action_rendered = format!(
        "match &self.pending_payload {{\n            \
         {enum_name}::{variant}(ev) => {{ {call} }}\n            \
         _ => debug_assert!(false, {msg:?}),\n        }}"
    );
}

fn build_trait(trait_name: &str, sigs: &BTreeMap<String, Vec<(String, String)>>) -> String {
    let mut methods = String::new();
    for (name, params) in sigs {
        let plist: String = params.iter().map(|(n, t)| format!(", {n}: {t}")).collect();
        methods.push_str(&format!("    fn {name}(&mut self{plist});\n"));
    }
    format!(
        "/// W3C SCXML G.7: host operations dispatched by `<sce:action>`.\n\
         /// The generated `Policy` is generic over an implementation of this\n\
         /// trait; the host supplies the side effects while the statechart keeps\n\
         /// each operation symbolic. No runtime script engine is involved.\n\
         pub trait {trait_name} {{\n{methods}}}\n"
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
                    quantity: None,
                    max_size: Some(64),
                },
                ForgeField {
                    id: "offset".to_string(),
                    sce_type: SceType::Uint32,
                    direction: Direction::In,
                    expr: None,
                    quantity: None,
                    max_size: None,
                },
            ],
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
        assert!(validate(&model, &fragment_schema(), "t").is_err());
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
}
