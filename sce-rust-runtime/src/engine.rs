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
//! ## Phase 1 scope
//!
//! - Lifecycle: `new`, `initialize`, `step`, `tick`, `stop`, `is_running`
//! - State queries: `get_current_state`, `get_active_states`, `is_in_final_state`
//! - Event submission: `raise`, `raise_external`, `process_event`
//! - Scheduler stubs: `schedule_event`, `cancel_event`, `has_ready_events`
//! - Hierarchical transition: `handle_hierarchical_transition` (150 lines ported from C++)
//!
//! Phases 2-4 expand scheduler, HTTP send, invoke support, and the full
//! parallel state processing machinery.

// Watching-zenoh RFC §5.J.2 (lines 1989-1994): `Arc`/`Mutex` back the
// parent→child external event queue plumbing in `get_external_queue_handle`,
// which is invoke-coupled. The codegen-time validator rejects `<invoke>` under
// `--no-std` via `codegen/no-std-invoke-not-supported`, so the handle is never
// reachable from emitted code; gate the import + method to `!no_std`.
#[cfg(not(feature = "no_std"))]
use std::sync::{Arc, Mutex};
// `Duration` is re-exported by `std::time` from `core::time` and is therefore
// available under both build profiles. `Instant`, however, is host-clock-coupled
// and lives only in `std`; the no_std variant routes its monotonic-time reads
// through the `Hal::now_ticks_ms` returning a u64 millisecond tick (see the
// `SchedTimePoint` alias below).
use core::time::Duration;
#[cfg(not(feature = "no_std"))]
use std::time::Instant;

use crate::event::{EventMetadata, EventType, EventWithMetadata};
use crate::hal::Hal;
use crate::helpers::event_queue::EventQueueManager;
use crate::helpers::{hierarchy, state_policy_concepts as concepts};
// Watching-zenoh RFC §5.J.2: the HTTP module is alloc-coupled
// (HashMap<String, Vec<String>> + reqwest) and whole-module-gated to `!no_std`
// in `lib.rs`. The codegen-time validator rejects
// `BasicHTTPEventProcessor` `<send>` under `--no-std` via
// `codegen/no-std-http-not-supported`, so the engine's HTTP fields + dispatch
// surface are unreachable from emitted no_std code.
#[cfg(not(feature = "no_std"))]
use crate::http::{HttpSendRequest, HttpSendResponse};
use crate::policy::StatePolicy;
use crate::{sce_log_debug, SceString};
#[cfg(feature = "no_std")]
use crate::MAX_SCHEDULED_EVENTS;

// ─────────────────────────────────────────────────────────────────────
// Scheduler time point alias (Watching-zenoh RFC §5.J.2 line 1984 HAL)
// ─────────────────────────────────────────────────────────────────────
// `SchedTimePoint` decouples the scheduler's comparable-timestamp type from
// the host clock. std builds use `Instant` (preserves existing behaviour);
// no_std builds use `u64` milliseconds — the `<P::Hal>::now_ticks_ms()`
// reading. The `PullScheduler` itself holds no clock source: all
// time-comparing methods take `now: SchedTimePoint` as a parameter (DI
// pattern), and `Engine<P>` resolves the per-build `now` via the
// `sched_now`/`sched_now_plus` helpers below.

/// Comparable timestamp used by the scheduler under each build profile.
///
/// - std: `std::time::Instant` — host monotonic clock, ABI-preserving.
/// - no_std: `u64` — millisecond ticks from `<P::Hal as Hal>::now_ticks_ms()`.
#[cfg(not(feature = "no_std"))]
pub type SchedTimePoint = Instant;
#[cfg(feature = "no_std")]
pub type SchedTimePoint = u64;

