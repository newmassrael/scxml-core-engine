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
use crate::forge::generator::host_param_type;
use crate::forge::model::{EventSchemaModel, ForgeKind, SceType};
use crate::generator::Language;
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
    /// spelling and sorted.
    ///
    /// The five hosted backends get their "the host supplied it" guarantee
    /// from the type system — an unimplemented interface method does not
    /// compile. C has no such check, so `_init_with_actions` walks this list
    /// and refuses a vtable with a NULL member, which is the same guarantee
    /// moved to the one call a C host has to make. Emitted rather than
    /// re-derived in the template so the names cannot drift from the ones
    /// [`build_interface`] declared.
    pub operation_names: Vec<String>,
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
    let method = method_name(lang, name);
    let recv = receiver(lang);
    let mut all: Vec<String> = Vec::new();
    if matches!(lang, Language::C11) {
        all.push("sm->actions.user_data".to_string());
    }
    all.extend(args.iter().cloned());
    let joined = all.join(", ");
    match lang {
        Language::Rust | Language::Cpp | Language::C11 => format!("{recv}{method}({joined});"),
        Language::Go | Language::Kotlin | Language::Python => format!("{recv}{method}({joined})"),
    }
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
/// [`lower_native_call`] are therefore total). `model` is the per-backend
/// codegen clone, never the parsed model.
///
/// `machine_name` is the caller's per-language machine token: the PascalCase
/// stem for the five hosted backends (matching what their
/// `build_*_event_payload` twins already use), the raw snake stem for C11.
pub fn render(model: &mut SCXMLModel, machine_name: &str, lang: Language) -> NativeActions {
    let schemas = model.imported_event_schemas.clone();
    // Read once, before the walk borrows `model` mutably. Every other raise
    // site in this generator is written under the same condition — a document
    // that declares no `error.execution` has no enum variant to name, and an
    // event nothing can match would be discarded on arrival anyway.
    let raises_error = model.events.contains("error.execution");

    let mut payload_events: BTreeSet<String> = BTreeSet::new();
    // Signatures keyed by action name; the first occurrence defines the
    // signature and `validate` has already proven every later occurrence
    // agrees, so a single interface method serves every call site. A
    // no-argument action (the only kind admissible in an eventless position)
    // registers an empty signature, which still emits its method.
    let mut sigs: BTreeMap<String, Signature> = BTreeMap::new();
    let mut any = false;

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
            lower_native_call(
                action,
                None,
                machine_name,
                lang,
                raises_error,
                &mut sigs,
                &mut payload_events,
            );
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
                lower_native_call(
                    action,
                    Some(&binding),
                    machine_name,
                    lang,
                    raises_error,
                    &mut sigs,
                    &mut payload_events,
                );
                any = true;
            }
        }
    }

    let name = interface_name(lang, machine_name);
    let operation_names: Vec<String> = sigs.keys().map(|n| method_name(lang, n)).collect();
    let (interface_def, interface_name) = if any {
        (build_interface(lang, &name, &sigs), name)
    } else {
        (String::new(), String::new())
    };

    NativeActions {
        interface_def,
        interface_name,
        payload_events,
        any,
        operation_names,
    }
}

/// Lower one `<sce:action>` to its `lang` call site (stored on
/// `action.native_action_rendered`) and fold its signature into `sigs`.
///
/// `binding` is `Some` only for a `<transition>` child, where the triggering
/// event's typed payload is in scope; `None` for an eventless position
/// (`<onentry>`/`<onexit>`/initial), where the action is necessarily
/// no-argument. A no-argument action lowers to a bare host call in either
/// case; an arg-bearing one reads its values from the event's typed payload
/// and is wrapped in that backend's tag check.
///
/// `raises_error` reaches [`guard_payload`] unchanged — it decides whether the
/// arm an untyped delivery takes says so with `error.execution` or stays empty.
fn lower_native_call(
    action: &mut Action,
    binding: Option<&PayloadBinding>,
    machine_name: &str,
    lang: Language,
    raises_error: bool,
    sigs: &mut BTreeMap<String, Signature>,
    payload_events: &mut BTreeSet<String>,
) {
    let name = action.native_action_name.clone();

    if action.params.is_empty() {
        sigs.entry(name.clone()).or_default();
        action.native_action_rendered = call(lang, &name, &[]);
        return;
    }

    // Arg-bearing: `validate` guarantees a transition binding with a resolved,
    // payload-eligible schema, so the lookups below are total.
    let binding = binding.expect("validated: arg-bearing native action is a <transition> child");
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

    payload_events.insert(binding.event.to_string());
    sigs.entry(name.clone()).or_insert(params);

    let stmt = call(lang, &name, &call_args);
    action.native_action_rendered = guard_payload(
        lang,
        machine_name,
        binding.event,
        &name,
        &stmt,
        raises_error,
    );
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
            let otherwise = if raises_error {
                format!(
                    " else {{\n    {machine_name}_raise_platform_error(\
                     sm, {upper}_EVENT_ERROR_EXECUTION, \"{msg}\");\n}}"
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
fn build_interface(
    lang: Language,
    interface_name: &str,
    sigs: &BTreeMap<String, Signature>,
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
            format!(
                "{}pub trait {interface_name} {{\n{methods}}}\n",
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
            format!(
                "/**\n{} */\ninterface {interface_name} {{\n{methods}}}\n",
                doc(" * ")
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
