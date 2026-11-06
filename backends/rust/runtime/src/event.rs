// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! Event wrapper types: [`EventWithMetadata`], [`EventMetadata`], [`EventType`].
//!
//! Ports the C++ nested struct `StaticExecutionEngine::EventWithMetadata`
//! (`sce/include/static/StaticExecutionEngine.h:114`) and the companion
//! `EventMetadata` struct from `sce/include/core/EventMetadata.h`.
//!
//! The SCXML execution algorithm (§scxml-5.10) requires that every event
//! carry metadata describing its origin, send ID, type, data, and more. The
//! `_event` system variable exposes these fields to ECMAScript expressions
//! (e.g., `_event.name`, `_event.data`, `_event.sendid`, `_event.origin`).
//!
//! SCE Protocol-Synthesis RFC §synth-5-J-2 (lines 1989-1994): string-typed fields are
//! backed by [`crate::SceString`], which is `std::string::String` under the
//! default std build and `heapless::String<MAX_EVENT_STRING_LEN>` under
//! `--no-default-features`. The cap and motivation are documented at
//! [`crate::MAX_EVENT_STRING_LEN`].

#[cfg(feature = "std")]
use crate::SceString;
// Only the std arm of `EventMetadata::external` stores string metadata; under
// no_std the `_event.*` strings are elided, so the converter is std-only.
#[cfg(feature = "std")]
use crate::sce_string_from_str;

/// Event type classification (§scxml-5.10.1).
///
/// Ports the C++ `std::string type` field of `EventWithMetadata`, which only
/// ever holds one of these three string literals. Using an enum eliminates
/// string comparisons on hot paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum EventType {
    /// Event raised internally via `<raise>` or a targetless `<send>` (W3C 5.10.1).
    Internal,
    /// Event from an external source (W3C 5.10.1 — the default classification).
    #[default]
    External,
    /// Platform-generated event (e.g., `error.execution`, `done.state.*`).
    Platform,
}

impl EventType {
    /// Return the C++ string-equivalent representation for interop and logging.
    pub const fn as_str(self) -> &'static str {
        match self {
            EventType::Internal => "internal",
            EventType::External => "external",
            EventType::Platform => "platform",
        }
    }

    /// The type [`as_str`](Self::as_str) spells as `name`, or `None` for any
    /// other text — what a saved state reads an event's type back through.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "internal" => Some(EventType::Internal),
            "external" => Some(EventType::External),
            "platform" => Some(EventType::Platform),
            _ => None,
        }
    }

    /// §scxml-5.10.1: classify the event being processed — `platform` for the
    /// events the processor itself raises (`error.*`, `done.*`), otherwise by
    /// the queue it was taken from. Ports C++ `EventTypeHelper::classifyEventType`,
    /// so every generated Rust machine asks this one function rather than
    /// carrying its own copy of the rule.
    pub fn classify(event_name: &str, from_external_queue: bool) -> EventType {
        if event_name.starts_with("error.") || event_name.starts_with("done.") {
            EventType::Platform
        } else if from_external_queue {
            EventType::External
        } else {
            EventType::Internal
        }
    }
}

/// Where a reply to an event goes: the Event I/O Processor that delivered it
/// and the address that processor accepts as a `<send>` target (§scxml-5.10.1,
/// docs/adr/0090).
///
/// Ports C++ `SCE::Core::EventOrigin`. The document reads the two as
/// `_event.origintype` and `_event.origin`; they are one value because the
/// address means something only relative to its processor. The processor that
/// delivers an event writes the pair once, where it builds the event, and
/// nothing converts it later. The type has no `Default`, so a pair with one
/// field set cannot be spelled by accident: an event with no origin holds
/// `None` instead.
///
/// Only under std, as every `_event` text field is (see [`EventMetadata`]).
#[cfg(feature = "std")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventOrigin {
    /// The canonical type URI of the Event I/O Processor that delivered the
    /// event (`_event.origintype`).
    pub processor: String,
    /// A URI that processor accepts as a `<send>` target (`_event.origin`).
    pub address: String,
}

#[cfg(feature = "std")]
impl EventOrigin {
    /// The pair of `processor`, the canonical type URI of an Event I/O
    /// Processor, and `address`, a URI that processor accepts as a target.
    pub fn new(processor: impl Into<String>, address: impl Into<String>) -> Self {
        Self {
            processor: processor.into(),
            address: address.into(),
        }
    }
}