/// Minimal scheduler stub for Phase 1.
///
/// 1:1 API parity with C++ `SCE::PullScheduler<EventType>`. Stores delayed
/// events with a `SchedTimePoint` ready-time and exposes pull-style queries
/// (`has_ready_events_at` / `pop_ready_event_at`) that take the caller's
/// current time as a parameter — a textbook dependency-injection split that
/// keeps the scheduler clock-source-agnostic and makes it unit-testable with
/// synthetic clocks.
///
/// Watching-zenoh RFC §5.J.2 (lines 1989-1994): under `--features=no_std`
/// the backing store is a stack-allocated `heapless::Vec` capped at
/// [`MAX_SCHEDULED_EVENTS`] (= 32 in v1; see the `lib.rs` doc-comment for the
/// reasoning and the deferred per-document tunable). Capacity overflow under
/// no_std is treated as a fatal configuration error (panic) per the same
/// "no silent transition drop" discipline the W3C SCXML algorithm follows.
///
/// Kept as a concrete (non-trait) type to match C++ `SCE::PullScheduler<Event> scheduler_;`.
#[derive(Debug)]
pub struct PullScheduler<E> {
    /// Pending entries: (event, event_data_json, send_id, ready_at).
    #[cfg(not(feature = "no_std"))]
    entries: Vec<ScheduledEntry<E>>,
    #[cfg(feature = "no_std")]
    entries: ::heapless::Vec<ScheduledEntry<E>, MAX_SCHEDULED_EVENTS>,
    next_auto_send_id: u64,
}

#[derive(Debug)]
struct ScheduledEntry<E> {
    event: E,
    event_data: SceString,
    send_id: SceString,
    ready_at: SchedTimePoint,
}

impl<E: Clone> PullScheduler<E> {
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

