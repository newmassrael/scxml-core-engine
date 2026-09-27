// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Which Event I/O Processor and `<invoke>` types this build has a
//! lowering path for — and the sites that name one it does not.
//!
//! Two things live here because they are one fact read two ways.
//!
//! **The accepted set.** [`is_supported_send_type`] is the single place
//! the set is spelled. It used to be spelled five more times, as a
//! literal list inside the Rust, Go, Python, Kotlin and C11 send
//! templates, so adding a processor meant editing six files and a
//! backend that was missed diverged silently — the generated code is
//! the only place the difference shows, and no test reads six templates
//! against each other. The templates now read
//! [`crate::model::Action::send_type_unsupported`], which this module
//! decides.
//!
//! **The sites.** A document may legitimately name a type no build
//! implements: the specification defines that case rather than leaving
//! it undefined, so such a document is valid SCXML with defined meaning
//! (see [`crate::model::UnsupportedInvokeInfo`] for the same argument on
//! the invoke side). The generated code is therefore correct to accept
//! it and raise at runtime — but until this module existed, that
//! decision was reached at build time and then discarded. A consumer
//! compiled green, passed every test that did not enter the state, and
//! met the refusal hours later in production.
//!
//! So the answer is published rather than enforced. [`analyze`] returns
//! the sites, `sce-codegen` projects them onto the stdout manifest as
//! `needs_host_processor` + `host_processor_causes`, and a build that
//! wants to fail on them can — while a build deliberately relying on the
//! runtime refusal keeps working unchanged. Rejecting here instead would
//! refuse a document the specification permits.
//!
//! Scope, stated because the check is narrower than the sentence a
//! reader would infer: only a **literal** `type` is judged. A `typeexpr`
//! resolves at runtime and no build-time walk can name its value, so
//! such a site is absent from the cause list and is refused by the
//! generated code's own dynamic check.

use std::borrow::Cow;

use crate::forge::error::SourceLocation;
use crate::generator::Language;
use crate::model::{Action, Invoke, SCXMLModel, State};

/// SCXML Event I/O Processor URI (§scxml-C-1) — the default when
/// `<send>` carries no `type`.
pub const SCXML_EVENT_PROCESSOR_TYPE: &str = "http://www.w3.org/TR/scxml/#SCXMLEventProcessor";

/// BasicHTTP Event I/O Processor URI (§scxml-C-2).
pub const BASIC_HTTP_EVENT_PROCESSOR_TYPE: &str =
    "http://www.w3.org/TR/scxml/#BasicHTTPEventProcessor";

/// Every `<send type="...">` value this build lowers to a delivery path.
///
/// Ordered as the specification introduces them. Exposed as a slice
/// rather than kept private so a test can assert the runtime-side
/// spelling in `sce/include/common/SendHelper.h` still names the same
/// set — the build decides at codegen time and the Interpreter decides
/// at evaluation time, and the two answering differently for one URI is
/// exactly the divergence this constant exists to make visible.
pub const SUPPORTED_SEND_TYPES: &[&str] =
    &[SCXML_EVENT_PROCESSOR_TYPE, BASIC_HTTP_EVENT_PROCESSOR_TYPE];

/// Whether `send_type` names an Event I/O Processor this build can
/// deliver through.
///
/// An empty string is the absent attribute, which §scxml-6.2 defines as
/// the SCXML Event I/O Processor, so it is supported. Every other value
/// outside [`SUPPORTED_SEND_TYPES`] is one the generated code refuses at
/// runtime.
pub fn is_supported_send_type(send_type: &str) -> bool {
    // §scxml-6.2: "If the SCXML Processor does not support the type that
    // is specified, it MUST place the event error.execution on the
    // internal event queue." Deciding membership is what makes that
    // refusal reachable; deciding it HERE is what lets the decision also
    // be reported instead of only performed.
    send_type.is_empty() || SUPPORTED_SEND_TYPES.contains(&send_type)
}

/// One site naming a processor type this build has no path for.
///
/// Separate from the wire record for the reason
/// [`crate::script_engine_analyzer::ScriptEngineCauseKind`] is: the
/// variant is the reviewable enumeration, and `kind` on the wire is a
/// stable kebab-case token that does not move when the variant is
/// renamed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostProcessorCauseKind {
    /// `<send type="...">` whose literal names no Event I/O Processor
    /// this build implements.
    SendType {
        /// Owning state, so the report names where the send lives.
        state_id: String,
        /// The `type` attribute verbatim.
        processor_type: String,
    },
    /// `<invoke type="...">` whose literal names no invoker this build
    /// implements.
    InvokeType {
        /// Owning state.
        state_id: String,
        /// `<invoke id="...">` — present for every invoke, auto-derived
        /// when the author wrote none.
        invoke_id: String,
        /// The `type` attribute verbatim.
        processor_type: String,
    },
}

/// A [`HostProcessorCauseKind`] together with where it was written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostProcessorCause {
    pub kind: HostProcessorCauseKind,
    pub location: Option<SourceLocation>,
}

impl HostProcessorCause {
    fn new(kind: HostProcessorCauseKind, location: Option<&SourceLocation>) -> Self {
        Self {
            kind,
            location: location.cloned(),
        }
    }

    /// Project onto the manifest wire shape. Exhaustive by construction —
    /// adding a variant without choosing its wire `kind` does not compile.
    pub fn to_wire(&self) -> HostProcessorCauseRecord {
        use HostProcessorCauseKind as K;
        let base = match &self.kind {
            K::SendType {
                state_id,
                processor_type,
            } => HostProcessorCauseRecord {
                kind: "send-type",
                processor_type: processor_type.clone(),
                state: Some(state_id.clone()),
                invoke: None,
                location: None,
            },
            K::InvokeType {
                state_id,
                invoke_id,
                processor_type,
            } => HostProcessorCauseRecord {
                kind: "invoke-type",
                processor_type: processor_type.clone(),
                state: Some(state_id.clone()),
                invoke: Some(invoke_id.clone()),
                location: None,
            },
        };
        HostProcessorCauseRecord {
            location: self.location.clone(),
            ..base
        }
    }
}

/// Wire projection of one [`HostProcessorCause`] — the shape the
/// `sce-codegen` stdout manifest carries in `host_processor_causes`.
///
/// `needs_host_processor` on its own tells a consumer that some site in
/// the document will refuse at runtime, but not which one. A build
/// gating on the flag would then fail with nothing to act on; these
/// records name the element and the URI, so the gate can point at a line
/// of SCXML.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct HostProcessorCauseRecord {
    /// Stable kebab-case discriminator: `send-type` or `invoke-type`.
    /// Consumers dispatch on this and must tolerate unknown values.
    pub kind: &'static str,
    /// The `type` attribute verbatim, so the report names the URI that
    /// was refused rather than only the element that carried it.
    pub processor_type: String,
    /// Owning state.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    /// `<invoke id="…">`, for the invoke-anchored cause.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invoke: Option<String>,
    /// Where in the source. Same `{file, line, col}` shape a diagnostic
    /// carries, so tooling anchors this exactly as it anchors a
    /// rejection.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<SourceLocation>,
}

