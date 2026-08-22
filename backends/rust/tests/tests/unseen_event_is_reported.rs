// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.13 + Appendix D: an event handed to a machine that has already
// stopped is never looked at, and the host that sent it can find out. Rust AOT.
//
// Appendix D's main event loop exits when the machine reaches a top-level final
// state. Refusing what arrives afterwards is the clause; saying nothing about
// it is not. And the silence is expensive precisely because it looks like the
// two outcomes a host CAN already read:
//
//   dequeued, no transition matched                 discarded_external_events
//   dequeued, matched, guard said no                nothing, correctly
//   never dequeued — the machine had stopped        this
//
// All three leave the configuration alone. Measured 2026-08-22: a consumer
// reported a guarded transition that "never fires" and rewrote the guard four
// times, ending with a trivially true arithmetic one. The same document driven
// here fired it first try, at that consumer's own pinned revision — so the
// guard was never the difference, and nothing in this engine could say what
// was.
//
// Fixture: integration_resources/unseen_event_is_reported/unseen_event_is_reported.scxml
// (canonical, shared with the C++ / C11 / Go / Kotlin / Python channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_unseen_event_is_reported.sh

use std::sync::Arc;

use sce_rust_runtime::{Engine, IScriptEngine};
use sce_rust_tests::integration::unseen_event_is_reported::{
    UnseenEventIsReportedEvent as Event, UnseenEventIsReportedPolicy as Policy,
};

fn started() -> (Engine<Policy>, Arc<dyn IScriptEngine>) {
    let script_engine: Arc<dyn IScriptEngine> = Arc::new(sce_rust_lua::LuaEngine::new());
    let mut engine = Engine::new(Policy::new(Arc::clone(&script_engine)));
    engine.initialize();
    (engine, script_engine)
}

/// The fixture's `<assign>` is the only witness that a delivery ran anything at
/// all — without it a passing run cannot be told apart from a run in which no
/// transition fired.
fn pokes(engine: &Engine<Policy>, script_engine: &Arc<dyn IScriptEngine>) -> i64 {
    sce_rust_runtime::helpers::datamodel_read::read_int(
        &**script_engine,
        engine.policy().session_id.as_deref(),
        "pokes",
    )
    .expect("the fixture declares `pokes` in its datamodel")
}

fn deliver(engine: &mut Engine<Policy>, event: Event) {
    engine.raise_external(event, "", "");
    engine.step();
}

/// The axis: an event the host queued after the machine stopped is counted.
#[test]
fn an_event_delivered_after_the_machine_stopped_is_counted() {
    let (mut engine, se) = started();
    assert_eq!(
        engine.unseen_external_events(),
        0,
        "nothing has been refused before the first event"
    );

    deliver(&mut engine, Event::Poke);
    assert_eq!(
        pokes(&engine, &se),
        1,
        "`poke`'s transition did not run, so nothing below is measuring a \
         machine that was working first"
    );

    deliver(&mut engine, Event::Finish);
    assert!(
        engine.is_in_final_state(),
        "`finish` should have taken the machine to its top-level final state"
    );
    assert_eq!(
        engine.unseen_external_events(),
        0,
        "`finish` was itself dequeued and handled — the machine stopped BECAUSE \
         of it, which is not the same as stopping before it"
    );

    // The delivery the axis is about.
    deliver(&mut engine, Event::Poke);

    assert_eq!(
        engine.unseen_external_events(),
        1,
        "the host queued `poke` on a machine that had reached its final state. \
         W3C SCXML Appendix D's loop had already ended, so the event was never \
         dequeued; before this count the host had no way to learn that"
    );
    assert_eq!(
        pokes(&engine, &se),
        1,
        "the refused delivery ran the document's transition anyway — the count \
         would then be reporting something that did not happen"
    );
}

