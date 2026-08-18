// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Which entry point a machine needs, and what happens when a host picks the
// other one.
//
// `Engine::step` runs a macrostep and never consults the delayed-send
// scheduler; `Engine::tick` drains the scheduler, ticks invoked children, and
// then runs the macrostep. A host driving a delayed-send machine with `step`
// alone therefore waits forever — the event is neither delivered nor refused,
// and before `StatePolicy::NEEDS_EVENT_SCHEDULER` a `build.rs` consumer had no
// route to the knowledge that it had picked wrong: `sce_build::compile_scxml`
// returns `()`, and the generate manifest that carries `needs_event_scheduler`
// only reaches CLI callers.
//
// That is not hypothetical. A downstream consumer of the build.rs route drives
// a machine whose manifest says `needs_event_scheduler: true` with eight
// `step()` calls and no `tick()`.
//
// Fixtures are W3C IRP documents already in the committed tree:
//   test175 — `<send delay>` / `<send delayexpr>` on entry, so *every*
//             transition out of `s0` depends on the scheduler.
//   test144 — no delayed send at all, the control for the counter.

use sce_rust_runtime::Engine;
use sce_rust_tests::generated::test144::Test144Policy;
use sce_rust_tests::generated::test175::{Test175Policy, Test175State};

fn scheduler_driven() -> Engine<Test175Policy> {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let mut e = Engine::new(Test175Policy::new(script_engine));
    e.initialize();
    e
}

/// The requirement is on the policy, where a consumer can read it without
/// running anything — the half of the contract `compile_scxml` could not
/// deliver. Checked in both directions so the constant cannot degenerate into
/// a value that is always `true` (which would be as useless as always `false`,
/// while looking like it works on the machine that needs it).
///
/// A `const` block rather than a `#[test]`: the value is compile-time, so a
/// regression should be a build failure and not a run that has to be launched
/// before anyone hears about it.
const _: () = {
    // test175 sends `event1` and `event2` with a delay; nothing leaves `s0`
    // without the scheduler.
    assert!(<Test175Policy as sce_rust_runtime::StatePolicy>::NEEDS_EVENT_SCHEDULER);
    // test144 has no delayed send and no invoked child — a `step()` loop drives
    // it completely.
    assert!(!<Test144Policy as sce_rust_runtime::StatePolicy>::NEEDS_EVENT_SCHEDULER);
};

/// The silent failure itself: a scheduler-driven machine stepped as many times
/// as a host cares to, still sitting where it started.
#[test]
fn stepping_a_delayed_send_machine_delivers_nothing() {
    let mut e = scheduler_driven();
    assert_eq!(e.get_current_state(), Test175State::S0);

    // Far more macrosteps than the document needs, and long enough for both
    // delays (0.5s and 1s) to have come due on any clock.
    for _ in 0..64 {
        e.step();
    }
    std::thread::sleep(std::time::Duration::from_millis(1200));
    for _ in 0..64 {
        e.step();
    }

    assert_eq!(
        e.get_current_state(),
        Test175State::S0,
        "`step` cannot reach the scheduler, so neither delayed event is ever delivered",
    );
}

/// …and the engine counts it, so the mistake is something a program can see
/// rather than a run that merely takes forever.
#[test]
fn the_engine_counts_the_macrosteps_that_no_tick_attended() {
    let mut e = scheduler_driven();
    assert_eq!(
        e.unattended_scheduler_steps(),
        0,
        "nothing has been stepped yet",
    );

    e.step();
    e.step();
    e.step();

    assert_eq!(
        e.unattended_scheduler_steps(),
        3,
        "every macrostep taken before a `tick` on a scheduler-driven machine counts",
    );
}

/// A host that owns a clock is not accused of anything. Once `tick` has run the
/// counter stops, because mixing the two calls is a legitimate driving loop —
/// `tick` is a superset of `step`, not its rival.
#[test]
fn a_host_that_ticks_stops_being_counted() {
    let mut e = scheduler_driven();
    e.step();
    assert_eq!(e.unattended_scheduler_steps(), 1);

    e.tick();
    for _ in 0..10 {
        e.step();
    }

    assert_eq!(
        e.unattended_scheduler_steps(),
        1,
        "the count is of macrosteps taken before any tick, so it freezes at the first one",
    );
}

/// The control. Without it, a counter that incremented on every `step` of every
/// machine would pass the assertions above while saying nothing.
#[test]
fn a_machine_with_no_delayed_send_is_never_counted() {
    let mut e = Engine::new(Test144Policy::new());
    e.initialize();

    for _ in 0..32 {
        e.step();
    }

    assert_eq!(
        e.unattended_scheduler_steps(),
        0,
        "test144 needs no scheduler, so a `step` loop is the right way to drive it",
    );
}

/// The other side of the verdict: the same document, driven with the entry
/// point its policy asks for, reaches the state the W3C document calls pass.
/// Without this, the assertions above would be satisfied by an engine that
/// simply never delivers delayed events to anyone.
#[test]
fn ticking_the_same_machine_delivers_both_delayed_events() {
    let mut e = scheduler_driven();

    let completed = e.run_until_completion(
        std::time::Duration::from_secs(5),
        std::time::Duration::from_millis(10),
    );

    assert!(completed, "the delayed events must arrive and end the run");
    assert_eq!(
        e.get_current_state(),
        Test175State::Pass,
        "`event1` (0.5s) before `event2` (1s) is what test175 asserts, and both need the scheduler",
    );
    assert_eq!(
        e.unattended_scheduler_steps(),
        0,
        "`run_until_completion` drives with `tick`, so nothing went unattended",
    );
}