    /// W3C SCXML 6.2: Schedule an event for delayed delivery, given an
    /// already-resolved `ready_at` time-point.
    ///
    /// If `send_id` is empty, an automatic ID is generated. Returns the ID
    /// used (caller can use it to cancel). The caller is responsible for
    /// computing `ready_at` from the current clock + delay — `Engine<P>`'s
    /// `schedule_event` wrapper does this via `sched_now_plus(delay)`.
    ///
    /// Watching-zenoh RFC §5.J.2: under `--features=no_std` an attempted
    /// push past [`MAX_SCHEDULED_EVENTS`] panics rather than silently dropping
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
        let entry = ScheduledEntry {
            event,
            event_data: crate::sce_string_from_str(event_data),
            send_id: effective_send_id.clone(),
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
    fn push_scheduled(&mut self, entry: ScheduledEntry<E>) {
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

    /// W3C SCXML 6.2.5: Cancel a scheduled event by send ID. Returns `true` if found.
    pub fn cancel_event(&mut self, send_id: &str) -> bool {
        let before = self.entries.len();
        self.entries.retain(|e| e.send_id != send_id);
        self.entries.len() < before
    }

    /// Whether any scheduled events are ready to fire (ready_at <= now).
    ///
    /// Caller supplies the current time — see `Engine<P>::has_ready_events` for
    /// the host-build wrapper that reads `<P::Hal>::now_ticks_ms()` / `Instant::now()`.
    pub fn has_ready_events_at(&self, now: SchedTimePoint) -> bool {
        self.entries.iter().any(|e| e.ready_at <= now)
    }

    /// Pop the next ready event and its data. Returns `None` if nothing is ready.
    ///
    /// Caller supplies the current time. Matches C++
    /// `PullScheduler::popReadyEvent(E&, string&) -> bool` (but returns
    /// an `Option` tuple instead of bool+out-params, which is the idiomatic Rust shape).
    pub fn pop_ready_event_at(&mut self, now: SchedTimePoint) -> Option<(E, SceString)> {
        let idx = self.entries.iter().position(|e| e.ready_at <= now)?;
        // heapless::Vec::remove is `pub fn remove(&mut self, index: usize) -> T`
        // — same shape as Vec::remove, so this works under both profiles.
        let entry = self.entries.remove(idx);
        Some((entry.event, entry.event_data))
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


impl<E: Clone> Default for PullScheduler<E> {
    fn default() -> Self {
        Self::new()
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Engine<P>
// ═══════════════════════════════════════════════════════════════════════════

/// The SCXML execution engine.
///
/// Generic over a [`StatePolicy`] `P` that encodes the state machine structure
/// at compile time. Matches C++ `StaticExecutionEngine<StatePolicy>`.
///
/// ## HAL routing (watching-zenoh RFC §5.J.2)
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
    /// W3C SCXML C.1: Internal event queue (high priority, `<raise>` + targetless sends).
    pub(crate) internal_queue: EventQueueManager<EventWithMetadata<P::Event>>,
    /// W3C SCXML C.1: External event queue (low priority, external sends).
    pub(crate) external_queue: EventQueueManager<EventWithMetadata<P::Event>>,
    /// Whether the engine is currently running (set false by `stop()` and final state).
    pub(crate) is_running: bool,
    /// W3C SCXML 6.4: Completion callback invoked when reaching a final state.
    ///
    /// Watching-zenoh RFC §5.J.2: `Box<dyn FnMut>` is alloc-coupled and gated to
    /// `!no_std`. Embedded consumers poll [`is_in_final_state`](Self::is_in_final_state)
    /// instead; a future no_std-compatible completion ABI (extern "C" fn +
    /// userdata) lands when a consumer demands it. Mirrors the gate applied to
    /// `helpers::entry_exit::execute_*_blocks` in B-γ2d-2.
    #[cfg(not(feature = "no_std"))]
    pub(crate) completion_callback: Option<Box<dyn FnMut()>>,
    /// W3C SCXML C.2: HTTP send dispatch callback.
    ///
    /// Watching-zenoh RFC §5.J.2: HTTP is rejected upstream under `--no-std`
    /// via `codegen/no-std-http-not-supported`, so the callback field + setter
    /// + dispatcher are all gated to `!no_std`. Generated no_std code never
    /// emits a `perform_http_send` call site.
    #[cfg(not(feature = "no_std"))]
    pub(crate) on_http_send: Option<Box<dyn FnMut(HttpSendRequest) -> Option<HttpSendResponse>>>,
    /// W3C SCXML 6.2: Delayed event scheduler.
    pub(crate) scheduler: PullScheduler<P::Event>,
    /// W3C SCXML 5.5 + 6.3.1: Donedata payload evaluated on top-level `<final>`,
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
            internal_queue: EventQueueManager::new(),
            external_queue: EventQueueManager::new(),
            is_running: false,
            #[cfg(not(feature = "no_std"))]
            completion_callback: None,
            #[cfg(not(feature = "no_std"))]
            on_http_send: None,
            scheduler: PullScheduler::new(),
            donedata_at_final: SceString::new(),
        }
    }

    // ════════════════════════════════════════
    // Scheduler clock readers (per-build cfg-branched, HAL-routed under no_std)
    // ════════════════════════════════════════

    /// Resolve the current scheduler time point for "now".
    ///
    /// Routes through `<P::Hal>::now_ticks_ms()` under no_std (returning a u64
    /// millisecond tick) and through `Instant::now()` under std. The textbook
    /// DI split keeps [`PullScheduler`] clock-source-agnostic and unit-testable.
    #[inline]
    fn sched_now(&self) -> SchedTimePoint {
        #[cfg(not(feature = "no_std"))]
        {
            Instant::now()
        }
        #[cfg(feature = "no_std")]
        {
            <P::Hal as Hal>::now_ticks_ms()
        }
    }

    /// Resolve `now + delay` for scheduling.
    ///
    /// Under std this is `Instant::now() + delay`. Under no_std this is
    /// `<P::Hal>::now_ticks_ms().saturating_add(delay.as_millis() as u64)` —
    /// `saturating_add` clamps a pathologically large delay to `u64::MAX`
    /// rather than wrapping (`u64::MAX` ms ≈ 585 million years, so the clamp
    /// is operationally indistinguishable from "infinity").
    #[inline]
    fn sched_now_plus(&self, delay: Duration) -> SchedTimePoint {
        #[cfg(not(feature = "no_std"))]
        {
            Instant::now() + delay
        }
        #[cfg(feature = "no_std")]
        {
            let delay_ms = delay.as_millis() as u64;
            <P::Hal as Hal>::now_ticks_ms().saturating_add(delay_ms)
        }
    }

    // ════════════════════════════════════════
    // HAL-routed queries (watching-zenoh RFC §5.J.2 line 1984)
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
        let policy_ptr: *mut P = &mut self.policy as *mut P;
        // SAFETY: see doc comment above. The policy field and the rest of
        // Engine's fields are disjoint; the split borrow lasts only for the
        // duration of the method call.
        unsafe {
            (*policy_ptr).execute_entry_actions(state, self);
        }
    }

    /// Execute the policy's `execute_exit_actions` with split-borrowed `self`.
    ///
    /// # Safety
    ///
    /// See [`Self::execute_on_entry`] for the full safety rationale. The
    /// `pre_transition_active` slice is borrowed from the caller's stack and
    /// does not interact with the split borrow.
    pub(crate) fn execute_on_exit(
        &mut self,
        state: P::State,
        pre_transition_active: &[P::State],
    ) {
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
    /// Matches C++ `StaticExecutionEngine::initialize()`. W3C SCXML 5.3
    /// guarantees datamodel initialization happens before any state entry.
    pub fn initialize(&mut self) {
        self.is_running = true;

        // W3C SCXML 5.3: Initialize datamodel before any state entry
        if concepts::has_data_model_init::<P>() {
            self.initialize_data_model_dispatch();
        }

        // W3C SCXML 3.3: Entry chain from root to initial leaf
        let entry_chain = hierarchy::build_entry_chain::<P>(self.current_state);
        for state in entry_chain {
            self.execute_on_entry(state);
        }
        // W3C SCXML 3.3: Resolve current_state to the deepest initial leaf
        self.resolve_current_state_to_leaf();

        // W3C SCXML C.1: Macrostep completion loop — drain internal + eventless
        // until a stable configuration is reached.
        sce_log_debug!("Engine::initialize: entering macrostep completion loop");
        loop {
            self.check_eventless_transitions();
            if !self.internal_queue.has_events() && !self.external_queue.has_events() {
                break;
            }
            self.process_event_queues();
        }
        sce_log_debug!("Engine::initialize: macrostep completion loop finished");

        // W3C SCXML 6.4: Execute pending invokes once stable
        if concepts::has_invoke_support::<P>() {
            let policy_ptr: *mut P = &mut self.policy as *mut P;
            // SAFETY: see execute_on_entry.
            unsafe {
                (*policy_ptr).execute_pending_invokes(self);
            }
            // Process done.invoke events raised by immediately-completed children
            sce_log_debug!("Engine::initialize: processing events raised by completed invokes");
            self.process_event_queues();
            self.check_eventless_transitions();
        }

        // W3C SCXML 6.4: Fire completion callback if we reached a final state during init.
        // Watching-zenoh RFC §5.J.2: Box<dyn FnMut> callback is alloc-coupled and gated
        // to `!no_std` (see field declaration above).
        #[cfg(not(feature = "no_std"))]
        if self.is_in_final_state() && self.completion_callback.is_some() {
            sce_log_debug!("Engine::initialize: reached final state during init, invoking completion callback");
            let active = self.get_active_states();
            let final_state = self.current_state;
            self.execute_on_exit(final_state, &active);
            if let Some(cb) = self.completion_callback.as_mut() {
                cb();
            }
        }
    }

    /// Process one macrostep: drain queues and run eventless transitions.
    ///
    /// Matches C++ `StaticExecutionEngine::step()`. Used by parent SMs to
    /// explicitly drive children after sending them events (W3C SCXML 6.4).
    pub fn step(&mut self) {
        self.process_event_queues();
        self.check_eventless_transitions();

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
    pub fn tick(&mut self) {
        if !self.is_running {
            return;
        }
        if self.is_in_final_state() {
            #[cfg(not(feature = "no_std"))]
            if let Some(cb) = self.completion_callback.as_mut() {
                sce_log_debug!("Engine::tick: final state already reached, invoking completion callback");
                cb();
            }
            return;
        }

        // W3C SCXML 6.2: Pop all ready scheduled events into the external queue.
        // Read the clock once per iteration via the cfg-branched helper — keeps
        // `PullScheduler` clock-source-agnostic (textbook DI split).
        loop {
            let now = self.sched_now();
            let Some((event, data)) = self.scheduler.pop_ready_event_at(now) else {
                break;
            };
            self.raise_external(event, &data, "");
        }

        // W3C SCXML 6.4: Tick child state machines
        if concepts::has_child_tick::<P>() {
            let policy_ptr: *mut P = &mut self.policy as *mut P;
            // SAFETY: see execute_on_entry.
            unsafe {
                (*policy_ptr).tick_children(self);
            }
        }

        // Delegate to step() for queue drain + completion callback
        self.step();

        // W3C SCXML 6.4: Execute pending invokes after macrostep
        if concepts::has_invoke_support::<P>() {
            let policy_ptr: *mut P = &mut self.policy as *mut P;
            // SAFETY: see execute_on_entry.
            unsafe {
                (*policy_ptr).execute_pending_invokes(self);
            }
        }
    }

    /// Stop the engine. Subsequent `tick`/`process_event` calls become no-ops.
    pub fn stop(&mut self) {
        self.is_running = false;
    }

    /// Whether the engine is running (not stopped or awaiting completion).
    pub fn is_running(&self) -> bool {
        self.is_running
    }

    /// Current active (leaf) state.
    pub fn get_current_state(&self) -> P::State {
        self.current_state
    }

    /// W3C SCXML 3.11: Full list of active states (for history recording + In() predicate).
    ///
    /// Non-parallel machines: returns the hierarchy `[leaf, parent, grandparent, ..., root]`.
    /// Parallel machines: returns the union of all active regions via
    /// [`StatePolicy::get_active_states`](crate::StatePolicy::get_active_states).
    ///
    /// Watching-zenoh RFC §5.J.2: returns the bounded
    /// [`hierarchy::StateChain`](crate::helpers::hierarchy::StateChain) which
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

    /// W3C SCXML 3.3: Whether the current state is a top-level final state.
    pub fn is_in_final_state(&self) -> bool {
        P::is_final_state(self.current_state)
    }

    /// W3C SCXML 5.5 + 6.3.1: Stash the donedata payload evaluated on a
    /// top-level `<final>` so the invoking parent can lift it onto
    /// `done.invoke.<id>._event.data`.
    ///
    /// Called from generated `execute_entry_actions` code on a child engine
    /// (1:1 port of the C++ AOT `stashDonedataAtFinal` / Kotlin
    /// `StateMachineEngine.stashDonedataAtFinal` contract).
    pub fn stash_donedata_at_final(&mut self, data: SceString) {
        self.donedata_at_final = data;
    }

    /// W3C SCXML 5.5 + 6.3.1: Read the donedata payload stashed by a
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

    /// W3C SCXML 6.4: Get shared handle to external queue for child→parent event passing.
    ///
    /// Returns an `Arc<Mutex<Vec<(event_name, event_data)>>>` that child state machines
    /// can push events into via `#_parent` send targets. Parent drains this in `tick_children()`.
    ///
    /// Watching-zenoh RFC §5.J.2: gated to `!no_std` because `Arc`/`Mutex`/`Vec` are
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

    /// W3C SCXML C.1: Raise an internal event (high priority).
    ///
    /// Matches C++ `raise(EventWithMetadata)`.
    pub fn raise(&mut self, event: EventWithMetadata<P::Event>) {
        self.internal_queue.raise(event);
    }

    /// W3C SCXML C.1 / 6.2: Raise an external event with optional data and origin.
    ///
    /// Matches C++ `raiseExternal(Event, const string&, const string&)`.
    pub fn raise_external(&mut self, event: P::Event, event_data: &str, origin: &str) {
        let meta = EventWithMetadata::with_fields(
            event,
            crate::sce_string_from_str(event_data),
            crate::sce_string_from_str(origin),
            SceString::new(), // send_id
            EventType::External,
            crate::sce_string_from_str(crate::helpers::scxml_constants::SCXML_EVENT_PROCESSOR_TYPE),
            SceString::new(), // invoke_id
            SceString::new(), // target
        );
        self.external_queue.raise(meta);

        // W3C SCXML 5.10.1: Mark next event as external for _event.type
        if concepts::has_external_event_flag::<P>() {
            self.policy.set_next_event_is_external(true);
        }
    }

    /// W3C SCXML 6.4.6: Raise an external event by name (for child autoforward).
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

    /// W3C SCXML 6.4.1: Raise an external event with full metadata (for child-to-parent).
    ///
    /// Matches C++ `raiseExternal(const EventWithMetadata&)`. Preserves `invokeid`
    /// for parent finalize handlers.
    pub fn raise_external_with_meta(&mut self, event: EventWithMetadata<P::Event>) {
        sce_log_debug!(
            "Engine::raise_external_with_meta: enqueuing external event with metadata"
        );

        // W3C SCXML 6.4.6: Autoforward
        if concepts::has_autoforward::<P>() {
            let name = P::get_event_name(event.event).to_string();
            let policy_ptr: *mut P = &mut self.policy as *mut P;
            // SAFETY: see execute_on_entry.
            unsafe {
                (*policy_ptr).forward_to_autoforward_children(&name, self);
            }
        }

        self.external_queue.raise(event);

        if concepts::has_external_event_flag::<P>() {
            self.policy.set_next_event_is_external(true);
        }
    }

    /// W3C SCXML 3.12: Process an external event (convenience API, runs one macrostep).
    ///
    /// Matches C++ `processEvent(Event)`.
    pub fn process_event(&mut self, event: P::Event) {
        if !self.is_running {
            return;
        }
        self.raise_external(event, "", "");
        self.step();
    }

    /// W3C SCXML 5.10: Process an external event with metadata.
    ///
    /// Matches C++ `processEvent(Event, const EventMetadata&)`.
    pub fn process_event_with_meta(&mut self, event: P::Event, metadata: EventMetadata) {
        if !self.is_running {
            return;
        }
        let meta = EventWithMetadata {
            event,
            metadata,
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
    /// Resolves the current clock via [`sched_now_plus`](Self::sched_now_plus)
    /// (`Instant::now() + delay` under std, `<P::Hal>::now_ticks_ms() + delay_ms`
    /// under no_std) and forwards to the clock-source-agnostic
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

    // ════════════════════════════════════════
    // Callbacks
    // ════════════════════════════════════════

    /// W3C SCXML 6.4: Register a callback invoked when the engine reaches a final state.
    ///
    /// Watching-zenoh RFC §5.J.2: gated to `!no_std` because `Box<dyn FnMut>` is
    /// alloc-coupled (mirrors `helpers::entry_exit::execute_*_blocks` gate from
    /// B-γ2d-2). Embedded consumers poll [`is_in_final_state`](Self::is_in_final_state)
    /// instead.
    #[cfg(not(feature = "no_std"))]
    pub fn set_completion_callback<F: FnMut() + 'static>(&mut self, callback: F) {
        self.completion_callback = Some(Box::new(callback));
    }

    /// W3C SCXML C.2: Register an HTTP send dispatcher callback.
    ///
    /// The callback receives an [`HttpSendRequest`] and returns an optional
    /// [`HttpSendResponse`]. When `Some`, the engine injects the response event
    /// into the external queue — enabling real HTTP round-trips against the
    /// shared W3C test server (`standalone_http_server.js`).
    ///
    /// Watching-zenoh RFC §5.J.2: gated to `!no_std` (HTTP itself is whole-module
    /// gated; the codegen-time validator rejects `BasicHTTPEventProcessor`
    /// `<send>` under `--no-std` via `codegen/no-std-http-not-supported`).
    #[cfg(not(feature = "no_std"))]
    pub fn set_http_send_callback<F>(&mut self, callback: F)
    where
        F: FnMut(HttpSendRequest) -> Option<HttpSendResponse> + 'static,
    {
        self.on_http_send = Some(Box::new(callback));
    }

    /// W3C SCXML C.2: Dispatch a BasicHTTP send through the registered callback.
    ///
    /// The callback is the sole dispatch mechanism. If it returns
    /// `Some(HttpSendResponse)`, the engine injects the response event into the
    /// external queue. The engine has no knowledge of HTTP transport — callers
    /// supply the implementation via [`set_http_send_callback`].
    ///
    /// Watching-zenoh RFC §5.J.2: gated to `!no_std` — see
    /// [`set_http_send_callback`] for the upstream rejection rationale.
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
            let response = cb(HttpSendRequest { target, event_name, content, params, send_id });
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

    // ════════════════════════════════════════
    // Convenience: runUntilCompletion
    // ════════════════════════════════════════

    /// Run the state machine to completion or timeout (W3C SCXML 6.2).
    ///
    /// Matches C++ `runUntilCompletion(timeout, pollInterval)`. Polls the scheduler
    /// and calls `tick()` in a loop until either the final state is reached or
    /// `timeout` elapses. Returns `true` on completion, `false` on timeout.
    ///
    /// Watching-zenoh RFC §5.J.2: gated to `!no_std` because the polling loop
    /// uses `std::thread::sleep` for cooperative blocking and `Instant::elapsed`
    /// for the timeout — both host-thread-coupled. no_std consumers drive their
    /// own executor loop, calling [`tick`](Self::tick) plus
    /// [`has_ready_events`](Self::has_ready_events) under their HAL waker
    /// (e.g. embassy `Signal`).
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
            std::thread::sleep(poll_interval);
            self.tick();
        }
        true
    }

    // ════════════════════════════════════════
    // Internal: microstep + macrostep implementation
    // ════════════════════════════════════════

    /// W3C SCXML D.1: Process both internal and external queues.
    pub(crate) fn process_event_queues(&mut self) {
        sce_log_debug!("Engine::process_event_queues: starting internal queue drain");

        // W3C SCXML C.1: Internal queue first
        while let Some(event_with_meta) = self.internal_queue.pop() {
            // W3C SCXML 5.4.1: Stop if top-level final state reached
            if P::is_final_state(self.current_state) && P::get_parent(self.current_state).is_none() {
                sce_log_debug!(
                    "Engine::process_event_queues: top-level final state reached, stopping"
                );
                return;
            }
            // W3C SCXML 5.10: Populate policy metadata from event (ports C++ populatePolicyFromMetadata)
            self.policy.populate_event_metadata(&event_with_meta.metadata);
            self.execute_transition(event_with_meta.event);
            self.policy.clear_event_metadata();
        }

        // W3C SCXML C.1: External queue second
        while let Some(event_with_meta) = self.external_queue.pop() {
            // W3C SCXML 6.5: Execute finalize before parent's own transition matching
            if concepts::has_finalize::<P>() {
                let policy_ptr: *mut P = &mut self.policy as *mut P;
                // SAFETY: see execute_on_entry.
                unsafe {
                    (*policy_ptr).execute_finalize_for_child_event(&event_with_meta, self);
                }
            }
            // W3C SCXML 5.10: Populate policy metadata from event (ports C++ populatePolicyFromMetadata)
            self.policy.populate_event_metadata(&event_with_meta.metadata);
            self.execute_transition(event_with_meta.event);
            self.policy.clear_event_metadata();
        }
    }

    /// W3C SCXML 3.13: Check and execute eventless transitions until stable.
    ///
    /// Uses bounded iteration to prevent infinite loops from cyclic eventless chains.
    /// Ported from C++ `EventProcessingAlgorithms.h:98-136`.
    pub(crate) fn check_eventless_transitions(&mut self) {
        const MAX_ITERATIONS: usize = 100;
        let null_event = P::null_event();

        for iteration in 0..MAX_ITERATIONS {
            let old_state = self.current_state;
            let pre_transition_states = self.get_active_states();
            let mut new_state = self.current_state;

            let took_transition = self.process_transition_dispatch(&mut new_state, null_event);
            if !took_transition {
                break;
            }

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

            if iteration == MAX_ITERATIONS - 1 {
                sce_log_debug!(
                    "Engine::check_eventless_transitions: max iterations reached ({})",
                    MAX_ITERATIONS
                );
            }
        }
    }

    /// W3C SCXML 3.12/3.13: Dispatch a single transition.
    ///
    /// Calls `process_transition` on the policy; if it returns `true`, performs
    /// the hierarchical exit/entry dance via `handle_hierarchical_transition`.
    pub(crate) fn execute_transition(&mut self, event: P::Event) -> bool {
        let old_state = self.current_state;
        let pre_transition_states = self.get_active_states();
        let mut new_state = self.current_state;

        let took_transition = self.process_transition_dispatch(&mut new_state, event);
        if !took_transition {
            return false;
        }

        self.current_state = new_state;
        let is_self_transition = old_state == new_state;
        let needs_hierarchical = (old_state != new_state)
            || (is_self_transition && !self.policy.last_transition_is_targetless());

        if !needs_hierarchical {
            // W3C SCXML 3.4: targetless transition — execute actions only
            self.execute_transition_actions_dispatch();
            return false;
        }

        // W3C SCXML 3.12: Hierarchical exit/entry
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
            // W3C SCXML 3.3: Still resolve the current_state leaf (execute_microstep
            // sets current_state = target or parallel parent; the macrostep loop needs
            // the deepest active atomic state).
            self.resolve_current_state_to_leaf();
        }
        self.check_eventless_transitions();
        true
    }

    /// W3C SCXML 3.12/3.13: Execute hierarchical exit/entry between two states.
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

        // W3C SCXML 5.9.2: Determine LCA based on transition type
        let lca: Option<P::State> = if self.policy.last_transition_is_internal() {
            let is_self_transition = old_state == new_state;
            let is_proper_descendant =
                !is_self_transition && P::is_descendant_of(new_state, old_state);
            let is_source_compound = P::is_compound_state(old_state);

            if is_proper_descendant && is_source_compound {
                // W3C SCXML 3.13: Internal to proper descendant in compound — source is LCA
                Some(old_state)
            } else {
                // W3C 3.13/5.9.2: Non-compound source or non-descendant — behaves as external
                hierarchy::find_lca::<P>(old_state, new_state)
            }
        } else {
            hierarchy::find_lca::<P>(old_state, new_state)
        };

        if let Some(lca_state) = lca {
            // W3C SCXML 3.13: Exit active descendants of old_state deepest first.
            // Build via the cfg-branched StateChain so the no_std heapless variant
            // is bounded by MAX_HIERARCHY_DEPTH (the active states slice is itself
            // a depth-bounded chain — descendants_to_exit ⊆ pre_transition_states).
            let mut descendants_to_exit: hierarchy::StateChain<P::State> = hierarchy::new_chain();
            for &s in pre_transition_states.iter() {
                if s != old_state && P::is_descendant_of(s, old_state) {
                    hierarchy::push_chain(&mut descendants_to_exit, s);
                }
            }
            // Sort by document order descending (deeper first). Both std `Vec` and
            // `heapless::Vec` impl `Deref<Target = [T]>`, so `sort_by` works on both.
            descendants_to_exit.sort_by(|a, b| P::get_document_order(*b).cmp(&P::get_document_order(*a)));

            for descendant in descendants_to_exit {
                sce_log_debug!("handle_hierarchical_transition: exit descendant {:?}", descendant);
                self.execute_on_exit(descendant, pre_transition_states);
            }

            // W3C SCXML 3.13: Exit from old_state up to (not including) LCA
            let exit_chain = hierarchy::build_exit_chain::<P>(old_state, lca_state);
            for state in exit_chain {
                sce_log_debug!("handle_hierarchical_transition: exit {:?}", state);
                self.execute_on_exit(state, pre_transition_states);
            }

            // W3C SCXML 3.10 (test 579): Ancestor/self transition — exit and re-enter target
            let is_target_active = pre_transition_states.contains(&new_state);
            if new_state == lca_state && is_target_active {
                sce_log_debug!(
                    "handle_hierarchical_transition: ancestor/self transition — exit target {:?}",
                    new_state
                );
                self.execute_on_exit(new_state, pre_transition_states);
            }

            // W3C SCXML 3.13: Execute transition actions between exit and entry
            self.execute_transition_actions_dispatch();

            // W3C SCXML 3.13: Enter from LCA down to new_state. Uses StateChain
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

            for state in &entry_chain {
                sce_log_debug!("handle_hierarchical_transition: enter {:?}", state);
                self.execute_on_entry(*state);
            }

            if let Some(&last) = entry_chain.last() {
                self.current_state = last;
            }

            // W3C SCXML 3.3: Resolve current_state to the deepest initial leaf.
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
            for state in &entry_chain {
                sce_log_debug!("handle_hierarchical_transition: enter from root: {:?}", state);
                self.execute_on_entry(*state);
            }

            if let Some(&last) = entry_chain.last() {
                self.current_state = last;
            }

            self.resolve_current_state_to_leaf();
        }
    }

    /// W3C SCXML 3.3: Walk current_state down through initial children to the leaf.
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
