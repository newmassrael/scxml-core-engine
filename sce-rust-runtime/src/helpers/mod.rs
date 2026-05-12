// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! W3C SCXML algorithm helpers.
//!
//! Ports the C++ helper headers at `sce/include/core/` (25 files) and
//! `sce/include/common/` (26 files). All helpers here are pure functions over
//! `P: StatePolicy`, matching the C++ pattern of template specialization
//! (e.g., `HierarchicalStateHelper<StatePolicy>::findLCA(a, b)`).
//!
//! ## Phase 1 scope
//!
//! - [`hierarchy`]: LCA, entry/exit chain construction (`HierarchicalStateHelper`)
//! - [`event_queue`]: FIFO internal/external queues (`EventQueueManager`)
//! - [`logger`]: thin `log` crate re-exports (`SCE_LOG_*` macros)
//! - [`scxml_constants`]: W3C URIs and string literals
//! - [`state_policy_concepts`]: Rust trait bounds replacing C++20 concepts
//!
//! ## Phase 2 scope
//!
//! - [`conflict_resolution`]: W3C Appendix D.2 transition conflict resolution
//! - [`state_entry`]: Deep initial state entry path calculation
//! - [`parallel_state`]: Parallel state processing (merged 6 C++ headers)
//! - [`history`]: History state recording and restoration
//! - [`event_matching`]: W3C 5.9.3 event descriptor matching
//! - [`entry_exit`]: Entry/exit action block execution with error isolation
//! - [`event_processing`]: W3C macrostep algorithm
//! - [`event_data`]: Event data JSON construction
//! - [`event_type`]: Event type classification
//! - [`in_predicate`]: In(stateId) predicate
//! - [`event_metadata`]: `_event.*` field construction
//! - [`string_utils`]: Platform event detection
//! - [`unique_id_generator`]: Session/send/invoke/event ID generation
//! - [`done_data`]: Donedata processing
//! - [`send`]: Send action helpers (static-target subset)
//! - [`assign`]: Assignment location validation (static-value variant)
//! - [`foreach`]: Foreach iteration (static variant)
//! - [`guard`]: Guard condition evaluation (In() predicate-based)
//! - [`datamodel_init`]: Datamodel initialization helpers
//! - [`url_encoding`]: RFC 3986 URL encoding/decoding

// Phase 1 modules
pub mod event_queue;
pub mod hierarchy;
pub mod logger;
pub mod scxml_constants;
pub mod state_policy_concepts;

// Phase 2 modules
pub mod assign;
pub mod conflict_resolution;
pub mod datamodel_init;
pub mod done_data;
pub mod entry_exit;
// Watching-zenoh RFC §5.J.2: event-data JSON construction is alloc-coupled
// (BTreeMap<String, Vec<String>> input, String output). No template emits
// calls into this helper today, so the no_std codegen has no consumer — the
// no_std variant lands with a heapless::String<N> output + &[(&str, &str)]
// input under a future atomic when consumer demand surfaces.
#[cfg(not(feature = "no_std"))]
pub mod event_data;
pub mod event_matching;
pub mod event_metadata;
pub mod event_processing;
pub mod event_type;
pub mod foreach;
pub mod guard;
pub mod history;
pub mod in_predicate;
pub mod parallel_state;
pub mod send;
pub mod state_entry;
pub mod string_utils;
pub mod unique_id_generator;
// Watching-zenoh RFC §5.J.2: invoke processing is alloc-coupled (Arc/Mutex/Vec/
// HashMap) and never reached under `--no-std` since the codegen-time validator
// rejects `<invoke>` via `codegen/no-std-invoke-not-supported`.
#[cfg(not(feature = "no_std"))]
pub mod invoke_processing;
pub mod url_encoding;
