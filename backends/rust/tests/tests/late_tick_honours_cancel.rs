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

use std::time::{Duration, Instant};

use sce_rust_runtime::{Engine, StatePolicy};
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
