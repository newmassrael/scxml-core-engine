// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! [`Engine<P>`]: the execution engine parameterized on a [`StatePolicy`].
//!
//! 1:1 port of C++ `SCE::Static::StaticExecutionEngine<StatePolicy>` from
//! `sce/include/static/StaticExecutionEngine.h`. Method names, behavior, and
//! field layout mirror C++ exactly. When reading alongside the C++ source,
//! every public method here maps to the C++ method of the same name (with
//! `snake_case` conversion).
//!
//! ## Threading model
//!
//! Single-threaded. `Engine<P>` is `Send` but **not** `Sync`. The microstep
//! loop assumes exclusive mutable access. Users needing multi-threaded access
//! wrap in `Arc<Mutex<Engine<P>>>`.
//!
//! ## Safety invariants
//!
//! The engine uses three scoped `unsafe` blocks to split-borrow `self.policy`
//! from the rest of `self` when dispatching policy methods that take both
//! `&mut self` (the policy) and `&mut Engine<Self>` (the engine). Each site
//! is documented with a safety comment. Generated code never writes `unsafe`.
//!
//! ## Public surface
//!
//! - Lifecycle: `new`, `initialize`, `step`, `tick`, `stop`, `is_running`
//! - State queries: `get_current_state`, `get_active_states`, `is_in_final_state`
//! - Event submission: `raise`, `raise_external`, `process_event`
//! - Delayed events (§scxml-6.2): `schedule_event`, `cancel_event`,
//!   `has_ready_events`, backed by [`PullScheduler`]
//! - Hierarchical transition: `handle_hierarchical_transition`
//!
//! HTTP send (§scxml-C-2, `http-send` feature) and `<invoke>` plumbing
//! (§scxml-6.4) are `!no_std`-gated.

// SCE Protocol-Synthesis RFC §synth-5-J-2 (lines 1989-1994): `Arc`/`Mutex` back the
// parent→child external event queue plumbing in `get_external_queue_handle`,
// which is invoke-coupled. The codegen-time validator rejects `<invoke>` under
// `--no-std` via `codegen/no-std-invoke-not-supported`, so the handle is never
// reachable from emitted code; gate the import + method to `!no_std`.
#[cfg(not(feature = "no_std"))]
use std::sync::{Arc, Mutex};
// `Duration` is re-exported by `std::time` from `core::time` and is therefore
// available under both build profiles. The scheduler does not read `Instant`
// directly under either profile: monotonic-time reads route through
// `<P::Hal>::now_ticks_ms()` so the [`Hal`] trait is the single host-clock
// surface (see the `SchedTimePoint` alias below and `sched_now` /
// `sched_now_plus` helpers).
//
// `Instant` is still used by `run_until_completion` for its outer wall-clock
// timeout budget — that is a CI-level deadline, not scheduler time, and must
// stay on the real host clock so a Hal-mock cannot suppress test timeouts.
use core::time::Duration;
#[cfg(not(feature = "no_std"))]
use std::time::Instant;

use crate::clock::SceClock;
use crate::event::{EventMetadata, EventType, EventWithMetadata};
use crate::hal::Hal;
use crate::helpers::configuration::ConfigurationRejection;
use crate::helpers::event_queue::EventQueueLike;
use crate::helpers::{hierarchy, state_policy_concepts as concepts};
use crate::sched_send_id::ScheduledSendIdLike;
// SCE Protocol-Synthesis RFC §synth-5-J-2: the HTTP module is alloc-coupled
// (HashMap<String, Vec<String>> + reqwest) and whole-module-gated to `!no_std`
// in `lib.rs`. The codegen-time validator rejects
// `BasicHTTPEventProcessor` `<send>` under `--no-std` via
// `codegen/no-std-http-not-supported`, so the engine's HTTP fields + dispatch
// surface are unreachable from emitted no_std code.
#[cfg(not(feature = "no_std"))]
use crate::http::{HttpSendRequest, HttpSendResponse};
use crate::policy::StatePolicy;
#[cfg(feature = "no_std")]
use crate::MAX_SCHEDULED_EVENTS;
use crate::{sce_log_debug, sce_log_error, SceString};

// ─────────────────────────────────────────────────────────────────────
// Scheduler time point alias (SCE Protocol-Synthesis RFC §synth-5-J-2 line 1984 HAL)
// ─────────────────────────────────────────────────────────────────────
// `SchedTimePoint` decouples the scheduler's comparable-timestamp type from
// the host clock. Both build profiles use `u64` millisecond ticks read from
// `<P::Hal as Hal>::now_ticks_ms()` — the [`Hal`] trait is the single host-
// clock surface, so a custom `H: Hal` impl (e.g. an advance-able TestHal for
// deterministic timer-firing tests) takes effect identically under std and
// no_std. The `PullScheduler` itself holds no clock source: all time-comparing
// methods take `now: SchedTimePoint` as a parameter (DI pattern), and
// `Engine<P>` resolves `now` via the `sched_now` / `sched_now_plus` helpers
// below.

/// How many links an `error.*` chain may have before the engine stops feeding
/// it — see [`Engine::error_cascade_events`].
///
/// §scxml-3.12.2 says what to do with an error event nothing matches. It does
/// not say what to do when something *does* match it and that handler fails
/// too: the failure raises the same error, the same transition answers it, and
/// the machine has no way out. Nothing in the specification bounds that, so
/// the number is this engine's to choose, and it is chosen to match
/// `check_eventless_transitions`' ceiling — the sibling case of a document
/// that cannot finish a macrostep, decided the same way for the same reason.
///
/// A hundred links is far past any repair strategy a document plausibly
/// spells (a handler that tries a fallback, then a second one, is three) and
/// far short of a number a host would wait through: measured 2026-08-19, the
/// Python engine ran 37,000 links a second on a two-line document, so an
/// unattended supervisor did not hang — it burned a core until it was killed.
pub(crate) const MAX_ERROR_CASCADE_DEPTH: u32 = 100;

/// How many microsteps one macrostep may take before this engine stops taking
/// them — see [`Engine::truncated_macrosteps`].
///
/// The specification defines a macrostep as a chain of microsteps ending in a
/// configuration where nothing is enabled by NULL and the internal queue is
/// empty, and its Principles and Constraints say in as many words that such a
/// chain need not exist: *"A microstep always terminates. A macrostep may not.
/// A macrostep that does not terminate may be said to consist of an infinitely
/// long sequence of microsteps. This is currently allowed."*
///
/// So the ceiling is not conformance — it is this engine declining a document
/// the specification permits, which is exactly why the decline has to be
/// visible.
///
/// One budget for the whole inner loop, not one per branch. Appendix D's loop
/// takes a microstep on an eventless transition *or* on an internal event, and
/// a document alternating the two is one chain, not two: budgeting the
/// branches separately leaves that chain unbounded, which is what a per-call
/// counter on the eventless branch alone did here until 2026-08-20.
///
/// Ten times [`MAX_ERROR_CASCADE_DEPTH`], and deliberately not equal to it.
/// This is the backstop; the cascade ceiling is a diagnostic that names the
/// error a handler keeps failing on, and a backstop that fires first makes
/// that diagnostic unreachable. Measured 2026-08-20: with both at a hundred,
/// a handler that raises one event of its own per link — two microsteps a
/// link, which is what a document that logs before it fails looks like — was
/// cut at fifty links by this ceiling and `error_cascade_events` never moved.
/// The factor of ten is the headroom that keeps the specific report reachable
/// for a handler raising up to eight events a link; a busier one is cut here
/// instead, which is coarser but still reported.
pub(crate) const MAX_MACROSTEP_MICROSTEPS: usize = 1000;

/// Comparable timestamp used by the scheduler: `u64` millisecond ticks read
/// from `<P::Hal as Hal>::now_ticks_ms()` under both std and no_std.
///
/// Under std the default `StdHal::now_ticks_ms()` implementation still reads
/// `std::time::Instant`, so the production clock source is unchanged from
/// pre-HAL-routing behaviour. The difference is that the scheduler now routes
/// through the trait, which means a consumer-provided `H: Hal` impl (assigned
/// via `StatePolicy::Hal = H`) is consulted on every scheduler read — making
/// synthetic-clock tests viable on host as well as embedded.
///
/// Resolution is milliseconds because the W3C SCXML `<send delay>` grammar
/// (§scxml-6.2.2 CSS2 duration) is integer ms/s/min/h; sub-ms scheduling
/// is out of contract.
pub type SchedTimePoint = u64;

/// §scxml-6.2: pull-style scheduler for `<send delay>` events.
///
/// 1:1 API parity with C++ `SCE::PullScheduler<EventType>`. Stores delayed
/// events with a `SchedTimePoint` ready-time and exposes pull-style queries
/// (`has_ready_events_at` / `pop_ready_event_at`) that take the caller's
/// current time as a parameter — a textbook dependency-injection split that
/// keeps the scheduler clock-source-agnostic and makes it unit-testable with
/// synthetic clocks.
///
/// SCE Protocol-Synthesis RFC §synth-5-J-2 (lines 1989-1994): under `--features=no_std`
/// the backing store is a stack-allocated `heapless::Vec` capped at
/// [`crate::MAX_SCHEDULED_EVENTS`] (= 32 in v1; see the `lib.rs` doc-comment for the
/// reasoning and the deferred per-document tunable). Capacity overflow under
/// no_std is treated as a fatal configuration error (panic) per the same
/// "no silent transition drop" discipline the W3C SCXML algorithm follows.
///
/// Kept as a concrete (non-trait) type to match C++ `SCE::PullScheduler<Event> scheduler_;`.
///
/// The `S` parameter is the per-entry cancel-key storage
/// ([`ScheduledSendIdLike`]); it defaults to [`SceString`] so direct
/// constructions (runtime unit tests, the C++-parity API) keep the
/// load-bearing string. Generated code threads in
/// [`StatePolicy::ScheduledSendId`], which is
/// [`ElidedSendId`](crate::ElidedSendId) for cancel-free machines.
#[derive(Debug)]
pub struct PullScheduler<E, S = SceString> {
    /// Pending entries: `(event, [event_data_json], send_id, ready_at)`.
    /// The `event_data_json` field is elided under no_std (see [`ScheduledEntry`]).
    #[cfg(not(feature = "no_std"))]
    entries: Vec<ScheduledEntry<E, S>>,
    #[cfg(feature = "no_std")]
    entries: ::heapless::Vec<ScheduledEntry<E, S>, MAX_SCHEDULED_EVENTS>,
    next_auto_send_id: u64,
}

#[derive(Debug)]
struct ScheduledEntry<E, S> {
    event: E,
    /// Delayed-send `_event.data` JSON payload.
    ///
    /// SCE Protocol-Synthesis RFC §synth-5-J-2: elided under `--features=no_std`. The no_std
    /// build has no script engine, and the scheduler drain
    /// ([`Engine::tick`] → [`Engine::raise_external`]) discards the data string
    /// under no_std (`let _ = (event_data, origin)`), so storing it per entry is
    /// pure dead weight (~264 B `heapless::String<256>` × `MAX_SCHEDULED_EVENTS`).
    /// Mirrors the `EventMetadata.data` profile-level elision (B-γ2d-1).
    #[cfg(not(feature = "no_std"))]
    event_data: SceString,
    /// Cancel key for §scxml-6.3 `<cancel sendid>`. Read only by
    /// [`PullScheduler::cancel_event`]; the storage type `S` is
    /// [`ElidedSendId`](crate::ElidedSendId) (zero-size) for documents with no
    /// `<cancel>`, dropping this field's `heapless::String<256>` under no_std.
    send_id: S,
    ready_at: SchedTimePoint,
}

impl<E: Clone, S: ScheduledSendIdLike> PullScheduler<E, S> {
    /// Construct an empty scheduler.
    pub fn new() -> Self {
        Self {
            #[cfg(not(feature = "no_std"))]
            entries: Vec::new(),
            #[cfg(feature = "no_std")]
            entries: ::heapless::Vec::new(),
            next_auto_send_id: 0,
        }
    }

    /// §scxml-6.2: Schedule an event for delayed delivery, given an
    /// already-resolved `ready_at` time-point.
    ///
    /// If `send_id` is empty, an automatic ID is generated. Returns the ID
    /// used (caller can use it to cancel). The caller is responsible for
    /// computing `ready_at` from the current clock + delay — `Engine<P>`'s
    /// `schedule_event` wrapper does this via `sched_now_plus(delay)`.
    ///
    /// SCE Protocol-Synthesis RFC §synth-5-J-2: under `--features=no_std` an attempted
    /// push past [`crate::MAX_SCHEDULED_EVENTS`] panics rather than silently dropping
    /// the event (W3C SCXML no-silent-drop discipline).
    pub fn schedule_event_at(
        &mut self,
        event: E,
        ready_at: SchedTimePoint,
        send_id: &str,
        event_data: &str,
    ) -> SceString {
        let effective_send_id: SceString = if send_id.is_empty() {
            self.next_auto_send_id += 1;
            format_auto_send_id(self.next_auto_send_id)
        } else {
            crate::sce_string_from_str(send_id)
        };
        // no_std elides the per-entry data string (see `ScheduledEntry`); the
        // parameter is then unused (mirrors `raise_external`'s `let _ = ...`).
        #[cfg(feature = "no_std")]
        let _ = event_data;
        let entry = ScheduledEntry {
            event,
            #[cfg(not(feature = "no_std"))]
            event_data: crate::sce_string_from_str(event_data),
            // `S::store` borrows the id; the load-bearing `SceString` impl
            // clones it, the zero-size `ElidedSendId` impl drops it. The id is
            // still returned below by move (callers may use it to `<cancel>`),
            // so cancel-free machines pay nothing for the discarded storage.
            send_id: S::store(&effective_send_id),
            ready_at,
        };
        self.push_scheduled(entry);
        effective_send_id
    }

    /// Push into [`Self::entries`] uniformly under std and no_std.
    ///
    /// Under std this is `Vec::push` (infallible). Under no_std this is
    /// `heapless::Vec::push` with an `.expect` panic — W3C SCXML mandates no
    /// silent transition drop, so an over-capacity schedule attempt is treated
    /// as a fatal configuration error rather than swallowed.
    #[inline]
    fn push_scheduled(&mut self, entry: ScheduledEntry<E, S>) {
        #[cfg(not(feature = "no_std"))]
        {
            self.entries.push(entry);
        }
        #[cfg(feature = "no_std")]
        {
            self.entries.push(entry).map_err(|_| ()).expect(
                "PullScheduler: heapless capacity exhausted (MAX_SCHEDULED_EVENTS=32 — author has more in-flight <send delay> than scheduler can hold; tune capacity or reduce concurrency)",
            );
        }
    }

    /// §scxml-6.2.5: Cancel a scheduled event by send ID. Returns `true` if found.
    pub fn cancel_event(&mut self, send_id: &str) -> bool {
        let before = self.entries.len();
        self.entries.retain(|e| !e.send_id.matches(send_id));
        self.entries.len() < before
    }

    /// Whether any scheduled events are ready to fire (ready_at <= now).
    ///
    /// Caller supplies the current time — see `Engine<P>::has_ready_events` for
    /// the wrapper that reads `<P::Hal>::now_ticks_ms()`.
    pub fn has_ready_events_at(&self, now: SchedTimePoint) -> bool {
        self.entries.iter().any(|e| e.ready_at <= now)
    }

    /// When the earliest still-queued entry comes due, whether or not it is
    /// ready yet. `None` when nothing is scheduled.
    ///
    /// The queue has always known this; nothing could ask. A host driving the
    /// machine has to decide when to call [`Engine::tick`] again, and without
    /// this it can only guess an interval — see
    /// [`Engine::time_until_next_scheduled_ms`] for the wrapper that turns the
    /// answer into a sleep, and for what guessing costs.
    pub fn next_ready_at(&self) -> Option<SchedTimePoint> {
        self.entries.iter().map(|e| e.ready_at).min()
    }

