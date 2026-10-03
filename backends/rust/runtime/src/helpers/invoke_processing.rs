// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! §scxml-6.4: Invoke processing algorithms — Single Source of Truth
//!
//! 1:1 port of C++ `sce/include/core/InvokeProcessingAlgorithms.h` and
//! `sce/include/common/FinalizeHelper.h`.
//!
//! Provides higher-level helpers used by generated invoke code:
//! - [`drain_and_raise_child_events`]: drain child-to-parent event queue and raise with metadata
//! - [`raise_done_invoke`]: generate done.invoke.{id} event when child completes
//!
//! SCE Protocol-Synthesis RFC §synth-5-J-2 (lines 1989-1994): the entire module is gated to
//! `cfg(not(feature = "no_std"))` because `Arc`/`Mutex`/`Vec`/`HashMap` are
//! `alloc`-coupled. Author-side `<invoke>` is rejected up-front by
//! `validate_no_std_compatibility` (diagnostic `codegen/no-std-invoke-not-supported`),
//! so under `--no-std` no generated code reaches this module.

#![cfg(not(feature = "no_std"))]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::event::EventMetadata;
use crate::invoke::ChildSession;
use crate::policy::StatePolicy;
use crate::Engine;

/// Type alias for the shared child-to-parent event queue.
///
/// Matches the `parent_external_queue` field on generated policies.
pub type ParentEventQueue = Arc<Mutex<Vec<(String, String)>>>;

/// §scxml-6.4: Drain child-to-parent events and raise them in the parent engine.
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
        // §scxml-3.12.1 + §scxml-5.10: the child names the event, and its names are
        // not the parent's, so one the parent does not write reaches it as the
        // event of the longest prefix it does, under the name the child sent.
        let mut metadata = EventMetadata {
            data: ev_data,
            invoke_id: active_invokes
                .get(invoke_key)
                .map_or_else(String::new, |cs| cs.invoke_id.clone()),
            origin: active_invokes
                .get(invoke_key)
                .map_or_else(String::new, |cs| cs.session_id.clone()),
            ..Default::default()
        };
        metadata.origin_type = super::scxml_constants::SCXML_EVENT_PROCESSOR_TYPE.to_string();
        if let Some(meta) = Engine::<P>::arriving_event(&ev_name, metadata) {
            engine.raise_external_with_meta(meta);
        }
    }
}

/// §scxml-6.4: Raise done.invoke event when child reaches final state.
///
/// Tries specific `done.invoke.{invoke_id}` first, falls back to generic `done.invoke`.
/// This handles both event naming patterns at runtime, replacing conditional template logic.
///
/// `donedata` is the child's stashed top-level `<final>` payload (from
/// [`Engine::donedata_at_final`](crate::Engine::donedata_at_final)). It is
/// lifted onto `_event.data` per §scxml-5.5 + 6.3.1. Pass an empty string
/// when the child has no `<donedata>`.
pub fn raise_done_invoke<P: StatePolicy>(
    invoke_id: &str,
    donedata: String,
    engine: &mut Engine<P>,
) {
    let specific = format!("done.invoke.{}", invoke_id);
    // §scxml-5.10: `_event.name` is the specific name, whichever descriptor of the
    // document (`done.invoke.<id>`, `done.invoke`, `done`) it was matched through.
    let metadata = EventMetadata {
        invoke_id: invoke_id.to_string(),
        data: donedata,
        ..Default::default()
    };
    if let Some(meta) = Engine::<P>::arriving_event(&specific, metadata) {
        engine.raise_external_with_meta(meta);
    }
}