/// A machine can stop two different ways, and each is refused at a different
/// door. Both have to answer, and a fix applied to one leaves a host that
/// stopped the machine the other way still unable to tell.
///
/// Reaching a top-level final state does NOT clear `is_running` — the main
/// event loop simply stops draining, so what the host queued is abandoned
/// there. `stop()` is the other way, and its own doc says the calls that
/// follow "become no-ops": `process_event` returns before anything is queued,
/// so the loop never sees the delivery at all.
///
/// Measured 2026-08-23: a round with only the final-state assertions above
/// left a mutation that deletes the door-side record CAUGHT by nothing — 0 of
/// 5 tests red — because every one of them stopped the machine the other way.
#[test]
fn a_machine_stopped_by_its_host_refuses_at_the_other_door() {
    let (mut engine, se) = started();
    deliver(&mut engine, Event::Poke);
    assert_eq!(pokes(&engine, &se), 1, "the machine was working first");

    engine.stop();
    assert!(
        !engine.is_running(),
        "`stop()` should have halted the engine"
    );
    assert!(
        !engine.is_in_final_state(),
        "and it halted the machine WITHOUT a final state, which is the whole \
         point of this test: the other assertions here reach a final state \
         instead, and that leaves `is_running` true"
    );
    assert_eq!(
        engine.unseen_external_events(),
        0,
        "stopping is not itself a refused event"
    );

    engine.process_event(Event::Poke);

    assert_eq!(
        engine.unseen_external_events(),
        1,
        "`process_event` returned early because the host had stopped the \
         engine, so the event never reached the queue the main event loop \
         drains. `stop()`'s own doc calls the calls that follow it no-ops — \
         and a no-op nobody can count is the silence this axis is about"
    );
    assert_eq!(
        engine.last_unseen_event(),
        Some(Event::Poke),
        "the door has to name what it refused, exactly as the loop does"
    );
    assert_eq!(
        pokes(&engine, &se),
        1,
        "the refused delivery ran the document's transition anyway"
    );
}

/// Why the query has to exist at all: every other accessor answers the same
/// before and after the refused delivery.
#[test]
fn the_refusal_is_not_derivable_from_any_other_accessor() {
    let (mut engine, se) = started();
    deliver(&mut engine, Event::Finish);

    let before = (
        engine.get_current_state(),
        engine.get_active_states().to_vec(),
        engine.is_running(),
        engine.is_in_final_state(),
        engine.discarded_external_events(),
        pokes(&engine, &se),
    );

    deliver(&mut engine, Event::Poke);

    let after = (
        engine.get_current_state(),
        engine.get_active_states().to_vec(),
        engine.is_running(),
        engine.is_in_final_state(),
        engine.discarded_external_events(),
        pokes(&engine, &se),
    );

    assert_eq!(
        before, after,
        "this fixture exists because a refused delivery is indistinguishable \
         through the accessors a host had; if they ever differ, the fixture \
         stopped measuring what it claims"
    );
    assert_eq!(
        engine.unseen_external_events(),
        1,
        "the two readings agree on everything else, so this count is the only \
         thing that separates `the machine never looked` from `it looked and \
         nothing matched`"
    );
}

/// The distinction the whole axis turns on: a discard and a refusal are
/// different facts, and each has its own count.
#[test]
fn a_discard_and_a_refusal_are_counted_separately() {
    let (mut engine, _se) = started();

    // `finish` is in the machine's vocabulary and `working` answers it, so to
    // get a discard the machine has to be running and unable to match. The
    // fixture has no such event by design — its axis is the refusal — so the
    // discard side is asserted as the zero it must stay at.
    deliver(&mut engine, Event::Poke);
    assert_eq!(
        engine.discarded_external_events(),
        0,
        "`poke` matched a targetless transition; nothing was discarded"
    );
    assert_eq!(
        engine.unseen_external_events(),
        0,
        "and the machine was running, so nothing was refused either"
    );

    deliver(&mut engine, Event::Finish);
    deliver(&mut engine, Event::Poke);

    assert_eq!(
        (
            engine.discarded_external_events(),
            engine.unseen_external_events()
        ),
        (0, 1),
        "a refusal must not be reported as a discard: the first says the \
         machine looked and nothing matched, the second says it never looked, \
         and a host acts differently on each"
    );
}

/// A count says an event went unlooked-at; a host debugging a supervisor that
/// stopped answering needs to know which one.
#[test]
fn the_engine_names_the_event_it_never_looked_at() {
    let (mut engine, _se) = started();
    assert_eq!(
        engine.last_unseen_event(),
        None,
        "nothing has been refused yet"
    );

    deliver(&mut engine, Event::Finish);
    deliver(&mut engine, Event::Poke);
    assert_eq!(
        engine.last_unseen_event(),
        Some(Event::Poke),
        "the engine counted a refusal but cannot say which event it refused"
    );

    // A second refusal under the other name: the record has to track the last
    // event THAT WAS REFUSED, which here is every later one — and the count
    // has to accumulate rather than latch.
    deliver(&mut engine, Event::Finish);
    assert_eq!(
        (engine.unseen_external_events(), engine.last_unseen_event()),
        (2, Some(Event::Finish)),
        "the count is a count, not a flag, and the name follows the refusals"
    );
}
