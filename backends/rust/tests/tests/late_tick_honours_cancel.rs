// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.2 + 6.3: a `<cancel>` still lands when the host ticked late —
// Rust AOT path.
//
// The scheduler queue is ordered by fire time and `Engine::tick` drains it.
// Draining it to exhaustion before running a macrostep is the defect: a host
// that wakes after two fire times have passed holds both entries, and putting
// both on the external queue makes the second undroppable before the first
// one's transitions have run. The `<cancel>` then executes against a queue the
// event has already left.
//
// The host below sleeps past BOTH fire times before its first tick, because
// that is the only condition under which the two dispatch orders differ. A
// host that wakes between them passes either way, which is why every existing
// suite was blind to this.
//
// Fixture: integration_resources/late_tick_honours_cancel/late_tick_honours_cancel.scxml
// (canonical, shared with the C++ / C11 / Go / Kotlin / Python channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_late_tick_honours_cancel.sh

use std::cell::Cell;
use std::time::{Duration, Instant};

use sce_rust_runtime::{Engine, SceClock, StatePolicy};
use sce_rust_tests::integration::late_tick_honours_cancel::{
    LateTickHonoursCancelPolicy, LateTickHonoursCancelState,
};

/// Long enough that both `<send delay>`s in `waiting` (100 ms and 200 ms) are
/// past due when the first tick runs, with margin for a loaded machine.
const PAST_BOTH_DEADLINES: Duration = Duration::from_millis(400);

fn started() -> Engine<LateTickHonoursCancelPolicy> {
    let mut engine = Engine::new(LateTickHonoursCancelPolicy::new());
    engine.initialize();
    engine
}

/// The fixture is only meaningful on a scheduler-driven machine, and the policy
/// is where a consumer reads that without running anything.
///
/// A `const` block rather than a `#[test]`, matching the sibling in
/// `delayed_sends_need_tick_not_step.rs`: the value is compile-time, so a
/// regression should be a build failure rather than a run someone has to
/// launch. The fixture arming two delayed `<send>`s is the precondition for
/// every assertion below — without it they would be measuring a machine that
/// lost them.
const _: () = {
    assert!(<LateTickHonoursCancelPolicy as StatePolicy>::NEEDS_EVENT_SCHEDULER);
};

/// The axis: one tick, taken after both deadlines passed, must still deliver
/// `poke` first and let `active`'s `<cancel sendid="s1">` drop `settle`.
#[test]
fn a_cancel_survives_a_tick_that_arrives_after_both_deadlines() {
    let mut engine = started();
    assert_eq!(
        engine.get_current_state(),
        LateTickHonoursCancelState::Waiting,
        "the machine should be waiting on its two delayed sends"
    );

    std::thread::sleep(PAST_BOTH_DEADLINES);
    engine.tick();

    assert_ne!(
        engine.get_current_state(),
        LateTickHonoursCancelState::CancelLost,
        "`settle` was delivered even though `active`'s `<cancel sendid=\"s1\">` ran \
         first. Both entries were past due when this tick started, so the scheduler \
         drain put them on the external queue together and the cancel found nothing \
         left to drop. W3C SCXML 6.3 cancels a send that has not been dispatched — \
         dispatch is one entry per macrostep, not one queue-flush per tick"
    );

    // The verdict is itself scheduler-driven, so a channel whose tick loop
    // stopped working fails here rather than passing by never moving.
    let start = Instant::now();
    while !engine.is_in_final_state() && start.elapsed() < Duration::from_secs(2) {
        std::thread::sleep(Duration::from_millis(20));
        engine.tick();
    }
    assert_eq!(
        engine.get_current_state(),
        LateTickHonoursCancelState::Pass,
        "the machine did not reach `pass` after the cancel; it is in {:?}",
        engine.get_current_state()
    );
}

/// A host that wakes between the two deadlines is the easy case, and it must
/// keep working — the fix is about the late wake-up, not about changing what a
/// punctual one does.
#[test]
fn a_punctual_host_reaches_the_same_verdict() {
    let mut engine = started();
    let start = Instant::now();
    while !engine.is_in_final_state() && start.elapsed() < Duration::from_secs(2) {
        std::thread::sleep(Duration::from_millis(10));
        engine.tick();
    }
    assert_eq!(
        engine.get_current_state(),
        LateTickHonoursCancelState::Pass,
        "a 10 ms tick loop, which wakes between the 100 ms and 200 ms deadlines, \
         must reach `pass`"
    );
}