/// Walk `model` and return every site naming a processor type this build
/// has no lowering path for.
///
/// Empty iff [`needs_host_processor`] would return `false`.
pub fn analyze(model: &SCXMLModel) -> Vec<HostProcessorCause> {
    let mut causes = Vec::new();
    for (state_id, state) in &model.states {
        collect_state_causes(state_id, state, &mut causes);
    }
    // `model.states` is keyed by state id, so the walk above visits
    // states alphabetically — a document whose `<send>` is written above
    // its `<invoke>` would be reported the other way round. This list is
    // read by a person looking for the line to open, so it is ordered by
    // that line. Sites with no location sort last rather than first:
    // absent position is not position zero.
    causes.sort_by_key(|c| {
        let (line, col) = c
            .location
            .as_ref()
            .map(|l| (l.line.unwrap_or(0), l.col.unwrap_or(0)))
            .unwrap_or((u32::MAX, u32::MAX));
        (line, col)
    });
    causes
}

/// `true` iff `analyze(model)` would return any cause.
///
/// Thin wrapper for callers that only need the answer — `analyze` is the
/// single traversal, and spelling `.is_empty()` at each call site is how
/// a second, subtly different predicate gets written.
pub fn needs_host_processor(model: &SCXMLModel) -> bool {
    !analyze(model).is_empty()
}

/// Record that this build's host serves `types`, and re-decide every
/// site the declaration reaches.
///
/// §scxml-6.2.5 makes the `type` an extensible identifier, so the set of
/// Event I/O Processors is open by design — but only the platform can
/// widen it, and until this function existed nothing in SCE let a
/// platform say so. A consumer could name a processor and get
/// `error.execution`; it could not name one and get delivery.
///
/// Called once, after the parse and before any backend renders, so the
/// three things that must agree cannot drift: what the emitted code
/// does, what [`analyze`] reports, and what the manifest publishes. A
/// declaration applied per-backend instead would let one language
/// deliver a send that another refuses.
///
/// A type outside [`SUPPORTED_SEND_TYPES`] and outside `types` is
/// untouched — still refused, still reported. Declaring one of the two
/// standard processors is a no-op rather than an error: it names
/// something already true.
///
/// A type under [`RESERVED_TYPE_PREFIX`] is refused — see
/// [`ReservedHostType`].
pub fn declare_host_processors(
    model: &mut SCXMLModel,
    types: &[String],
) -> Result<(), ReservedHostType> {
    declare_host_surfaces(model, types, &[])
}

/// The prefix SCE keeps for the Event I/O Processors and invoke types it
/// defines itself: [`MESH_PROCESSOR_TYPE`] and `sce:mesh-rpc`.
///
/// §scxml-6.2.5 leaves the processor set open to the platform, and a
/// platform that shares a namespace with its host has to say which half
/// is whose. SCE's half is `sce:`. A host declaring a type there would
/// not be adding a processor but taking over one of SCE's — a
/// `--host-processor sce:mesh` would make every Mesh send the host's to
/// deliver, with nothing on the wire saying the router had been replaced.
pub const RESERVED_TYPE_PREFIX: &str = "sce:";

/// Whether a host may not declare or register `processor_type`: it starts
/// with [`RESERVED_TYPE_PREFIX`], spelled exactly.
///
/// The build's copy of the rule every runtime's registration applies. Every
/// copy reads `sce-build/tests/fixtures/host_processor/reserved_type_cases.json`,
/// so a type one of them refuses is one they all refuse.
pub fn is_reserved_type(processor_type: &str) -> bool {
    processor_type.starts_with(RESERVED_TYPE_PREFIX)
}

/// Which host declaration a [`ReservedHostType`] came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostSurface {
    /// `<send type>` — `--host-processor`, `host_processor_types`.
    Send,
    /// `<invoke type>` — `--host-invoker`, `host_invoker_types`.
    Invoke,
}

impl HostSurface {
    /// The command-line flag that declares this surface, which is how the
    /// refusal names it: the CLI and the `build.rs` facade take the same
    /// lists, and the flag is the spelling both readers know.
    pub fn flag(self) -> &'static str {
        match self {
            HostSurface::Send => "--host-processor",
            HostSurface::Invoke => "--host-invoker",
        }
    }
}

/// A host declared a type under [`RESERVED_TYPE_PREFIX`].
///
/// Refused rather than ignored: ignoring it would leave the host believing
/// its handler serves the type while SCE's processor does, and the first
/// sign would be an event arriving from the wrong place.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error(
    "{} {type_name}: the `{RESERVED_TYPE_PREFIX}` prefix is reserved for the \
     processors SCE defines itself; a host type needs another prefix, such as `x-`",
    surface.flag()
)]
pub struct ReservedHostType {
    pub surface: HostSurface,
    pub type_name: String,
}

/// The first declared type under [`RESERVED_TYPE_PREFIX`], `<send>` half
/// first.
pub fn check_host_declarations(
    send_types: &[String],
    invoke_types: &[String],
) -> Result<(), ReservedHostType> {
    let surfaces = [
        (HostSurface::Send, send_types),
        (HostSurface::Invoke, invoke_types),
    ];
    for (surface, types) in surfaces {
        if let Some(type_name) = types.iter().find(|t| is_reserved_type(t)) {
            return Err(ReservedHostType {
                surface,
                type_name: type_name.clone(),
            });
        }
    }
    Ok(())
}

/// The reserved `<param>` a host-run `<invoke>` names its deadline with, in
/// milliseconds — read by each runtime, never handed to the host. One name
/// for both invoke types that have a deadline; `sce:mesh-rpc` accepts it
/// beside its own `_mesh_deadline_ms`.
pub const HOST_INVOKE_DEADLINE_PARAM: &str = "_sce_deadline_ms";

/// Read the text of a deadline value as milliseconds: one or more ASCII
/// digits, optionally followed by `.` and one or more `0`, within a signed
/// 64-bit count; anything else is `None`.
///
/// The build's copy of the grammar each runtime applies at run time
/// (`parse_host_invoke_deadline_ms` and its siblings), for the deadlines
/// this crate reads itself — a `sce:mesh-rpc` invoke's, under either name.
/// Every copy is held to one table,
/// `sce-build/tests/fixtures/host_processor/host_invoke_deadline_values.json`,
/// so a deadline the build accepts is one every runtime accepts.
pub fn parse_deadline_ms(written: &str) -> Option<u64> {
    let (whole, fraction) = match written.split_once('.') {
        Some((whole, fraction)) => (whole, Some(fraction)),
        None => (written, None),
    };
    if whole.is_empty() || !whole.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    if let Some(fraction) = fraction {
        if fraction.is_empty() || !fraction.bytes().all(|b| b == b'0') {
            return None;
        }
    }
    // All ASCII digits, so the only way this fails is a value past i64::MAX.
    whole.parse::<i64>().ok().map(|ms| ms as u64)
}