/// Metadata fields attached to events (§scxml-5.10).
///
/// Ports C++ `SCE::Core::EventMetadata`. Every event flowing through the
/// engine carries an `EventMetadata`, which is copied into `_event.*` fields
/// in the script engine via [`EventMetadataHelper`](crate::helpers).
///
/// ⚠ `event_type` does NOT decide anything. Routing is decided by which queue a
/// caller pushes to, and the script-visible `_event.type` is decided by the
/// engine where it DEQUEUES the event — the one moment the queue is known —
/// through [`EventType::classify`] and
/// [`Engine::is_current_event_external`](crate::Engine::is_current_event_external)
/// (2026-09-26; before that an enqueue-time flag typed interleaved events
/// wrongly). So this field is a second producer nothing reads, which is why
/// [`platform`](Self::platform) and [`internal`](Self::internal) below have no
/// callers. It stays on every profile because removing it would change the
/// no_std layout.
/// The `SceString` and origin fields gated below are W3C `_event.*` script/invoke/HTTP
/// metadata with no no_std reader: reading `_event.data` (schemaless/dynamic) /
/// `_event.sendid` / `_event.origin` / `_event.origintype` is an ECMAScript
/// access that forces `NEEDS_SCRIPT_ENGINE` (rejected under no_std via
/// `codegen/no-std-script-not-supported`), and `<invoke>` / HTTP send (the
/// producers of `invoke_id` / `origin*` / the HTTP `target`) are no_std-rejected
/// too. So they are `#[cfg(feature = "std")]`, dropping ~1.3 KiB of dead
/// inline `heapless::String<256>` per queued event on MCU targets. (Typed event
/// data rides the `Payload` channel — `_event.data.<field>` reads the payload
/// sum, never this JSON `data` string.)
#[derive(Debug, Clone, Default)]
pub struct EventMetadata {
    /// `_event.name` — the name the event ARRIVED under (§scxml-5.10), when that is
    /// not the name of the enum member that carries it. The event is carried as the
    /// member of a name the document writes, and §scxml-3.12.1 lets `request.new`
    /// take a transition on `request`, so the member alone loses the longer name.
    /// Set by a door that resolved a name through a prefix
    /// ([`StatePolicy::resolve_event_by_name`](crate::StatePolicy::resolve_event_by_name));
    /// empty means the member's own name IS the name, which is every event the
    /// document itself raises or sends. No `_event.name` reader exists under
    /// no_std (reading it is an ECMAScript access), so the field is elided there.
    #[cfg(feature = "std")]
    pub name: SceString,
    /// `_event.data` — event payload as a JSON string (empty when no payload).
    #[cfg(feature = "std")]
    pub data: SceString,
    /// `_event.type` — classification (internal / external / platform).
    pub event_type: EventType,
    /// `_event.sendid` — send ID from originating `<send>` (empty if none).
    #[cfg(feature = "std")]
    pub send_id: SceString,
    /// `_event.origin` and `_event.origintype` — the reply address the
    /// processor that delivered the event wrote when it built it, as one value
    /// (docs/adr/0090); `None` for an event no processor stamped, and then the
    /// document reads both fields as the empty string.
    #[cfg(feature = "std")]
    pub origin: Option<EventOrigin>,
    /// The id of the session that sent the event, written by the processor
    /// that delivered it from that session: the SCXML Event I/O Processor for a
    /// local session, and the Mesh transport for a remote child or parent
    /// (SCE_MESH.md). Empty for any other sender: a host, BasicHTTP, a Mesh peer
    /// that is no session of this tree. It is independent of the reply address
    /// in [`origin`](Self::origin), which no rule ties to it. Not an `_event`
    /// field: `<finalize>` and a saved state find the sending session by it,
    /// and nothing reads the origin's address to find a session (docs/adr/0090,
    /// section 1).
    #[cfg(feature = "std")]
    pub sender: SceString,
    /// `_event.invokeid` — invoke ID if event came from a child invoke (W3C 6.4.1).
    #[cfg(feature = "std")]
    pub invoke_id: SceString,
    /// The token of the host-run invocation this event completes, set only by
    /// `Engine::complete_host_invoke` after it confirmed the invocation was
    /// still running. A host invoke's `done.invoke.<id>` that arrives without
    /// one did not come through that check, so the engine refuses it
    /// (§scxml-6.4). Not script-visible: `_event` has no field for it.
    #[cfg(feature = "std")]
    pub host_invoke_token: Option<u64>,
}

impl EventMetadata {
    /// Construct platform metadata (e.g., for `error.execution`, `done.state.*`).
    pub fn platform() -> Self {
        #[cfg(feature = "std")]
        {
            Self {
                event_type: EventType::Platform,
                ..Default::default()
            }
        }
        #[cfg(not(feature = "std"))]
        {
            Self {
                event_type: EventType::Platform,
            }
        }
    }

