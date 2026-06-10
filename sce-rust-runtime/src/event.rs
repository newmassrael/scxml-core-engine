// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! Event wrapper types: [`EventWithMetadata`], [`EventMetadata`], [`EventType`].
//!
//! Ports the C++ nested struct `StaticExecutionEngine::EventWithMetadata`
//! (`sce/include/static/StaticExecutionEngine.h:114`) and the companion
//! `EventMetadata` struct from `sce/include/core/EventMetadata.h`.
//!
//! The SCXML execution algorithm (W3C SCXML 5.10) requires that every event
//! carry metadata describing its origin, send ID, type, data, and more. The
//! `_event` system variable exposes these fields to ECMAScript expressions
//! (e.g., `_event.name`, `_event.data`, `_event.sendid`, `_event.origin`).
//!
//! Watching-zenoh RFC §synth-5-J-2 (lines 1989-1994): string-typed fields are
//! backed by [`crate::SceString`], which is `std::string::String` under the
//! default std build and `heapless::String<MAX_EVENT_STRING_LEN>` under
//! `--features=no_std`. The cap and motivation are documented at
//! [`crate::MAX_EVENT_STRING_LEN`].

use crate::SceString;
// Only the std arm of `EventMetadata::external` stores string metadata; under
// no_std the `_event.*` strings are elided, so the converter is std-only.
#[cfg(not(feature = "no_std"))]
use crate::sce_string_from_str;

/// Event type classification (W3C SCXML 5.10.1).
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
}

/// Metadata fields attached to events (W3C SCXML 5.10).
///
/// Ports C++ `SCE::Core::EventMetadata`. Every event flowing through the
/// engine carries an `EventMetadata`, which is copied into `_event.*` fields
/// in the script engine via [`EventMetadataHelper`](crate::helpers).
/// The `event_type` discriminant drives internal/external queue routing (not
/// just the `_event.type` script binding), so it is present on every profile.
/// The five `SceString` fields gated below are W3C `_event.*` script/invoke/HTTP
/// metadata with no no_std reader: reading `_event.data` (schemaless/dynamic) /
/// `_event.sendid` / `_event.origin` / `_event.origintype` is an ECMAScript
/// access that forces `NEEDS_SCRIPT_ENGINE` (rejected under no_std via
/// `codegen/no-std-script-not-supported`), and `<invoke>` / HTTP send (the
/// producers of `invoke_id` / `origin*` / the HTTP `target`) are no_std-rejected
/// too. So they are `#[cfg(not(feature = "no_std"))]`, dropping ~1.3 KiB of dead
/// inline `heapless::String<256>` per queued event on MCU targets. (Typed event
/// data rides the `Payload` channel — `_event.data.<field>` reads the payload
/// sum, never this JSON `data` string.)
#[derive(Debug, Clone, Default)]
pub struct EventMetadata {
    /// `_event.data` — event payload as a JSON string (empty when no payload).
    #[cfg(not(feature = "no_std"))]
    pub data: SceString,
    /// `_event.type` — classification (internal / external / platform).
    pub event_type: EventType,
    /// `_event.sendid` — send ID from originating `<send>` (empty if none).
    #[cfg(not(feature = "no_std"))]
    pub send_id: SceString,
    /// `_event.origin` — origin URI (empty if no origin).
    #[cfg(not(feature = "no_std"))]
    pub origin: SceString,
    /// `_event.origintype` — type of origin (e.g., SCXML Event I/O Processor URI).
    #[cfg(not(feature = "no_std"))]
    pub origin_type: SceString,
    /// `_event.invokeid` — invoke ID if event came from a child invoke (W3C 6.4.1).
    #[cfg(not(feature = "no_std"))]
    pub invoke_id: SceString,
}

impl EventMetadata {
    /// Construct platform metadata (e.g., for `error.execution`, `done.state.*`).
    pub fn platform() -> Self {
        #[cfg(not(feature = "no_std"))]
        {
            Self {
                event_type: EventType::Platform,
                ..Default::default()
            }
        }
        #[cfg(feature = "no_std")]
        {
            Self {
                event_type: EventType::Platform,
            }
        }
    }

    /// Construct internal metadata (e.g., for `<raise>`).
    pub fn internal() -> Self {
        #[cfg(not(feature = "no_std"))]
        {
            Self {
                event_type: EventType::Internal,
                ..Default::default()
            }
        }
        #[cfg(feature = "no_std")]
        {
            Self {
                event_type: EventType::Internal,
            }
        }
    }

    /// Construct external metadata with a send ID and origin (e.g., for `<send>`).
    ///
    /// Under `--features=no_std` the `_event.sendid` / `_event.origin` string
    /// metadata is elided (no script-engine reader exists), so the `send_id` /
    /// `origin` arguments are accepted for a uniform call shape across profiles
    /// but not stored.
    pub fn external(send_id: SceString, origin: SceString) -> Self {
        #[cfg(not(feature = "no_std"))]
        {
            Self {
                event_type: EventType::External,
                send_id,
                origin,
                origin_type: sce_string_from_str(
                    crate::helpers::scxml_constants::SCXML_EVENT_PROCESSOR_TYPE,
                ),
                ..Default::default()
            }
        }
        #[cfg(feature = "no_std")]
        {
            let _ = (send_id, origin);
            Self {
                event_type: EventType::External,
            }
        }
    }
}

/// An event wrapped with its full W3C SCXML 5.10 metadata.
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
    /// W3C SCXML C.2: HTTP POST target URL (empty if not an HTTP send).
    ///
    /// `#[cfg(not(feature = "no_std"))]`: HTTP send is no_std-rejected
    /// (`codegen/no-std-http-not-supported`), so this field has no producer or
    /// reader on MCU targets — elided to drop its inline `heapless::String<256>`
    /// from every queued event (mirrors the `!no_std`-gated `on_http_send` field
    /// on [`Engine`](crate::Engine)).
    #[cfg(not(feature = "no_std"))]
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
            #[cfg(not(feature = "no_std"))]
            target: SceString::new(),
        }
    }

    /// Set `_event.data` (the JSON-string baseline) from a borrowed string.
    ///
    /// No-op under `--features=no_std`: typed event data rides the `Payload`
    /// channel there (`_event.data.<field>` reads the payload sum), and the
    /// schemaless `_event.data` baseline this string carries has no no_std
    /// reader (dynamic `_event.data` needs the rejected script engine). Routing
    /// the generated send sites through this method keeps their call shape
    /// uniform across both profiles while the no_std build stores nothing.
    pub fn set_event_data(&mut self, data: &str) {
        #[cfg(not(feature = "no_std"))]
        {
            self.metadata.data = crate::sce_string_from_str(data);
        }
        #[cfg(feature = "no_std")]
        {
            let _ = data;
        }
    }
}
