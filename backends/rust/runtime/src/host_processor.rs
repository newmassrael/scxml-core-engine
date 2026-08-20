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

/// A reply from a host-served processor, turned back into an event.
///
/// `None` from a handler means "performed, no reply" — the common case
/// for a fire-and-forget act. `Some` is the request/reply shape: the
/// engine raises the named event on the external queue, which is where a
/// reply from outside the machine belongs (§scxml-C-1).
///
/// A handler that cannot answer *now* — the usual case for real work —
/// returns `None` and raises the reply later through its own handle to
/// the engine's queue. That is not a second mechanism: the engine does
/// not distinguish an event a handler raised from one it was handed.
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
pub(crate) type HostSendHandler =
    Box<dyn FnMut(HostSendRequest) -> Option<HostSendResponse> + Send + 'static>;

/// The set of processors a host has registered handlers for.
///
/// A map rather than a list of `(type, handler)` pairs because dispatch
/// happens per `<send>` execution and the lookup is on the hot path of
/// every host-served send.
#[derive(Default)]
pub(crate) struct HostProcessorRegistry {
    handlers: HashMap<String, HostSendHandler>,
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
}
