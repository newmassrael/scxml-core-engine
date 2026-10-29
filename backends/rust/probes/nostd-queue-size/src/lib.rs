// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Per-machine no_std event-queue-sizing gate (compile-time `size_of` assertion).
//!
//! The sibling
//! `machine` crate is a generated `--no-std` state machine from
//! `sce-build/tests/fixtures/no_std/event_queue_capacity_probe.scxml`, which
//! declares `<scxml sce:capacity="2">` and contains no `<cancel>`. Codegen
//! therefore sizes that machine's W3C Appendix D internal/external queues to
//! depth 2 via `StatePolicy::EventQueue`, and selects the zero-size
//! `ElidedSendId` for `StatePolicy::ScheduledSendId` (cancel-free).
//!
//! The const assertions below fail the build if any of the per-machine no_std
//! footprint levers silently reverts — each is gated at its own type so the
//! bound is robust against unrelated machine state:
//!
//! - **Queue lever**: a `<sce:capacity>` reversion to the depth-64 default
//!   (the "capacity branch silently reverted to the bare form" regression, or
//!   `EventQueueManager<T, N>` ceasing to honor `N`).
//! - **Metadata lever**: a `_event.*` `SceString` re-added to the queued
//!   `EventMetadata` (which is 1 B under no_std).
//! - **Scheduler lever**: a per-entry `heapless::String<256>` re-added to
//!   `ScheduledEntry` under no_std — either the unconditional `event_data`
//!   payload, or the `send_id` cancel key (elided to `ElidedSendId` here
//!   because the fixture is cancel-free).
//!
//! A plain compile gate cannot catch any of these — the reverted form still
//! compiles — so these `size_of` assertions are the load-bearing gates.
//! Measured on thumbv7em-none-eabihf: `size_of::<Engine<P>>()` for this machine
//! is ~832 B, down from ~9.0 KiB before the scheduler `send_id` elision
//! (~17.5 KiB before the scheduler `event_data` elision, ~23.7 KiB before the
//! metadata elision). The `send_id` elision alone removes 256 B × 32 = 8 KiB.

#![no_std]

use sce_nostd_queue_size_machine::EventQueueCapacityProbePolicy;
use sce_rust_runtime::engine::PullScheduler;
use sce_rust_runtime::{Engine, EventMetadata, StatePolicy};

/// The probe machine's event enum — the `E` in `PullScheduler<E, S>` /
/// `ScheduledEntry<E, S>` for the scheduler lever bound below.
type ProbeEvent = <EventQueueCapacityProbePolicy as StatePolicy>::Event;

/// The probe machine's scheduled-send-id storage — the `S` in
/// `PullScheduler<E, S>`. Codegen selects `ElidedSendId` (zero-size) here
/// because the fixture has no `<cancel>`; this is exactly the type the
/// generated `Engine` embeds, so the scheduler bound below measures the real
/// resident size, not the `SceString`-default `PullScheduler<E>`.
type ProbeSendId = <EventQueueCapacityProbePolicy as StatePolicy>::ScheduledSendId;

// The levers are gated *directly* at their own types (robust — independent of
// the machine's other state), plus a loose whole-`Engine` sanity bound.
// Measured on thumbv7em-none-eabihf.

/// Queue lever. `<EventQueueCapacityProbePolicy>::EventQueue` (one of the two
/// W3C Appendix D queues) measures 16 B at the declared `<sce:capacity="2">`;
/// a depth-64 reversion measures 140 B. Isolating the queue type keeps this
/// insensitive to the scheduler / policy size, so the bound stays meaningful
/// even though each queued event is now tiny (metadata elided, below).
const QUEUE_TYPE_BOUND: usize = 64;

const _: () = assert!(
    core::mem::size_of::<<EventQueueCapacityProbePolicy as StatePolicy>::EventQueue>()
        <= QUEUE_TYPE_BOUND,
    "StatePolicy::EventQueue for the <sce:capacity=\"2\"> machine exceeds its bound: the event \
     queue likely regressed to the depth-64 default (the per-machine type reverted to the bare \
     form, or EventQueueManager<T, N> stopped honoring N). See the EventQueue associated type \
     in backends/rust/runtime/src/policy.rs and backends/rust/runtime/src/helpers/event_queue.rs.",
);

/// Metadata lever. After eliding the five `_event.*` `SceString`s
/// (`data` / `sendid` / `origin` / `origintype` / `invokeid`) under no_std,
/// `EventMetadata` is 1 B (just `event_type`); reintroducing any one
/// `heapless::String<256>` pushes it to ≥ 265 B. Independent of any machine.
const META_SIZE_BOUND: usize = 64;

const _: () = assert!(
    core::mem::size_of::<EventMetadata>() <= META_SIZE_BOUND,
    "no_std EventMetadata exceeds its size bound: a `_event.*` string field that should be \
     #[cfg(feature = \"std\")] (data / sendid / origin / origintype / invokeid) was \
     reintroduced into the queued metadata. See the EventMetadata doc-comment in \
     backends/rust/runtime/src/event.rs.",
);

