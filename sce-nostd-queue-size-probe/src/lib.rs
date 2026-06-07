// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Per-machine no_std event-queue-sizing gate (compile-time `size_of` assertion).
//!
//! RFC `claudedocs/rfc-nostd-per-machine-event-queue-sizing.md`. The sibling
//! `machine` crate is a generated `--no-std` state machine from
//! `sce-build/tests/fixtures/no_std/event_queue_capacity_probe.scxml`, which
//! declares `<scxml sce:capacity="2">`. Codegen therefore sizes that machine's
//! W3C Appendix D internal/external queues to depth 2 via
//! `StatePolicy::EventQueue`.
//!
//! The const assertions below fail the build if any of the three per-machine
//! no_std footprint levers silently reverts — each is gated at its own type so
//! the bound is robust against unrelated machine state:
//!
//! - **Queue lever**: a `<sce:capacity>` reversion to the depth-64 default
//!   (the "capacity branch silently reverted to the bare form" regression, or
//!   `EventQueueManager<T, N>` ceasing to honor `N`).
//! - **Metadata lever**: a `_event.*` `SceString` re-added to the queued
//!   `EventMetadata` (which is 1 B under no_std).
//! - **Scheduler lever**: the per-entry delayed-send `event_data` string
//!   re-added to `ScheduledEntry` under no_std.
//!
//! A plain compile gate cannot catch any of these — the reverted form still
//! compiles — so these `size_of` assertions are the load-bearing gates.
//! Measured on thumbv7em-none-eabihf: `size_of::<Engine<P>>()` for this machine
//! is ~9.0 KiB (down from ~17.5 KiB before the scheduler lever, ~23.7 KiB
//! before the metadata lever).

#![no_std]

use sce_nostd_queue_size_machine::EventQueueCapacityProbePolicy;
use sce_rust_runtime::engine::PullScheduler;
use sce_rust_runtime::{Engine, EventMetadata, StatePolicy};

/// The probe machine's event enum — the `E` in `PullScheduler<E>` /
/// `ScheduledEntry<E>` for the scheduler lever bound below.
type ProbeEvent = <EventQueueCapacityProbePolicy as StatePolicy>::Event;

// The three levers are gated *directly* at their own types (robust — independent
// of the machine's other state; the per-entry `send_id` string now dominates the
// scheduler), plus a loose whole-`Engine` sanity bound. Measured on
// thumbv7em-none-eabihf.

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
     form, or EventQueueManager<T, N> stopped honoring N). \
     See claudedocs/rfc-nostd-per-machine-event-queue-sizing.md.",
);

/// Metadata lever. After eliding the five `_event.*` `SceString`s
/// (`data` / `sendid` / `origin` / `origintype` / `invokeid`) under no_std,
/// `EventMetadata` is 1 B (just `event_type`); reintroducing any one
/// `heapless::String<256>` pushes it to ≥ 265 B. Independent of any machine.
const META_SIZE_BOUND: usize = 64;

const _: () = assert!(
    core::mem::size_of::<EventMetadata>() <= META_SIZE_BOUND,
    "no_std EventMetadata exceeds its size bound: a `_event.*` string field that should be \
     #[cfg(not(feature = \"no_std\"))] (data / sendid / origin / origintype / invokeid) was \
     reintroduced into the queued metadata. See claudedocs/rfc-nostd-event-metadata-elision.md.",
);

/// Scheduler lever. Each `ScheduledEntry` elides its delayed-send
/// `event_data` `heapless::String<256>` (~264 B) under no_std — the no_std
/// build has no script engine and the scheduler drain discards the data
/// string, so storing it per entry is dead weight. `PullScheduler<ProbeEvent>`
/// measures ~8.7 KiB at `MAX_SCHEDULED_EVENTS = 32`; reintroducing the per-entry
/// data string adds `264 × 32 ≈ 8.4 KiB`, pushing it past ~17 KiB. The bound
/// sits between the two so the elision can't silently revert. (The per-entry
/// `send_id` string is intentionally retained — it is load-bearing for `<cancel>`;
/// its per-machine elision is deferred, see `MAX_SCHEDULED_EVENTS` in `lib.rs`.)
const SCHEDULER_TYPE_BOUND: usize = 12 * 1024;

const _: () = assert!(
    core::mem::size_of::<PullScheduler<ProbeEvent>>() <= SCHEDULER_TYPE_BOUND,
    "PullScheduler<E> exceeds its bound: the per-entry delayed-send `event_data` \
     string was likely reintroduced into ScheduledEntry under no_std (it must be \
     #[cfg(not(feature = \"no_std\"))]). See the ScheduledEntry doc-comment in \
     sce-rust-runtime/src/engine.rs.",
);

/// Loose whole-`Engine` sanity bound (measured ~9.0 KiB after the scheduler
/// `event_data` elision, down from ~17.5 KiB). Catches a gross regression that
/// balloons the engine without tripping the three precise per-lever bounds
/// above (queue / metadata / scheduler).
const ENGINE_SANITY_BOUND: usize = 32 * 1024;

const _: () = assert!(
    core::mem::size_of::<Engine<EventQueueCapacityProbePolicy>>() <= ENGINE_SANITY_BOUND,
    "Engine<EventQueueCapacityProbePolicy> exceeds its loose no_std sanity bound — some per-machine \
     no_std footprint regressed. See claudedocs/rfc-nostd-per-machine-event-queue-sizing.md.",
);
