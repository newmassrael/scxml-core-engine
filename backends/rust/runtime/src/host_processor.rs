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
}

/// An `<invoke>` the host was running, at the point its state exited.
#[derive(Debug, Clone, Default)]
pub struct HostInvokeCancel {
    /// The `type` the `<invoke>` named.
    pub processor_type: String,
    /// The invoke being cancelled — the same id its
    /// [`HostInvokeEvent::Start`] carried.
    pub invoke_id: String,
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
    /// Delivered only for an invocation that actually started: a state
    /// that exits before the macrostep ends never runs its invoke, and
    /// cancelling something that never began would have the host tearing
    /// down state it never built.
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
    /// host raises `done.invoke.<invoke_id>` itself when it finishes.
    /// SCE does not synthesise a completion the host did not report — an
    /// invoked process that never terminates never fires `done.invoke`,
    /// which is what §scxml-6.4 says.
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
    /// `(processor_type, invoke_id)` for every invocation the host was
    /// told to start and has not been told to stop.
    ///
    /// Held here rather than left to the generated machine because
    /// "did this one start?" is the question the cancel path has to
    /// answer, and answering it in each backend's template would be the
    /// same bookkeeping written once per language. It also keeps the
    /// emitted exit chain to an unconditional call: the engine decides
    /// whether there is anything to cancel.
    started: std::collections::BTreeSet<(String, String)>,
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
    pub(crate) fn start_invoke(
        &mut self,
        request: HostInvokeRequest,
    ) -> Option<Option<HostInvokeResponse>> {
        let key = (request.processor_type.clone(), request.invoke_id.clone());
        let handler = self.invokers.get_mut(&request.processor_type)?;
        let response = handler(HostInvokeEvent::Start(request));
        self.started.insert(key);
        Some(response)
    }

    /// Cancel an invocation, if it started.
    ///
    /// Returns whether a `Cancel` was delivered. A state that exits
    /// before the macrostep settles never ran its invoke, so there is
    /// nothing to tear down and nothing is sent.
    pub(crate) fn cancel_invoke(&mut self, processor_type: &str, invoke_id: &str) -> bool {
        let key = (processor_type.to_string(), invoke_id.to_string());
        if !self.started.remove(&key) {
            return false;
        }
        let Some(handler) = self.invokers.get_mut(processor_type) else {
            return false;
        };
        handler(HostInvokeEvent::Cancel(HostInvokeCancel {
            processor_type: processor_type.to_string(),
            invoke_id: invoke_id.to_string(),
        }));
        true
    }
}
