// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! W3C SCXML G.7 — `<sce:action>` Custom Action Element: native host-trait
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
//! v1 contract (enforced by [`validate`]): a native action is a *direct
//! `<transition>` child*, and every `<sce:arg>` is a bare
//! `_event.data.<field>` reference resolving to a declared, payload-eligible
//! field on the triggering event's imported EventSchema. Anything else —
//! a native action in `<onentry>`/`<onexit>`/`<if>`/`<foreach>`, a
//! literal/derived argument, an unknown field, or an enum-typed schema — is
//! rejected at the validation stage rather than silently routed through a
//! script engine. The construct is engine-free *by definition*, so it never
//! degrades to a runtime fallback.

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
    // Document-wide signature table: a `name` that recurs on more than one
    // transition must carry the same argument types every time, so a single
    // generated `Actions` trait method can serve every call site. Detecting
    // a conflict here is fail-fast at SCE's own validation stage rather than
    // deferring it to a type error in the downstream compiler.
    let mut signatures: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for state in scxml.states.values() {
        // A native action is only meaningful as a direct <transition> child,
        // where a triggering event (hence a typed payload) is in scope. Any
        // other placement is rejected up front.
        let off_transition = state
            .on_entry_blocks
            .iter()
            .chain(state.on_exit_blocks.iter())
            .flatten()
            .chain(state.initial_transition_actions.iter())
            .chain(state.initial_history_default_actions.iter());
        for action in off_transition {
            if let Some(na) = first_native(action) {
                return Err(placement_err(
                    na,
                    diag_label,
                    "supported only as a direct <transition> child \
                     (found in <onentry>/<onexit>/initial executable content)",
                ));
            }
        }

        for transition in &state.transitions {
            let schema = imported_schemas.get(&transition.event);
            for action in &transition.actions {
                if is_native(action) {
                    let sig = validate_args(action, schema, &transition.event, diag_label)?;
                    match signatures.get(&action.native_action_name) {
                        Some(prev) if *prev != sig => {
                            return Err(located_on_action(
                                action,
                                diag_label,
                                ValidationError::NativeActionSignatureConflict {
                                    name: action.native_action_name.clone(),
                                    detail: format!(
                                        "argument types ({}) here disagree with ({}) on \
                                         another transition",
                                        sig.join(", "),
                                        prev.join(", "),
                                    ),
                                },
                            ));
                        }
                        Some(_) => {}
                        None => {
                            signatures.insert(action.native_action_name.clone(), sig);
                        }
                    }
                } else if let Some(na) = first_native(action) {
                    return Err(placement_err(
                        na,
                        diag_label,
                        "supported only as a direct <transition> child \
                         (found nested inside <if>/<foreach>)",
                    ));
                }
            }
        }
    }
    Ok(())
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

/// Lower every `<sce:action>` on `model`'s transitions to its Rust call site,
/// storing the rendered code on `Action::native_action_rendered`, and return
/// the trait definition + payload-event union.
///
/// Assumes [`validate`] already passed, so every argument is a known,
/// payload-eligible `_event.data.<field>` reference (the `unwrap`s below are
/// therefore total). `model` is the per-backend codegen clone, never the
/// parsed model.
pub fn render_rust(model: &mut SCXMLModel, machine_name: &str) -> RustNativeActions {
    let enum_name = format!("{machine_name}Payload");
    let trait_name = format!("{machine_name}Actions");
    let schemas = model.imported_event_schemas.clone();

    let mut payload_events: BTreeSet<String> = BTreeSet::new();
    // Method signatures keyed by action name. First occurrence defines the
    // signature; a same-named action with a divergent payload type would
    // surface as a type error at the generated-trait call site (loud, in the
    // compile gate) rather than silently — consistent per-name signatures are
    // the documented v1 contract.
    let mut sigs: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    let mut any = false;

    for state in model.states.values_mut() {
        for transition in &mut state.transitions {
            let event = transition.event.clone();
            let schema = schemas.get(&event);
            let variant = filters::to_event_variant(event.clone());
            for action in &mut transition.actions {
                if !is_native(action) {
                    continue;
                }
                any = true;
                let name = action.native_action_name.clone();
                let mut call_args: Vec<String> = Vec::new();
                let mut params: Vec<(String, String)> = Vec::new();
                let mut uses_payload = false;

                for arg in &action.params {
                    let field = arg_field(&arg.expr).expect("validated: bare _event.data field");
                    let f = schema
                        .expect("validated: arg-bearing native action has a schema")
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
                    uses_payload = true;
                }

                if uses_payload {
                    payload_events.insert(event.clone());
                }
                sigs.entry(name.clone()).or_insert(params);

                let call = format!("self.actions.{}({});", name, call_args.join(", "));
                action.native_action_rendered = if uses_payload {
                    // The typed arguments are read from the event's payload
                    // variant. An event raised by name (not via the generated
                    // typed inject) carries no payload variant and cannot
                    // supply them — a contract violation, NOT a silent skip:
                    // the `_` arm `debug_assert!`s so a debug/test build fails
                    // loudly, while a release build compiles it away (no MCU
                    // cost). This mirrors the typed-guard channel's documented
                    // default-payload contract.
                    let msg = format!(
                        "native action '{name}' requires the typed payload of its \
                         triggering event; raise the event via its generated typed inject"
                    );
                    format!(
                        "match &self.pending_payload {{\n            \
                         {enum_name}::{variant}(ev) => {{ {call} }}\n            \
                         _ => debug_assert!(false, {msg:?}),\n        }}"
                    )
                } else {
                    call
                };
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
    fn action_in_onentry_rejected() {
        let model = parse(
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" initial="idle">
                <state id="idle"><onentry>
                    <sce:action name="reset_slot"/>
                </onentry></state>
            </scxml>"#,
        );
        assert!(validate(&model, &fragment_schema(), "t").is_err());
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
