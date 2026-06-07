// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Regression: `Engine::schedule_event` / `tick` / `has_ready_events` must
// resolve "now" through `<P::Hal as Hal>::now_ticks_ms()` under both std and
// no_std. Pre-fix the std path read `Instant::now()` directly and the Hal
// trait was decorative on host builds — a consumer-supplied synthetic-clock
// `Hal` impl had no effect on scheduler resolution, blocking deterministic
// timer-driven SCXML tests under host CI.
//
// This fixture wires a `MockHal` whose tick comes from an `AtomicU64` the
// test advances by hand, then asserts that a delayed `<send>`-equivalent
// schedule fires exactly when the mock clock crosses the ready-time. If
// `sched_now` regresses to reading `Instant::now()` directly, step (3)
// below fails because the real wall clock never reaches the synthetic
// ready-time during the test's wall-clock lifetime (and conversely the
// scheduler would fire as soon as the real clock advanced past the synthetic
// 0-anchored ready-time, which would also fail step (2)).

use core::time::Duration;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};

use sce_rust_runtime::{Engine, Hal, StatePolicy};

// ─────────────────────────────────────────────────────────────
// MockHal: process-global tick the test drives by hand
// ─────────────────────────────────────────────────────────────

static MOCK_TICK_MS: AtomicU64 = AtomicU64::new(0);

// `MOCK_TICK_MS` is one process-global clock, but cargo runs the `#[test]`
// functions in this binary on PARALLEL threads. Two tests that drive the
// clock (`scheduler_consults_hal_under_std`,
// `scheduler_resolution_is_milliseconds`) therefore race: one test's
// `mock_set_ticks` can interleave between the other's anchor and its
// `schedule_event`, so `ready_at` is computed against the wrong epoch and a
// later assertion observes a clock below it. The race only manifests under
// load (workspace `cargo test`), not in isolation. Serialize every
// clock-driving test on this lock so each runs with exclusive ownership of
// the global tick. `into_inner` ignores poisoning: a failing assertion in
// one test must surface as that test's failure, not cascade into a panic in
// the next test's `lock()`.
static CLOCK_LOCK: Mutex<()> = Mutex::new(());

fn lock_clock() -> MutexGuard<'static, ()> {
    CLOCK_LOCK.lock().unwrap_or_else(PoisonError::into_inner)
}

fn mock_set_ticks(ms: u64) {
    MOCK_TICK_MS.store(ms, Ordering::SeqCst);
}

#[derive(Debug, Clone, Copy, Default)]
struct MockHal;

impl Hal for MockHal {
    fn now_ticks_ms() -> u64 {
        MOCK_TICK_MS.load(Ordering::SeqCst)
    }
    fn wake() {}
    fn irq_save<F, R>(f: F) -> R
    where
        F: FnOnce() -> R,
    {
        f()
    }
}

// ─────────────────────────────────────────────────────────────
// Minimal StatePolicy — enough surface for Engine::new / schedule_event /
// has_ready_events. No transitions exercised; we only need the scheduler
// path to read MockHal.
// ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum St {
    S0,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Ev {
    Null,
    Delayed,
}

struct MockPolicy {
    last_internal: bool,
    last_targetless: bool,
    last_source: St,
}

impl MockPolicy {
    fn new() -> Self {
        Self {
            last_internal: false,
            last_targetless: false,
            last_source: St::S0,
        }
    }
}

impl StatePolicy for MockPolicy {
    type State = St;
    type Event = Ev;
    type Payload = ();
    type Hal = MockHal;
    type EventQueue = sce_rust_runtime::EventQueueManager<
        sce_rust_runtime::EventWithMetadata<Self::Event, Self::Payload>,
    >;

    fn initial_state() -> Self::State {
        St::S0
    }
    fn is_final_state(_s: Self::State) -> bool {
        false
    }
    fn get_parent(_s: Self::State) -> Option<Self::State> {
        None
    }
    fn is_compound_state(_s: Self::State) -> bool {
        false
    }
    fn is_descendant_of(_d: Self::State, _a: Self::State) -> bool {
        false
    }
    fn get_document_order(_s: Self::State) -> u32 {
        0
    }
    fn null_event() -> Self::Event {
        Ev::Null
    }