    /// Find-and-remove the ready entry that came due first — the single source
    /// of the scan/remove logic both `pop_ready_event_at` profiles project from.
    ///
    /// Deadline order, not insertion order: the caller dispatches these one at
    /// a time and runs a macrostep between them, so whichever comes out first
    /// is the one whose transitions run first. Picking by insertion would let a
    /// later-scheduled event be delivered ahead of an earlier one whenever the
    /// host woke after both came due, which is the difference between a
    /// `<cancel>` landing and being lost. `Iterator::min_by_key` keeps the
    /// first of equal keys, so same-millisecond entries stay in insertion
    /// order.
    ///
    /// `#[inline]` so each profile's projection compiles to the same code as a
    /// hand-inlined scan (no extra `ScheduledEntry` move on the timer-fire
    /// path). `heapless::Vec::remove` is `pub fn remove(&mut self, index:
    /// usize) -> T` — same shape as `Vec::remove`, so this works on both
    /// profiles.
    #[inline]
    fn pop_ready_entry(&mut self, now: SchedTimePoint) -> Option<ScheduledEntry<E, S>> {
        let idx = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| e.ready_at <= now)
            .min_by_key(|(_, e)| e.ready_at)
            .map(|(idx, _)| idx)?;
        Some(self.entries.remove(idx))
    }

    /// Pop the next ready event and its data. Returns `None` if nothing is ready.
    ///
    /// Caller supplies the current time. Matches C++
    /// `PullScheduler::popReadyEvent(E&, string&) -> bool` (but returns
    /// an `Option` tuple instead of bool+out-params, which is the idiomatic Rust shape).
    #[cfg(not(feature = "no_std"))]
    pub fn pop_ready_event_at(&mut self, now: SchedTimePoint) -> Option<(E, SceString)> {
        self.pop_ready_entry(now)
            .map(|entry| (entry.event, entry.event_data))
    }

    /// no_std variant of [`pop_ready_event_at`](Self::pop_ready_event_at).
    ///
    /// The delayed-send data string is elided under no_std (see the private
    /// `ScheduledEntry`'s field docs), so the popped event carries no data. The no_std
    /// drain in [`Engine::tick`] passes `""` to
    /// [`raise_external`](Engine::raise_external), which discards it anyway —
    /// returning `Option<E>` instead of `Option<(E, SceString)>` avoids moving
    /// a 264 B empty `heapless::String` out on every timer fire.
    #[cfg(feature = "no_std")]
    pub fn pop_ready_event_at(&mut self, now: SchedTimePoint) -> Option<E> {
        self.pop_ready_entry(now).map(|entry| entry.event)
    }
}

/// Build the synthetic `auto_send_{N}` id for [`PullScheduler::schedule_event_at`]
/// when the caller passes an empty `send_id`.
///
/// std uses `format!`; no_std writes into a fresh `SceString` via
/// `core::fmt::Write` — heapless's `push_str` is `Result`-returning but the
/// 256-byte cap is far above any realistic `auto_send_<u64>` rendering
/// (`auto_send_18446744073709551615` is 32 bytes).
#[inline]
fn format_auto_send_id(counter: u64) -> SceString {
    #[cfg(not(feature = "no_std"))]
    {
        format!("auto_send_{}", counter)
    }
    #[cfg(feature = "no_std")]
    {
        use core::fmt::Write;
        let mut s = SceString::new();
        let _ = write!(&mut s, "auto_send_{}", counter);
        s
    }
}

impl<E: Clone, S: ScheduledSendIdLike> Default for PullScheduler<E, S> {
    fn default() -> Self {
        Self::new()
    }
}

/// What the engine did with one event it offered to the active configuration.
///
/// This used to be a bare `bool` meaning "the configuration changed", which
/// answers `false` for two unrelated outcomes: an event no transition matched
/// at all, and a targetless internal transition that ran its actions in place.
/// Only the first is the discard the spec's compound-state clause describes —
/// cited at the dequeue that records it — and a count keyed off the old bool
/// would have reported a handled event as one, so the two facts are spelled
/// apart rather than inferred from each other.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EventOutcome {
    /// No transition matched the event in any active state, so it is discarded.
    Discarded,
    /// A transition was selected. `configuration_changed` is `false` for a
    /// targetless internal transition, which leaves the configuration alone.
    Taken { configuration_changed: bool },
}

// ═══════════════════════════════════════════════════════════════════════════
// Engine<P>
// ═══════════════════════════════════════════════════════════════════════════

/// The SCXML execution engine.
///
/// Generic over a [`StatePolicy`] `P` that encodes the state machine structure
/// at compile time. Matches C++ `StaticExecutionEngine<StatePolicy>`.
///
/// ## HAL routing (SCE Protocol-Synthesis RFC §synth-5-J-2)
///
/// The std-touching surface (ticks / wake / irq-save) is reachable through
/// the [`crate::hal::Hal`] trait via the policy's [`StatePolicy::Hal`]
/// associated type. Two consumer methods on [`Engine`]
/// — [`Self::hal_now_ticks_ms`] and [`Self::hal_wake`] — forward to
/// `<P::Hal>::*`. Generated code emits `type Hal = StdHal;` per policy by
/// default, so existing call sites read the host monotonic clock and a
/// no-op waker. A future codegen `-l rust --no-std` flag (Atomic B-β) can
/// substitute a no_std HAL impl per emitted policy without changing the
/// `Engine<P>` shape on the runtime side.
pub struct Engine<P: StatePolicy> {
    /// Generated per-SM policy struct (datamodel, last-transition flags, etc.).
    pub(crate) policy: P,
    /// Currently active state (or deepest active state for parallel machines).
    pub(crate) current_state: P::State,
    /// W3C SCXML Appendix D: Internal event queue (high priority, `<raise>` + targetless sends).
    ///
    /// The concrete (per-machine, no_std-sized) queue type is supplied by the
    /// policy via [`StatePolicy::EventQueue`]; the engine drives it through the
    /// [`EventQueueLike`] abstraction (mirrors C++ `EventQueueAdapter`).
    pub(crate) internal_queue: P::EventQueue,
    /// W3C SCXML Appendix D: External event queue (low priority, external sends).
    pub(crate) external_queue: P::EventQueue,
    /// Whether the engine is currently running (set false by `stop()` and final state).
    pub(crate) is_running: bool,
    /// §scxml-6.4: Completion callback invoked when reaching a final state.
    ///
    /// SCE Protocol-Synthesis RFC §synth-5-J-2: `Box<dyn FnMut>` is alloc-coupled and gated to
    /// `!no_std`. Embedded consumers poll [`is_in_final_state`](Self::is_in_final_state)
    /// instead; a future no_std-compatible completion ABI (extern "C" fn +
    /// userdata) lands when a consumer demands it. Mirrors the gate applied to
    /// `helpers::entry_exit::execute_*_blocks` in B-γ2d-2.
    ///
    /// The `+ Send` bound keeps `Engine<P>: Send` for any `P: Send`, so a host
    /// may move an engine onto a worker thread (a boxed closure with no
    /// auto-trait bound would otherwise make the whole struct `!Send`). The
    /// regression guard is `engine_is_send` in `tests/hello_world.rs`.
    #[cfg(not(feature = "no_std"))]
    pub(crate) completion_callback: Option<Box<dyn FnMut() + Send>>,
    /// §scxml-C-2: HTTP send dispatch callback.
    ///
    /// SCE Protocol-Synthesis RFC §synth-5-J-2: HTTP is rejected upstream under `--no-std`
    /// via `codegen/no-std-http-not-supported`, so the callback field + setter +
    /// dispatcher are all gated to `!no_std`. Generated no_std code never
    /// emits a `perform_http_send` call site.
    ///
    /// Carries the same `+ Send` bound as `completion_callback` so the engine
    /// stays host-movable; see that field for the rationale.
    /// §scxml-6.2.5: handlers for the Event I/O Processor types this
    /// host serves, keyed by the `type` string a `<send>` names.
    ///
    /// Gated to `!no_std` for the reason `on_http_send` is: the registry
    /// is a heap-allocated map of boxed closures. Codegen never emits a
    /// host-processor dispatch under `--no-std` — a declared type is
    /// refused at build time there.
    #[cfg(not(feature = "no_std"))]
    pub(crate) host_processors: crate::host_processor::HostProcessorRegistry,
    #[cfg(not(feature = "no_std"))]
    pub(crate) on_http_send:
        Option<Box<dyn FnMut(HttpSendRequest) -> Option<HttpSendResponse> + Send>>,
    /// §scxml-6.2: Delayed event scheduler.
    ///
    /// The per-entry cancel-key storage is supplied by the policy via
    /// [`StatePolicy::ScheduledSendId`] — [`ElidedSendId`](crate::ElidedSendId)
    /// for cancel-free machines (no_std ring shrinks by the per-entry
    /// `send_id` string), [`SceString`] when the document uses `<cancel>`.
    pub(crate) scheduler: PullScheduler<P::Event, P::ScheduledSendId>,
    /// Where this engine reads "now" from — see [`SceClock`].
    pub(crate) clock: SceClock,
    /// The reading [`clock`](Self::clock) gave when the current turn began, or
    /// `None` between turns. See [`begin_turn`](Self::begin_turn).
    pub(crate) turn_now: Option<SchedTimePoint>,
    /// Whether [`tick`](Self::tick) has ever run on this engine.
    ///
    /// A machine whose policy sets
    /// [`NEEDS_EVENT_SCHEDULER`](StatePolicy::NEEDS_EVENT_SCHEDULER) has delayed
    /// events that only `tick` can deliver, so a host driving it with `step`
    /// alone waits forever with nothing said. This flag is what tells the two
    /// cases apart: once `tick` has run the host owns a clock and its `step`
    /// calls are its own business, so the count below stops.
    pub(crate) tick_has_run: bool,
    /// Macrosteps taken on a scheduler-driven machine before any
    /// [`tick`](Self::tick) — see [`unattended_scheduler_steps`](Self::unattended_scheduler_steps).
    pub(crate) unattended_scheduler_steps: u32,
    /// Events taken off the external queue that no transition matched — see
    /// [`discarded_external_events`](Self::discarded_external_events).
    pub(crate) discarded_external_events: u32,
    /// The most recent event this engine discarded — see
    /// [`last_discarded_event`](Self::last_discarded_event).
    pub(crate) last_discarded_event: Option<P::Event>,
    /// External events this machine never dequeued because it had stopped —
    /// see [`unseen_external_events`](Self::unseen_external_events).
    pub(crate) unseen_external_events: u32,
    /// The most recent event this machine never looked at — see
    /// [`last_unseen_event`](Self::last_unseen_event).
    pub(crate) last_unseen_event: Option<P::Event>,
    /// Events delivered with a payload the datamodel could not read as
    /// structure — see [`undecodable_payloads`](Self::undecodable_payloads).
    pub(crate) undecodable_payloads: u32,
    /// The most recent event whose payload could not be read — see
    /// [`last_undecodable_payload`](Self::last_undecodable_payload).
    pub(crate) last_undecodable_payload: Option<P::Event>,
    /// `error.*` events this engine raised that no transition matched — see
    /// [`unhandled_error_events`](Self::unhandled_error_events).
    pub(crate) unhandled_error_events: u32,
    /// The most recent `error.*` event that went unhandled — see
    /// [`last_unhandled_error`](Self::last_unhandled_error).
    pub(crate) last_unhandled_error: Option<P::Event>,
    /// Whether the drain is currently executing a transition selected by an
    /// `error.*` event — the state in which a newly raised error is a *link in
    /// a chain* rather than a first failure.
    ///
    /// This is the whole discriminator behind
    /// [`error_cascade_events`](Self::error_cascade_events): a document that
    /// answers five hundred separate failures cleanly never sets it twice in a
    /// row, and one whose error handler fails sets it on every link.
    pub(crate) handling_error_event: bool,
    /// How many links the current error chain has, reset the moment the drain
    /// does anything else — see [`error_cascade_events`](Self::error_cascade_events).
    pub(crate) error_cascade_depth: u32,
    /// `error.*` events refused because the chain that raised them had reached
    /// [`MAX_ERROR_CASCADE_DEPTH`] — see
    /// [`error_cascade_events`](Self::error_cascade_events).
    pub(crate) error_cascade_events: u32,
    /// The most recent `error.*` event refused that way — see
    /// [`last_error_cascade_event`](Self::last_error_cascade_event).
    pub(crate) last_error_cascade_event: Option<P::Event>,
    /// Macrosteps this engine stopped at [`MAX_MACROSTEP_MICROSTEPS`] with the
    /// chain still going — see
    /// `truncated_macrosteps`.
    ///
    /// Gone under `no_macrostep_diagnostics`, and the RAM is the point of
    /// removing it: this and its sibling below are per-ENGINE, so an MCU image
    /// holding five machines carries five copies whether or not anything reads
    /// them.
    #[cfg(not(feature = "no_macrostep_diagnostics"))]
    pub(crate) truncated_macrosteps: u32,
    /// The state the drain was in when it last stopped that way — see
    /// `last_truncated_macrostep_state`.
    #[cfg(not(feature = "no_macrostep_diagnostics"))]
    pub(crate) last_truncated_macrostep_state: Option<P::State>,
    /// Microsteps taken by the macrostep now in progress, against
    /// [`MAX_MACROSTEP_MICROSTEPS`].
    ///
    /// A field rather than a local, for the reason Appendix D's loop is one loop:
    /// the eventless branch and the internal-event branch take turns inside a
    /// single macrostep, so a counter that lives in either one alone is reset by
    /// the other and bounds nothing. Cleared where a macrostep begins, which is
    /// the external dequeue.
    pub(crate) macrostep_microsteps_taken: usize,
    /// Whether the macrostep now in progress has already been stopped at
    /// [`MAX_MACROSTEP_MICROSTEPS`].
    ///
    /// The drain is reached twice per macrostep — once from
    /// [`execute_transition`](Self::execute_transition) and once from
    /// §scxml-D-mainEventLoop's own loop — so without this the ceiling is not
    /// a ceiling: each caller gets a fresh budget and the machine takes twice
    /// the microsteps it was allowed, counting each refusal separately.
    /// Cleared where the algorithm's main event loop starts a macrostep, which
    /// is the external dequeue.
    pub(crate) macrostep_truncated: bool,
    /// §scxml-5.5 + 6.3.1: Donedata payload evaluated on top-level `<final>`,
    /// lifted onto `done.invoke.<id>._event.data` by the invoking parent.
    ///
    /// Mirrors the C++ AOT `stashDonedataAtFinal` / `donedataAtFinal()` contract
    /// and the Kotlin `StateMachineEngine.donedataAtFinal` field. Populated by
    /// generated `execute_entry_actions` code on a child's top-level final and
    /// read by the parent's [`helpers::invoke_processing::raise_done_invoke`]
    /// before emitting `done.invoke.<id>`. Typed as [`SceString`] so the no_std
    /// build composes with the capped-string convention used across
    /// `EventMetadata` (B-γ2d-1).
    pub(crate) donedata_at_final: SceString,
}

impl<P: StatePolicy> Engine<P> {
    // ════════════════════════════════════════
    // Construction
    // ════════════════════════════════════════