/// [`declare_host_processors`] together with the `<invoke>` half.
///
/// The two are separate lists because they are separate contracts. A host
/// that can deliver an event is not thereby able to run an invoked
/// process with a lifecycle — start, cancel, and a `done.invoke` when it
/// finishes — and a single list would make declaring one silently claim
/// the other. §scxml-6.4.1 leaves the invokable set to the platform in
/// the same words §scxml-6.2.5 uses for `<send>`, and SCE keeps the two
/// answers separable for the same reason the specification states them
/// separately.
///
/// Every route a declaration takes into a build passes through here, so
/// this is where [`check_host_declarations`] runs; the model is left
/// untouched when it refuses.
pub fn declare_host_surfaces(
    model: &mut SCXMLModel,
    send_types: &[String],
    invoke_types: &[String],
) -> Result<(), ReservedHostType> {
    check_host_declarations(send_types, invoke_types)?;
    model.host_processor_types = send_types.to_vec();
    model.host_invoker_types = invoke_types.to_vec();
    if !invoke_types.is_empty() {
        for state in model.states.values_mut() {
            for invoke in &mut state.invokes {
                if let Invoke::Unsupported(info) = invoke {
                    if invoke_types.iter().any(|t| t == &info.invoke_type) {
                        info.host_served = true;
                    }
                }
            }
        }
        model.refresh_invokes_view();
        // §scxml-3.12.1: a host completion falls back to the generic
        // `done.invoke` when the document names no specific one, as an SCXML
        // child's does, so the generic event has to exist for it to land on.
        // Analysis registers it for SCXML and mesh invokes; a host-served one
        // is only known to be served from here.
        if model
            .invokes
            .iter()
            .any(|i| matches!(i, Invoke::Unsupported(info) if info.host_served))
        {
            model.events.insert("done.invoke".to_string());
        }
        // A deadline (`_sce_deadline_ms`) is armed on the engine's scheduler
        // and, if it passes, raises `error.invoke.<id>` — or the generic
        // `error.invoke` — so the machine must be driven with `tick()` and the
        // generic event must exist. `needs_tick_driving` was settled by
        // analysis before this declaration, so it is settled again here.
        if model.invokes.iter().any(|i| {
            matches!(i, Invoke::Unsupported(info) if info.host_served
                && info.base.params.iter().any(|p| p.name == HOST_INVOKE_DEADLINE_PARAM))
        }) {
            model.events.insert("error.invoke".to_string());
            model.needs_event_scheduler = Some(true);
            model.needs_tick_driving = model.needs_event_scheduler_driving();
            model.has_host_invoke_deadline = true;
        }
        // A host-served invoke evaluates its request when it starts, which
        // analysis — run before this declaration — could not know it would.
        crate::script_engine_analyzer::record_host_invoke_causes(model);
    }
    if send_types.is_empty() && invoke_types.is_empty() {
        return Ok(());
    }
    let types = send_types;
    visit_actions_mut(model, &mut |action| claim_action(action, types));
    // Re-derived rather than filtered: the causes are the projection of
    // the flags, and recomputing them from the flags just changed is what
    // keeps `needs_host_processor` and the emitted code the same answer.
    model.host_processor_causes = analyze(model);
    record_delayed_host_sends(model);
    Ok(())
}

/// The most host-run invocations this machine can have in flight at once.
///
/// §scxml-6.4: an `<invoke>` runs while its state is active and is cancelled
/// when the state exits, and a state is in the configuration at most once, so
/// one element is at most one invocation. The count that can be live together
/// is therefore the configuration's: a state's own host-served invokes plus,
/// below it, the SUM over a `<parallel>`'s regions (all active together) or
/// the MAX over a compound state's children (one at a time). The document's
/// top level is exclusive like a compound state.
///
/// A backend holding the started set in fixed storage sizes it against this,
/// so a ceiling too small for the document is a build error instead of a
/// cancel that silently never reaches the host. Counting every site instead
/// would refuse documents whose invocations can never overlap.
pub fn host_invocation_peak(model: &SCXMLModel) -> usize {
    fn peak(model: &SCXMLModel, state_id: &str) -> usize {
        let Some(state) = model.states.get(state_id) else {
            return 0;
        };
        let own = state
            .invokes
            .iter()
            .filter(|i| matches!(i, Invoke::Unsupported(info) if info.host_served))
            .count();
        let children = state.children.iter().map(|c| peak(model, c));
        own + if state.is_parallel {
            children.sum()
        } else {
            children.max().unwrap_or(0)
        }
    }
    model
        .states
        .iter()
        .filter(|(_, s)| s.parent.is_none())
        .map(|(id, _)| peak(model, id))
        .max()
        .unwrap_or(0)
}

/// Whether `action` is a host-served `<send>` the engine must WAIT before
/// performing (§scxml-6.2.4).
///
/// The delay forms are read the way the templates read them: a literal
/// `delay`, its parsed `delay_ms`, or a `delayexpr` whose value is not
/// known until the send runs. A `delayexpr` counts even though it may
/// evaluate to zero — the storage has to exist before the answer does.
fn is_delayed_host_send(action: &Action) -> bool {
    action.send_type_host_served
        && (!action.delayexpr.is_empty()
            || action.delay_ms > 0
            || (!action.delay.is_empty() && action.delay != "0s" && action.delay != "0ms"))
}

/// Set [`SCXMLModel::has_delayed_host_send`] and
/// [`SCXMLModel::delayed_host_send_max_params`] from the claimed sends.
///
/// Runs after [`claim_action`] rather than beside it, because until the
/// declaration is applied every one of these sends is a refusal and a
/// refusal has no delay to honour.
fn record_delayed_host_sends(model: &mut SCXMLModel) {
    let mut found = false;
    let mut max_params = 0usize;
    let mut visit = |action: &Action| {
        if is_delayed_host_send(action) {
            found = true;
            // §scxml-5.10: the host receives the namelist pairs as well as
            // the `<param>` ones, so both count toward the slot's width.
            let pairs = action.params.len() + action.namelist.split_whitespace().count();
            max_params = max_params.max(pairs);
        }
    };
    for state in model.states.values() {
        for trans in &state.transitions {
            walk_actions(&trans.actions, &mut visit);
        }
        for block in state
            .on_entry_blocks
            .iter()
            .chain(state.on_exit_blocks.iter())
        {
            walk_actions(block, &mut visit);
        }
        walk_actions(&state.initial_transition_actions, &mut visit);
        walk_actions(&state.initial_history_default_actions, &mut visit);
    }
    model.has_delayed_host_send = found;
    model.delayed_host_send_max_params = max_params;
}