/// The deadline the host would have to guess is one the engine can state.
/// `run_until_completion` uses it, so an interval far coarser than the
/// document's delays no longer decides the outcome.
#[test]
fn the_engine_says_when_it_is_next_due() {
    let mut engine = started();

    let due = engine
        .time_until_next_scheduled_ms()
        .expect("two delayed sends are armed, so a deadline is owed");
    assert!(
        due <= 100,
        "the nearer of the two armed sends is 100 ms out; the engine answered {due} ms, \
         which would send a host past the earlier deadline"
    );
    // The lower bound is the half that catches an answer of "due now", which
    // reads as a working query and costs the caller a spin that never sleeps —
    // on the no_std profile, a core that never idles.
    assert!(
        due > 0,
        "the nearer send is 100 ms out and nothing is due yet, but the engine answered \
         0 ms. A host sleeping on that answer does not sleep at all"
    );

    // A poll interval coarser than either delay: with the deadline in hand this
    // is a ceiling on the wait, not the wait itself.
    let started_at = Instant::now();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(500));
    let took = started_at.elapsed();
    assert!(completed, "the machine did not complete within 3 s");
    assert_eq!(
        engine.get_current_state(),
        LateTickHonoursCancelState::Pass,
        "a 500 ms poll interval decided the verdict — the wait must be shortened to \
         the scheduler's own next deadline, or a coarse interval silently steps over \
         the deadlines the document distinguishes between"
    );
    // Correctness is not the whole of it: the document's own deadlines are
    // 100 ms + 100 ms, so an engine that sleeps the caller's interval regardless
    // finishes no sooner than 1 s. Timeliness is what the deadline query buys
    // once the dispatch order has made the verdict safe either way.
    assert!(
        took < Duration::from_millis(450),
        "the machine's own deadlines total 200 ms, and it took {took:?} — the poll \
         interval was slept in full rather than shortened to the next deadline, so \
         every delayed event lands as late as the caller's guess"
    );

    assert_eq!(
        engine.time_until_next_scheduled_ms(),
        None,
        "nothing is scheduled once the machine is finished, so no wake-up is owed"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// §scxml-6.2.2 — the clock the deadlines are measured from
//
// Everything above drives the machine on the wall clock, which is what a
// production host does and what the push runs. It is also why this document
// reached a push before it reached a test: the two `<send delay>`s in
// `waiting` were armed against two separate readings, so a host descheduled
// between them by more than the 100 ms separating their delays got the later
// send's deadline first. The cases below take the clock away from the machine
// the suite runs on and hand it to the test, so the verdict is about the
// engine.
// ═══════════════════════════════════════════════════════════════════════════

thread_local! {
    /// `(now_ms, step_ms, readings)` for [`stepping_now`].
    static STEPPING: Cell<(u64, u64, u32)> = const { Cell::new((0, 0, 0)) };
}

/// A clock that jumps forward on every reading.
///
/// This is what a descheduled host looks like from inside the engine: two
/// readings taken for what the document calls one instant come back different.
/// A real one does it unpredictably and only under load, which is why the
/// defect it exposes reached a push before it reached a test; this one does it
/// on every reading, so the cases below are a verdict about the engine rather
/// than about the machine the suite runs on.
fn stepping_now() -> u64 {
    STEPPING.with(|c| {
        let (now, step, readings) = c.get();
        let next = now.saturating_add(step);
        c.set((next, step, readings + 1));
        next
    })
}

/// Arm [`stepping_now`] with a stall of `step_ms` per reading and reset its
/// counter. Thread-local, so parallel test threads do not share it.
fn install_stepping(step_ms: u64) {
    STEPPING.with(|c| c.set((0, step_ms, 0)));
}

/// How many readings [`stepping_now`] has served since [`install_stepping`].
fn stepping_readings() -> u32 {
    STEPPING.with(|c| c.get().2)
}

fn started_on(clock: SceClock) -> Engine<LateTickHonoursCancelPolicy> {
    let mut engine = Engine::new(LateTickHonoursCancelPolicy::new());
    engine.set_clock(clock);
    engine.initialize();
    engine
}

/// The axis of this round: a host descheduled between the fixture's two
/// `<send delay>`s must not change which of them fires first.
///
/// Swept rather than pinned to one value. The threshold is arithmetic — the
/// stall has to reach the 100 ms separating the two delays before the later
/// deadline can overtake the earlier one — and a case pinned at one stall
/// would pass for a fix that moved the threshold instead of removing it.
/// Measured on the pre-latch engine: 1, 50 and 99 pass, and 100 is the first
/// failure.
#[test]
fn a_host_descheduled_between_two_sends_keeps_their_order() {
    for stall_ms in [1_u64, 50, 99, 100, 101, 150, 1000] {
        install_stepping(stall_ms);
        let mut engine = started_on(SceClock::Source(stepping_now));

        assert_ne!(
            engine.get_current_state(),
            LateTickHonoursCancelState::CancelLost,
            "a host stalled {stall_ms} ms between the two <send delay>s of one \
             <onentry> reordered them: `settle` (200 ms) came due before `poke` \
             (100 ms) because each send took its own reading. §scxml-6.2.2 makes a \
             delay the wait the DOCUMENT asks for, and the time the host spent \
             descheduled is not part of it"
        );

        // Drive it to a verdict. One tick is one reading, so time moves
        // `stall_ms` per tick and the smallest stall in the sweep needs a few
        // hundred of them to cross the document's 200 ms of deadlines.
        for _ in 0..4096 {
            if engine.is_in_final_state() {
                break;
            }
            engine.tick();
        }
        assert_eq!(
            engine.get_current_state(),
            LateTickHonoursCancelState::Pass,
            "with a {stall_ms} ms stall per clock reading the machine ended in {:?}; \
             the document's `<cancel sendid=\"s1\">` must still drop `settle`",
            engine.get_current_state()
        );
    }
}

/// A tick dispatches what was due when the host called it — not what its own
/// slowness made due while it ran.
///
/// The engine takes exactly one reading per turn, so a tick that runs several
/// macrosteps cannot chase the deadlines those macrosteps cost it. Counted
/// rather than inferred: the stall here (150 ms) is larger than every delay in
/// the document, so an engine re-reading per pass would run the whole machine
/// inside one tick.
#[test]
fn a_tick_reads_the_clock_once_however_much_it_does() {
    install_stepping(150);
    let mut engine = started_on(SceClock::Source(stepping_now));
    let after_initialize = stepping_readings();
    assert_eq!(
        after_initialize, 1,
        "initialize() is one turn and must take one reading; it took {after_initialize}"
    );

    engine.tick();
    let after_tick = stepping_readings() - after_initialize;
    assert_eq!(
        after_tick, 1,
        "tick() is one turn and must take one reading; it took {after_tick}. A tick \
         that re-reads the clock while it works extends its own window and dispatches \
         entries the host has not yet reached"
    );
}

/// The host-owned clock: the same generated machine, driven by
/// `advance_time_ms`, reaches its verdict on the test's schedule.
///
/// This is the contract the Python channel has had all along (`advance_time` /
/// `now_ms`) and the Kotlin channel gained with the turn latch. A machine
/// driven this way has no dependency on the load of the build machine at all.
#[test]
fn a_manual_clock_drives_the_machine_to_the_same_verdict() {
    let mut engine = started_on(SceClock::Manual(0));
    assert_eq!(
        engine.get_current_state(),
        LateTickHonoursCancelState::Waiting,
        "nothing is due at t=0, so the machine waits on its two delayed sends"
    );

    // Past both deadlines in one move — the late wake-up the fixture is about.
    engine.advance_time_ms(400);
    assert_ne!(
        engine.get_current_state(),
        LateTickHonoursCancelState::CancelLost,
        "a single 400 ms advance stepped over both deadlines; `poke` must still be \
         dispatched first so `active`'s <cancel sendid=\"s1\"> can drop `settle`"
    );

    engine.advance_time_ms(100);
    assert_eq!(
        engine.get_current_state(),
        LateTickHonoursCancelState::Pass,
        "`finish` is armed for 100 ms after `active` is entered, so the machine \
         should be done; it is in {:?}",
        engine.get_current_state()
    );
    assert_eq!(
        engine.now_ms(),
        500,
        "the host moved this clock 400 + 100 ms and nothing else may move it"
    );
}

/// Determinism is the point, so it is asserted as such: the same call sequence
/// twice, and the intermediate states compared rather than only the verdict.
///
/// The wall-clock cases above cannot make this assertion — they would be
/// re-measuring the load on the build machine, which is exactly the dependency
/// this seam removes.
#[test]
fn a_manual_clock_run_repeats_exactly() {
    fn trace() -> Vec<LateTickHonoursCancelState> {
        let mut engine = started_on(SceClock::Manual(0));
        let mut seen = vec![engine.get_current_state()];
        for _ in 0..6 {
            engine.advance_time_ms(100);
            seen.push(engine.get_current_state());
        }
        seen
    }

    let first = trace();
    let second = trace();
    assert_eq!(
        first, second,
        "two identical sequences of advance_time_ms produced different traces; a \
         host-owned clock that is not reproducible is not host-owned"
    );
    assert!(
        first.contains(&LateTickHonoursCancelState::Pass),
        "the trace never reached `pass`: {first:?}"
    );
    assert!(
        !first.contains(&LateTickHonoursCancelState::CancelLost),
        "the trace reached `cancelLost`: {first:?}"
    );
}

/// One generated artifact, two kinds of host.
///
/// The `Hal` seam this backend already had cannot answer this: it is reached
/// through `P::Hal`, so the clock is fixed when the machine is compiled and a
/// host that owns time would need its own policy. Here the same
/// `LateTickHonoursCancelPolicy` runs on the wall clock and on host-owned time
/// and lands in the same configuration.
#[test]
fn one_generated_machine_serves_both_kinds_of_host() {
    let mut wall = started_on(SceClock::Hal);
    let start = Instant::now();
    while !wall.is_in_final_state() && start.elapsed() < Duration::from_secs(2) {
        std::thread::sleep(Duration::from_millis(10));
        wall.tick();
    }

    let mut host_owned = started_on(SceClock::Manual(0));
    for _ in 0..8 {
        if host_owned.is_in_final_state() {
            break;
        }
        host_owned.advance_time_ms(100);
    }

    assert_eq!(
        wall.get_current_state(),
        host_owned.get_current_state(),
        "the same generated machine reached different configurations on the wall \
         clock and on a host-owned clock"
    );
    assert_eq!(
        host_owned.get_current_state(),
        LateTickHonoursCancelState::Pass,
        "both hosts should reach `pass`"
    );
}

/// `advance_time_ms` on a clock the host does not own is a programming error,
/// not a no-op: the caller believes it owns time and it does not, so the events
/// it is waiting for would arrive on a schedule it did not choose.
#[test]
#[should_panic(expected = "needs SceClock::Manual")]
fn advance_time_ms_refuses_a_clock_the_host_does_not_own() {
    let mut engine = started();
    engine.advance_time_ms(100);
}

/// The clock is installed before the machine arms anything against it.
///
/// Swapping it afterwards would leave the scheduler holding deadlines computed
/// from two incomparable time bases — `waiting`'s `<onentry>` has already armed
/// both sends by the time `initialize` returns.
#[test]
#[should_panic(expected = "before initialize()")]
fn the_clock_cannot_be_swapped_after_the_machine_armed_its_deadlines() {
    let mut engine = started();
    engine.set_clock(SceClock::Manual(0));
}

/// A host on the wall clock still gets an absolute reading, so it can
/// correlate the engine's deadlines with its own log.
#[test]
fn now_ms_answers_on_every_kind_of_clock() {
    let wall = started();
    let a = wall.now_ms();
    std::thread::sleep(Duration::from_millis(20));
    let b = wall.now_ms();
    assert!(
        b >= a,
        "the wall clock went backwards between two readings: {a} then {b}"
    );

    let manual = started_on(SceClock::Manual(7));
    assert_eq!(
        manual.now_ms(),
        7,
        "a manual clock reads exactly what the host set, and initialize() must not \
         have moved it"
    );
}