    /// Construct a new engine with the given policy instance.
    ///
    /// The initial state is set to `P::initial_state()`. The engine is not yet
    /// running — call [`initialize`](Self::initialize) to enter the initial
    /// configuration and begin processing events.
    pub fn new(policy: P) -> Self {
        Self {
            current_state: P::initial_state(),
            policy,
            internal_queue: P::EventQueue::default(),
            external_queue: P::EventQueue::default(),
            is_running: false,
            #[cfg(not(feature = "no_std"))]
            completion_callback: None,
            #[cfg(not(feature = "no_std"))]
            host_processors: Default::default(),
            #[cfg(not(feature = "no_std"))]
            on_http_send: None,
            scheduler: PullScheduler::new(),
            clock: SceClock::Hal,
            turn_now: None,
            tick_has_run: false,
            unattended_scheduler_steps: 0,
            discarded_external_events: 0,
            last_discarded_event: None,
            unseen_external_events: 0,
            last_unseen_event: None,
            undecodable_payloads: 0,
            last_undecodable_payload: None,
            unhandled_error_events: 0,
            last_unhandled_error: None,
            handling_error_event: false,
            error_cascade_depth: 0,
            error_cascade_events: 0,
            last_error_cascade_event: None,
            #[cfg(not(feature = "no_macrostep_diagnostics"))]
            truncated_macrosteps: 0,
            #[cfg(not(feature = "no_macrostep_diagnostics"))]
            last_truncated_macrostep_state: None,
            macrostep_microsteps_taken: 0,
            macrostep_truncated: false,
            donedata_at_final: SceString::new(),
        }
    }

    // ════════════════════════════════════════
    // Scheduler clock readers (per-build cfg-branched, HAL-routed under no_std)
    // ════════════════════════════════════════

    /// Take a fresh reading from [`clock`](Self::clock), whatever turn the
    /// engine is in.
    ///
    /// Only [`begin_turn`](Self::begin_turn) and the between-turn branch of
    /// [`sched_now`](Self::sched_now) call this — everything else asks
    /// `sched_now`, so that the turn latch is the default and a live reading
    /// is the exception that has to be spelled.
    #[inline]
    fn clock_read(&self) -> SchedTimePoint {
        match self.clock {
            SceClock::Hal => <P::Hal as Hal>::now_ticks_ms(),
            SceClock::Manual(now) => now,
            SceClock::Source(read) => read(),
        }
    }

    /// §scxml-3.13: what time it is, for everything this turn arms or judges.
    ///
    /// The clause executes a microstep's executable content as one unit and a
    /// macrostep as a chain of those, so "now" is a property of the turn the
    /// engine is in rather than of the statement asking for it. Between turns
    /// there is no turn for it to be a property of, and the host's queries
    /// ([`time_until_next_scheduled_ms`](Self::time_until_next_scheduled_ms),
    /// [`now_ms`](Self::now_ms)) read the clock live.
    #[inline]
    fn sched_now(&self) -> SchedTimePoint {
        match self.turn_now {
            Some(latched) => latched,
            None => self.clock_read(),
        }
    }

    /// Open a turn: take the single [`clock`](Self::clock) reading that
    /// everything inside it uses.
    ///
    /// Returns whether this call is the one that opened it, which
    /// [`end_turn`](Self::end_turn) needs so a nested entry point
    /// ([`process_event`](Self::process_event) delegating to
    /// [`step`](Self::step), [`tick`](Self::tick) doing the same) does not
    /// close the outer turn early.
    ///
    /// §scxml-6.2.2 makes a delay the wait the DOCUMENT asks for — "how long
    /// the processor should wait before dispatching the message". Time the
    /// host spent descheduled between two statements of one microstep is not
    /// part of any delay the document wrote, so it must not reach the
    /// deadline. Reading the clock per statement instead was two defects at
    /// once, both measured on this backend:
    ///
    /// - Two `<send delay>`s executed by one `<onentry>` took a reading each,
    ///   so a host descheduled between them by more than the gap between their
    ///   delays got the later send's deadline first — and the engine then
    ///   dispatched them in that order, so the document's `<cancel>` arrived
    ///   after the event it named. Which of two events the author ordered
    ///   arrives first became a fact about the host's scheduler.
    /// - The dispatch loop in [`tick`](Self::tick) re-read it on every pass, so
    ///   a tick slow enough to cross the next deadline dispatched that entry
    ///   too, then the one after it — the engine chasing deadlines its own
    ///   slowness created, in a loop the host cannot get between.
    ///
    /// Neither is reachable from a clock that is read once per turn.
    #[inline]
    fn begin_turn(&mut self) -> bool {
        if self.turn_now.is_some() {
            return false;
        }
        self.turn_now = Some(self.clock_read());
        true
    }

    /// Close a turn opened by [`begin_turn`](Self::begin_turn).
    #[inline]
    fn end_turn(&mut self, opened: bool) {
        if opened {
            self.turn_now = None;
        }
    }

    /// Resolve `now + delay` for scheduling.
    ///
    /// Adds `delay.as_millis() as u64` to [`sched_now`](Self::sched_now) via
    /// `saturating_add`, clamping a pathologically large delay to `u64::MAX`
    /// rather than wrapping (`u64::MAX` ms ≈ 584 million years, so the clamp is
    /// operationally indistinguishable from "infinity").
    ///
    /// Resolution is milliseconds on both profiles — the W3C SCXML `<send
    /// delay>` grammar is integer ms/s/min/h, so sub-ms truncation does not
    /// affect spec-conformant state machines. Internal call sites that need
    /// finer resolution should not route through the scheduler at all.
    #[inline]
    fn sched_now_plus(&self, delay: Duration) -> SchedTimePoint {
        let delay_ms = delay.as_millis() as u64;
        self.sched_now().saturating_add(delay_ms)
    }

    // ════════════════════════════════════════
    // HAL-routed queries (SCE Protocol-Synthesis RFC §synth-5-J-2 line 1984)
    // ════════════════════════════════════════

    /// Return the policy's [`Hal`]-routed monotonic millisecond tick count.
    ///
    /// Forwards to `<P::Hal>::now_ticks_ms()`. The [`StatePolicy::Hal`]
    /// associated type determines the dispatch target — generated code
    /// today emits `type Hal = StdHal;` so `Engine<MyPolicy>` reads the
    /// host monotonic clock. A future no_std codegen emission can substitute
    /// a different HAL impl (Atomic B-β).
    pub fn hal_now_ticks_ms(&self) -> u64 {
        <P::Hal as Hal>::now_ticks_ms()
    }

    /// Signal the policy's HAL that the runtime has work to drain.
    ///
    /// Forwards to `<P::Hal>::wake()`. On `StdHal` (current generated-code
    /// default) this is a no-op (single-threaded `!Sync` design — see
    /// crate-level doc). No_std HAL impls typically wire this to the
    /// consumer's executor waker (e.g. embassy `Signal::signal()`).
    pub fn hal_wake(&self) {
        <P::Hal as Hal>::wake();
    }

    // ════════════════════════════════════════
    // Internal split-borrow helper
    //
    // The generated `Policy::execute_entry_actions(state, engine)` and similar
    // methods need mutable access to the policy AND mutable access to the
    // engine at the same time. Rust's borrow checker cannot verify this is
    // safe because both handles point into `self`. We use a scoped raw pointer
    // cast to split the borrows; this is safe because the policy field does
    // not alias any other engine field during the scoped call.
    //
    // This pattern is equivalent to C++ `policy_.executeEntryActions(state, *this)`
    // where the compiler has no aliasing restriction at all. In Rust, we
    // document the invariant and scope the unsafe block to three sites.
    // ════════════════════════════════════════

    /// Execute the policy's `execute_entry_actions` with split-borrowed `self`.
    ///
    /// # Safety
    ///
    /// This function dereferences a raw pointer to `self.policy` while
    /// simultaneously passing `&mut self` to the policy method. This is sound
    /// because:
    /// 1. The policy method only mutates fields within the policy struct via
    ///    the `&mut self` receiver.
    /// 2. The engine fields accessed via the `engine: &mut Engine<P>` parameter
    ///    do NOT overlap with the policy field (`self.policy` is a distinct
    ///    struct field with its own memory).
    /// 3. The pointer is not held beyond the scope of the single call.
    ///
    /// The generated code contract (`StatePolicy::execute_entry_actions`) must
    /// not alias the engine's policy field through `engine.policy` — but this
    /// is a non-issue because the Engine does not expose `policy` publicly.
    pub(crate) fn execute_on_entry(&mut self, state: P::State) {
        self.execute_on_entry_with_path(state, None);
    }

    /// [`Self::execute_on_entry`] for a state that is only an ANCESTOR of the
    /// entry target.
    ///
    /// `path_child` names the child of `state` the entry set already holds —
    /// §scxml-D-addAncestorStatesToEnter, which adds such a state without its
    /// default initial child. See `StatePolicy::execute_entry_actions`.
    pub(crate) fn execute_on_entry_as_ancestor(&mut self, state: P::State, path_child: P::State) {
        self.execute_on_entry_with_path(state, Some(path_child));
    }

    fn execute_on_entry_with_path(&mut self, state: P::State, path_child: Option<P::State>) {
        let policy_ptr: *mut P = &mut self.policy as *mut P;
        // SAFETY: see doc comment above. The policy field and the rest of
        // Engine's fields are disjoint; the split borrow lasts only for the
        // duration of the method call.
        unsafe {
            (*policy_ptr).execute_entry_actions(state, self, path_child);
        }
    }

    /// Execute the policy's `execute_exit_actions` with split-borrowed `self`.
    ///
    /// # Safety
    ///
    /// See [`Self::execute_on_entry`] for the full safety rationale. The
    /// `pre_transition_active` slice is borrowed from the caller's stack and
    /// does not interact with the split borrow.
    pub(crate) fn execute_on_exit(&mut self, state: P::State, pre_transition_active: &[P::State]) {
        let policy_ptr: *mut P = &mut self.policy as *mut P;
        // SAFETY: same as execute_on_entry.
        unsafe {
            (*policy_ptr).execute_exit_actions(state, self, pre_transition_active);
        }
    }

    /// Execute the policy's `process_transition` with split-borrowed `self`.
    ///
    /// # Safety
    ///
    /// See [`Self::execute_on_entry`]. The `current_state` parameter is an
    /// owned local variable on the caller's stack; `process_transition` may
    /// mutate it through the `&mut P::State` reference without any aliasing
    /// with engine fields.
    pub(crate) fn process_transition_dispatch(
        &mut self,
        current_state: &mut P::State,
        event: P::Event,
    ) -> bool {
        let policy_ptr: *mut P = &mut self.policy as *mut P;
        // SAFETY: same as execute_on_entry.
        unsafe { (*policy_ptr).process_transition(current_state, event, self) }
    }

    /// Execute the policy's `execute_transition_actions` with split-borrowed `self`.
    ///
    /// # Safety
    ///
    /// Same as [`Self::execute_on_entry`].
    pub(crate) fn execute_transition_actions_dispatch(&mut self) {
        let policy_ptr: *mut P = &mut self.policy as *mut P;
        // SAFETY: same as execute_on_entry.
        unsafe { (*policy_ptr).execute_transition_actions(self) }
    }

    /// Execute the policy's `initialize_data_model` with split-borrowed `self`.
    pub(crate) fn initialize_data_model_dispatch(&mut self) {
        let policy_ptr: *mut P = &mut self.policy as *mut P;
        // SAFETY: same as execute_on_entry.
        unsafe { (*policy_ptr).initialize_data_model(self) }
    }

    // ════════════════════════════════════════
    // Lifecycle (matches C++ public API)
    // ════════════════════════════════════════

    /// Enter the initial configuration and run the macrostep loop until stable.
    ///
    /// Matches C++ `StaticExecutionEngine::initialize()`. §scxml-5.3
    /// guarantees datamodel initialization happens before any state entry.
    pub fn initialize(&mut self) {
        // §scxml-3.13: entering the initial configuration is one turn, and the
        // `<onentry>` handlers it runs arm their `<send delay>`s against one
        // instant — see `begin_turn` for what reading the clock per `<send>`
        // did to two of them.
        let opened = self.begin_turn();
        self.initialize_in_turn();
        self.end_turn(opened);
    }

    fn initialize_in_turn(&mut self) {
        self.is_running = true;

        // §scxml-5.3: Initialize datamodel before any state entry
        if concepts::has_data_model_init::<P>() {
            self.initialize_data_model_dispatch();
        }

        // §scxml-3.3: Entry chain from root to initial leaf
        let entry_chain = hierarchy::build_entry_chain::<P>(self.current_state);
        for state in entry_chain {
            self.execute_on_entry(state);
        }
        // §scxml-3.3: Resolve current_state to the deepest initial leaf
        self.resolve_current_state_to_leaf();

        // §scxml-D-mainEventLoop: hand over to the outer loop. The macrostep
        // completes on eventless transitions and internal events, then the
        // invokes for the states just entered run, and only then is anything
        // taken off the external queue — so an `autoforward` child is live for
        // every event `<onentry>` queued on the way in.
        sce_log_debug!("Engine::initialize: entering main event loop");
        self.run_main_event_loop();
        sce_log_debug!("Engine::initialize: main event loop settled");

        // §scxml-6.4: Fire completion callback if we reached a final state during init.
        // SCE Protocol-Synthesis RFC §synth-5-J-2: Box<dyn FnMut> callback is alloc-coupled and gated
        // to `!no_std` (see field declaration above).
        #[cfg(not(feature = "no_std"))]
        if self.is_in_final_state() && self.completion_callback.is_some() {
            sce_log_debug!(
                "Engine::initialize: reached final state during init, invoking completion callback"
            );
            let active = self.get_active_states();
            let final_state = self.current_state;
            self.execute_on_exit(final_state, &active);
            if let Some(cb) = self.completion_callback.as_mut() {
                cb();
            }
        }
    }

    /// Enter a configuration this machine was already in, WITHOUT running
    /// `<onentry>`.
    ///
    /// [`initialize`](Self::initialize) is the other door and the contrast is
    /// the whole point: it enters the document's initial configuration and runs
    /// the entry actions of every state on the way in. This one enters a
    /// configuration the caller names and runs none of them. A host that has
    /// persisted where a machine was, and is bringing it back in a new process,
    /// wants the second: §scxml-3.8 entry actions are "executed when the state
    /// is entered", and re-executing them is a replay of what the earlier run
    /// already did — an `<onentry><send>` would post its event a second time,
    /// to a peer that already received it.
    ///
    /// # What it takes, and why two arguments
    ///
    /// Exactly what the two readers on this engine publish:
    /// [`get_active_states`](Self::get_active_states) and
    /// [`get_current_state`](Self::get_current_state). Handing back both is not
    /// redundancy — for a machine with `<parallel>` states the configuration
    /// does not determine the current state. `current_state` is the leaf the
    /// engine descended to through `get_initial_or_history_child`, so which
    /// region it sits in is a fact about the transition history rather than
    /// about the configuration, and a chain alone cannot recover it. For a
    /// machine without parallel states `current` is the chain's leaf and the
    /// check below simply confirms the caller agrees.
    ///
    /// # What it refuses
    ///
    /// Every chain that is not a configuration of THIS document — see
    /// [`configuration::validate`](crate::helpers::configuration::validate) for
    /// the rules and [`ConfigurationRejection`] for what each refusal says. Validation runs before any mutation, so a
    /// refused call leaves the engine exactly as it was; entering "near" the
    /// requested configuration is the one outcome this door must never produce,
    /// because a host has no way to detect it afterwards.
    ///
    /// # What it does not do
    ///
    /// - No `<onentry>`, and no `<onexit>`: no state is entered or left.
    /// - No macrostep. [`initialize`](Self::initialize) settles the machine
    ///   before returning; this does not, because the configuration handed in
    ///   was already a settled one — running the loop here could take an
    ///   eventless transition the earlier run had no reason to take, and fire
    ///   the `<send>`s on the way. The host drives the machine on from here
    ///   with [`step`](Self::step) or [`tick`](Self::tick) as it otherwise
    ///   would.
    /// - No datamodel restore. §scxml-5.3 declaration still runs, so the
    ///   variables exist with their document defaults and a host can then put
    ///   its saved values back through `IScriptEngine` — the engine does not
    ///   persist datamodel state and does not pretend to. (Plain code span, not
    ///   an intra-doc link: `scripting` is gated out of the no_std docs profile,
    ///   where a link to it cannot resolve.)
    pub fn enter_at(
        &mut self,
        configuration: &hierarchy::StateChain<P::State>,
        current: P::State,
    ) -> Result<(), ConfigurationRejection<P::State>> {
        // Before anything is touched: a rejection must not half-enter.
        crate::helpers::configuration::validate::<P>(configuration, current)?;

        // §scxml-5.3: the datamodel is declared before anything can read it.
        // This is not a state entry action — `<datamodel>` holds `<data>`, not
        // executable content — so it runs here for the same reason it runs in
        // `initialize`: a `cond` or an `assign` evaluated after this call would
        // otherwise reference variables that were never declared.
        if concepts::has_data_model_init::<P>() {
            self.initialize_data_model_dispatch();
        }

        self.current_state = current;

        // §scxml-3.4: a machine that keeps its own active set is handed it back.
        // The condition is the one the generator emits `set_active_states`
        // under, so a policy reached here has the override.
        if concepts::has_active_states::<P>() {
            self.policy.set_active_states(configuration.clone());
        }

        self.is_running = true;
        Ok(())
    }