/// Apply `visit` to every action in `actions` and to everything nested
/// inside them.
///
/// The same recursion [`claim_action`] makes, for the same reason: a
/// `<send>` inside `<if>` / `<foreach>` is as real as a top-level one, and
/// a walk that missed it would leave the C11 entry without the storage
/// that send needs.
fn walk_actions(actions: &[Action], visit: &mut impl FnMut(&Action)) {
    for action in actions {
        visit(action);
        for block in action.nested_blocks() {
            walk_actions(block.actions, visit);
        }
    }
}

/// The `<send type>` a Mesh send is lowered to on every backend whose Mesh
/// router is a host processor (SCE_MESH.md §mesh-18.4) — every backend but
/// C++, whose generated `TransportRouter` takes the send through its own
/// mesh-send hook.
pub const MESH_PROCESSOR_TYPE: &str = "sce:mesh";

/// The peer a `<send target>` names, when it names one: `#` followed by at
/// least one character, where `#_` stays reserved for the targets
/// §scxml-6.2.4 defines (`#_internal`, `#_parent`, `#_<invokeid>`, ...).
///
/// The build's copy of the predicate the C++ core's `SendHelper::isMeshTarget`
/// and the Rust and Kotlin cores' `mesh_peer` apply at run time. Every copy
/// reads `tests/mesh/mesh_target_cases.json`, so a target one of them routes
/// over Mesh is one they all do.
pub fn mesh_peer(target: &str) -> Option<&str> {
    target
        .strip_prefix('#')
        .filter(|peer| !peer.is_empty() && !peer.starts_with('_'))
}

/// Whether `action` is a `<send>` to a Mesh peer that the build can see as
/// one: a literal `target` naming a peer, through the SCXML Event I/O
/// Processor (written or defaulted), with no `typeexpr` or `targetexpr` that
/// could make it something else at run time.
fn is_static_mesh_send(action: &Action) -> bool {
    action.action_type == "send"
        && action.targetexpr.is_empty()
        && action.typeexpr.is_empty()
        && (action.send_type.is_empty() || action.send_type == SCXML_EVENT_PROCESSOR_TYPE)
        && mesh_peer(&action.target).is_some()
}

/// Whether `language` delivers a Mesh send through a host-registered router
/// rather than a generated one.
pub fn routes_mesh_through_host(language: Language) -> bool {
    !matches!(language, Language::Cpp)
}

/// Whether a machine generated for `language` needs its host to register a
/// Mesh router: it sends to a peer, and the peer is reached through the host.
///
/// Published on the manifest beside `needs_host_processor`, so a host learns
/// from the build — not from a first `error.execution` — that this machine
/// talks to other machines.
pub fn needs_mesh_router(model: &SCXMLModel, language: Language) -> bool {
    routes_mesh_through_host(language) && sends_to_a_mesh_peer(model)
}

/// Whether any `<send>` in `model` names a Mesh peer the build can see — the
/// document fact [`needs_mesh_router`] qualifies by language.
pub fn sends_to_a_mesh_peer(model: &SCXMLModel) -> bool {
    any_action(model, &mut is_static_mesh_send)
}

/// `model` as `language` renders it: on a backend whose Mesh router is a host
/// processor, every `<send target="#peer">` becomes a host-served
/// `sce:mesh` send.
///
/// The send templates already carry a host-served send the whole way — its
/// arguments evaluated once, its `_event.data` as a local delivery would have
/// had it, its delay honoured and its `<cancel>` reached — so a Mesh send
/// is one more of them rather than a second path. Lowered per language
/// rather than once after the parse because C++ keeps its own route; the
/// model is borrowed untouched when there is nothing to lower.
pub fn lower_mesh_sends(model: &SCXMLModel, language: Language) -> Cow<'_, SCXMLModel> {
    if !needs_mesh_router(model, language) {
        return Cow::Borrowed(model);
    }
    let mut lowered = model.clone();
    visit_actions_mut(&mut lowered, &mut |action| {
        if is_static_mesh_send(action) {
            action.send_type = MESH_PROCESSOR_TYPE.to_string();
            action.send_type_unsupported = false;
            action.send_type_host_served = true;
        }
    });
    // The host serves this type for this machine, so it is one of the host's
    // types exactly as a declared one is: every template that emits the
    // host-send surface — the C11 registry, its `perform_host_send` and its
    // registration entry points — is gated on this list, and a lowered send
    // with no surface behind it does not compile. Recorded on the lowered
    // copy only, so the manifest still echoes what the host declared.
    if !lowered
        .host_processor_types
        .iter()
        .any(|t| t == MESH_PROCESSOR_TYPE)
    {
        lowered
            .host_processor_types
            .push(MESH_PROCESSOR_TYPE.to_string());
    }
    // A delayed Mesh send waits on the delayed-send queue as any delayed
    // host-served send does, so the queue's storage is sized with it.
    record_delayed_host_sends(&mut lowered);
    Cow::Owned(lowered)
}

/// Whether `pred` holds for any action in `model`, nested ones included.
fn any_action(model: &SCXMLModel, pred: &mut impl FnMut(&Action) -> bool) -> bool {
    let mut found = false;
    let mut visit = |action: &Action| found |= pred(action);
    for state in model.states.values() {
        for trans in &state.transitions {
            walk_actions(&trans.actions, &mut visit);
        }
        for block in state
            .on_entry_blocks
            .iter()
            .chain(state.on_exit_blocks.iter())
        {
            walk_actions(block, &mut visit);
        }
        walk_actions(&state.initial_transition_actions, &mut visit);
        walk_actions(&state.initial_history_default_actions, &mut visit);
    }
    found
}

/// Apply `visit` to every action in `model` and everything nested inside it.
fn visit_actions_mut(model: &mut SCXMLModel, visit: &mut impl FnMut(&mut Action)) {
    fn walk(actions: &mut [Action], visit: &mut impl FnMut(&mut Action)) {
        for action in actions {
            visit(action);
            for block in action.nested_blocks_mut() {
                walk(block, visit);
            }
        }
    }
    for state in model.states.values_mut() {
        for trans in &mut state.transitions {
            walk(&mut trans.actions, visit);
        }
        for block in state
            .on_entry_blocks
            .iter_mut()
            .chain(state.on_exit_blocks.iter_mut())
        {
            walk(block, visit);
        }
        walk(&mut state.initial_transition_actions, visit);
        walk(&mut state.initial_history_default_actions, visit);
    }
}

