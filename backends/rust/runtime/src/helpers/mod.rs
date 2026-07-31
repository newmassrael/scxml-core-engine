// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! W3C SCXML algorithm helpers.
//!
//! Ports of the C++ helper headers (`sce/include/core/`,
//! `sce/include/common/`) that the Rust engine and generated code consume.
//! All helpers here are pure functions over `P: StatePolicy`, matching the
//! C++ pattern of template specialization (e.g.,
//! `HierarchicalStateHelper<StatePolicy>::findLCA(a, b)`).
//!
//! The surface is consumed-only by policy: C++ helpers whose algorithms the
//! Rust AOT generator emits inline on the generated machine (W3C Appendix
//! D.2 conflict resolution, parallel regions, history, deep state entry) or
//! whose role lives in generated scripting glue (script-engine guard,
//! `In()` predicate, `_event.*` object construction) have no Rust module —
//! the generated machine and the codegen filters are their single home.
//!
//! ## Module index
//!
//! - [`datamodel_init`]: Datamodel initialization helpers
//! - [`done_data`]: Donedata processing
//! - [`entry_exit`]: Entry/exit action block execution with error isolation
//! - `event_data` (std-only): Event data JSON construction
//! - [`event_matching`]: W3C 5.9.3 event descriptor matching
//! - [`event_queue`]: FIFO internal/external queues (`EventQueueManager`)
//! - [`foreach`]: Foreach iteration (static variant)
//! - [`hierarchy`]: LCA, entry/exit chain construction (`HierarchicalStateHelper`)
//! - `invoke_processing` (std-only): §scxml-6.4 invoke processing algorithms
//! - `io_processors` (std-only): §scxml-C-1-1 `_ioprocessors` descriptors
//! - [`logger`]: thin `log` crate re-exports (`SCE_LOG_*` macros)
//! - [`scxml_constants`]: W3C URIs and string literals
//! - [`send`]: Send action helpers (static-target subset)
//! - [`state_policy_concepts`]: Rust trait bounds replacing C++20 concepts
//! - [`string_utils`]: Platform event detection
//! - [`unique_id_generator`]: Session/send/invoke/event ID generation
//! - `url_encoding` (std-only): RFC 3986 URL encoding/decoding
//!
//! Modules marked "std-only" are unavailable under `--features no_std` (see
//! the `#[cfg]` rationale on each `pub mod` below).

// Index discipline: std-only entries above are plain code spans, not
// intra-doc links — a cfg-gated target cannot resolve in the no_std docs
// profile (both profiles are doc-gated). rustdoc therefore can never catch
// a stale "(std-only)" marker; tests/helpers_index_drift.rs keeps the
// marker set in lockstep with the actual `cfg(not(feature = "no_std"))`
// gating instead.

pub mod datamodel_init;
pub mod done_data;
pub mod entry_exit;
// Watching-zenoh RFC §synth-5-J-2: event-data JSON construction is alloc-coupled
// (BTreeMap<String, Vec<String>> input, String output). No template emits
// calls into this helper today, so the no_std codegen has no consumer — the
// no_std variant lands with a heapless::String<N> output + &[(&str, &str)]
// input under a future atomic when consumer demand surfaces.
#[cfg(not(feature = "no_std"))]
pub mod event_data;
pub mod event_matching;
pub mod event_queue;
pub mod foreach;
pub mod hierarchy;
// Watching-zenoh RFC §synth-5-J-2: invoke processing is alloc-coupled (Arc/Mutex/Vec/
// HashMap) and never reached under `--no-std` since the codegen-time validator
// rejects `<invoke>` via `codegen/no-std-invoke-not-supported`.
#[cfg(not(feature = "no_std"))]
pub mod invoke_processing;
// §scxml-C-1-1 / §scxml-C-2-3: the `_ioprocessors` entry set. Gated for the
// same reason `url_encoding` is — the entries exist to be published into a
// script engine, which no no_std machine has.
#[cfg(not(feature = "no_std"))]
pub mod io_processors;
pub mod logger;
pub mod scxml_constants;
pub mod send;
pub mod state_policy_concepts;
pub mod string_utils;
pub mod unique_id_generator;
pub mod url_encoding;
