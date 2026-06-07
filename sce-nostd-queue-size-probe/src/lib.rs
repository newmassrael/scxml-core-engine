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
//! The const assertion below fails the build if the generated `Engine`'s size
//! regresses to the depth-64 default — the "capacity branch silently reverted
//! to the bare form" regression (or a runtime regression where
//! `EventQueueManager<T, N>` stops honoring `N`). Either way the two depth-64
//! queues alone would add `2 × 64 × size_of::<EventWithMetadata<_>>()` (~205 KiB
//! for this machine), so the bound below is comfortably between the real
//! depth-2 size and any depth-64 reversion.
//!
//! Measured at the RFC landing: `size_of::<Engine<P>>()` for this machine is
//! ~23.7 KiB at depth 2 versus ~222 KiB if reverted to depth 64. A plain
//! compile gate cannot catch the reversion (the bare form still compiles), so
//! this size assertion is the load-bearing gate.

#![no_std]

use sce_nostd_queue_size_machine::EventQueueCapacityProbePolicy;
use sce_rust_runtime::{Engine, EventMetadata, StatePolicy};

// The two levers are gated *directly* at their own types (robust — independent
// of the machine's other state, which the scheduler now dominates), plus a
// loose whole-`Engine` sanity bound. Measured on thumbv7em-none-eabihf.

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

/// Loose whole-`Engine` sanity bound (measured 17.5 KiB; the scheduler ring
/// dominates the non-queue state). Catches a gross regression that balloons the
/// engine without tripping the two precise bounds above.
const ENGINE_SANITY_BOUND: usize = 32 * 1024;

const _: () = assert!(
    core::mem::size_of::<Engine<EventQueueCapacityProbePolicy>>() <= ENGINE_SANITY_BOUND,
    "Engine<EventQueueCapacityProbePolicy> exceeds its loose no_std sanity bound — some per-machine \
     no_std footprint regressed. See claudedocs/rfc-nostd-per-machine-event-queue-sizing.md.",
);