    fn get_event_name(e: Self::Event) -> &'static str {
        match e {
            Ev::Null => "",
            Ev::Delayed => "delayed",
        }
    }
    fn get_event_from_name(name: &str) -> Option<Self::Event> {
        match name {
            "delayed" => Some(Ev::Delayed),
            _ => None,
        }
    }
    fn get_state_name(_s: Self::State) -> &'static str {
        "s0"
    }

    fn last_transition_is_internal(&self) -> bool {
        self.last_internal
    }
    fn set_last_transition_is_internal(&mut self, v: bool) {
        self.last_internal = v;
    }
    fn last_transition_is_targetless(&self) -> bool {
        self.last_targetless
    }
    fn set_last_transition_is_targetless(&mut self, v: bool) {
        self.last_targetless = v;
    }
    fn last_transition_source_state(&self) -> Self::State {
        self.last_source
    }
    fn set_last_transition_source_state(&mut self, s: Self::State) {
        self.last_source = s;
    }

    fn execute_entry_actions(&mut self, _s: Self::State, _eng: &mut Engine<Self>) {}
    fn execute_exit_actions(
        &mut self,
        _s: Self::State,
        _eng: &mut Engine<Self>,
        _pre: &[Self::State],
    ) {
    }
    fn process_transition(
        &mut self,
        _cur: &mut Self::State,
        _e: Self::Event,
        _eng: &mut Engine<Self>,
    ) -> bool {
        false
    }
    fn execute_transition_actions(&mut self, _eng: &mut Engine<Self>) {}
}

// ─────────────────────────────────────────────────────────────
// Regression: schedule_event / has_ready_events route through Hal
// ─────────────────────────────────────────────────────────────

#[test]
fn scheduler_consults_hal_under_std() {
    // Exclusive ownership of the process-global mock clock for this test's
    // duration — no parallel test may mutate `MOCK_TICK_MS` between the
    // anchor below and the assertions.
    let _clock = lock_clock();

    // Anchor mock clock at a known epoch so the test is independent of any
    // prior mutation.
    mock_set_ticks(1_000_000);

    let mut engine = Engine::<MockPolicy>::new(MockPolicy::new());
    engine.initialize();

    // (1) Schedule a 5s delayed event. ready_at = now_ticks_ms() + 5000.
    let _send_id = engine.schedule_event(Ev::Delayed, Duration::from_secs(5), "sid1", "");

    // (2) Clock still at 1_000_000 → not ready.
    assert!(
        !engine.has_ready_events(),
        "scheduler must not fire before Hal-reported clock crosses ready_at \
         (regression: pre-fix std path read Instant::now() directly, which would \
         have started at a fresh wall-clock anchor and never compared against \
         the synthetic ready_at)"
    );

    // (3) Advance mock to one tick past ready_at → must be ready. If sched_now
    // regresses to Instant::now(), this assertion fails because real wall-clock
    // milliseconds since process start are nowhere near 1_005_001.
    mock_set_ticks(1_005_001);
    assert!(
        engine.has_ready_events(),
        "scheduler must observe Hal clock advancing past ready_at \
         (regression: pre-fix std path was Hal-blind)"
    );
}

#[test]
fn scheduler_resolution_is_milliseconds() {
    // Serialize against the other clock-driving test (see `CLOCK_LOCK`).
    let _clock = lock_clock();

    // Sub-ms Duration must truncate to 0ms — same fire-immediately semantics
    // as Duration::ZERO. Documents the resolution contract; if a future
    // refactor introduces sub-ms scheduling support, this test should be
    // updated alongside the spec change rather than silently rounded.
    mock_set_ticks(2_000_000);

    let mut engine = Engine::<MockPolicy>::new(MockPolicy::new());
    engine.initialize();

    let _ = engine.schedule_event(Ev::Delayed, Duration::from_micros(500), "sid2", "");

    // ready_at = 2_000_000 + (500us as ms = 0) = 2_000_000, i.e. now → ready immediately.
    assert!(
        engine.has_ready_events(),
        "sub-ms delay truncates to 0ms; event is ready at current Hal tick"
    );
}
