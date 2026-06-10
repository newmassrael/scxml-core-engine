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
//! - `conflict_resolution` (std-only): W3C Appendix D.2 transition conflict resolution
//! - `state_entry` (std-only): Deep initial state entry path calculation
//! - `parallel_state` (std-only): Parallel state processing (merged 6 C++ headers)
//! - `history` (std-only): History state recording and restoration
//! - [`event_matching`]: W3C 5.9.3 event descriptor matching
//! - [`entry_exit`]: Entry/exit action block execution with error isolation
//! - [`event_processing`]: W3C macrostep algorithm
//! - `event_data` (std-only): Event data JSON construction
//! - [`event_type`]: Event type classification
//! - [`in_predicate`]: In(stateId) predicate
//! - `event_metadata` (std-only): `_event.*` field construction
//! - [`string_utils`]: Platform event detection
//! - [`unique_id_generator`]: Session/send/invoke/event ID generation
//! - [`done_data`]: Donedata processing
//! - [`send`]: Send action helpers (static-target subset)
//! - [`assign`]: Assignment location validation (static-value variant)
//! - [`foreach`]: Foreach iteration (static variant)
//! - [`guard`]: Guard condition evaluation (In() predicate-based)
//! - [`datamodel_init`]: Datamodel initialization helpers
//! - `url_encoding` (std-only): RFC 3986 URL encoding/decoding
//!
//! Modules marked "std-only" are `!no_std`-gated (see the `#[cfg]` rationale
//! on each `pub mod` below); their index entries stay plain code spans because
//! an intra-doc link to them cannot resolve in the no_std docs profile.

// Phase 1 modules
pub mod event_queue;
pub mod hierarchy;
pub mod logger;
pub mod scxml_constants;
pub mod state_policy_concepts;

// Phase 2 modules
pub mod assign;
// Watching-zenoh RFC §5.J.2: W3C Appendix D.2 conflict resolution helpers use
// `Vec<P::State>` / `Vec<TransitionDescriptor<P::State>>` accumulation patterns
// that are alloc-coupled. The Rust AOT codegen template
// (`tools/codegen/templates/rust/conflict_resolution.rs.jinja2`) re-implements
// these algorithms inline on the generated `Self` impl rather than calling
// into this module, so the no_std build has zero current consumer. Mirrors
// `event_data` / `invoke_processing` whole-module gate precedent.
#[cfg(not(feature = "no_std"))]
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
// Watching-zenoh RFC §5.J.2: builds the script-engine `_event.*` object from the
// W3C string metadata. The script engine (and that metadata) are absent under
// no_std — reading `_event.sendid` / `_event.origin` / dynamic `_event.data` is
// rejected (`codegen/no-std-script-not-supported`), so no no_std machine reaches
// this helper. Gated so the no_std `EventMetadata` (which elides those strings)
// has no caller referencing the dropped fields.
#[cfg(not(feature = "no_std"))]
pub mod event_metadata;
pub mod event_processing;
pub mod event_type;
pub mod foreach;
pub mod guard;
// Watching-zenoh RFC §5.J.2: history-state helpers use `Vec<P::State>` +
// `HashSet<P::State>` accumulators. Zero template consumer — the Rust AOT
// generator emits history-state handling inline on the generated `Self` impl.
#[cfg(not(feature = "no_std"))]
pub mod history;
pub mod in_predicate;
// Watching-zenoh RFC §5.J.2: parallel-region helpers use `Vec` / `HashSet` /
// `HashMap<S, S>` accumulators across 30+ public fns. Zero template consumer —
// the Rust AOT generator emits region traversal inline.
#[cfg(not(feature = "no_std"))]
pub mod parallel_state;
pub mod send;
// Watching-zenoh RFC §5.J.2: deep-state-entry helpers use `Vec<Vec<P::State>>`
// + `HashSet<S>` accumulators. Zero template consumer.
#[cfg(not(feature = "no_std"))]
pub mod state_entry;
pub mod string_utils;
pub mod unique_id_generator;
// Watching-zenoh RFC §5.J.2: invoke processing is alloc-coupled (Arc/Mutex/Vec/
// HashMap) and never reached under `--no-std` since the codegen-time validator
// rejects `<invoke>` via `codegen/no-std-invoke-not-supported`.
#[cfg(not(feature = "no_std"))]
pub mod invoke_processing;
pub mod url_encoding;
