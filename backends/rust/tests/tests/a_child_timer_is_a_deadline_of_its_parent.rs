// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A child session's `<send delay>` is a deadline of the machine that invoked
// it — Rust AOT path.
//
// A host that drives a scheduler-owning machine asks the engine when it next
// needs a tick and sleeps that long. The parent's tick advances every running
// child by the same step, so a moment a child needs is a moment the host must
// not step over. An answer that counts only the parent's own scheduler tells a
// host with nothing of the parent's armed that there is nothing to wait for,
// and the child's timer is never fired.
//
// Driven entirely on `SceClock::Manual`: no case sleeps, and each move lands
// exactly on the deadline the engine reported, which is the use the answer
// exists for.
//
// Fixture: tests/integration/a_child_timer_is_a_deadline_of_its_parent.scxml.
// It is outside `integration_resources/` for the reason
// `scripts/regen_a_child_timer_is_a_deadline_of_its_parent.sh` states.
//
// Regeneration (after fixture or template edit):
//   scripts/regen_a_child_timer_is_a_deadline_of_its_parent.sh

use sce_rust_runtime::{Engine, SceClock, StatePolicy};
use sce_rust_tests::integration::a_child_timer_is_a_deadline_of_its_parent::{
    AChildTimerIsADeadlineOfItsParentPolicy as Policy,
    AChildTimerIsADeadlineOfItsParentState as State,
};

/// Both assertions below are about a machine the host drives by asking when to
/// tick; a policy that did not need the scheduler would have nothing to ask.
const _: () = {
    assert!(<Policy as StatePolicy>::NEEDS_EVENT_SCHEDULER);
};

fn started() -> Engine<Policy> {
    let mut engine = Engine::new(Policy::new());
    engine.set_clock(SceClock::Manual(0));
    engine.initialize();
    engine
}

/// The parent arms nothing, so the only deadline in the run is the child's
/// first timer, 200 ms after the child started.
#[test]
fn the_engine_names_its_childs_first_timer_as_its_own_next_deadline() {
    let engine = started();
    assert_eq!(
        engine.get_current_state(),
        State::Waiting,
        "the parent should be waiting on its child"
    );
    assert_eq!(
        engine.time_until_next_scheduled_ms(),
        Some(200),
        "the child armed `<send delay=\"200ms\">` when it started and the parent \
         ticks the child, so that deadline is the parent's. An answer of `None` \
         tells a host there is nothing to wait for while a running child's timer \
         is pending"
    );
}

/// The child arms its second timer when the first fires, so the whole run is
/// two moves of 200 ms and the engine has to name the second one only after
/// the first has been taken.
#[test]
fn a_host_walking_time_by_the_answer_reaches_the_end_of_the_child() {
    let mut engine = started();
    let mut walked = Vec::new();
    while let Some(ms) = engine.time_until_next_scheduled_ms() {
        assert!(
            walked.len() < 8,
            "the engine keeps naming deadlines: {walked:?}"
        );
        walked.push(ms);
        engine.advance_time_ms(ms);
    }
    assert_eq!(
        walked,
        vec![200, 200],
        "each move should land on the child's next timer, the second of which \
         is armed by the first firing"
    );
    assert_eq!(
        engine.terminal_state(),
        Some(State::Finished),
        "the child's last timer ended it, so the parent should have taken \
         `done.invoke.kid` and finished"
    );
    assert_eq!(
        engine.time_until_next_scheduled_ms(),
        None,
        "nothing is armed once the child has ended"
    );
}
