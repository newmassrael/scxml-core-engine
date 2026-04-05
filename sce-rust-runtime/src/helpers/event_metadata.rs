// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! W3C SCXML 5.10: Event metadata (`_event.*` fields) construction helpers.
//!
//! 1:1 port of `sce/include/common/EventMetadataHelper.h`. Provides utilities
//! for populating and clearing `_event.*` system variable fields from
//! `EventWithMetadata` wrappers.
//!
//! In the C++ AOT engine, metadata fields are stored as `pendingEvent*_` members
//! on the policy struct and bound to the script engine's `_event` object. In
//! Rust, the policy trait does not expose raw struct fields; instead, generated
//! code can call these helpers to extract metadata from `EventWithMetadata` and
//! pass it to the script engine via `IScriptEngine::set_variable`.

use crate::event::{EventMetadata, EventType, EventWithMetadata};

/// Populate an [`EventMetadata`] from individual field values.
///
/// Convenience constructor matching C++ `EventMetadataHelper::setEventMetadata`.
pub fn build_metadata(
    origin: &str,
    origin_type: &str,
    send_id: &str,
    invoke_id: &str,
) -> EventMetadata {
    EventMetadata {
        data: String::new(),
        event_type: EventType::External,
        send_id: send_id.to_string(),
        origin: origin.to_string(),
        origin_type: origin_type.to_string(),
        invoke_id: invoke_id.to_string(),
    }
}

/// W3C SCXML 6.3.1: Create a `done.invoke` event with invoke ID.
///
/// All metadata fields except `invoke_id` are empty. The event type is
/// platform (done.invoke is a platform event).
///
/// Ports C++ `EventMetadataHelper::createDoneInvokeEvent`.
pub fn create_done_invoke_event<E>(event: E, invoke_id: &str) -> EventWithMetadata<E> {
    EventWithMetadata {
        event,
        metadata: EventMetadata {
            data: String::new(),
            event_type: EventType::Platform,
            send_id: String::new(),
            origin: String::new(),
            origin_type: String::new(),
            invoke_id: invoke_id.to_string(),
        },
        target: String::new(),
    }
}

/// Extract metadata fields from an `EventWithMetadata` as a tuple for script engine binding.
///
/// Returns `(name, data, type, sendid, origin, origintype, invokeid)` -- the seven
/// W3C SCXML 5.10.1 system variable fields.
///
/// This is the Rust equivalent of C++ `populatePolicyFromMetadata`, adapted for
/// the Rust architecture where generated code does not expose raw struct fields.
pub fn extract_event_fields<E: Copy>(
    event_with_meta: &EventWithMetadata<E>,
    get_event_name: impl Fn(E) -> &'static str,
) -> EventFields {
    EventFields {
        name: get_event_name(event_with_meta.event).to_string(),
        data: event_with_meta.metadata.data.clone(),
        event_type: event_with_meta.metadata.event_type.as_str().to_string(),
        send_id: event_with_meta.metadata.send_id.clone(),
        origin: event_with_meta.metadata.origin.clone(),
        origin_type: event_with_meta.metadata.origin_type.clone(),
        invoke_id: event_with_meta.metadata.invoke_id.clone(),
    }
}

/// W3C SCXML 5.10.1: Extracted event fields for script engine binding.
#[derive(Debug, Clone, Default)]
pub struct EventFields {
    /// `_event.name`
    pub name: String,
    /// `_event.data`
    pub data: String,
    /// `_event.type` -- "internal", "external", or "platform"
    pub event_type: String,
    /// `_event.sendid`
    pub send_id: String,
    /// `_event.origin`
    pub origin: String,
    /// `_event.origintype`
    pub origin_type: String,
    /// `_event.invokeid`
    pub invoke_id: String,
}