    /// Process one macrostep: drain queues and run eventless transitions.
    ///
    /// Matches C++ `StaticExecutionEngine::step()`. Used by parent SMs to
    /// explicitly drive children after sending them events (§scxml-6.4).
    ///
    /// # Which of `step` and [`tick`](Self::tick) to call
    ///
    /// Do not guess, and do not read it off the document: the generator
    /// decided while compiling and says so in two places, both derived
    /// from the same answer.
    ///
    /// - `StatePolicy::NEEDS_EVENT_SCHEDULER` on the generated policy —
    ///   `false` means this method is enough, `true` means [`tick`](Self::tick).
    /// - `needs_event_scheduler` on the `sce-codegen` stdout manifest,
    ///   for a build system deciding without compiling the machine first.
    ///
    /// Both queues are drained here, delayed sends are not: `step` never
    /// consults the scheduler and never ticks an invoked child. A machine
    /// that needs either and is driven by `step` alone loses those events
    /// — so this method logs an error the first time it is called on one,
    /// rather than letting the loss be silent.
    ///
    /// An *undelayed* `<send>`, whatever its target, is an ordinary
    /// external event and arrives here. Written out because the reverse
    /// was reported in 2026-08 by a consumer whose real problem was an
    /// aborted `<onentry>` block: an error in executable content abandons
    /// the actions after it (§scxml-4.2), so a `<send>` written below a
    /// failing one never runs, and no amount of `tick` recovers it.
    pub fn step(&mut self) {
        // §scxml-3.13: one host call, one reading. The macrostep below can
        // enter a state whose `<onentry>` arms several `<send delay>`s, and
        // they are one instant's worth of executable content however long the
        // host takes to run them — see `begin_turn`.
        let opened = self.begin_turn();
        self.step_in_turn();
        self.end_turn(opened);
    }

    fn step_in_turn(&mut self) {
        // A machine with delayed sends hands `step` a queue it cannot reach:
        // `run_main_event_loop` never consults the scheduler, so the event is
        // neither delivered nor refused. Say it once — the host is driving with
        // the wrong call, and every later macrostep would repeat the same word.
        if concepts::needs_event_scheduler::<P>() && !self.tick_has_run {
            self.unattended_scheduler_steps = self.unattended_scheduler_steps.saturating_add(1);
            if self.unattended_scheduler_steps == 1 {
                sce_log_error!(
                    "Engine::step: this machine has delayed sends and no tick() has run; \
                     delayed events will never fire — drive it with Engine::tick()"
                );
            }
        }

        self.run_main_event_loop();

        #[cfg(not(feature = "no_std"))]
        if self.is_in_final_state() && self.completion_callback.is_some() {
            sce_log_debug!("Engine::step: invoking completion callback");
            if let Some(cb) = self.completion_callback.as_mut() {
                cb();
            }
        }
    }

    /// Poll the scheduler for ready delayed events, then run a macrostep.
    ///
    /// Matches C++ `StaticExecutionEngine::tick()`. Called periodically by
    /// callers that have delayed `<send>` operations.
    ///
    /// A superset of [`step`](Self::step): it drains the delayed-send
    /// scheduler and ticks invoked child sessions, then runs the same
    /// macrostep. Calling it on a machine that needs neither is correct
    /// and merely does two lookups that find nothing — which is why the
    /// generated `NEEDS_EVENT_SCHEDULER` constant is worth reading rather
    /// than defaulting either way. See [`step`](Self::step) for where the
    /// answer is published.
    pub fn tick(&mut self) {
        // §scxml-3.13: one turn, one reading. Everything below judges due
        // against the instant this tick began, and everything the macrosteps
        // below arm is measured from it — so a tick dispatches what was due
        // when the host called it, and cannot be extended by how long it takes
        // to run (see `begin_turn`).
        let opened = self.begin_turn();
        self.tick_in_turn();
        self.end_turn(opened);
    }

    fn tick_in_turn(&mut self) {
        // Recorded before the running check: a host that calls `tick` owns a
        // clock whatever the engine's lifecycle says, and the count exists to
        // find hosts that never call it at all.
        self.tick_has_run = true;
        if !self.is_running {
            return;
        }
        if self.is_in_final_state() {
            #[cfg(not(feature = "no_std"))]
            if let Some(cb) = self.completion_callback.as_mut() {
                sce_log_debug!(
                    "Engine::tick: final state already reached, invoking completion callback"
                );
                cb();
            }
            return;
        }

        // §scxml-6.2: dispatch the ready scheduled events, earliest deadline
        // first and one macrostep apart. "Due" is judged against the instant
        // this tick began, not against a clock re-read on every pass — a tick
        // that chased its own slowness would dispatch entries the host had not
        // yet reached, in a loop the host cannot get between (see
        // `begin_turn`). `PullScheduler` stays clock-source-agnostic either
        // way: it is handed the reading rather than taking one.
        //
        // One at a time, not all at once. `<cancel>` drops an event that has
        // not been dispatched yet, and a host that woke late holds several past
        // their deadlines: promoting them together makes every later one
        // undroppable before the earlier one's transitions have had a chance to
        // run. That is how a settle timer — arm a long `<send delay>`, cancel
        // it when the short signal arrives first — delivers the event it was
        // told to cancel. Measured 2026-08-19 across the Rust, Go and Python
        // backends alike, the Python one on a virtual clock where the host's
        // step size alone decided it.
        loop {
            let now = self.sched_now();
            let Some(popped) = self.scheduler.pop_ready_event_at(now) else {
                break;
            };
            #[cfg(not(feature = "no_std"))]
            {
                let (event, data) = popped;
                self.raise_external(event, &data, "");
            }
            // no_std: the data string is elided in the scheduler, and
            // `raise_external` discards it under no_std anyway.
            #[cfg(feature = "no_std")]
            self.raise_external(popped, "", "");

            // The macrostep this event drives may `<cancel>` a later one, so
            // the queue is re-consulted after it rather than before.
            self.run_main_event_loop();
            if !self.is_running || self.is_in_final_state() {
                break;
            }
        }

        // §scxml-6.4: Tick child state machines
        if concepts::has_child_tick::<P>() {
            let policy_ptr: *mut P = &mut self.policy as *mut P;
            // SAFETY: see execute_on_entry.
            unsafe {
                (*policy_ptr).tick_children(self);
            }
        }

        // Delegate to step() for the main event loop + completion callback.
        // §scxml-6.4's invokes are part of that loop and run there, ahead of
        // the external dequeue rather than after it.
        self.step();
    }

    /// Stop the engine. Subsequent `tick`/`process_event` calls become no-ops.
    pub fn stop(&mut self) {
        self.is_running = false;
    }

    /// Whether the engine is running (not stopped or awaiting completion).
    pub fn is_running(&self) -> bool {
        self.is_running
    }

    /// How many macrosteps ran on a scheduler-driven machine before any
    /// [`tick`](Self::tick).
    ///
    /// Non-zero means the host is driving with [`step`](Self::step) a machine
    /// whose policy sets
    /// [`NEEDS_EVENT_SCHEDULER`](StatePolicy::NEEDS_EVENT_SCHEDULER): the
    /// delayed events sitting in the scheduler have had no opportunity to fire.
    /// It stops counting once `tick` has run, so a host that mixes the two
    /// reads zero-or-more from its start-up and nothing after. Always `0` for a
    /// machine with no delayed send, whatever the host calls.
    ///
    /// A test harness can assert on it; a supervising host can log it. Either
    /// way the wiring mistake becomes something a program can see, which is
    /// what a `step`-only loop otherwise never offers.
    pub fn unattended_scheduler_steps(&self) -> u32 {
        self.unattended_scheduler_steps
    }

    /// How many events this engine took off the external queue and discarded
    /// because no transition in any active state matched them.
    ///
    /// Discarding is what the clause requires. This is the part the clause does
    /// not cover: the host that queued the event cannot otherwise tell that
    /// outcome from a handled one, because a self transition, a targetless
    /// internal transition and a discard all leave the configuration alone.
    /// Comparing the count across a drive is what turns "the machine ignored
    /// what I sent" into something the program can see — the event name the
    /// host used may simply not be one this configuration answers.
    ///
    /// The Interpreter has answered this all along
    /// (`StateMachine::processEvent`'s `TransitionResult::success`, and
    /// `getStatistics().failedTransitions`); this is the generated engines'
    /// side of the same question, so a document moving to AOT keeps it.
    ///
    /// Counts external-queue events only. An internal `<raise>` that matches
    /// nothing is discarded too, but both ends of that are inside the document.
    pub fn discarded_external_events(&self) -> u32 {
        // §scxml-3.1.2 is the clause: an event no transition matches is
        // discarded. Cited in the body rather than the doc comment because the
        // ledger's Rust resolver binds a citation to the symbol that encloses
        // it, and a `///` line encloses nothing.
        self.discarded_external_events
    }

    /// The most recent event [`discarded_external_events`](Self::discarded_external_events)
    /// counted, or `None` while that count is zero.
    ///
    /// A count says something went nowhere; this says which thing did, which is
    /// the question a host debugging a stalled supervisor actually has. Copy,
    /// so reading it costs nothing on the no_std profile.
    pub fn last_discarded_event(&self) -> Option<P::Event> {
        self.last_discarded_event
    }

    /// Record which reading W3C SCXML B.2.8.1 gave the payload just bound.
    ///
    /// Called by generated code immediately after it binds `_event`, because
    /// that is the only moment the rung is known — see
    /// [`PayloadReading`](crate::PayloadReading). Four of the five readings are
    /// the ladder working and are recorded by being ignored; the fifth is the
    /// one a host is wrong about.
    pub fn note_payload_reading(&mut self, event: P::Event, reading: crate::PayloadReading) {
        // W3C SCXML B.2.8.1 is the clause the reading comes from, named in
        // prose rather than as a `§` citation: this method REPORTS the rung a
        // script engine chose, and a bound citation here would claim it
        // implements the clause. The ladder itself is where that claim belongs.
        if reading.is_undecodable() {
            self.undecodable_payloads = self.undecodable_payloads.saturating_add(1);
            self.last_undecodable_payload = Some(event);
        }
    }

    /// How many events arrived carrying a payload that announced itself as
    /// structure and that the datamodel could not read as one.
    ///
    /// The clause requires the fallback: content the processor cannot
    /// interpret becomes a space-normalized string. What it does not require —
    /// and what nothing here used to provide — is any way for the host that
    /// SENT that payload to learn its fields have stopped existing. The
    /// document reads `_event.data.field`, gets nothing, assigns nothing, and
    /// the run continues; measured 2026-08-22 on three independent Lua
    /// implementations, a payload in Lua's own table syntax silently emptied
    /// every variable the receiving transition assigned, including the one
    /// that primes the next session.
    ///
    /// Counts only the reading a host can act on. Prose delivered as text is
    /// the ladder working (W3C test 562) and is not counted, because a
    /// diagnostic that fires when nothing is wrong is one nobody reads.
    ///
    /// Comparing the count across a delivery is what turns "the machine
    /// ignored my payload" into a fact, and
    /// [`last_undecodable_payload`](Self::last_undecodable_payload) says which
    /// delivery it was.
    pub fn undecodable_payloads(&self) -> u32 {
        self.undecodable_payloads
    }

    /// The most recent event [`undecodable_payloads`](Self::undecodable_payloads)
    /// counted, or `None` while that count is zero.
    ///
    /// Copy, so reading it costs nothing on the no_std profile — the same
    /// shape as [`last_discarded_event`](Self::last_discarded_event), and for
    /// the same reason: a count says something was lost, this says what.
    pub fn last_undecodable_payload(&self) -> Option<P::Event> {
        self.last_undecodable_payload
    }

    /// Record one external event this machine will never look at.
    fn note_unseen_event(&mut self, event: P::Event) {
        // §scxml-D-mainEventLoop: the loop that would have dequeued this has
        // ended, so the event is not "pending" — it is over.
        self.unseen_external_events = self.unseen_external_events.saturating_add(1);
        self.last_unseen_event = Some(event);
    }

    /// Empty the external queue into the count above, at the moment the main
    /// event loop ends.
    ///
    /// Drained rather than left in place so each event is counted exactly
    /// once: a host that keeps calling `step()` on a halted machine would
    /// otherwise re-count the same queue on every call, and a count that grows
    /// while nothing arrives is a count nobody can use.
    fn record_unseen_external_events(&mut self) {
        while let Some(meta) = self.external_queue.pop() {
            self.note_unseen_event(meta.event);
        }
    }

    /// How many external events the host handed this machine that it never
    /// looked at, because it had already stopped.
    ///
    /// §scxml-D-mainEventLoop exits when the machine reaches a top-level final
    /// state, and W3C SCXML 3.13 is explicit that the interpreter is then
    /// done. Refusing the event is therefore correct — and, exactly as with
    /// [`discarded_external_events`](Self::discarded_external_events) and
    /// [`undecodable_payloads`](Self::undecodable_payloads), being unable to
    /// SAY it happened is not part of the clause.
    ///
    /// This is the count that separates the third explanation from the other
    /// two. A host that sent an event and saw nothing move has three
    /// candidates:
    ///
    /// | what happened | which count moves |
    /// | --- | --- |
    /// | dequeued, no transition matched | `discarded_external_events` |
    /// | dequeued, a transition matched but its guard was false | neither |
    /// | never dequeued — the machine had stopped | this one |
    ///
    /// Measured 2026-08-22: a consumer reported a guarded transition that
    /// "never fires", and four rewrites of the guard later the guard was still
    /// the suspect. Driving the same document here fired it on the first try,
    /// at that consumer's own pinned revision — so the difference was never
    /// the guard, and nothing in this engine could have said so.
    pub fn unseen_external_events(&self) -> u32 {
        self.unseen_external_events
    }

    /// The most recent event [`unseen_external_events`](Self::unseen_external_events)
    /// counted, or `None` while that count is zero.
    ///
    /// A count says an event went unlooked-at; this says which one, which is
    /// what a host debugging a supervisor that stopped answering actually
    /// needs.
    pub fn last_unseen_event(&self) -> Option<P::Event> {
        self.last_unseen_event
    }

