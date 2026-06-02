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
//! Watching-zenoh RFC §5.J.2 (lines 1989-1994): string-typed fields are
//! backed by [`crate::SceString`], which is `std::string::String` under the
//! default std build and `heapless::String<MAX_EVENT_STRING_LEN>` under
//! `--features=no_std`. The cap and motivation are documented at
//! [`crate::MAX_EVENT_STRING_LEN`].

use crate::{sce_string_from_str, SceString};

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
#[derive(Debug, Clone, Default)]
pub struct EventMetadata {
    /// `_event.data` — event payload as a JSON string (empty when no payload).
    pub data: SceString,
    /// `_event.type` — classification (internal / external / platform).
    pub event_type: EventType,
    /// `_event.sendid` — send ID from originating `<send>` (empty if none).
    pub send_id: SceString,
    /// `_event.origin` — origin URI (empty if no origin).
    pub origin: SceString,
    /// `_event.origintype` — type of origin (e.g., SCXML Event I/O Processor URI).
    pub origin_type: SceString,
    /// `_event.invokeid` — invoke ID if event came from a child invoke (W3C 6.4.1).
    pub invoke_id: SceString,
}

impl EventMetadata {
    /// Construct platform metadata (e.g., for `error.execution`, `done.state.*`).
    pub fn platform() -> Self {
        Self {
            event_type: EventType::Platform,
            ..Default::default()
        }
    }

    /// Construct internal metadata (e.g., for `<raise>`).
    pub fn internal() -> Self {
        Self {
            event_type: EventType::Internal,
            ..Default::default()
        }
    }

    /// Construct external metadata with a send ID and origin (e.g., for `<send>`).
    pub fn external(send_id: SceString, origin: SceString) -> Self {
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
}

/// An event wrapped with its full W3C SCXML 5.10 metadata.
///
/// Ports the C++ nested struct `StaticExecutionEngine<Policy>::EventWithMetadata`
/// at `StaticExecutionEngine.h:114`. The generic parameter `E` is the generated
/// `Policy::Event` enum type.
///
/// The second parameter `P` is the typed payload (EventSchema MCU native-lowering
/// RFC §10.2). It defaults to `()` so every existing one-parameter use
/// (`EventWithMetadata<E>`) keeps compiling; for a schema-carrying document the
/// engine instantiates it with the policy's `Self::Payload` sum so the typed
/// payload rides with its event through the queues.
#[derive(Debug, Clone)]
pub struct EventWithMetadata<E, P = ()> {
    /// The typed event value (e.g., `Test332Event::Foo`).
    pub event: E,
    /// Typed event payload (RFC §10.2). `()` for schemaless events; the
    /// per-document `<Doc>Payload` sum for schema-carrying events.
    pub payload: P,
    /// Event metadata (data, type, sendid, origin, origintype, invokeid).
    pub metadata: EventMetadata,
    /// W3C SCXML C.2: HTTP POST target URL (empty if not an HTTP send).
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
            target: SceString::new(),
        }
    }
}