/// Move one `<send>` from "refused" to "dispatched to the host", if the
/// declaration names its type.
///
/// Reached through [`visit_actions_mut`], which recurses for the reason
/// [`collect_action_causes`] does: a `<send>` nested in `<if>` /
/// `<foreach>` must be claimed by a declaration exactly as a top-level one
/// is, or the same document delivers in one position and refuses in the
/// other.
fn claim_action(action: &mut Action, types: &[String]) {
    if action.send_type_unsupported && types.iter().any(|t| t == &action.send_type) {
        action.send_type_unsupported = false;
        action.send_type_host_served = true;
    }
}

fn collect_state_causes(state_id: &str, state: &State, out: &mut Vec<HostProcessorCause>) {
    for trans in &state.transitions {
        for action in &trans.actions {
            collect_action_causes(state_id, action, out);
        }
    }
    for block in state
        .on_entry_blocks
        .iter()
        .chain(state.on_exit_blocks.iter())
    {
        for action in block {
            collect_action_causes(state_id, action, out);
        }
    }
    for action in &state.initial_transition_actions {
        collect_action_causes(state_id, action, out);
    }
    for action in &state.initial_history_default_actions {
        collect_action_causes(state_id, action, out);
    }
    for invoke in &state.invokes {
        collect_invoke_causes(state_id, invoke, out);
    }
}

/// Recurse through the containers that hold executable content, so a
/// `<send>` nested in `<if>` / `<foreach>` is reported like a top-level
/// one. A walk that only read the block's own actions would report the
/// flat case and stay silent on the nested one — the same document,
/// two answers.
fn collect_action_causes(state_id: &str, action: &Action, out: &mut Vec<HostProcessorCause>) {
    if action.send_type_unsupported {
        out.push(HostProcessorCause::new(
            HostProcessorCauseKind::SendType {
                state_id: state_id.to_string(),
                processor_type: action.send_type.clone(),
            },
            action.source_location.as_ref(),
        ));
    }
    // Taken from `Action::nested_blocks` rather than matched on
    // `action_type`, so a container added to the model is walked by
    // default — the failure mode of the other spelling is silence. This
    // walk used to chain the four fields by hand, which got the default
    // right for the containers it knew and no others.
    for block in action.nested_blocks() {
        for nested in block.actions {
            collect_action_causes(state_id, nested, out);
        }
    }
}