    /// How many `error.*` events this engine raised that no transition in any
    /// active state answered.
    ///
    /// The clause requires the processor to signal its own failures as
    /// `error.*` events on the internal queue, and says in the same breath that
    /// "they are ignored if no transition is found that matches them". Being
    /// ignored is the clause. Being unable to say it happened is not, and the
    /// difference matters to exactly one party: the host, which did not write
    /// the document, cannot see the failure anywhere in the configuration, and
    /// is the only one positioned to do something about it. A supervisor
    /// driving a machine whose `<assign>` silently fails every round reads
    /// `is_running() == true` and a plausible state forever.
    ///
    /// This is the sibling of
    /// [`discarded_external_events`](Self::discarded_external_events), and the
    /// two are deliberately separate counts rather than one. That one stops at
    /// the external queue because an author's unmatched `<raise>` has both ends
    /// inside the document; an error event's sender is the engine, so the same
    /// reasoning does not reach it. An author's `<raise>` that matches nothing
    /// is still not counted here.
    ///
    /// An error the document *did* answer is not counted either — the document
    /// dealt with it, and its handling is visible in the configuration the host
    /// can already read. What this counts is only the silent case.
    ///
    /// The C++ Interpreter has answered this all along, through
    /// `getLastStateMachineError()` and the `error.execution` message it raises
    /// with the failure text; this is the generated engines' side of it.
    pub fn unhandled_error_events(&self) -> u32 {
        // §scxml-3.12.2 is the clause: error events go on the internal queue
        // and are ignored when nothing matches them. Cited in the body because
        // the ledger's Rust resolver binds a citation to the symbol enclosing
        // it, and a `///` line encloses nothing.
        self.unhandled_error_events
    }

    /// The most recent `error.*` event
    /// [`unhandled_error_events`](Self::unhandled_error_events) counted, or
    /// `None` while that count is zero.
    ///
    /// Which error it was narrows a silent failure from "something in this
    /// machine is broken" to a class: `error.execution` is the document's own
    /// executable content failing, `error.communication` is a `<send>` or
    /// `<invoke>` that could not reach its target — two different repairs, and
    /// a count alone does not separate them. Copy, so reading it costs nothing
    /// on the no_std profile.
    pub fn last_unhandled_error(&self) -> Option<P::Event> {
        self.last_unhandled_error
    }

    /// How many `error.*` events this engine refused to queue because the
    /// error handler that raised them had been failing for a hundred links
    /// running (`MAX_ERROR_CASCADE_DEPTH`, internal — the number is the
    /// engine's to choose and the count is what a host reads).
    ///
    /// The clause says an unmatched error event is ignored, and
    /// [`unhandled_error_events`](Self::unhandled_error_events) is that case.
    /// This is its opposite and its worse half: the document *does* match the
    /// error, and the handler fails the same way every time. The failure
    /// raises `error.execution`, the same transition answers it, and the drain
    /// never empties. Nothing in §scxml-3.12.2 covers it — the clause bounds
    /// what happens to an error nobody wants, not an error everybody wants and
    /// nobody can handle.
    ///
    /// Left to run, that is not a hang: it is a core at 100% forever. Measured
    /// 2026-08-19 on a two-line document, the Python engine turned 37,000
    /// links a second while its configuration never moved and
    /// `is_running()` stayed `true` — the exact reading an unattended
    /// supervisor takes as healthy. So the engine stops feeding the chain and
    /// says how often it had to, which is the one fact that separates "this
    /// machine is idle" from "this machine's error handling is broken".
    ///
    /// A document that fails five hundred times cleanly counts zero here: the
    /// chain is measured from *handler to handler*, not from failure to
    /// failure, and any other internal event resets it. Nothing is discarded
    /// that a working document would have processed.
    pub fn error_cascade_events(&self) -> u32 {
        self.error_cascade_events
    }

    /// The most recent `error.*` event
    /// [`error_cascade_events`](Self::error_cascade_events) refused, or `None`
    /// while that count is zero.
    ///
    /// Which error it was names the repair: `error.execution` is a handler
    /// whose own executable content fails, `error.communication` a handler
    /// that answers an unreachable target by talking to it again. Copy, so
    /// reading it costs nothing on the no_std profile.
    pub fn last_error_cascade_event(&self) -> Option<P::Event> {
        self.last_error_cascade_event
    }

    /// How many macrosteps this engine stopped short because their chain was
    /// still going after `MAX_MACROSTEP_MICROSTEPS` microsteps (internal — the
    /// number is the engine's to choose and the count is what a host reads).
    ///
    /// The specification says a macrostep ends in a configuration where
    /// nothing is enabled by NULL and no internal event is left, and its
    /// Principles and Constraints add that a macrostep *may not terminate* and
    /// that this "is currently allowed". A document with a cyclic eventless
    /// transition is therefore not malformed, and neither is one whose
    /// `<raise>` answers itself; both are documents whose macrostep is
    /// infinite, and an engine that runs either to the letter never returns.
    ///
    /// Both are counted here, because they are the same fact to a host: the
    /// macrostep it just drove did not reach a stable configuration. Which
    /// chain it was is what
    /// [`last_truncated_macrostep_state`](Self::last_truncated_macrostep_state)
    /// points at.
    ///
    /// This engine does not run it to the letter. It stops, and this count is
    /// how a host learns that it did — because every other reading says the
    /// opposite: [`get_current_state`](Self::get_current_state) answers,
    /// [`is_running`](Self::is_running) is `true`, and the call returned in
    /// microseconds. The configuration behind those answers is *not* the
    /// stable one §scxml-3.13 promises; it is wherever the hundredth microstep
    /// happened to land, and the document has more to do that this engine will
    /// not do.
    ///
    /// A document whose chain is a hundred microsteps long and then settles
    /// counts zero: the ceiling is on microsteps *taken*, and the macrostep is
    /// only counted here when the loop still had work after them — a
    /// transition enabled by NULL, or an event left on the internal queue.
    /// Long chains are ordinary; endless ones are not.
    ///
    /// Absent under `no_macrostep_diagnostics`, deliberately. A consumer who
    /// compiled the report out and then called this would be handed a `0` that
    /// means "not counted" while reading it as "did not happen" — a wrong
    /// answer where a compile error is available.
    #[cfg(not(feature = "no_macrostep_diagnostics"))]
    pub fn truncated_macrosteps(&self) -> u32 {
        self.truncated_macrosteps
    }

    /// The state this engine was in when it last stopped a macrostep that way,
    /// or `None` while [`truncated_macrosteps`](Self::truncated_macrosteps) is
    /// zero.
    ///
    /// Which state it was is the whole repair: an endless chain is a closed
    /// walk through the state graph, and this names one state on it — the
    /// source of the transition that was refused, or the state the drain was
    /// standing in when it stopped taking internal events. The count alone
    /// says a document somewhere cannot settle; this says where to look. Copy,
    /// so reading it costs nothing on the no_std profile.
    ///
    /// Absent under `no_macrostep_diagnostics`, for the reason its sibling is.
    #[cfg(not(feature = "no_macrostep_diagnostics"))]
    pub fn last_truncated_macrostep_state(&self) -> Option<P::State> {
        self.last_truncated_macrostep_state
    }

    /// Current active (leaf) state.
    pub fn get_current_state(&self) -> P::State {
        self.current_state
    }

    /// §scxml-3.11: Full list of active states (for history recording + In() predicate).
    ///
    /// Non-parallel machines: returns the hierarchy `[leaf, parent, grandparent, ..., root]`.
    /// Parallel machines: returns the union of all active regions via
    /// [`StatePolicy::get_active_states`].
    ///
    /// SCE Protocol-Synthesis RFC §synth-5-J-2: returns the bounded
    /// [`StateChain`](crate::helpers::hierarchy::StateChain) which
    /// aliases `Vec<P::State>` under std (ABI-preserving) and
    /// `heapless::Vec<P::State, MAX_HIERARCHY_DEPTH>` under no_std. The parallel
    /// branch (`policy.get_active_states()` returning a heap `Vec`) is gated to
    /// `!no_std` because policy.rs's default impl is itself alloc-coupled (the
    /// policy.rs port lands in B-γ2d-5). Reuses the existing
    /// `MAX_HIERARCHY_DEPTH=16` capacity invariant — no new capacity constant
    /// (D-1 lockin preserved).
    pub fn get_active_states(&self) -> hierarchy::StateChain<P::State> {
        #[cfg(not(feature = "no_std"))]
        if concepts::has_active_states::<P>() {
            return self.policy.get_active_states();
        }
        // Walk from current state up to root via the cfg-branched chain helper.
        // Bounded by MAX_HIERARCHY_DEPTH=16 under no_std (panics on overflow per
        // W3C SCXML no-silent-drop discipline — a hierarchy walk exceeding depth
        // 16 indicates a generator bug or cyclic parent relationship).
        let mut active: hierarchy::StateChain<P::State> = hierarchy::new_chain();
        let mut current = Some(self.current_state);
        while let Some(state) = current {
            hierarchy::push_chain(&mut active, state);
            current = P::get_parent(state);
        }
        active
    }

    /// §scxml-3.7: Whether this session has ended — that is, whether the
    /// current state is a `<final>` whose parent is the `<scxml>` element.
    ///
    /// §scxml-D-enterStates sets `running = false` for a `<final>` only when
    /// `isSCXMLElement(s.parent)`; a nested one queues `done.state.<parent>`
    /// and the machine carries on. So the structural question — "is this
    /// state a `<final>` element" — is [`StatePolicy::is_final_state`], and
    /// it is not the completion criterion on its own. Everything that means
    /// "the machine is done" keys on this method: `run_until_completion`, the
    /// completion callback, and the `done.invoke.<id>` a parent emits for an
    /// invoked child.
    pub fn is_in_final_state(&self) -> bool {
        P::is_final_state(self.current_state) && P::get_parent(self.current_state).is_none()
    }

    /// §scxml-5.5 + 6.3.1: Stash the donedata payload evaluated on a
    /// top-level `<final>` so the invoking parent can lift it onto
    /// `done.invoke.<id>._event.data`.
    ///
    /// Called from generated `execute_entry_actions` code on a child engine
    /// (1:1 port of the C++ AOT `stashDonedataAtFinal` / Kotlin
    /// `StateMachineEngine.stashDonedataAtFinal` contract).
    pub fn stash_donedata_at_final(&mut self, data: SceString) {
        self.donedata_at_final = data;
    }

    /// §scxml-5.5 + 6.3.1: Read the donedata payload stashed by a
    /// top-level `<final>` on this engine. Returns an empty string when the
    /// final had no `<donedata>`, matching C++ AOT / Kotlin semantics.
    pub fn donedata_at_final(&self) -> &str {
        &self.donedata_at_final
    }

    /// Access the inner policy (read-only).
    ///
    /// Used by parent engine to drain child's parent_external_queue.
    pub fn policy(&self) -> &P {
        &self.policy
    }

    /// §scxml-6.4: Get shared handle to external queue for child→parent event passing.
    ///
    /// Returns an `Arc<Mutex<Vec<(event_name, event_data)>>>` that child state machines
    /// can push events into via `#_parent` send targets. Parent drains this in `tick_children()`.
    ///
    /// SCE Protocol-Synthesis RFC §synth-5-J-2: gated to `!no_std` because `Arc`/`Mutex`/`Vec` are
    /// alloc-coupled and the `<invoke>` author surface that wires this handle into
    /// generated code is rejected at codegen time under `--no-std`.
    #[cfg(not(feature = "no_std"))]
    pub fn get_external_queue_handle(&self) -> Arc<Mutex<Vec<(String, String)>>> {
        // Each call creates a new shared queue; the generated policy stores it.
        Arc::new(Mutex::new(Vec::new()))
    }

    // ════════════════════════════════════════
    // Event submission (matches C++ raise / raiseExternal overloads)
    // ════════════════════════════════════════

    /// §scxml-C-1: Raise an internal event (high priority).
    ///
    /// Matches C++ `raise(EventWithMetadata)`.
    ///
    /// An `error.*` event raised while an error handler is running is refused
    /// once the chain reaches `MAX_ERROR_CASCADE_DEPTH` — see
    /// [`error_cascade_events`](Self::error_cascade_events) for why the engine
    /// is the one that has to stop it. Only the engine's own error events are
    /// refused: an author's `<raise>` inside an error handler is the document
    /// doing its job and rides the queue like any other.
    pub fn raise(&mut self, event: EventWithMetadata<P::Event, P::Payload>) {
        // §scxml-3.12.2 names the error events this refuses; the clause itself
        // is silent on a handler that fails, which is why the ceiling is a
        // choice this engine documents rather than a rule it implements.
        if self.handling_error_event
            && crate::helpers::event_matching::is_error_event(P::get_event_name(event.event))
        {
            self.error_cascade_depth = self.error_cascade_depth.saturating_add(1);
            if self.error_cascade_depth >= MAX_ERROR_CASCADE_DEPTH {
                self.error_cascade_events = self.error_cascade_events.saturating_add(1);
                self.last_error_cascade_event = Some(event.event);
                if self.error_cascade_events == 1 {
                    sce_log_error!(
                        "Engine::raise: an error handler has raised an error {} times over; \
                         refusing to feed the chain — the document's error handling is failing",
                        MAX_ERROR_CASCADE_DEPTH
                    );
                }
                return;
            }
        }
        self.internal_queue.raise(event);
    }

    /// §scxml-C-1 / 6.2: Raise an external event with optional data and origin.
    ///
    /// Matches C++ `raiseExternal(Event, const string&, const string&)`.
    pub fn raise_external(&mut self, event: P::Event, event_data: &str, origin: &str) {
        let meta = EventWithMetadata {
            event,
            payload: P::Payload::default(),
            // no_std elides the `_event.origin` / `origintype` string metadata
            // (no script reader); the `origin` argument has no MCU consumer.
            metadata: {
                #[cfg(not(feature = "no_std"))]
                {
                    EventMetadata {
                        data: crate::sce_string_from_str(event_data),
                        event_type: EventType::External,
                        origin: crate::sce_string_from_str(origin),
                        origin_type: crate::sce_string_from_str(
                            crate::helpers::scxml_constants::SCXML_EVENT_PROCESSOR_TYPE,
                        ),
                        ..Default::default()
                    }
                }
                #[cfg(feature = "no_std")]
                {
                    let _ = (event_data, origin);
                    EventMetadata {
                        event_type: EventType::External,
                    }
                }
            },
            #[cfg(not(feature = "no_std"))]
            target: SceString::new(),
        };
        self.external_queue.raise(meta);

        // §scxml-5.10.1: Mark next event as external for _event.type
        if concepts::has_external_event_flag::<P>() {
            self.policy.set_next_event_is_external(true);
        }
    }

    /// EventSchema native lowering: raise an external event carrying a
    /// typed payload.
    ///
    /// This is the single typed-inject seam: it pairs an event with its
    /// `Self::Payload` value so the name ↔ payload-type invariant is established
    /// in exactly one place (illegal pairings can't be constructed elsewhere).
    /// The payload rides with the event through the external queue and is bound
    /// to the policy at dispatch via
    /// [`StatePolicy::populate_event_payload`], where `_event.data.<field>`
    /// guards read it natively — no script engine, no string serialization, so
    /// the value path holds on no_std MCU. A consumer
    /// decodes its wire bytes into the payload struct and calls this; everything
    /// inside is SCE's.
    pub fn raise_external_typed(&mut self, event: P::Event, payload: P::Payload) {
        let meta = EventWithMetadata {
            event,
            payload,
            metadata: {
                #[cfg(not(feature = "no_std"))]
                {
                    EventMetadata {
                        event_type: EventType::External,
                        origin_type: crate::sce_string_from_str(
                            crate::helpers::scxml_constants::SCXML_EVENT_PROCESSOR_TYPE,
                        ),
                        ..Default::default()
                    }
                }
                #[cfg(feature = "no_std")]
                {
                    EventMetadata {
                        event_type: EventType::External,
                    }
                }
            },
            #[cfg(not(feature = "no_std"))]
            target: SceString::new(),
        };
        self.external_queue.raise(meta);

        // §scxml-5.10.1: Mark next event as external for _event.type
        if concepts::has_external_event_flag::<P>() {
            self.policy.set_next_event_is_external(true);
        }
    }

