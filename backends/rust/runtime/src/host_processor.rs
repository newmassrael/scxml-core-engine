// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Host-supplied Event I/O Processors — the payload types a host
//! registers a handler for.
//!
//! §scxml-6.2.5 makes a `<send>` `type` an extensible identifier, so the
//! set of Event I/O Processors is open by design. SCE implements two of
//! them; anything else was refused with `error.execution` and there was
//! no way for a platform to widen the set — a consumer could name a
//! processor and be refused, but could not name one and be served.
//!
//! A host declares the types it serves at build time (so codegen emits a
//! dispatch instead of a refusal) and registers a handler for each at
//! run time. The two halves must agree: a type dispatched with nothing
//! registered raises `error.execution` exactly as an undeclared one
//! would, because from the document's point of view nothing performed
//! the act either way.
//!
//! Shaped after [`crate::http`], which is the same idea fixed to one
//! type: a request struct carrying what `<send>` said, and an optional
//! response the engine turns back into an event. The difference is the
//! key — this one is looked up by the `type` string.
//!
//! Gated to `!no_std` for the reason the HTTP hook is: the registry is a
//! heap-allocated map of boxed closures, and a `no_std` build has no
//! allocator to hold it.

#![cfg(not(feature = "no_std"))]

use std::collections::HashMap;

/// What a `<send>` addressed to a host-served processor said.
///
/// Every field is what the document wrote, not an interpretation of it:
/// a handler that wants to reject a malformed request needs to see the
/// same thing the author typed.
#[derive(Debug, Clone, Default)]
pub struct HostSendRequest {
    /// The `type` this send named. Present even though the handler was
    /// looked up by it, because one handler may be registered for
    /// several types and would otherwise have to be told which it is by
    /// a closure capture per registration.
    pub processor_type: String,
    /// `<send event="...">`, or the value `eventexpr` evaluated to.
    pub event_name: String,
    /// `<send target="...">`, empty when the document named none.
    /// §scxml-6.2 leaves the meaning of a target to the processor, so
    /// SCE passes it through without interpreting it.
    pub target: String,
    /// Inline `<content>`, empty when the document carried none.
    pub content: String,
    /// `<param>` values, keyed by name. A repeated name keeps every
    /// value in document order rather than the last one winning —
    /// §scxml-6.2 permits repetition and dropping it would lose data the
    /// author wrote.
    pub params: HashMap<String, Vec<String>>,
    /// The send's id (§scxml-6.2.4), auto-generated when the document
    /// declared none. A handler correlating a reply, or honouring a
    /// `<cancel>`, needs it.
    pub send_id: String,
}

/// One event a host-served act produced.
///
/// The engine raises each on the external queue, which is where a reply
/// from outside the machine belongs (§scxml-C-1).
///
/// A handler answers with a LIST of these, in the order the document
/// should see them — see `HostSendHandler`. Empty is "performed,
/// nothing to report", which is the common case for a fire-and-forget
/// act and for real work that will answer later through the host's own
/// loop.
///
/// Named without a link because this type is `pub` and that one is
/// `pub(crate)`: an intra-doc link from public documentation to a
/// private item is what `rustdoc-links` refuses.
#[derive(Debug, Clone, Default)]
pub struct HostSendResponse {
    /// Event to raise. An unknown name is dropped, matching what the
    /// engine does with any event the generated machine does not
    /// declare.
    pub event_name: String,
    /// `_event.data` for the raised event (§scxml-5.10.1).
    pub event_data: String,
}

/// A registered handler and the type it answers for.
///
/// Answers with the events the act produced, IN ORDER. A list rather
/// than a single reply, for one reason: an act can produce two
/// observations the document must see in a particular order, and every
/// other way of expressing that costs portability or hides state.
///
/// `examples/ai_loop/` is the case. Its `priming` state leaves on
/// `prompt.sent` — "the session has been told what it is here for" — and
/// only then is the machine somewhere a turn result means anything; its
/// own comment says reporting the turn first leaves the run sitting in
/// `priming` forever. So prompting a fresh session produces exactly two
/// events with exactly one correct order.
///
/// The two alternatives were measured and rejected:
///
/// * Let the handler re-enter the engine and raise the extra events. A
///   C++ handler can, because it is called through a `std::function`
///   while only the queue is being mutated; this one cannot, because
///   [`HostProcessorRegistry::handler_for`] hands out a `&mut` borrowed
///   from the engine. A host written against the C++ freedom would not
///   port — the single-engine door this surface exists to remove.
/// * Return one event and have the host deliver the rest on its next
///   step. That works on both engines and puts a pending slot back in
///   the host — the hidden host-side state that moving an act into the
///   document is supposed to remove.
///
/// A list needs no re-entrancy on any backend, so the engines are
/// equivalent by construction rather than by agreement, and the order is
/// the one the host wrote down.
pub(crate) type HostSendHandler =
    Box<dyn FnMut(HostSendRequest) -> Vec<HostSendResponse> + Send + 'static>;