    /// Construct internal metadata (e.g., for `<raise>`).
    pub fn internal() -> Self {
        #[cfg(feature = "std")]
        {
            Self {
                event_type: EventType::Internal,
                ..Default::default()
            }
        }
        #[cfg(not(feature = "std"))]
        {
            Self {
                event_type: EventType::Internal,
            }
        }
    }

    /// Construct external metadata with a send ID and no origin: an event no
    /// Event I/O Processor stamped — one a host raises naming no party, a reply
    /// a host processor gives naming none, an HTTP response (docs/adr/0090,
    /// sections 2 and 3). `_event.origin` and `_event.origintype` read empty.
    ///
    /// Under `--no-default-features` the `_event.sendid` string metadata is
    /// elided (no script-engine reader exists), so `send_id` is accepted for a
    /// uniform call shape across profiles but not stored.
    pub fn external(send_id: &str) -> Self {
        #[cfg(feature = "std")]
        {
            Self {
                event_type: EventType::External,
                send_id: sce_string_from_str(send_id),
                ..Default::default()
            }
        }
        #[cfg(not(feature = "std"))]
        {
            let _ = send_id;
            Self {
                event_type: EventType::External,
            }
        }
    }

    /// Construct the external metadata the SCXML Event I/O Processor writes
    /// for an event the session `sender` sent: the pair of its own type and
    /// the location `sender` publishes, and `sender` beside it (docs/adr/0090,
    /// section 2). A machine that holds no session id sends an event with no
    /// origin.
    ///
    /// Under `--no-default-features` neither the send id nor the origin is
    /// stored, as [`external`](Self::external) stores neither; both arguments
    /// are borrowed so that profile builds nothing for them.
    pub fn sent_by_session(send_id: &str, sender: &str) -> Self {
        #[cfg(feature = "std")]
        {
            Self {
                origin: crate::helpers::io_processors::scxml_origin_of(sender),
                sender: sce_string_from_str(sender),
                ..Self::external(send_id)
            }
        }
        #[cfg(not(feature = "std"))]
        {
            let _ = sender;
            Self::external(send_id)
        }
    }

    /// `_event.origin`: the address of the event's origin, or the empty string
    /// when it has none (§scxml-5.10.1).
    #[cfg(feature = "std")]
    pub fn origin_address(&self) -> &str {
        self.origin
            .as_ref()
            .map_or("", |origin| origin.address.as_str())
    }

    /// `_event.origintype`: the processor of the event's origin, or the empty
    /// string when it has none (§scxml-5.10.1).
    #[cfg(feature = "std")]
    pub fn origin_processor(&self) -> &str {
        self.origin
            .as_ref()
            .map_or("", |origin| origin.processor.as_str())
    }
}

/// An event wrapped with its full §scxml-5.10 metadata.
///
/// Ports the C++ nested struct `StaticExecutionEngine<Policy>::EventWithMetadata`
/// at `StaticExecutionEngine.h:114`. The generic parameter `E` is the generated
/// `Policy::Event` enum type.
///
/// The second parameter `P` is the typed payload (EventSchema native
/// lowering). It defaults to `()` so every existing one-parameter use
/// (`EventWithMetadata<E>`) keeps compiling; for a schema-carrying document the
/// engine instantiates it with the policy's `Self::Payload` sum so the typed
/// payload rides with its event through the queues.
#[derive(Debug, Clone)]
pub struct EventWithMetadata<E, P = ()> {
    /// The typed event value (e.g., `Test332Event::Foo`).
    pub event: E,
    /// Typed event payload. `()` for schemaless events; the
    /// per-document `<Doc>Payload` sum for schema-carrying events.
    pub payload: P,
    /// Event metadata (data, type, sendid, origin, origintype, invokeid).
    pub metadata: EventMetadata,
    /// §scxml-C-2: HTTP POST target URL (empty if not an HTTP send).
    ///
    /// `#[cfg(feature = "std")]`: HTTP send is no_std-rejected
    /// (`codegen/no-std-http-not-supported`), so this field has no producer or
    /// reader on MCU targets — elided to drop its inline `heapless::String<256>`
    /// from every queued event (mirrors the `!no_std`-gated `on_http_send` field
    /// on [`Engine`](crate::Engine)).
    #[cfg(feature = "std")]
    pub target: SceString,
}

impl<E, P: Default> EventWithMetadata<E, P> {
    /// Construct an `EventWithMetadata` from a bare event with default metadata
    /// and a default (`Self::Payload::default()`) payload slot.
    pub fn new(event: E) -> Self {
        Self {
            event,
            payload: P::default(),
            metadata: EventMetadata::default(),
            #[cfg(feature = "std")]
            target: SceString::new(),
        }
    }