fn collect_invoke_causes(state_id: &str, invoke: &Invoke, out: &mut Vec<HostProcessorCause>) {
    // §scxml-6.4.1: an `<invoke>` naming a type the platform does not
    // support raises `error.execution` and starts no child. The parser
    // already classified this site into `Invoke::Unsupported` so the
    // runtime observable exists; reading that classification here is what
    // gives the build one too.
    let Invoke::Unsupported(info) = invoke else {
        return;
    };
    // A declared invoker means the host runs this one, so there is no
    // longer a site with no path — which is what this list reports.
    if info.host_served {
        return;
    }
    out.push(HostProcessorCause::new(
        HostProcessorCauseKind::InvokeType {
            state_id: state_id.to_string(),
            invoke_id: info.base.invoke_id.clone(),
            processor_type: info.invoke_type.clone(),
        },
        info.base.source_location.as_ref(),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::SCXMLParser;

    fn parse(scxml: &str) -> SCXMLModel {
        SCXMLParser::new().parse_string(scxml, "test").unwrap()
    }

    /// tests/mesh/mesh_target_cases.json: the table every copy of the
    /// mesh-target predicate reads.
    #[test]
    fn a_mesh_peer_is_read_by_the_shared_table() {
        let table: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/mesh/mesh_target_cases.json"))
                .expect("the table is JSON");
        let cases = table["cases"].as_array().expect("the table has cases");
        assert!(cases.len() >= 10, "the table lost cases: {}", cases.len());
        for case in cases {
            let target = case["target"].as_str().expect("a target is a string");
            assert_eq!(mesh_peer(target), case["peer"].as_str(), "{target:?}");
        }
    }

    /// tests/fixtures/host_processor/reserved_type_cases.json: the table every copy
    /// of the reserved-type rule reads, at build time and at run time.
    #[test]
    fn a_reserved_type_is_read_by_the_shared_table() {
        let table: serde_json::Value = serde_json::from_str(include_str!(
            "../tests/fixtures/host_processor/reserved_type_cases.json"
        ))
        .expect("the table is JSON");
        let cases = table["cases"].as_array().expect("the table has cases");
        assert!(cases.len() >= 10, "the table lost cases: {}", cases.len());
        for case in cases {
            let name = case["type"].as_str().expect("a type is a string");
            let reserved = case["reserved"].as_bool().expect("reserved is a bool");
            assert_eq!(is_reserved_type(name), reserved, "{name:?}");
        }
    }

    /// Every send the lowering must reach and every one it must leave.
    const MESH_SENDS: &str = r##"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
        datamodel="ecmascript" initial="s">
      <datamodel><data id="t" expr="'#hmi'"/></datamodel>
      <state id="s">
        <onentry>
          <send event="plain" target="#hmi"/>
          <send event="typed" target="#hmi" type="http://www.w3.org/TR/scxml/#SCXMLEventProcessor"/>
          <if cond="true"><send event="nested" target="#hmi"/></if>
          <send event="parent" target="#_parent"/>
          <send event="dynamic" targetexpr="t"/>
          <send event="typedexpr" target="#hmi" typeexpr="'x'"/>
          <send event="http" target="#hmi" type="http://www.w3.org/TR/scxml/#BasicHTTPEventProcessor"/>
        </onentry>
      </state>
    </scxml>"##;

    fn sends(model: &SCXMLModel) -> Vec<(String, String, bool)> {
        let mut out = Vec::new();
        any_action(model, &mut |a: &Action| {
            if a.action_type == "send" {
                out.push((
                    a.event.clone(),
                    a.send_type.clone(),
                    a.send_type_host_served,
                ));
            }
            false
        });
        out.sort();
        out
    }

    #[test]
    fn a_static_peer_send_is_lowered_to_the_mesh_router_and_nothing_else_is() {
        let model = parse(MESH_SENDS);
        assert!(needs_mesh_router(&model, Language::Rust));
        let lowered = lower_mesh_sends(&model, Language::Rust);
        let mesh: Vec<_> = sends(&lowered)
            .into_iter()
            .filter(|(_, ty, served)| ty == MESH_PROCESSOR_TYPE && *served)
            .map(|(event, _, _)| event)
            .collect();
        assert_eq!(mesh, ["nested", "plain", "typed"]);
        // Every other send is exactly what the parse made of it.
        let untouched = |m: &SCXMLModel| {
            sends(m)
                .into_iter()
                .filter(|(event, _, _)| !["nested", "plain", "typed"].contains(&event.as_str()))
                .collect::<Vec<_>>()
        };
        assert_eq!(untouched(&lowered), untouched(&model));
        // The lowered copy lists the type among the host's, so every template
        // gated on that list emits the host-send surface a lowered send calls
        // (the C11 registry did not, and the machine did not compile); the
        // parsed model — what the manifest echoes — is left as declared.
        assert_eq!(lowered.host_processor_types, [MESH_PROCESSOR_TYPE]);
        assert!(model.host_processor_types.is_empty());
    }

    #[test]
    fn cpp_keeps_its_own_route_and_is_left_unlowered() {
        let model = parse(MESH_SENDS);
        assert!(!needs_mesh_router(&model, Language::Cpp));
        assert!(matches!(
            lower_mesh_sends(&model, Language::Cpp),
            Cow::Borrowed(_)
        ));
    }

    #[test]
    fn a_document_without_a_peer_send_is_borrowed_as_it_is() {
        let model = parse(
            r##"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s">
                 <state id="s"><onentry><send event="e" target="#_internal"/></onentry></state>
               </scxml>"##,
        );
        assert!(!needs_mesh_router(&model, Language::Kotlin));
        assert!(matches!(
            lower_mesh_sends(&model, Language::Kotlin),
            Cow::Borrowed(_)
        ));
    }

    #[test]
    fn a_delayed_peer_send_waits_on_the_delayed_send_queue() {
        let model = parse(
            r##"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s">
                 <state id="s"><onentry>
                   <send event="later" target="#hmi" delay="1s" namelist="x"/>
                 </onentry></state>
               </scxml>"##,
        );
        assert!(!model.has_delayed_host_send);
        let lowered = lower_mesh_sends(&model, Language::C11);
        assert!(lowered.has_delayed_host_send);
        assert_eq!(lowered.delayed_host_send_max_params, 1);
    }

    /// The build reads a deadline by the grammar every runtime reads it by,
    /// held to the same table they are.
    #[test]
    fn a_deadline_is_read_by_the_shared_table() {
        let table: serde_json::Value = serde_json::from_str(include_str!(
            "../tests/fixtures/host_processor/host_invoke_deadline_values.json"
        ))
        .expect("the table is JSON");
        let accepted = table["accepted"].as_array().expect("`accepted` is a list");
        let refused = table["refused"].as_array().expect("`refused` is a list");
        // A floor: an empty table would pass every assertion below.
        assert!(
            !accepted.is_empty() && !refused.is_empty(),
            "the table is empty"
        );
        for pair in accepted {
            let written = pair[0].as_str().expect("written is a string");
            let ms: u64 = pair[1]
                .as_str()
                .expect("ms is a string")
                .parse()
                .expect("ms is a number");
            assert_eq!(parse_deadline_ms(written), Some(ms), "{written:?}");
        }
        for written in refused {
            let written = written.as_str().expect("a refused value is a string");
            assert_eq!(parse_deadline_ms(written), None, "{written:?} was accepted");
        }
    }

    /// Wrap `body` in the smallest document that parses, so each test
    /// reads as the one construct it is about.
    fn doc(body: &str) -> String {
        format!(
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s">{body}</scxml>"#
        )
    }

    /// The peak is the configuration's, not the document's: regions of a
    /// `<parallel>` add up, siblings of a compound state do not, and an
    /// invoke the host does not serve is not counted at all. Five host sites
    /// here, of which at most three are live together.
    #[test]
    fn the_host_invocation_peak_is_the_largest_configuration() {
        let mut model = parse(&doc(r#"<state id="s" initial="p">
                 <invoke id="top" type="x-host"/>
                 <parallel id="p">
                   <state id="a"><invoke id="a1" type="x-host"/></state>
                   <state id="b"><invoke id="b1" type="x-host"/></state>
                 </parallel>
                 <state id="q">
                   <invoke id="q1" type="x-host"/>
                   <invoke id="q2" type="x-other"/>
                 </state>
               </state>
               <state id="t"><invoke id="t1" type="x-host"/></state>"#));
        assert_eq!(
            host_invocation_peak(&model),
            0,
            "nothing is host-served yet"
        );
        declare_host_surfaces(&mut model, &[], &["x-host".to_string()])
            .expect("x- is not reserved");
        // `s` + both regions of `p` = 3; `s` + `q` = 2 (q2 is not served);
        // `t` = 1.
        assert_eq!(host_invocation_peak(&model), 3);
    }

    #[test]
    fn the_two_types_the_specification_names_are_supported() {
        assert!(is_supported_send_type(SCXML_EVENT_PROCESSOR_TYPE));
        assert!(is_supported_send_type(BASIC_HTTP_EVENT_PROCESSOR_TYPE));
    }

    /// §scxml-6.2 makes an absent `type` the SCXML Event I/O Processor,
    /// so the empty string is the default and not an unknown value. Read
    /// the other way this is the false-positive guard: every `<send>`
    /// without a `type` attribute reaches this predicate, so getting it
    /// wrong would report the whole corpus.
    #[test]
    fn an_absent_type_is_the_default_processor_not_an_unknown_one() {
        assert!(is_supported_send_type(""));
        let model = parse(&doc(
            r#"<state id="s"><onentry><send event="e"/></onentry></state>"#,
        ));
        assert!(analyze(&model).is_empty());
        assert!(!needs_host_processor(&model));
    }

    #[test]
    fn a_type_outside_the_set_is_not_supported() {
        assert!(!is_supported_send_type("x-example-host"));
        // Near-misses, because a substring or prefix comparison would
        // accept these and no fixture in the corpus would notice.
        assert!(!is_supported_send_type("http://www.w3.org/TR/scxml/#"));
        assert!(!is_supported_send_type(
            "http://www.w3.org/TR/scxml/#SCXMLEventProcessorX"
        ));
    }

    #[test]
    fn an_unsupported_send_type_is_reported_with_its_state_and_uri() {
        let model = parse(&doc(
            r#"<state id="s"><onentry><send type="x-example-host" event="e"/></onentry></state>"#,
        ));
        let causes = analyze(&model);
        assert_eq!(causes.len(), 1, "{causes:?}");
        assert_eq!(
            causes[0].kind,
            HostProcessorCauseKind::SendType {
                state_id: "s".to_string(),
                processor_type: "x-example-host".to_string(),
            }
        );
        assert!(needs_host_processor(&model));
    }

    #[test]
    fn an_unsupported_invoke_type_is_reported_with_its_invoke_id() {
        let model = parse(&doc(
            r#"<state id="s"><invoke id="probe" type="x-example-host"/></state>"#,
        ));
        let causes = analyze(&model);
        assert_eq!(causes.len(), 1, "{causes:?}");
        assert_eq!(
            causes[0].kind,
            HostProcessorCauseKind::InvokeType {
                state_id: "s".to_string(),
                invoke_id: "probe".to_string(),
                processor_type: "x-example-host".to_string(),
            }
        );
    }

    /// A `<send>` inside `<if>` / `<foreach>` refuses at runtime exactly
    /// as a top-level one does, so a walk that only read the block's own
    /// actions would give the same document two answers depending on
    /// where the author put it.
    #[test]
    fn a_send_nested_in_executable_content_is_still_reported() {
        let model = parse(&doc(r#"<state id="s"><onentry>
                 <if cond="true"><send type="x-in-then" event="a"/>
                 <else/><send type="x-in-else" event="b"/></if>
                 <foreach array="xs" item="x"><send type="x-in-foreach" event="c"/></foreach>
               </onentry></state>"#));
        let found: Vec<String> = analyze(&model)
            .iter()
            .map(|c| match &c.kind {
                HostProcessorCauseKind::SendType { processor_type, .. } => processor_type.clone(),
                other => panic!("expected a send cause, got {other:?}"),
            })
            .collect();
        assert_eq!(found, vec!["x-in-then", "x-in-else", "x-in-foreach"]);
    }

    /// The list is read by a person looking for the line to open, so it
    /// runs down the document. `model.states` is keyed by state id, so
    /// without the sort this document reports `later` before `earlier`.
    #[test]
    fn causes_are_ordered_by_source_position_not_by_state_id() {
        let model = parse(&doc(
            r#"<state id="zebra"><onentry><send type="x-first" event="a"/></onentry>
                 <transition event="go" target="alpha"/></state>
               <state id="alpha"><onentry><send type="x-second" event="b"/></onentry></state>"#,
        ));
        let causes = analyze(&model);
        let lines: Vec<u32> = causes
            .iter()
            .map(|c| c.location.as_ref().and_then(|l| l.line).unwrap_or(0))
            .collect();
        let types: Vec<&str> = causes
            .iter()
            .map(|c| match &c.kind {
                HostProcessorCauseKind::SendType { processor_type, .. } => processor_type.as_str(),
                other => panic!("expected send causes, got {other:?}"),
            })
            .collect();
        assert_eq!(types, vec!["x-first", "x-second"], "lines were {lines:?}");
        assert!(lines[0] <= lines[1], "not ordered by line: {lines:?}");
    }

    /// The model flag every backend's send template reads must agree
    /// with the predicate — they are one decision, and a template
    /// reading a flag the analyzer did not set would emit a send where
    /// the report promised a refusal.
    #[test]
    fn the_template_flag_agrees_with_the_predicate() {
        let model = parse(&doc(r#"<state id="s"><onentry>
                 <send event="default"/>
                 <send type="http://www.w3.org/TR/scxml/#SCXMLEventProcessor" event="named"/>
                 <send type="x-example-host" event="refused"/>
               </onentry></state>"#));
        let flags: Vec<bool> = model.states["s"].on_entry_blocks[0]
            .iter()
            .map(|a| a.send_type_unsupported)
            .collect();
        assert_eq!(flags, vec![false, false, true]);
        for action in &model.states["s"].on_entry_blocks[0] {
            assert_eq!(
                action.send_type_unsupported,
                !action.send_type.is_empty() && !is_supported_send_type(&action.send_type),
                "flag disagrees with the predicate for type {:?}",
                action.send_type,
            );
        }
    }

    /// The wire `kind` tokens are the consumer's dispatch key. Renaming
    /// a Rust variant must not move them, which only holds if something
    /// reads them.
    #[test]
    fn wire_kinds_are_the_documented_tokens() {
        let send = HostProcessorCause::new(
            HostProcessorCauseKind::SendType {
                state_id: "s".into(),
                processor_type: "x".into(),
            },
            None,
        );
        let invoke = HostProcessorCause::new(
            HostProcessorCauseKind::InvokeType {
                state_id: "s".into(),
                invoke_id: "i".into(),
                processor_type: "x".into(),
            },
            None,
        );
        assert_eq!(send.to_wire().kind, "send-type");
        assert_eq!(invoke.to_wire().kind, "invoke-type");
        // The URI is what a reader acts on; an empty one would make the
        // record name only the element.
        assert_eq!(send.to_wire().processor_type, "x");
        assert_eq!(invoke.to_wire().invoke.as_deref(), Some("i"));
    }

    /// The declaration is what the whole second half of this feature
    /// turns on: a `<send>` moves from refused to dispatched, and the
    /// report stops naming it because there is no longer anything to
    /// report.
    #[test]
    fn a_declaration_moves_a_send_from_refused_to_host_served() {
        let mut model = parse(&doc(
            r#"<state id="s"><onentry><send type="x-example-host" event="e"/></onentry></state>"#,
        ));
        assert!(needs_host_processor(&model));

        declare_host_processors(&mut model, &["x-example-host".to_string()])
            .expect("x- is not reserved");

        let action = &model.states["s"].on_entry_blocks[0][0];
        assert!(!action.send_type_unsupported, "still marked refused");
        assert!(action.send_type_host_served, "not marked host-served");
        assert!(
            model.host_processor_causes.is_empty(),
            "a served type is still reported: {:?}",
            model.host_processor_causes,
        );
    }

    /// The two flags are the two arms of one template branch. A site
    /// carrying both would emit a refusal and a dispatch for one
    /// `<send>`; a site carrying neither is an ordinary send.
    #[test]
    fn the_two_send_verdicts_are_mutually_exclusive() {
        let mut model = parse(&doc(r#"<state id="s"><onentry>
                 <send event="plain"/>
                 <send type="x-example-host" event="served"/>
                 <send type="x-other-host" event="refused"/>
               </onentry></state>"#));
        declare_host_processors(&mut model, &["x-example-host".to_string()])
            .expect("x- is not reserved");
        let verdicts: Vec<(bool, bool)> = model.states["s"].on_entry_blocks[0]
            .iter()
            .map(|a| (a.send_type_unsupported, a.send_type_host_served))
            .collect();
        assert_eq!(verdicts, vec![(false, false), (false, true), (true, false)]);
    }

    /// The false-positive direction: a declaration names one type and
    /// must not claim its neighbours. Without this the feature would
    /// silently deliver sends the host never agreed to serve.
    #[test]
    fn a_declaration_claims_only_the_type_it_names() {
        let mut model = parse(&doc(
            r#"<state id="s"><onentry><send type="x-example-host-2" event="e"/></onentry></state>"#,
        ));
        declare_host_processors(&mut model, &["x-example-host".to_string()])
            .expect("x- is not reserved");
        let action = &model.states["s"].on_entry_blocks[0][0];
        assert!(action.send_type_unsupported, "a neighbour type was claimed");
        assert!(!action.send_type_host_served);
        assert_eq!(model.host_processor_causes.len(), 1);
    }

    /// The invoke half of the same false-positive guard.
    ///
    /// Added after a mutation SURVIVED: widening the invoke claim to `true`
    /// — one declared invoker taking over EVERY unsupported `<invoke>` in
    /// the document — reddened nothing. The two tests that looked like they
    /// covered it could not: one declares only a send type, so the
    /// `invoke_types.is_empty()` guard short-circuits before the claim runs,
    /// and the other names the same type the document names, where "claims
    /// everything" and "claims exactly this" give identical answers. Only a
    /// declaration naming a type the document does NOT carry separates them.
    #[test]
    fn an_invoker_declaration_claims_only_the_type_it_names() {
        let mut model = parse(&doc(
            r#"<state id="s"><invoke id="probe" type="x-example-host-2"/></state>"#,
        ));
        declare_host_surfaces(&mut model, &[], &["x-example-host".to_string()])
            .expect("x- is not reserved");

        let Invoke::Unsupported(info) = &model.states["s"].invokes[0] else {
            panic!("the fixture's <invoke> stopped being classified Unsupported");
        };
        assert!(
            !info.host_served,
            "a neighbour invoke type was claimed by the declaration",
        );
        assert_eq!(
            model.host_processor_causes.len(),
            1,
            "an unclaimed <invoke> stopped being reported: {:?}",
            model.host_processor_causes,
        );
    }

    /// The positive direction of the same walk, so the guard above cannot
    /// be satisfied by an implementation that claims nothing at all.
    #[test]
    fn an_invoker_declaration_claims_the_type_it_does_name() {
        let mut model = parse(&doc(
            r#"<state id="s"><invoke id="probe" type="x-example-host"/></state>"#,
        ));
        declare_host_surfaces(&mut model, &[], &["x-example-host".to_string()])
            .expect("x- is not reserved");

        let Invoke::Unsupported(info) = &model.states["s"].invokes[0] else {
            panic!("the fixture's <invoke> stopped being classified Unsupported");
        };
        assert!(info.host_served, "the declared invoke type was not claimed");
        assert!(
            model.host_processor_causes.is_empty(),
            "a claimed <invoke> is still reported as having no path: {:?}",
            model.host_processor_causes,
        );
    }

    /// A declaration reaches a nested `<send>` too, or the same document
    /// dispatches at the top level and refuses inside an `<if>`.
    #[test]
    fn a_declaration_reaches_nested_executable_content() {
        let mut model = parse(&doc(r#"<state id="s"><onentry>
                 <if cond="true"><send type="x-example-host" event="a"/></if>
               </onentry></state>"#));
        declare_host_processors(&mut model, &["x-example-host".to_string()])
            .expect("x- is not reserved");
        assert!(
            model.host_processor_causes.is_empty(),
            "a nested send was left refused: {:?}",
            model.host_processor_causes,
        );
    }

    /// An `<invoke>` is a separate contract with a separate lifecycle,
    /// so a send-side declaration must not quietly claim it — the build
    /// would then promise a child nothing starts.
    #[test]
    fn a_send_declaration_does_not_claim_an_invoke() {
        let mut model = parse(&doc(
            r#"<state id="s"><invoke id="probe" type="x-example-host"/></state>"#,
        ));
        declare_host_processors(&mut model, &["x-example-host".to_string()])
            .expect("x- is not reserved");
        assert_eq!(
            model.host_processor_causes.len(),
            1,
            "the invoke stopped being reported: {:?}",
            model.host_processor_causes,
        );
    }

    /// Declaring nothing is the overwhelmingly common build, and it must
    /// leave every verdict exactly where the parse put it.
    #[test]
    fn an_empty_declaration_changes_nothing() {
        let mut model = parse(&doc(
            r#"<state id="s"><onentry><send type="x-example-host" event="e"/></onentry></state>"#,
        ));
        let before = analyze(&model);
        declare_host_processors(&mut model, &[]).expect("an empty declaration is not reserved");
        assert_eq!(analyze(&model), before);
        assert!(model.states["s"].on_entry_blocks[0][0].send_type_unsupported);
        assert!(model.host_processor_types.is_empty());
    }

    /// A host cannot take over one of SCE's own processors by declaring
    /// its type: each surface refuses the prefix and names the flag it came
    /// through, and the refused model is the parsed one, unchanged.
    #[test]
    fn a_declaration_under_the_reserved_prefix_is_refused() {
        let body = r#"<state id="s"><onentry><send type="sce:mesh" event="e"/></onentry></state>"#;
        for (send, invoke, surface) in [
            (
                vec![MESH_PROCESSOR_TYPE.to_string()],
                vec![],
                HostSurface::Send,
            ),
            (
                vec![],
                vec!["sce:mesh-rpc".to_string()],
                HostSurface::Invoke,
            ),
        ] {
            let mut model = parse(&doc(body));
            let before = analyze(&model);
            let refused = declare_host_surfaces(&mut model, &send, &invoke)
                .expect_err("a type under the reserved prefix must be refused");
            assert_eq!(refused.surface, surface);
            assert!(refused.to_string().starts_with(surface.flag()));
            assert!(model.host_processor_types.is_empty());
            assert!(model.host_invoker_types.is_empty());
            assert_eq!(analyze(&model), before);
        }
        // The prefix is the whole rule: a name that merely contains it is
        // a host's to use.
        let mut model = parse(&doc(body));
        declare_host_processors(&mut model, &["x-sce:mesh".to_string()])
            .expect("only a leading prefix is reserved");
    }

    /// The set is decided here at build time and again by
    /// `SendHelper::isSupportedSendType` at Interpreter evaluation time
    /// — two engines answering one question. They are allowed to be two
    /// implementations; they are not allowed to be two answers, and
    /// nothing but this test reads them against each other.
    #[test]
    fn the_cpp_runtime_spelling_names_the_same_set() {
        let header = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../sce/include/common/SendHelper.h"
        ))
        .expect("the C++ send helper is part of this workspace");
        let body = header
            .split_once("static bool isSupportedSendType(")
            .expect("SendHelper still declares isSupportedSendType")
            .1;
        let body = &body[..body.find("\n    }").expect("the function is brace-closed")];
        for accepted in SUPPORTED_SEND_TYPES {
            // The SCXML processor URI reaches the C++ side through a
            // named constant rather than a literal, so accept either
            // spelling — what is being pinned is that the URI is
            // reachable from that function, not how it is written.
            let named_constant = accepted
                .rsplit_once('#')
                .map(|(_, frag)| body.contains(frag))
                .unwrap_or(false);
            assert!(
                body.contains(accepted) || named_constant,
                "the build accepts {accepted} but the C++ runtime check does not name it",
            );
        }
    }
}