    /// §scxml-6.4.1: Raise an external event by name (for child autoforward).
    ///
    /// Matches C++ `raiseExternal(const string&, const string&)`. If the name does
    /// not match any known event, the call is silently ignored (the child may
    /// simply not have that event declared).
    pub fn raise_external_by_name(&mut self, event_name: &str, event_data: &str) {
        if let Some(event) = P::get_event_from_name(event_name) {
            self.raise_external(event, event_data, "");
        } else {
            sce_log_debug!(
                "Engine::raise_external_by_name: event '{}' not in enum, ignoring",
                event_name
            );
        }
    }

    /// §scxml-6.4: Raise an autoforwarded external event, name-addressed but
    /// carrying the source event's `_event` fields.
    ///
    /// The autoforward path is the one place an event must leave the machine
    /// that owns its enum, so it crosses by name while the metadata travels
    /// with it — §6.4 mandates an exact copy. Unknown names degrade silently:
    /// a child is not required to declare every event its parent forwards.
    pub fn raise_external_by_name_with_meta(&mut self, event_name: &str, metadata: &EventMetadata) {
        let Some(event) = P::get_event_from_name(event_name) else {
            sce_log_debug!(
                "Engine::raise_external_by_name_with_meta: event '{}' not in enum, ignoring",
                event_name
            );
            return;
        };
        // `target` stays default: the copy is delivered to this machine, never
        // re-routed to the original event's target.
        self.raise_external_with_meta(EventWithMetadata {
            event,
            payload: P::Payload::default(),
            metadata: metadata.clone(),
            #[cfg(not(feature = "no_std"))]
            target: SceString::new(),
        });
    }

    /// §scxml-6.4.1: Raise an external event with full metadata (for child-to-parent).
    ///
    /// Matches C++ `raiseExternal(const EventWithMetadata&)`. Preserves `invokeid`
    /// for parent finalize handlers.
    pub fn raise_external_with_meta(&mut self, event: EventWithMetadata<P::Event, P::Payload>) {
        sce_log_debug!("Engine::raise_external_with_meta: enqueuing external event with metadata");

        self.external_queue.raise(event);

        if concepts::has_external_event_flag::<P>() {
            self.policy.set_next_event_is_external(true);
        }
    }

    /// §scxml-3.12: Process an external event (convenience API, runs one macrostep).
    ///
    /// Matches C++ `processEvent(Event)`.
    pub fn process_event(&mut self, event: P::Event) {
        if !self.is_running {
            // Refused rather than queued, so the drain in
            // `run_main_event_loop` never sees it — which is why the count is
            // taken here as well as there. See
            // [`unseen_external_events`](Self::unseen_external_events).
            self.note_unseen_event(event);
            return;
        }
        self.raise_external(event, "", "");
        self.step();
    }

    /// §scxml-5.10: Process an external event with metadata.
    ///
    /// Matches C++ `processEvent(Event, const EventMetadata&)`.
    pub fn process_event_with_meta(&mut self, event: P::Event, metadata: EventMetadata) {
        if !self.is_running {
            // Same refusal as `process_event`, same reason it is counted here.
            self.note_unseen_event(event);
            return;
        }
        let meta = EventWithMetadata {
            event,
            payload: P::Payload::default(),
            metadata,
            #[cfg(not(feature = "no_std"))]
            target: SceString::new(),
        };
        self.external_queue.raise(meta);
        self.step();
    }

    // ════════════════════════════════════════
    // Scheduler passthrough
    // ════════════════════════════════════════

    /// Schedule an event for delayed delivery. Returns the send ID.
    ///
    /// Resolves the current clock via `sched_now_plus`
    /// — `<P::Hal>::now_ticks_ms() + delay_ms` under both profiles — and
    /// forwards to the clock-source-agnostic
    /// [`PullScheduler::schedule_event_at`].
    pub fn schedule_event(
        &mut self,
        event: P::Event,
        delay: Duration,
        send_id: &str,
        event_data: &str,
    ) -> SceString {
        let ready_at = self.sched_now_plus(delay);
        self.scheduler
            .schedule_event_at(event, ready_at, send_id, event_data)
    }

    /// Cancel a previously scheduled event by send ID.
    pub fn cancel_event(&mut self, send_id: &str) -> bool {
        self.scheduler.cancel_event(send_id)
    }

    /// Whether the scheduler has events ready to fire.
    pub fn has_ready_events(&self) -> bool {
        self.scheduler.has_ready_events_at(self.sched_now())
    }

    /// How long until this machine next needs [`tick`](Self::tick), in
    /// milliseconds. `Some(0)` means something is due now; `None` means the
    /// scheduler is empty and no clock-driven wake-up is owed.
    ///
    /// [`NEEDS_EVENT_SCHEDULER`](StatePolicy::NEEDS_EVENT_SCHEDULER) tells a
    /// host *which* entry point to drive the machine with. This tells it
    /// *when*, and a host that cannot ask has only one move left: pick a
    /// polling interval. That guess is not free in either direction — measured
    /// on a document whose `<send delay="200ms">` is cancelled by a 100 ms
    /// signal, a 1 ms interval spends 180 wasted ticks to be on time, a 500 ms
    /// one fires 300 ms late, and a 250 ms one steps over both deadlines at
    /// once and reaches a state the document forbids. An interval cannot
    /// straddle two deadlines it was never told about.
    ///
    /// The answer feeds a host loop directly: `std::thread::sleep`, a tokio
    /// `sleep`, or an embassy `Timer::after` on the no_std profile, where the
    /// alternative is a poll that never lets the core idle.
    pub fn time_until_next_scheduled_ms(&self) -> Option<u64> {
        let next = self.scheduler.next_ready_at()?;
        Some(next.saturating_sub(self.sched_now()))
    }

    // ════════════════════════════════════════
    // Clock (§scxml-6.2.2)
    // ════════════════════════════════════════

    /// Where this engine reads "now" from — see [`SceClock`].
    pub fn clock(&self) -> SceClock {
        self.clock
    }

    /// Install the [`SceClock`] this engine measures its `<send delay>`
    /// deadlines against.
    ///
    /// Must be called before [`initialize`](Self::initialize): the entry
    /// configuration's `<onentry>` can arm delayed sends, and swapping the
    /// clock under deadlines already computed from another one would leave the
    /// queue holding two incomparable time bases. That is a programming error
    /// rather than a recoverable condition, so it panics — the same fail-loud
    /// convention [`NoOpHal`](crate::NoOpHal) uses for a HAL that was never
    /// wired.
    ///
    /// # Panics
    ///
    /// If the engine has already been initialized.
    pub fn set_clock(&mut self, clock: SceClock) {
        assert!(
            !self.is_running,
            "Engine::set_clock must be called before initialize(): this engine has \
             already armed its entry configuration against the previous clock, and \
             deadlines from two clocks do not compare"
        );
        self.clock = clock;
    }

    /// Move this engine's clock forward by `ms` and run whatever that made due
    /// (§scxml-6.2).
    ///
    /// The host-owned twin of [`tick`](Self::tick): `tick` asks a clock that
    /// moves on its own what time it is, this one *sets* what time it is and
    /// then ticks. A machine driven exclusively through here has no dependency
    /// on the load of the machine it runs on — the same sequence of calls
    /// produces the same configuration every time, which is what a simulation,
    /// a replay and a deterministic test each need.
    ///
    /// # Panics
    ///
    /// If [`clock`](Self::clock) is not [`SceClock::Manual`]. That is not a
    /// no-op but a programming error: the caller believes it owns time and it
    /// does not, so the events it is waiting for would arrive on a schedule it
    /// did not choose. Refusing loudly is the same call the Kotlin and Python
    /// channels make on the same contract.
    pub fn advance_time_ms(&mut self, ms: u64) {
        let SceClock::Manual(now) = self.clock else {
            panic!(
                "Engine::advance_time_ms needs SceClock::Manual; this engine has a \
                 clock whose time the host does not own. Call set_clock(SceClock::Manual(0)) \
                 before initialize(), or drive this machine with tick() and \
                 time_until_next_scheduled_ms()"
            );
        };
        self.clock = SceClock::Manual(now.saturating_add(ms));
        self.tick();
    }

    /// This engine's current reading of [`clock`](Self::clock), in
    /// milliseconds since that clock's origin.
    ///
    /// The absolute counterpart of
    /// [`time_until_next_scheduled_ms`](Self::time_until_next_scheduled_ms)'s
    /// relative answer. A host owning time through [`SceClock::Manual`] uses it
    /// to say where in the run it is; a host on the wall clock uses it to
    /// correlate an engine's deadlines with its own log.
    ///
    /// Inside a turn this is the turn's latched instant, which is what the
    /// engine itself is judging against; between turns it is a live reading.
    /// A clock that went backwards between two readings would un-due an entry
    /// the scheduler had already judged ready, so an [`SceClock::Source`] must
    /// be non-decreasing.
    pub fn now_ms(&self) -> u64 {
        self.sched_now()
    }

    // ════════════════════════════════════════
    // Callbacks
    // ════════════════════════════════════════