    /// Construct an `error.*` event the processor itself raised.
    ///
    /// The single way generated code raises a platform error, so the two things
    /// such an event owes its reader are filled in one place rather than at each
    /// site that happens to remember:
    ///
    /// * `_event.type` is `"platform"`. [`new`](Self::new) defaults the metadata
    ///   to [`EventType::External`], which is what every Rust raise site used to
    ///   pass — the one backend of six that did.
    /// * `_event.data` carries `message`. The clause closes with "platforms MAY
    ///   include additional information about the nature of the error in the
    ///   'data' field", and a document that answers `error.execution` was being
    ///   handed `undefined` instead of the name of what broke.
    ///
    /// `message` describes the failing construct and is a literal formed at
    /// generation time, not the underlying engine's error text: it must survive
    /// `--no-default-features`, where there is no allocator to build one and
    /// [`set_event_data`](Self::set_event_data) stores nothing anyway. The
    /// dynamic detail stays on the log line beside the call.
    pub fn platform_error(event: E, message: &str) -> Self {
        // §scxml-3.12.2: the processor signals its own failures as `error.*`,
        // and "platforms MAY include additional information about the nature of
        // the error in the 'data' field". Cited in the body rather than the doc
        // comment because the ledger's Rust resolver binds a citation to the
        // symbol enclosing it, and a `///` line encloses nothing.
        let mut raised = Self::new(event);
        raised.metadata.event_type = EventType::Platform;
        raised.set_event_data(message);
        raised
    }

    /// Set `_event.data` (the JSON-string baseline) from a borrowed string.
    ///
    /// No-op under `--no-default-features`: typed event data rides the `Payload`
    /// channel there (`_event.data.<field>` reads the payload sum), and the
    /// schemaless `_event.data` baseline this string carries has no no_std
    /// reader (dynamic `_event.data` needs the rejected script engine). Routing
    /// the generated send sites through this method keeps their call shape
    /// uniform across both profiles while the no_std build stores nothing.
    pub fn set_event_data(&mut self, data: &str) {
        #[cfg(feature = "std")]
        {
            self.metadata.data = crate::sce_string_from_str(data);
        }
        #[cfg(not(feature = "std"))]
        {
            let _ = data;
        }
    }
}

#[cfg(test)]
#[cfg(feature = "std")]
mod tests {
    use super::*;

    /// The constructor every generated backend routes its `error.*` raises
    /// through must actually carry what it was handed.
    ///
    /// Written while chasing an `_event.data` that arrived nil in the seven
    /// channel fixture, to cut the chain at its first link rather than keep
    /// guessing further down it.
    #[test]
    fn platform_error_carries_its_message_and_classification() {
        let raised: EventWithMetadata<u8, ()> =
            EventWithMetadata::platform_error(7, "<assign> to 'x' failed");

        assert_eq!(raised.event, 7);
        assert_eq!(
            raised.metadata.data.as_str(),
            "<assign> to 'x' failed",
            "the message is what a document answering error.execution reads out \
             of `_event.data`; an empty string tells it only THAT something failed"
        );
        assert_eq!(
            raised.metadata.event_type,
            EventType::Platform,
            "an event the processor raised is `platform` per W3C SCXML 5.10.1; \
             `new()` defaults to External, which is what every Rust raise site \
             passed before this constructor existed"
        );
    }

    /// docs/adr/0090, section 2: an event a session sends carries the pair
    /// the SCXML processor writes, and the sender's id beside it.
    #[test]
    fn a_session_send_carries_the_scxml_pair_and_its_sender() {
        let sent = EventMetadata::sent_by_session("s-1", "a b");

        assert_eq!(
            sent.origin,
            Some(EventOrigin::new(
                crate::helpers::scxml_constants::SCXML_EVENT_PROCESSOR_TYPE,
                "sce://scxml/a%20b",
            )),
            "the origin is the location the sender publishes, percent-encoded as \
             `_ioprocessors` publishes it"
        );
        assert_eq!(sent.sender.as_str(), "a b");
        assert_eq!(sent.send_id.as_str(), "s-1");
    }

    /// docs/adr/0090, section 3: no constructor invents an origin, and a
    /// machine with no session id sends one with none.
    #[test]
    fn an_event_no_processor_stamped_reads_both_fields_empty() {
        let posted = EventMetadata::external("");
        assert_eq!(posted.origin, None);
        assert_eq!(posted.origin_address(), "");
        assert_eq!(posted.origin_processor(), "");

        let anonymous = EventMetadata::sent_by_session("", "");
        assert_eq!(anonymous.origin, None);
        assert!(anonymous.sender.is_empty());
    }
}