/// The reserved `<param>` a host-run `<invoke>` names its deadline with, in
/// milliseconds. The engine reads it and does not hand it to the host: past
/// the deadline, an invocation still running is cancelled and the document
/// receives `error.invoke.<id>` instead of its completion. A neutral name
/// rather than `sce:mesh-rpc`'s `_mesh_deadline_ms`, so the two invoke types
/// converge on one spelling.
pub const HOST_INVOKE_DEADLINE_PARAM: &str = "_sce_deadline_ms";

/// Read the text of a [`HOST_INVOKE_DEADLINE_PARAM`] value as milliseconds.
///
/// One or more ASCII digits, optionally followed by `.` and one or more `0` —
/// a `<param expr>` reaches the request as the text of its value, and a script
/// engine may render a whole number `5000.0` — within a signed 64-bit count.
/// Anything else is `None`: no sign, no whitespace, no exponent, no digit
/// separator.
///
/// Spelled out rather than left to a number parser because every runtime
/// implements it and the languages' parsers disagree (Go reads hex floats,
/// Python reads `1_000`); the table they are all held to is
/// `sce-build/tests/fixtures/host_processor/host_invoke_deadline_values.json`.
pub fn parse_host_invoke_deadline_ms(written: &str) -> Option<u64> {
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

/// An `<invoke>` the host runs, at the point the state was entered.
///
/// §scxml-6.4.1 leaves the invokable set to the platform in the same
/// words §scxml-6.2.5 uses for `<send>`, so a host may implement its own
/// `type` here too — but an invoke is not a send. It has a lifetime: it
/// starts when the state is entered, it is cancelled if the state exits,
/// and the document may be waiting on `done.invoke.<id>`. That is why
/// the handler receives a [`HostInvokeEvent`] rather than a bare request.
#[derive(Debug, Clone, Default)]
pub struct HostInvokeRequest {
    /// The `type` this `<invoke>` named.
    pub processor_type: String,
    /// The invoke's id (§scxml-6.4.1), auto-generated when the document
    /// declared none. This is the name the document waits on: a
    /// completion is `done.invoke.<invoke_id>`, so a host that finishes
    /// asynchronously must keep it.
    pub invoke_id: String,
    /// `<invoke src="...">`, empty when the document named none. SCE does
    /// not interpret it — what a src means is the invoked processor's
    /// business (§scxml-6.4.1).
    pub src: String,
    /// `<param>` values, keyed by name; a repeated name keeps every value
    /// in document order.
    pub params: std::collections::HashMap<String, Vec<String>>,
    /// Inline `<content>`, empty when the document carried none.
    pub content: String,
    /// Which start of this invoke this is. The engine assigns it, and a host
    /// that finishes later hands it back to `Engine::complete_host_invoke`.
    ///
    /// The id alone cannot say it: a state that exits and is entered again
    /// starts the same `<invoke>` a second time under the same id, and a
    /// result the first run produces after it was cancelled would otherwise
    /// read as the second run's (§scxml-6.4 — once the state has exited, what
    /// the cancelled process sends is ignored). Distinct for every start of
    /// every invocation within one engine.
    pub token: u64,
}

/// An `<invoke>` the host was running, at the point its state exited.
#[derive(Debug, Clone, Default)]
pub struct HostInvokeCancel {
    /// The `type` the `<invoke>` named.
    pub processor_type: String,
    /// The invoke being cancelled — the same id its
    /// [`HostInvokeEvent::Start`] carried.
    pub invoke_id: String,
    /// The token its [`HostInvokeEvent::Start`] carried, so a host running
    /// more than one start of the same id stops the right one.
    pub token: u64,
}

/// One turn of a host-run invoke's lifecycle.
///
/// Both arms go to one registered handler rather than to two separately
/// registered callbacks, because a host that can start an invocation and
/// cannot stop it is not a working invoker — and two registrations make
/// that state reachable. One handler means the pair is registered
/// together or not at all.
#[derive(Debug, Clone)]
pub enum HostInvokeEvent {
    /// §scxml-6.4: the state was entered and the macrostep has settled.
    /// Begin the invoked process.
    Start(HostInvokeRequest),
    /// §scxml-6.4: the state exited. Stop it.
    ///
    /// Delivered only for an invocation that is still running: one that
    /// never started (its state exited before the macrostep ended) has
    /// nothing to tear down, and one that already completed has nothing left
    /// to stop — `done.invoke` said the process is over.
    Cancel(HostInvokeCancel),
}

/// A host invoker's answer to [`HostInvokeEvent::Start`].
///
/// Read only for `Start`; a response to a `Cancel` is ignored, because
/// there is nothing left for it to mean.
#[derive(Debug, Clone, Default)]
pub struct HostInvokeResponse {
    /// Payload for an immediate `done.invoke.<invoke_id>`, for an
    /// invocation that completed before returning.
    ///
    /// `None` is the ordinary case: the work outlives the call, and the
    /// host reports it through `Engine::complete_host_invoke` with the
    /// request's token when it finishes. SCE does not synthesise a completion
    /// the host did not report — an invoked process that never terminates
    /// never fires `done.invoke`, which is what §scxml-6.4 says.
    pub done_data: Option<String>,
}

/// A registered invoke-lifecycle handler.
pub(crate) type HostInvokeHandler =
    Box<dyn FnMut(HostInvokeEvent) -> Option<HostInvokeResponse> + Send + 'static>;

/// The set of processors a host has registered handlers for.
///
/// A map rather than a list of `(type, handler)` pairs because dispatch
/// happens per `<send>` execution and the lookup is on the hot path of
/// every host-served send.
#[derive(Default)]
pub(crate) struct HostProcessorRegistry {
    handlers: HashMap<String, HostSendHandler>,
    invokers: HashMap<String, HostInvokeHandler>,
    /// `(processor_type, invoke_id)` → token for every invocation that was
    /// started and has neither completed nor been cancelled.
    ///
    /// Held here rather than left to the generated machine because "is
    /// this one still running?" is the question both the cancel path and a
    /// completion have to answer, and answering it in each backend's
    /// template would be the same bookkeeping written once per language. It
    /// also keeps the emitted exit chain to an unconditional call: the
    /// engine decides whether there is anything to cancel.
    ///
    /// One entry per key is enough: an `<invoke>` belongs to one state and a
    /// state is in the configuration at most once, so the same id is never
    /// running twice. A second start under the same id replaces the entry,
    /// and its new token is what makes the first run's late reply stale.
    started: std::collections::BTreeMap<(String, String), u64>,
    /// The token the next start receives.
    next_token: u64,
}

impl HostProcessorRegistry {
    /// Register `handler` for `processor_type`, replacing any handler
    /// already registered for it.
    ///
    /// Replacing rather than refusing: registration is host
    /// configuration, and a host that re-registers during setup means
    /// the later call. Refusing would make the order of two setup
    /// functions load-bearing.
    pub(crate) fn register(&mut self, processor_type: &str, handler: HostSendHandler) {
        self.handlers.insert(processor_type.to_string(), handler);
    }

    /// The handler for `processor_type`, or `None` when the host
    /// declared the type at build time and never registered one.
    pub(crate) fn handler_for(&mut self, processor_type: &str) -> Option<&mut HostSendHandler> {
        self.handlers.get_mut(processor_type)
    }

    /// Whether a handler is registered for `processor_type`.
    ///
    /// Distinct from "the handler ran and returned `None`", which is the
    /// ordinary fire-and-forget reply. Without this question a generated
    /// send site cannot tell a processor that did its work silently from
    /// one that was never wired up — and it must, because only the
    /// second is an error.
    pub(crate) fn is_registered(&self, processor_type: &str) -> bool {
        self.handlers.contains_key(processor_type)
    }

    /// Register `handler` as the invoker for `processor_type`.
    pub(crate) fn register_invoker(&mut self, processor_type: &str, handler: HostInvokeHandler) {
        self.invokers.insert(processor_type.to_string(), handler);
    }

    /// Whether an invoker is registered for `processor_type`.
    pub(crate) fn invoker_is_registered(&self, processor_type: &str) -> bool {
        self.invokers.contains_key(processor_type)
    }

    /// Start an invocation, recording it so the cancel path can find it.
    ///
    /// `None` when no invoker is registered — the caller turns that into
    /// the §scxml-6.4.1 `error.execution`, because an invoke nobody ran
    /// is the same fact whether the type was undeclared or the handler
    /// was never wired up.
    ///
    /// The token is assigned and recorded BEFORE the handler runs, so a
    /// handler that completes synchronously is completing an invocation the
    /// engine already knows is running. Returned beside the response for
    /// that synchronous completion to name.
    pub(crate) fn start_invoke(
        &mut self,
        mut request: HostInvokeRequest,
    ) -> Option<(u64, Option<HostInvokeResponse>)> {
        let handler = self.invokers.get_mut(&request.processor_type)?;
        let token = self.next_token;
        self.next_token = self.next_token.wrapping_add(1);
        request.token = token;
        self.started.insert(
            (request.processor_type.clone(), request.invoke_id.clone()),
            token,
        );
        let response = handler(HostInvokeEvent::Start(request));
        Some((token, response))
    }

    /// Take the running invocation `(processor_type, invoke_id, token)` out of
    /// the set, reporting whether it was there.
    ///
    /// This is the one door a completion goes through: taking it is what
    /// makes the completion exactly-once, since a second completion, a
    /// cancel, or a stale token from an earlier start all find nothing.
    pub(crate) fn take_started(
        &mut self,
        processor_type: &str,
        invoke_id: &str,
        token: u64,
    ) -> bool {
        let key = (processor_type.to_string(), invoke_id.to_string());
        if self.started.get(&key) != Some(&token) {
            return false;
        }
        self.started.remove(&key);
        true
    }

    /// Cancel an invocation, if it is still running.
    ///
    /// Returns the token of the start that was cancelled, or `None` when no
    /// `Cancel` was delivered. One whose state exited before the macrostep
    /// settled never started, and one that already completed is over, so
    /// neither has anything to tear down. The token is what lets the engine
    /// drop that start's pending deadline.
    pub(crate) fn cancel_invoke(&mut self, processor_type: &str, invoke_id: &str) -> Option<u64> {
        let key = (processor_type.to_string(), invoke_id.to_string());
        let token = self.started.remove(&key)?;
        self.deliver_cancel(processor_type, invoke_id, token)
            .then_some(token)
    }

    /// End the start `(processor_type, invoke_id, token)` because its
    /// deadline passed: take it out of the set and tell the host to stop.
    ///
    /// Through the same door a completion takes, so exactly one of the two
    /// happens: a completion that arrives afterwards finds nothing, and a
    /// deadline that fires after a completion finds nothing either.
    pub(crate) fn expire_invoke(
        &mut self,
        processor_type: &str,
        invoke_id: &str,
        token: u64,
    ) -> bool {
        self.take_started(processor_type, invoke_id, token)
            && self.deliver_cancel(processor_type, invoke_id, token)
    }

    fn deliver_cancel(&mut self, processor_type: &str, invoke_id: &str, token: u64) -> bool {
        let Some(handler) = self.invokers.get_mut(processor_type) else {
            return false;
        };
        handler(HostInvokeEvent::Cancel(HostInvokeCancel {
            processor_type: processor_type.to_string(),
            invoke_id: invoke_id.to_string(),
            token,
        }));
        true
    }
}

/// Whether an event named `event_name` carrying `_event.invokeid` =
/// `invoke_id` is the completion of a host-run invocation: a
/// `done.invoke.<id>` whose `<id>` is one of `host_invoke_ids`, or the generic
/// `done.invoke` a document that names no specific completion receives,
/// whose invokeid is one of them.
///
/// Such an event is accepted only through `Engine::complete_host_invoke`,
/// which is the one path that knows the invocation is still running. Raised
/// any other way it could be a cancelled run's late reply, so the engine
/// refuses it at dequeue.
pub(crate) fn is_host_invoke_completion(
    event_name: &str,
    invoke_id: &str,
    host_invoke_ids: &[&str],
) -> bool {
    match event_name.strip_prefix(crate::invoke::DONE_INVOKE_PREFIX) {
        Some(id) => host_invoke_ids.contains(&id),
        None => {
            event_name == crate::invoke::DONE_INVOKE_EVENT && host_invoke_ids.contains(&invoke_id)
        }
    }
}