    /// §scxml-6.4: Register a callback invoked when the engine reaches a final state.
    ///
    /// SCE Protocol-Synthesis RFC §synth-5-J-2: gated to `!no_std` because `Box<dyn FnMut>` is
    /// alloc-coupled (mirrors `helpers::entry_exit::execute_*_blocks` gate from
    /// B-γ2d-2). Embedded consumers poll [`is_in_final_state`](Self::is_in_final_state)
    /// instead.
    #[cfg(not(feature = "no_std"))]
    pub fn set_completion_callback<F: FnMut() + Send + 'static>(&mut self, callback: F) {
        self.completion_callback = Some(Box::new(callback));
    }

    /// §scxml-C-2: Register an HTTP send dispatcher callback.
    ///
    /// The callback receives an [`HttpSendRequest`] and returns an optional
    /// [`HttpSendResponse`]. When `Some`, the engine injects the response event
    /// into the external queue — enabling real HTTP round-trips against the
    /// shared W3C test server (`standalone_http_server.js`).
    ///
    /// SCE Protocol-Synthesis RFC §synth-5-J-2: gated to `!no_std` (HTTP itself is whole-module
    /// gated; the codegen-time validator rejects `BasicHTTPEventProcessor`
    /// `<send>` under `--no-std` via `codegen/no-std-http-not-supported`).
    #[cfg(not(feature = "no_std"))]
    pub fn set_http_send_callback<F>(&mut self, callback: F)
    where
        F: FnMut(HttpSendRequest) -> Option<HttpSendResponse> + Send + 'static,
    {
        self.on_http_send = Some(Box::new(callback));
    }

    /// §scxml-C-2: Dispatch a BasicHTTP send through the registered callback.
    ///
    /// The callback is the sole dispatch mechanism. If it returns
    /// `Some(HttpSendResponse)`, the engine injects the response event into the
    /// external queue. The engine has no knowledge of HTTP transport — callers
    /// supply the implementation via [`set_http_send_callback`](Self::set_http_send_callback).
    ///
    /// SCE Protocol-Synthesis RFC §synth-5-J-2: gated to `!no_std` — see
    /// [`set_http_send_callback`](Self::set_http_send_callback) for the upstream rejection rationale.
    #[cfg(not(feature = "no_std"))]
    pub fn perform_http_send(
        &mut self,
        target: String,
        event_name: String,
        content: String,
        params: std::collections::HashMap<String, Vec<String>>,
        send_id: String,
    ) {
        if let Some(cb) = self.on_http_send.as_mut() {
            let response = cb(HttpSendRequest {
                target,
                event_name,
                content,
                params,
                send_id,
            });
            if let Some(resp) = response {
                if let Some(evt) = P::get_event_from_name(&resp.event_name) {
                    let mut meta = EventWithMetadata::new(evt);
                    meta.metadata = EventMetadata::external(SceString::new(), SceString::new());
                    meta.metadata.data = resp.event_data;
                    self.external_queue.raise(meta);
                }
            }
        }
    }

    /// §scxml-6.2.5: register `handler` as the Event I/O Processor for
    /// `processor_type`, so `<send type="processor_type">` reaches the
    /// host instead of raising `error.execution`.
    ///
    /// The build must also have been told about the type
    /// (`sce-codegen --host-processor <type>`, or
    /// `host_processor_types` on the `build.rs` facade): codegen decides
    /// at compile time whether a site is a dispatch or a refusal, and a
    /// registration alone cannot change emitted code. Registering a type
    /// the build did not declare is inert — which is why the build
    /// reports the declaration on its manifest, so the mismatch is
    /// visible rather than silent.
    ///
    /// Replaces any handler already registered for the type; see
    /// `HostProcessorRegistry::register` for why replacing beats
    /// refusing.
    #[cfg(not(feature = "no_std"))]
    pub fn register_event_processor<F>(&mut self, processor_type: &str, handler: F)
    where
        F: FnMut(
                crate::host_processor::HostSendRequest,
            ) -> Option<crate::host_processor::HostSendResponse>
            + Send
            + 'static,
    {
        self.host_processors
            .register(processor_type, Box::new(handler));
    }

    /// §scxml-6.4.1: register `handler` as the invoker for
    /// `processor_type`, so `<invoke type="processor_type">` starts a
    /// host-run process instead of raising `error.execution`.
    ///
    /// The build must also have been told about the type
    /// (`sce-codegen --host-invoker <type>`): codegen decides at compile
    /// time whether a site starts an invocation or refuses, and a
    /// registration alone cannot change emitted code.
    ///
    /// The handler receives both halves of the lifecycle —
    /// [`crate::host_processor::HostInvokeEvent::Start`] when the state
    /// is entered and the macrostep has settled, `Cancel` when the state
    /// exits. One registration rather than two because a host that can
    /// start an invocation and cannot stop it is not a working invoker.
    ///
    /// What SCE does NOT route to a host invoker: parent-to-child
    /// `<send target="#_invokeid">`, `autoforward`, and `<finalize>`.
    /// Those are session-to-session mechanics between two SCXML
    /// documents; a host invoker gets its input from the `Start`
    /// request's `<param>` / `<content>` and answers by raising events on
    /// the engine. Stated here because the surrounding §scxml-6.4
    /// machinery exists for SCXML children and silence would read as a
    /// promise.
    #[cfg(not(feature = "no_std"))]
    pub fn register_invoker<F>(&mut self, processor_type: &str, handler: F)
    where
        F: FnMut(
                crate::host_processor::HostInvokeEvent,
            ) -> Option<crate::host_processor::HostInvokeResponse>
            + Send
            + 'static,
    {
        self.host_processors
            .register_invoker(processor_type, Box::new(handler));
    }

    /// Whether an invoker is registered for `processor_type`.
    #[cfg(not(feature = "no_std"))]
    pub fn has_invoker(&self, processor_type: &str) -> bool {
        self.host_processors.invoker_is_registered(processor_type)
    }

    /// §scxml-6.4: start a host-run invocation, and raise
    /// `done.invoke.<invoke_id>` if the host completed it synchronously.
    ///
    /// Returns `false` when no invoker is registered, which the generated
    /// site turns into `error.execution` — the same event an undeclared
    /// type produces, because the document asked for a process to be run
    /// and none was.
    ///
    /// Called from the generated invoke site, which is why it is public.
    #[cfg(not(feature = "no_std"))]
    pub fn perform_host_invoke(
        &mut self,
        request: crate::host_processor::HostInvokeRequest,
    ) -> bool {
        let invoke_id = request.invoke_id.clone();
        let Some(response) = self.host_processors.start_invoke(request) else {
            return false;
        };
        // §scxml-6.4: a completion the host reported now. One it reports
        // later arrives the same way, by raising the event itself — the
        // engine does not distinguish the two, and it never synthesises a
        // completion the host did not report.
        if let Some(done_data) = response.and_then(|r| r.done_data) {
            let event_name = crate::invoke::create_done_invoke_event_name(&invoke_id);
            if let Some(evt) = P::get_event_from_name(&event_name) {
                let mut meta = EventWithMetadata::new(evt);
                meta.metadata = EventMetadata::external(SceString::new(), SceString::new());
                meta.metadata.data = done_data;
                self.external_queue.raise(meta);
            }
        }
        true
    }

    /// §scxml-6.4: stop a host-run invocation whose state has exited.
    ///
    /// Unconditional from the emitted exit chain; the engine knows
    /// whether the invocation ever started and stays silent when it did
    /// not. Returns whether a cancel was delivered.
    #[cfg(not(feature = "no_std"))]
    pub fn cancel_host_invoke(&mut self, processor_type: &str, invoke_id: &str) -> bool {
        self.host_processors
            .cancel_invoke(processor_type, invoke_id)
    }

    /// Whether a handler is registered for `processor_type`.
    ///
    /// The generated send site asks this to tell a processor that ran
    /// and had nothing to reply from one that was never registered.
    /// Both return `None` from
    /// [`perform_host_send`](Self::perform_host_send), and only the
    /// second is an error.
    #[cfg(not(feature = "no_std"))]
    pub fn has_event_processor(&self, processor_type: &str) -> bool {
        self.host_processors.is_registered(processor_type)
    }

    /// §scxml-6.2: dispatch a `<send>` addressed to a host-served
    /// processor, and raise the handler's reply if it gave one.
    ///
    /// With no handler registered the send raises `error.execution`,
    /// the same outcome an undeclared type produces. That is the point:
    /// the document asked for an act, and from its side "no processor
    /// implements this type" and "the processor was never wired up" are
    /// one fact. Reporting them differently would make a wiring mistake
    /// look like a document error, or worse, look like success.
    ///
    /// Called from the generated send site, which is why it is public.
    #[cfg(not(feature = "no_std"))]
    pub fn perform_host_send(
        &mut self,
        request: crate::host_processor::HostSendRequest,
    ) -> Option<crate::host_processor::HostSendResponse> {
        let processor_type = request.processor_type.clone();
        let handler = self.host_processors.handler_for(&processor_type)?;
        let response = handler(request);
        if let Some(resp) = response.as_ref() {
            if let Some(evt) = P::get_event_from_name(&resp.event_name) {
                let mut meta = EventWithMetadata::new(evt);
                // §scxml-C-1: a reply from outside the machine arrives on
                // the external queue, like any event the host raises.
                meta.metadata = EventMetadata::external(SceString::new(), SceString::new());
                meta.metadata.data = resp.event_data.clone();
                self.external_queue.raise(meta);
            }
        }
        response
    }

    // ════════════════════════════════════════
    // Convenience: runUntilCompletion
    // ════════════════════════════════════════

    /// Run the state machine to completion or timeout (§scxml-6.2).
    ///
    /// Matches C++ `runUntilCompletion(timeout, pollInterval)`. Calls `tick()`
    /// in a loop until either the final state is reached or `timeout` elapses.
    /// Returns `true` on completion, `false` on timeout.
    ///
    /// `poll_interval` is a ceiling on how long this waits between ticks, not
    /// the interval it actually sleeps: when the scheduler knows a nearer
    /// deadline
    /// ([`time_until_next_scheduled_ms`](Self::time_until_next_scheduled_ms)),
    /// the sleep is shortened to land on it. A caller that passes an interval
    /// coarser than the document's delays therefore no longer steps over them.
    ///
    /// SCE Protocol-Synthesis RFC §synth-5-J-2: gated to `!no_std` because the polling loop
    /// uses `std::thread::sleep` for cooperative blocking and `Instant::elapsed`
    /// for the timeout — both host-thread-coupled. no_std consumers drive their
    /// own executor loop, calling [`tick`](Self::tick) under their HAL waker
    /// (e.g. embassy `Timer::after` on the same
    /// [`time_until_next_scheduled_ms`](Self::time_until_next_scheduled_ms)
    /// this uses).
    #[cfg(not(feature = "no_std"))]
    pub fn run_until_completion(&mut self, timeout: Duration, poll_interval: Duration) -> bool {
        // W3C SCXML: if already stopped but reached final state during initialize(), return true
        if !self.is_running {
            return self.is_in_final_state();
        }

        let start = Instant::now();
        while !self.is_in_final_state() {
            if start.elapsed() > timeout {
                return false;
            }
            // The scheduler's own answer wins whenever it is nearer: sleeping
            // past a deadline is what turns a coarse interval into a document
            // that behaves differently, and waking on it costs nothing extra.
            let wait = match self.time_until_next_scheduled_ms() {
                Some(ms) => poll_interval.min(Duration::from_millis(ms)),
                None => poll_interval,
            };
            std::thread::sleep(wait);
            self.tick();
        }
        true
    }

    // ════════════════════════════════════════
    // Internal: microstep + macrostep implementation
    // ════════════════════════════════════════

    /// §scxml-D-mainEventLoop: the outer loop, and the only place the three
    /// public entry points express macrostep semantics.
    ///
    /// Appendix D names the external queue exactly once per iteration and it
    /// is *after* `invoke(inv)`:
    ///
    /// ```text
    /// while running:
    ///     while running and not macrostepDone:      # eventless + internal only
    ///         ... selectEventlessTransitions() / internalQueue.dequeue() ...
    ///     for state in statesToInvoke.sort(entryOrder):
    ///         for inv in state.invoke.sort(documentOrder):
    ///             invoke(inv)
    ///     statesToInvoke.clear()
    ///     if not internalQueue.isEmpty(): continue
    ///     externalEvent = externalQueue.dequeue()
    /// ```
    ///
    /// Folding the external drain into the macrostep-completion loop instead
    /// is a different algorithm, not a shorter one. The invoked children do
    /// not exist yet while that drain runs, so everything `<onentry>` queued
    /// for this session on the way in is consumed with no `autoforward` child
    /// to receive it — and there is no later point at which it is delivered.
    /// One external event per iteration for the same reason: a state entered
    /// by event N's transition must have its invokes started before N+1 comes
    /// off the queue.
    pub(crate) fn run_main_event_loop(&mut self) {
        loop {
            // §scxml-D-mainEventLoop: complete the macrostep on eventless
            // transitions and internal events alone.
            loop {
                self.check_eventless_transitions();
                if !self.internal_queue.has_events() {
                    break;
                }
                self.process_internal_queue();
                if self.macrostep_truncated {
                    // Either branch may have spent the last of the budget.
                    // Without this the loop turns forever on a chain that is
                    // no longer being drained: the queue stays non-empty
                    // precisely because the drain refused it.
                    break;
                }
            }

            if !self.is_running || self.is_in_final_state() {
                // §scxml-D-mainEventLoop ends here, and whatever the host put
                // on the external queue ends with it. That is the clause: a
                // machine that has reached a top-level final state exits the
                // interpreter, and events that arrive afterwards are not
                // processed. Saying nothing about it is not the clause.
                //
                // The host cannot tell this outcome from the two it already
                // has. `discarded_external_events` counts an event that WAS
                // dequeued and matched nothing; a guard that evaluated false
                // leaves no count at all. All three leave the configuration
                // alone, so "I sent it and nothing happened" has three
                // explanations and the host could distinguish none of them —
                // measured 2026-08-22 against a consumer that spent four
                // attempts rewriting a guard that was never evaluated.
                self.record_unseen_external_events();
                break;
            }

            // §scxml-6.4: invokes for states entered during this macrostep.
            if concepts::has_invoke_support::<P>() {
                let policy_ptr: *mut P = &mut self.policy as *mut P;
                // SAFETY: see execute_on_entry.
                unsafe {
                    (*policy_ptr).execute_pending_invokes(self);
                }
            }

            // §scxml-D-mainEventLoop: invoking may have raised internal error
            // events (and a child that completed synchronously may already
            // have raised `done.invoke`); handle them before touching the
            // external queue.
            //
            //
            // Not when this macrostep was already stopped at the ceiling: the
            // queue is non-empty *because* the drain refused it, so looping
            // back is a spin that takes no microstep, logs nothing, and never
            // ends. Falling through to the external dequeue instead is what
            // keeps a machine inside an endless chain reachable at all — the
            // event that rescues it is on that queue, and the clause's
            // internal-first priority would otherwise hold it behind a chain
            // that never ends.
            if !self.macrostep_truncated && self.internal_queue.has_events() {
                continue;
            }

            if !self.process_next_external_event() {
                break;
            }
        }
    }

    /// §scxml-C-1: Drain the internal queue (high priority).
    ///
    /// Bounded by the same macrostep budget the eventless branch spends, and
    /// for the same reason: a `<raise>` answered by a transition that raises
    /// again is a macrostep that never ends, exactly as a cyclic eventless
    /// transition is. Until 2026-08-20 this branch had no ceiling in any of the
    /// seven engines here, so that document did not return at all.
    pub(crate) fn process_internal_queue(&mut self) {
        if self.macrostep_truncated {
            // The eventless branch of this same macrostep already ran out of
            // budget. Draining now would hand the chain a second one.
            return;
        }
        sce_log_debug!("Engine::process_internal_queue: starting internal queue drain");

        while self.internal_queue.has_events() {
            if self.macrostep_microsteps_taken == MAX_MACROSTEP_MICROSTEPS {
                // Work is still queued one microstep past the budget, so this
                // is the case the specification calls a macrostep that does not
                // terminate. Refuse the microstep rather than take it: the
                // event stays on the queue, which is where the next macrostep
                // will find it, and the count says the configuration a host
                // reads now is not a stable one.
                self.record_truncated_macrostep(self.current_state);
                #[cfg(not(feature = "no_macrostep_diagnostics"))]
                sce_log_error!(
                    "Engine::process_internal_queue: macrostep still going after {} microsteps; stopped",
                    MAX_MACROSTEP_MICROSTEPS
                );
                return;
            }
            let Some(event_with_meta) = self.internal_queue.pop() else {
                break;
            };
            // §scxml-5.4.1: Stop if top-level final state reached. Same
            // predicate as everything else that means "the machine is done" —
            // spelling the parent check out a second time here is what let the
            // public one drift away from it.
            if self.is_in_final_state() {
                sce_log_debug!(
                    "Engine::process_internal_queue: top-level final state reached, stopping"
                );
                return;
            }
            // §scxml-5.10: Populate policy metadata from event (ports C++ populatePolicyFromMetadata)
            self.policy
                .populate_event_metadata(&event_with_meta.metadata);
            // EventSchema native lowering: bind the typed payload
            // that rode with this event so `_event.data.<field>` guards read it
            // natively. No-op for schemaless policies (`Payload = ()`).
            self.policy.populate_event_payload(&event_with_meta.payload);
            // §scxml-3.12.2: the processor raises `error.*` into this queue and
            // the clause says they "are ignored if no transition is found that
            // matches them". Ignoring them is the clause; staying silent about
            // it is not. `discarded_external_events` deliberately stops at the
            // external queue because an unmatched `<raise>` has both ends
            // inside the document — but the sender of an error event is this
            // engine, so that reasoning does not reach it. The host never wrote
            // the document, cannot see the failure in the configuration, and is
            // the only party able to act on it.
            //
            // The selection runs first and unconditionally: it is what
            // processes every internal event, and making it the right-hand
            // side of an `&&` would skip it for everything that is not an
            // error.
            // An error raised from here on is raised *by an error handler*,
            // which is the one situation the engine cannot leave to the
            // document: the handler that failed is the same one that will
            // answer the failure. The flag is what `raise` reads to tell that
            // apart from a first failure, and it is cleared before anything
            // else can run so a chain cannot be attributed to the wrong event.
            let is_error = crate::helpers::event_matching::is_error_event(P::get_event_name(
                event_with_meta.event,
            ));
            // The chain is not ended by the drain doing something else. An
            // earlier draft reset the depth on every non-error event, which
            // reads as the careful choice and is the opposite: a handler that
            // raises its own event before failing — a document that logs, then
            // fails, which is most of them — leaves the queue alternating
            // `tick, error, tick, error…`, and each `tick` put the ceiling
            // back out of reach. The count needs no such guard, because it
            // only ever rises while an error handler is running.
            self.handling_error_event = is_error;
            let outcome = self.execute_transition(event_with_meta.event);
            self.handling_error_event = false;
            if outcome != EventOutcome::Discarded {
                // Appendix D: the loop turn that selects nothing takes no
                // microstep, so it spends no budget. Only a turn that answered
                // the event moved the machine, and only those are what a
                // ceiling on microsteps can be counted in.
                self.macrostep_microsteps_taken += 1;
            }
            if outcome == EventOutcome::Discarded && is_error {
                self.unhandled_error_events = self.unhandled_error_events.saturating_add(1);
                self.last_unhandled_error = Some(event_with_meta.event);
                sce_log_debug!(
                    "Engine::process_internal_queue: error event matched no transition; unhandled"
                );
            }
            self.policy.clear_event_metadata();
        }
        // The queue emptied, so the chain — refused or merely finished — is
        // over. A machine whose next macrostep starts a new one starts it from
        // zero, and the count of what was refused stays where the host reads it.
        self.error_cascade_depth = 0;
    }

    /// §scxml-D-mainEventLoop: take exactly one event off the external queue,
    /// run the preliminary `<finalize>` / `autoforward` step against it, then
    /// select transitions. Returns `false` when the queue was empty.
    ///
    /// One event, not a drain: Appendix D returns to the top of the outer loop
    /// after each external event, so a state entered by this event's
    /// transition gets its invokes started before the next one is dequeued.
    pub(crate) fn process_next_external_event(&mut self) -> bool {
        let Some(event_with_meta) = self.external_queue.pop() else {
            return false;
        };
        // §scxml-D-mainEventLoop: taking an event off the external queue is
        // where a macrostep begins, so it is where the previous one's ceiling
        // stops applying. A machine left inside an endless chain gets a full
        // budget for each event it is given, and each refusal is counted
        // separately — which is what tells a host that spins once from one
        // that spins on everything.
        //
        // Here and not at the entry to the loop above, which reads like the
        // more general boundary and is not one: a machine whose chain was
        // refused would spend a whole budget re-walking it before it ever
        // looked at the event the host sent to get it out. The refused events
        // stay queued either way — this is where the budget that drains them
        // comes back.
        self.macrostep_truncated = false;
        self.macrostep_microsteps_taken = 0;
        {
            // §scxml-6.5: Execute finalize before parent's own transition matching
            if concepts::has_finalize::<P>() {
                let policy_ptr: *mut P = &mut self.policy as *mut P;
                // SAFETY: see execute_on_entry.
                unsafe {
                    (*policy_ptr).execute_finalize_for_child_event(&event_with_meta, self);
                }
            }
            // §scxml-D-mainEventLoop: autoforward belongs to the same
            // preliminary step as `<finalize>` above — both run against the
            // event this drain has just popped off the external queue, and
            // both run before transition selection. §scxml-6.4.2 fixes the
            // position in prose as well: the parent forwards "at the point at
            // which it removes it from the external event queue".
            //
            // Forwarding where the event is *enqueued* instead is a different
            // algorithm, not an earlier one. `raise_external_with_meta` runs
            // inside whatever executable content produced the event, so a
            // transition body that queues two events hands the child both of
            // them before the parent has processed either — the child runs a
            // whole event ahead and the two sessions stop agreeing on what has
            // happened. Run-to-completion is a property of this loop's shape,
            // so the forward has to live in the loop.
            //
            // §scxml-6.4 requires an exact copy, so the metadata rides along
            // with the name. `get_event_name` already returns `&'static str`;
            // pass it directly without an alloc-coupled `to_string()`
            // round-trip, which keeps this compiling under no_std. `target` is
            // deliberately not forwarded: it is a routing decision owned by
            // the `<send>` that produced the original event, and inheriting it
            // would re-route the child's copy instead of delivering it.
            if concepts::has_autoforward::<P>() {
                let name = P::get_event_name(event_with_meta.event);
                let metadata = event_with_meta.metadata.clone();
                let policy_ptr: *mut P = &mut self.policy as *mut P;
                // SAFETY: see execute_on_entry.
                unsafe {
                    (*policy_ptr).forward_to_autoforward_children(name, &metadata, self);
                }
            }
            // §scxml-5.10: Populate policy metadata from event (ports C++ populatePolicyFromMetadata)
            self.policy
                .populate_event_metadata(&event_with_meta.metadata);
            // EventSchema native lowering: bind the typed payload
            // that rode with this event so `_event.data.<field>` guards read it
            // natively. No-op for schemaless policies (`Payload = ()`).
            self.policy.populate_event_payload(&event_with_meta.payload);
            // §scxml-3.1.2: "If no transition matches in any state, the event
            // is discarded." Discarding it is the rule; being unable to say so
            // is not part of the rule. The host that put this event on the
            // queue is the one party that cannot see the outcome — a discard
            // leaves the configuration exactly as a self transition does — and
            // it is the party that got the event wrong. Recorded on the
            // external queue only: an internal `<raise>` that matches nothing
            // is the document's own business, and both ends of it are in the
            // document.
            if self.execute_transition(event_with_meta.event) == EventOutcome::Discarded {
                self.discarded_external_events = self.discarded_external_events.saturating_add(1);
                self.last_discarded_event = Some(event_with_meta.event);
                sce_log_debug!(
                    "Engine::process_next_external_event: no transition matched; discarded"
                );
            }
            self.policy.clear_event_metadata();
        }
        true
    }

    /// Publish a macrostep this engine stopped short, from whichever branch of
    /// Appendix D's inner loop ran out of budget.
    ///
    /// One function, two callers, for the reason the budget is one number: a
    /// host reads a macrostep that did not reach a stable configuration, and
    /// the branch it died in is a detail of the document, not of the contract.
    /// Two copies of this would be two chances for one of them to stop setting
    /// the flag that keeps the same chain from being handed a second budget.
    ///
    /// `macrostep_truncated` is set on BOTH sides of the feature gate below,
    /// and that is the line between the bound and its report. The flag is what
    /// stops the drain re-entering a chain it already refused — remove it and
    /// the engine spins. The counter and the state snapshot are what a host
    /// READS, and a consumer who reads neither can compile them out with
    /// `no_macrostep_diagnostics` (see this crate's Cargo.toml for why that
    /// one is opt-out rather than opt-in).
    pub(crate) fn record_truncated_macrostep(&mut self, state: P::State) {
        #[cfg(not(feature = "no_macrostep_diagnostics"))]
        {
            self.truncated_macrosteps = self.truncated_macrosteps.saturating_add(1);
            self.last_truncated_macrostep_state = Some(state);
        }
        #[cfg(feature = "no_macrostep_diagnostics")]
        let _ = state;
        self.macrostep_truncated = true;
    }

    /// §scxml-3.13: Check and execute eventless transitions until stable.
    ///
    /// Bounded at [`MAX_MACROSTEP_MICROSTEPS`] microsteps and, when the chain
    /// is still going at that point, reported through
    /// [`truncated_macrosteps`](Self::truncated_macrosteps) — the ceiling is a
    /// departure from a document the specification allows, so it is not a
    /// silent one. The budget is the macrostep's, not this call's: see
    /// [`MAX_MACROSTEP_MICROSTEPS`]. Ported from C++
    /// `EventProcessingAlgorithms.h:98-136`.
    pub(crate) fn check_eventless_transitions(&mut self) {
        if self.macrostep_truncated {
            // This macrostep was already stopped at the ceiling. Re-entering
            // the drain would hand the same chain a second budget, which is
            // the runaway the ceiling exists to refuse.
            return;
        }
        let null_event = P::null_event();
        // Microsteps taken, not loop turns: the turn that finds nothing
        // enabled is how a macrostep ends, and counting it would spend the
        // budget on the proof that no budget was needed. The count lives on
        // the engine because the macrostep does — see
        // [`Engine::macrostep_microsteps_taken`].

        loop {
            let old_state = self.current_state;
            let pre_transition_states = self.get_active_states();
            let mut new_state = self.current_state;

            let took_transition = self.process_transition_dispatch(&mut new_state, null_event);
            if !took_transition {
                // Nothing is enabled by NULL — the macrostep has
                // reached the stable configuration the clause describes, and
                // nothing was refused however long the chain was.
                break;
            }

            if self.macrostep_microsteps_taken == MAX_MACROSTEP_MICROSTEPS {
                // The chain is still going one microstep past the budget, so
                // this is the case the specification calls a macrostep that
                // does not terminate. Refuse the microstep rather than take it, and
                // publish the refusal: the configuration left behind is not a
                // stable one and only this counter says so.
                self.record_truncated_macrostep(old_state);
                #[cfg(not(feature = "no_macrostep_diagnostics"))]
                sce_log_error!(
                    "Engine::check_eventless_transitions: macrostep still going after {} microsteps; stopped",
                    MAX_MACROSTEP_MICROSTEPS
                );
                break;
            }
            self.macrostep_microsteps_taken += 1;

            self.current_state = new_state;
            let needs_hierarchical =
                (old_state != new_state) || (!self.policy.last_transition_is_targetless());

            if !needs_hierarchical {
                // Targetless transition -- execute actions only
                self.execute_transition_actions_dispatch();
                continue;
            }

            // Hierarchical exit/entry
            // For parallel state machines, `process_transition` already performed a full
            // microstep (exit/transition-actions/entry) via `execute_microstep` in the
            // policy. Calling `handle_hierarchical_transition` again would double-run
            // onexit/onentry (see `execute_transition` for the full explanation).
            if !P::HAS_PARALLEL_STATES {
                self.handle_hierarchical_transition(old_state, new_state, &pre_transition_states);
            } else {
                self.resolve_current_state_to_leaf();
            }

            // Check for final state
            if self.is_in_final_state() {
                break;
            }
        }
    }

    /// §scxml-3.12 / §scxml-3.13: Dispatch a single transition.
    ///
    /// Calls `process_transition` on the policy; if it returns `true`, performs
    /// the hierarchical exit/entry dance via `handle_hierarchical_transition`.
    pub(crate) fn execute_transition(&mut self, event: P::Event) -> EventOutcome {
        let old_state = self.current_state;
        let pre_transition_states = self.get_active_states();
        let mut new_state = self.current_state;

        let took_transition = self.process_transition_dispatch(&mut new_state, event);
        if !took_transition {
            return EventOutcome::Discarded;
        }

        self.current_state = new_state;
        let is_self_transition = old_state == new_state;
        let needs_hierarchical = (old_state != new_state)
            || (is_self_transition && !self.policy.last_transition_is_targetless());

        if !needs_hierarchical {
            // §scxml-3.4: targetless transition — execute actions only
            self.execute_transition_actions_dispatch();
            return EventOutcome::Taken {
                configuration_changed: false,
            };
        }

        // §scxml-3.12: Hierarchical exit/entry
        //
        // For parallel state machines the generated `process_transition` already called
        // `execute_microstep` internally (it handles exit actions, transition actions,
        // entry actions, and history recording per Appendix D.2). Calling
        // `handle_hierarchical_transition` again would double-run onexit/onentry actions
        // and, worse, exit states from `pre_transition_states` that were already restored
        // (test 504: Var1/Var2/Var3 increment too many times).
        if !P::HAS_PARALLEL_STATES {
            self.handle_hierarchical_transition(old_state, new_state, &pre_transition_states);
        } else {
            // §scxml-3.3: Still resolve the current_state leaf (execute_microstep
            // sets current_state = target or parallel parent; the macrostep loop needs
            // the deepest active atomic state).
            self.resolve_current_state_to_leaf();
        }
        self.check_eventless_transitions();
        EventOutcome::Taken {
            configuration_changed: true,
        }
    }

    /// §scxml-3.12 / §scxml-3.13: Execute hierarchical exit/entry between two states.
    ///
    /// 1:1 port of C++ `StaticExecutionEngine::handleHierarchicalTransition`
    /// (`StaticExecutionEngine.h:151-299`). Handles:
    /// - Internal vs external transition LCA calculation (W3C 5.9.2)
    /// - Active descendant exit before source exit (W3C 3.13)
    /// - Exit chain to LCA
    /// - Ancestor/self transition target re-entry (W3C 3.10, test 579)
    /// - Transition action execution between exit and entry
    /// - Entry chain from LCA to new state
    /// - No-LCA top-level case
    pub(crate) fn handle_hierarchical_transition(
        &mut self,
        old_state: P::State,
        new_state: P::State,
        pre_transition_states: &[P::State],
    ) {
        sce_log_debug!(
            "Engine::handle_hierarchical_transition: {:?} -> {:?}",
            old_state,
            new_state
        );

        // §scxml-5.9.2: Determine LCA based on transition type
        let lca: Option<P::State> = if self.policy.last_transition_is_internal() {
            let is_self_transition = old_state == new_state;
            let is_proper_descendant =
                !is_self_transition && P::is_descendant_of(new_state, old_state);
            let is_source_compound = P::is_compound_state(old_state);

            if is_proper_descendant && is_source_compound {
                // §scxml-3.13: Internal to proper descendant in compound — source is LCA
                Some(old_state)
            } else {
                // W3C 3.13/5.9.2: Non-compound source or non-descendant — behaves as external
                hierarchy::find_lca::<P>(old_state, new_state)
            }
        } else {
            hierarchy::find_lca::<P>(old_state, new_state)
        };

        if let Some(lca_state) = lca {
            // §scxml-3.13: Exit active descendants of old_state deepest first.
            // Build via the cfg-branched StateChain so the no_std heapless variant
            // is bounded by MAX_HIERARCHY_DEPTH (the active states slice is itself
            // a depth-bounded chain — descendants_to_exit ⊆ pre_transition_states).
            let mut descendants_to_exit: hierarchy::StateChain<P::State> = hierarchy::new_chain();
            for &s in pre_transition_states.iter() {
                if s != old_state && P::is_descendant_of(s, old_state) {
                    hierarchy::push_chain(&mut descendants_to_exit, s);
                }
            }
            // Sort by document order descending (deeper first). Use
            // `sort_unstable_by` (in `core::slice`, no alloc) rather than
            // `sort_by` (alloc-coupled): document-order values are distinct
            // by construction so stability is irrelevant, and the unstable
            // variant compiles under both std and `--features=no_std`.
            descendants_to_exit
                .sort_unstable_by(|a, b| P::get_document_order(*b).cmp(&P::get_document_order(*a)));

            for descendant in descendants_to_exit {
                sce_log_debug!(
                    "handle_hierarchical_transition: exit descendant {:?}",
                    descendant
                );
                self.execute_on_exit(descendant, pre_transition_states);
            }

            // §scxml-3.13: Exit from old_state up to (not including) LCA
            let exit_chain = hierarchy::build_exit_chain::<P>(old_state, lca_state);
            for state in exit_chain {
                sce_log_debug!("handle_hierarchical_transition: exit {:?}", state);
                self.execute_on_exit(state, pre_transition_states);
            }

            // §scxml-3.10 (test 579): Ancestor/self transition — exit and re-enter target
            let is_target_active = pre_transition_states.contains(&new_state);
            if new_state == lca_state && is_target_active {
                sce_log_debug!(
                    "handle_hierarchical_transition: ancestor/self transition — exit target {:?}",
                    new_state
                );
                self.execute_on_exit(new_state, pre_transition_states);
            }

            // §scxml-3.13: Execute transition actions between exit and entry
            self.execute_transition_actions_dispatch();

            // §scxml-3.13: Enter from LCA down to new_state. Uses StateChain
            // so the no_std heapless variant is bounded by MAX_HIERARCHY_DEPTH —
            // same depth invariant as `build_entry_chain_from_ancestor` itself.
            let entry_chain: hierarchy::StateChain<P::State> = if new_state == lca_state {
                // Ancestor/self case — enter full subtree from target.
                let full = hierarchy::build_entry_chain::<P>(new_state);
                let mut filtered: hierarchy::StateChain<P::State> = hierarchy::new_chain();
                for s in full.into_iter() {
                    if s == lca_state || P::is_descendant_of(s, lca_state) {
                        hierarchy::push_chain(&mut filtered, s);
                    }
                }
                filtered
            } else {
                hierarchy::build_entry_chain_from_ancestor::<P>(new_state, lca_state)
            };

            // §scxml-D-addAncestorStatesToEnter: everything but the last link
            // is an ancestor of the target, and an ancestor is entered WITHOUT
            // its default initial child — the entry set already holds the next
            // link. Only the target itself takes defaults.
            for (i, state) in entry_chain.iter().enumerate() {
                sce_log_debug!("handle_hierarchical_transition: enter {:?}", state);
                match entry_chain.get(i + 1) {
                    Some(&next) => self.execute_on_entry_as_ancestor(*state, next),
                    None => self.execute_on_entry(*state),
                }
            }

            if let Some(&last) = entry_chain.last() {
                self.current_state = last;
            }

            // §scxml-3.3: Resolve current_state to the deepest initial leaf.
            // execute_entry_actions for compound states recursively enters initial
            // children, but current_state must track the deepest leaf for
            // eventless transition checks to work correctly.
            self.resolve_current_state_to_leaf();
        } else {
            // No LCA — top-level transition, exit all ancestors of old_state
            sce_log_debug!("handle_hierarchical_transition: no LCA (top-level)");

            let mut current = Some(old_state);
            while let Some(state) = current {
                sce_log_debug!("handle_hierarchical_transition: exit to root: {:?}", state);
                self.execute_on_exit(state, pre_transition_states);
                current = P::get_parent(state);
            }

            self.execute_transition_actions_dispatch();

            let entry_chain = hierarchy::build_entry_chain::<P>(new_state);
            // §scxml-D-addAncestorStatesToEnter, as above: only the last link
            // is the target, and only the target takes defaults.
            for (i, state) in entry_chain.iter().enumerate() {
                sce_log_debug!(
                    "handle_hierarchical_transition: enter from root: {:?}",
                    state
                );
                match entry_chain.get(i + 1) {
                    Some(&next) => self.execute_on_entry_as_ancestor(*state, next),
                    None => self.execute_on_entry(*state),
                }
            }

            if let Some(&last) = entry_chain.last() {
                self.current_state = last;
            }

            self.resolve_current_state_to_leaf();
        }
    }

    /// §scxml-3.3: Walk current_state down through initial children to the leaf.
    ///
    /// For **non-parallel** SMs the generated `execute_entry_actions` does NOT recurse
    /// into compound→initial child (the engine's entry chain already covers ancestors).
    /// This method descends into the compound's initial child, calling `execute_on_entry`
    /// for each level, until it reaches an atomic leaf.
    ///
    /// For **parallel** SMs the generated `execute_entry_actions` already recurses (matching
    /// C++ `executeEntryActions` L319-343), so this is just a pointer walk without entry.
    fn resolve_current_state_to_leaf(&mut self) {
        const MAX_DEPTH: usize = 50;
        for _ in 0..MAX_DEPTH {
            if !P::is_compound_state(self.current_state) {
                break;
            }
            let child = self.policy.get_initial_or_history_child(self.current_state);
            if child == self.current_state {
                break; // No child to descend into
            }
            self.current_state = child;
            if !P::HAS_PARALLEL_STATES {
                // Non-parallel: template doesn't recurse, so we enter here.
                self.execute_on_entry(child);
            }
        }
    }
}
