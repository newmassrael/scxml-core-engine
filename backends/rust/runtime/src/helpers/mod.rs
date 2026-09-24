// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! W3C SCXML algorithm helpers.
//!
//! Ports of the C++ helper headers (`sce/include/core/`,
//! `sce/include/common/`) that the Rust engine and generated code consume.
//! Most helpers here are pure functions over `P: StatePolicy`, matching the
//! C++ pattern of template specialization; [`microstep`] is written over its
//! own `Document` / `Run` traits instead, the way the C++ procedures take a
//! `Host`.
//!
//! W3C SCXML Appendix D's microstep — selection, conflict removal, the exit
//! and entry sets, history, deep and multi-target entry — is [`microstep`],
//! written once here and read by the engine for every machine, with the
//! generated policy supplying only its document's tables and hooks. C++
//! helpers whose role lives in generated scripting glue (script-engine guard,
//! `In()` predicate, `_event.*` object construction) have no Rust module — the
//! generated machine and the codegen filters are their single home.
//!
//! ## Module index
//!
//! - [`configuration`]: §scxml-3.3 / §scxml-3.4 restored-configuration validation
//! - [`datamodel_init`]: Datamodel initialization helpers
//! - `datamodel_read` (std-only): Typed reads of a live datamodel variable
//! - [`done_data`]: Donedata processing
//! - [`entry_exit`]: Entry/exit action block execution with error isolation
//! - `event_data` (std-only): Event data JSON construction
//! - [`event_matching`]: W3C 5.9.3 event descriptor matching
//! - [`event_queue`]: FIFO internal/external queues (`EventQueueManager`)
//! - [`foreach`]: Foreach iteration (static variant)
//! - [`hierarchy`]: the bounded state chain configurations are held in (`StateChain`)
//! - `invoke_processing` (std-only): §scxml-6.4 invoke processing algorithms
//! - `io_processors` (std-only): §scxml-C-1-1 `_ioprocessors` descriptors
//! - [`logger`]: thin `log` crate re-exports (`SCE_LOG_*` macros)
//! - [`microstep`]: Appendix D's microstep — selection, conflict removal, exit
//!   set, entry set (`MicrostepAlgorithms` / `EntrySetAlgorithms`)
//! - [`scxml_constants`]: W3C URIs and string literals
//! - [`send`]: Send action helpers (static-target subset)
//! - [`state_policy_concepts`]: Rust trait bounds replacing C++20 concepts
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

// §scxml-3.3 / §scxml-3.4: whether a chain a host hands back is a configuration
// this document can hold. Ungated — it reads the policy's static hierarchy and
// allocates nothing, so the no_std profile asks the same question the std one
// does. `Engine::enter_at` is its only caller.
pub mod configuration;
pub mod datamodel_init;
// Typed reads of a live datamodel variable. Gated for the same reason
// `io_processors` is — a typed accessor is emitted only for a `<data>` that
// carries an initializer, and such a document needs a script engine, which no
// no_std machine has.
#[cfg(not(feature = "no_std"))]
pub mod datamodel_read;
pub mod done_data;
pub mod entry_exit;
// SCE Protocol-Synthesis RFC §synth-5-J-2: event-data JSON construction is alloc-coupled
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
// SCE Protocol-Synthesis RFC §synth-5-J-2: invoke processing is alloc-coupled (Arc/Mutex/Vec/
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
// §scxml-D-microstepProcedure: the appendix's procedures over a document and a
// running machine the caller supplies. Ungated — every collection it builds is
// a `StateChain` or a `SceTransitionBuf`, which the no_std profile bounds.
pub mod microstep;
pub mod scxml_constants;
pub mod send;
pub mod state_policy_concepts;
pub mod unique_id_generator;
pub mod url_encoding;