/// Scheduler lever. Each `ScheduledEntry` elides two per-entry
/// `heapless::String<256>`s under no_std: the unconditional delayed-send
/// `event_data` payload (no script engine to read it), and — for a cancel-free
/// machine like this fixture — the `send_id` cancel key (`StatePolicy::ScheduledSendId`
/// = `ElidedSendId`, since the timer drain never reads it and no `<cancel>`
/// matches on it). `PullScheduler<ProbeEvent, ProbeSendId>` measures **528 B** at
/// `MAX_SCHEDULED_EVENTS = 32`; reintroducing either string adds 256 B × 32 =
/// 8 KiB, pushing it to ≥ 8.7 KiB. The bound sits between the two so neither
/// elision can silently revert (reverting `send_id` = flipping the fixture's
/// `type ScheduledSendId` to `SceString`; reverting `event_data` = un-`cfg`-ing
/// the field in `ScheduledEntry`).
const SCHEDULER_TYPE_BOUND: usize = 2 * 1024;

const _: () = assert!(
    core::mem::size_of::<PullScheduler<ProbeEvent, ProbeSendId>>() <= SCHEDULER_TYPE_BOUND,
    "PullScheduler<E, S> exceeds its bound: a per-entry delayed-send string was likely \
     reintroduced into ScheduledEntry under no_std — either the `event_data` payload (it must \
     be #[cfg(feature = \"std\")]) or the `send_id` cancel key (the cancel-free machine's \
     `type ScheduledSendId` must be `ElidedSendId`, not `SceString`). See the ScheduledEntry \
     doc-comment and the ScheduledSendId associated type in backends/rust/runtime/src/engine.rs.",
);

/// The probe machine's tables (docs/adr/0064): the rows of the document the
/// buffers below are sized for.
const PROBE_TABLES: &sce_rust_runtime::DocumentTables =
    <EventQueueCapacityProbePolicy as StatePolicy>::TABLES;

/// The document the bounds below are those of: three flat states and no
/// `<history>`. A set of its states is a bit vector of three states, one word.
const _: () = assert!(
    PROBE_TABLES.states.len() == 3 && PROBE_TABLES.histories.is_empty(),
    "the flat probe machine's tables are not those of three flat states with no history: the \
     fixture gained a state or a history, or the generator wrote rows the document does not \
     have (sce-build/src/document_tables.rs)",
);

/// Document-bound lever. Appendix D's procedures hold the sets of states they
/// build, an enabled set and the walks' stack in the workspace of the call that
/// runs a macrostep, so every slot of their capacities is paid on the stack at
/// the deepest point of a call. The generator sizes them to what this
/// document's state tree bounds, and this machine is three flat states with no
/// `<history>`, so its sets are bit vectors of three states, one transition is
/// enabled, the walks owe three steps for its one level and three of headroom,
/// and no history default is ever owed: where a bound written for every
/// document was 16 and then 32 (516 B an enabled set). The workspace is
/// asserted to be exactly the type of those bounds, since a capacity that is
/// merely small enough would let the generator drift back toward a constant
/// without a word: this item stops compiling (a mismatched type) when the
/// machine's `type Workspace` is spelled with other capacities, because the
/// generator stopped reading the document's state tree
/// (sce-build/src/microstep_bounds.rs) or the Rust template's `type Workspace`
/// (tools/codegen/templates/rust/state_machine.rs.jinja2) changed.
const _: <EventQueueCapacityProbePolicy as StatePolicy>::Workspace =
    sce_rust_runtime::Workspace::<{ sce_rust_runtime::state_scratch(3) }, 1, 6, 0>::new();

/// Constant-construction lever. A bare-metal program holds its engine in a
/// `static`, so nothing is built at start-up and no copy of the engine is made on
/// a stack: `Engine::new_const` over the generated policy's `new_const`, which
/// holds the id to the Nmtoken rule while the program is compiled. A `static`'s
/// initialiser must be a constant expression, so this item stops compiling if the
/// generated policy loses its constants (`ConstStatePolicy`), if `build` stops
/// being a `const fn`, or if the engine gains a field with a starting value that
/// is not one (a heap-backed collection under the bare-metal profile, say).
pub static ENGINE_BUILT_IN_ITS_SLOT: Engine<EventQueueCapacityProbePolicy> =
    Engine::new_const(EventQueueCapacityProbePolicy::new_const("queue-size-probe"));

/// The configuration and history records the engine keeps for the whole run,
/// spelled to the document too: a configuration of one word for its three
/// states and no `<history>` to record. A store of a constant's size would be
/// paid in static RAM by every machine for as long as it runs.
const _: () = assert!(
    core::mem::size_of::<<EventQueueCapacityProbePolicy as StatePolicy>::RunStore>()
        == core::mem::size_of::<
            sce_rust_runtime::RunStore<{ sce_rust_runtime::state_words(3) }, 0, 0>,
        >(),
    "the machine's run store is not the size of a store of the document's bounds: its type was \
     spelled with other capacities. See the Rust template's `type RunStore` \
     (tools/codegen/templates/rust/state_machine.rs.jinja2).",
);

/// Loose whole-`Engine` sanity bound (measured ~832 B after the scheduler
/// `send_id` elision, down from ~9.0 KiB). Catches a gross regression that
/// balloons the engine without tripping the precise per-lever bounds above
/// (queue / metadata / scheduler); set so a single per-entry-string reversion
/// (~+8 KiB → ~9 KiB) also trips here independently as defense in depth.
const ENGINE_SANITY_BOUND: usize = 8 * 1024;

const _: () = assert!(
    core::mem::size_of::<Engine<EventQueueCapacityProbePolicy>>() <= ENGINE_SANITY_BOUND,
    "Engine<EventQueueCapacityProbePolicy> exceeds its loose no_std sanity bound — some per-machine \
     no_std footprint regressed. See the per-lever bounds above for the precise gates.",
);
