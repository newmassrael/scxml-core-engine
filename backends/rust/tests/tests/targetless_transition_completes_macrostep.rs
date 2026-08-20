// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML Appendix D's main event loop returns to
// `selectEventlessTransitions()` after every microstep, and drains the
// internal queue in the same inner loop. It never asks whether the microstep
// it just took moved the machine — it cannot, because W3C SCXML 3.13 defines a
// transition with no `target` as one that exits and enters nothing and runs
// its content in place. Rust AOT path.
//
// Measured 2026-08-20, the two C++ engines end the macrostep at such a
// transition: whatever its content enabled is never walked, and the host is
// handed a configuration the clause says is not stable. This channel is the
// side of that comparison that was already right, and it is here so the
// contract is stated for every backend rather than only for the ones that
// broke it.
//
// `eventless_macrostep_is_bounded` owns how FAR a chain may run; this one owns
// whether the chain is entered at all.
//
// Fixture: integration_resources/targetless_transition_completes_macrostep/targetless_transition_completes_macrostep.scxml
// (canonical, shared with the C++ / C11 / Go / Kotlin / Python channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_targetless_transition_completes_macrostep.sh

use std::sync::Arc;

use sce_rust_runtime::{Engine, IScriptEngine};
use sce_rust_tests::integration::targetless_transition_completes_macrostep::{
    TargetlessTransitionCompletesMacrostepEvent as Event,
    TargetlessTransitionCompletesMacrostepPolicy as Policy,
    TargetlessTransitionCompletesMacrostepState as State,
};

fn started() -> (Engine<Policy>, Arc<dyn IScriptEngine>) {
    let script_engine: Arc<dyn IScriptEngine> = Arc::new(sce_rust_lua::LuaEngine::new());
    let mut engine = Engine::new(Policy::new(Arc::clone(&script_engine)));
    engine.initialize();
    (engine, script_engine)
}

/// The fixture's `<assign>`s are the only witness of how far the macrostep
/// got: every outcome here leaves the machine in a state the configuration
/// alone cannot tell apart from a macrostep that stopped one microstep early.
fn counter(engine: &Engine<Policy>, script_engine: &Arc<dyn IScriptEngine>, name: &str) -> i64 {
    sce_rust_runtime::helpers::datamodel_read::read_int(
        &**script_engine,
        engine.policy().session_id.as_deref(),
        name,
    )
    .unwrap_or_else(|| panic!("the fixture declares `{name}` in its datamodel"))
}

/// The axis: a transition that moves nothing still ends a microstep, so the
/// macrostep continues into whatever its content enabled.
///
/// `chained == 1, polished == 0` is the signature of an engine that resumes
/// the chain only after a transition that MOVED the machine: it takes the link
/// that moves and stops before the link that does not. `chained == 0` is the
/// signature of one that never entered the chain at all. Both are failures of
/// the same clause, and the two counters are what tell them apart.
#[test]
fn a_targetless_transition_does_not_end_the_macrostep() {
    let (mut engine, se) = started();

    engine.raise_external(Event::Arm, "", "");
    engine.step();

    assert_eq!(
        counter(&engine, &se, "armed"),
        1,
        "the targetless transition ran its content — without this the rest \
         measures a lost event rather than a stopped macrostep"
    );
    assert_eq!(
        counter(&engine, &se, "chained"),
        1,
        "and the eventless transition that content enabled was taken in the \
         SAME macrostep, which is the whole of what Appendix D's inner loop \
         promises a host"
    );
    assert_eq!(
        counter(&engine, &se, "polished"),
        1,
        "including the chain's last link, which is targetless itself: an \
         engine that walks the chain only while the machine keeps moving \
         stops exactly here"
    );
    assert_eq!(
        engine.get_current_state(),
        State::Settled,
        "and the host is handed the stable configuration, not the one the \
         machine was passing through"
    );
}

/// The other side of the same inner loop: what a targetless transition raises
/// is answered before the host gets control back.
///
/// Appendix D dequeues the internal queue in the same loop that selects
/// eventless transitions, so an engine that returns at the targetless
/// microstep strands the raise there. `answered == 0` with the machine
/// running and in the state the document names is exactly that.
#[test]
fn a_raise_from_a_targetless_transition_is_answered_in_the_same_macrostep() {
    let (mut engine, se) = started();

    engine.raise_external(Event::Ping, "", "");
    engine.step();

    assert_eq!(
        counter(&engine, &se, "answered"),
        1,
        "the internal event the targetless transition raised was dequeued and \
         matched inside this macrostep"
    );
    assert_eq!(
        engine.get_current_state(),
        State::Idle,
        "neither transition moves the machine, which is the point: the \
         macrostep has to continue anyway"
    );
}

/// The control, and the reason a zero above means anything: a targetless
/// transition that enables nothing leaves the machine exactly where it was,
/// and having run is still observable.
///
/// Without this, an engine that dropped every targetless transition on the
/// floor would fail the two tests above with the same numbers as one that
/// took them and stopped early.
#[test]
fn a_targetless_transition_that_enables_nothing_changes_nothing_else() {
    let (mut engine, se) = started();

    engine.raise_external(Event::Quiet, "", "");
    engine.step();

    assert_eq!(counter(&engine, &se, "quiet"), 1, "the transition fired");
    assert_eq!(
        counter(&engine, &se, "chained"),
        0,
        "and nothing else did: the eventless transition's guard is still \
         closed, so an engine that walked the chain here would be firing a \
         transition the document did not enable"
    );
    assert_eq!(counter(&engine, &se, "polished"), 0);
    assert_eq!(counter(&engine, &se, "answered"), 0);
    assert_eq!(engine.get_current_state(), State::Idle);
    assert!(engine.is_running());
}

/// The other microstep that ends where it began: a transition whose target is
/// its own source.
///
/// It is not targetless — W3C SCXML 3.13 gives it an exit and an entry — but a
/// macrostep loop that continues only while the configuration keeps changing
/// drops it for the same reason and, in the C++ AOT engine, in the same line of
/// code. `entries == 1` is that engine: the transition was selected, nothing
/// ran, and the chain ended.
#[test]
fn an_eventless_self_transition_exits_and_re_enters() {
    let (mut engine, se) = started();

    engine.raise_external(Event::Recycle, "", "");
    engine.step();

    assert_eq!(
        counter(&engine, &se, "entries"),
        2,
        "the state is entered once by `recycle` and once more by the eventless \
         self transition its entry enabled — a self transition exits and \
         re-enters, so `<onentry>` runs again"
    );
    assert_eq!(
        engine.get_current_state(),
        State::Recycled,
        "and the guard closes behind it, so the machine rests here rather than \
         spinning"
    );
}

/// A macrostep, not a one-shot: the second targetless transition is followed
/// the same way the first was.
///
/// An engine that ran the inner loop once per machine — or that latched some
/// "already settled" flag on the way out — passes the tests above and fails
/// this one.
#[test]
fn the_second_targetless_transition_is_followed_too() {
    let (mut engine, se) = started();

    engine.raise_external(Event::Quiet, "", "");
    engine.step();
    engine.raise_external(Event::Ping, "", "");
    engine.step();
    assert_eq!(counter(&engine, &se, "answered"), 1);

    engine.raise_external(Event::Ping, "", "");
    engine.step();

    assert_eq!(
        counter(&engine, &se, "answered"),
        2,
        "the raise in the third macrostep was answered like the one in the \
         second — the inner loop belongs to every macrostep, not to the first"
    );
    assert_eq!(counter(&engine, &se, "quiet"), 1);
    assert_eq!(engine.get_current_state(), State::Idle);
}
