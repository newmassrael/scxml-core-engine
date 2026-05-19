// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! W3C SCXML 6.4: Invoke processing algorithms — Single Source of Truth
//!
//! 1:1 port of C++ `sce/include/core/InvokeProcessingAlgorithms.h` and
//! `sce/include/common/FinalizeHelper.h`.
//!
//! Provides higher-level helpers used by generated invoke code:
//! - [`drain_and_raise_child_events`]: drain child-to-parent event queue and raise with metadata
//! - [`raise_done_invoke`]: generate done.invoke.{id} event when child completes
//! - [`is_platform_event`]: W3C SCXML 6.4.6 platform event filter for autoforward
//!
//! Watching-zenoh RFC §5.J.2 (lines 1989-1994): the entire module is gated to
//! `cfg(not(feature = "no_std"))` because `Arc`/`Mutex`/`Vec`/`HashMap` are
//! `alloc`-coupled. Author-side `<invoke>` is rejected up-front by
//! `validate_no_std_compatibility` (diagnostic `codegen/no-std-invoke-not-supported`),
//! so under `--no-std` no generated code reaches this module.

#![cfg(not(feature = "no_std"))]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::event::EventWithMetadata;
use crate::invoke::ChildSession;
use crate::policy::StatePolicy;
use crate::Engine;

/// Type alias for the shared child-to-parent event queue.
///
/// Matches the `parent_external_queue` field on generated policies.
pub type ParentEventQueue = Arc<Mutex<Vec<(String, String)>>>;

/// W3C SCXML 6.4: Drain child-to-parent events and raise them in the parent engine.
///
/// 1:1 port of the C++ event draining pattern from `InvokeProcessingAlgorithms`.
/// Replaces 3 inline code blocks per invoke (after init, before tick, after tick).
///
/// # Arguments
/// - `queue`: child's `parent_external_queue` (populated by child's `#_parent` send)
/// - `active_invokes`: parent's active invoke tracking map
/// - `invoke_key`: invoke element ID (e.g., `"_invoke_0"`) to look up metadata
/// - `engine`: parent's engine for `raise_external_with_meta`
pub fn drain_and_raise_child_events<P: StatePolicy>(
    queue: &Option<ParentEventQueue>,
    active_invokes: &HashMap<String, ChildSession>,
    invoke_key: &str,
    engine: &mut Engine<P>,
) {
    let parent_events: Vec<(String, String)> = if let Some(ref q) = queue {
        let mut locked = q.lock().unwrap();
        locked.drain(..).collect()
    } else {
        Vec::new()
    };

    for (ev_name, ev_data) in parent_events {
        if let Some(event) = P::get_event_from_name(&ev_name) {
            let mut meta = EventWithMetadata::new(event);
            meta.metadata.data = ev_data;
            meta.metadata.invoke_id = active_invokes
                .get(invoke_key)
                .map_or_else(String::new, |cs| cs.invoke_id.clone());
            meta.metadata.origin = active_invokes
                .get(invoke_key)
                .map_or_else(String::new, |cs| cs.session_id.clone());
            meta.metadata.origin_type =
                super::scxml_constants::SCXML_EVENT_PROCESSOR_TYPE.to_string();
            engine.raise_external_with_meta(meta);
        }
    }
}

/// W3C SCXML 6.4: Raise done.invoke event when child reaches final state.
///
/// Tries specific `done.invoke.{invoke_id}` first, falls back to generic `done.invoke`.
/// This handles both event naming patterns at runtime, replacing conditional template logic.
///
/// `donedata` is the child's stashed top-level `<final>` payload (from
/// [`Engine::donedata_at_final`](crate::Engine::donedata_at_final)). It is
/// lifted onto `_event.data` per W3C SCXML 5.5 + 6.3.1. Pass an empty string
/// when the child has no `<donedata>`.
pub fn raise_done_invoke<P: StatePolicy>(
    invoke_id: &str,
    donedata: String,
    engine: &mut Engine<P>,
) {
    let specific = format!("done.invoke.{}", invoke_id);
    let event = P::get_event_from_name(&specific).or_else(|| P::get_event_from_name("done.invoke"));
    if let Some(ev) = event {
        let mut meta = EventWithMetadata::new(ev);
        meta.metadata.invoke_id = invoke_id.to_string();
        meta.metadata.data = donedata;
        engine.raise_external_with_meta(meta);
    }
}

/// W3C SCXML 6.4.6: Check if an event is a platform event (should not autoforward).
///
/// 1:1 port of C++ `InvokeProcessingAlgorithms::isPlatformEvent`.
/// Platform events start with `"#_"` and must never be autoforwarded to children.
pub fn is_platform_event(event_name: &str) -> bool {
    event_name.starts_with("#_")
}
