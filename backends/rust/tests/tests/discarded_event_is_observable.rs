// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.1.2: "If no transition matches in any state, the event is
// discarded" — and the host that fed it in can find out. Rust AOT path.
//
// Discarding is the clause; staying silent about it is not part of the clause.
// Three outcomes leave the configuration identical, so no accessor that existed
// before this fixture separates them:
//
//   poke    self transition       handled (exits and re-enters `idle`)
//   nudge   targetless internal   handled (actions only, no exit/entry)
//   settle  no matching           DISCARDED — the host's event went nowhere
//
// The C++ Interpreter in this repository answers all three:
// `StateMachine::processEvent` returns a `TransitionResult` whose `success` is
// false for the last one, and `getStatistics().failedTransitions` counts them.
// The six generated backends computed the same fact at the same point of
// Appendix D's `mainEventLoop` and dropped it, so a document that moved from
// the Interpreter to a generated engine lost a signal its host was reading.
//
// `nudge` is in the fixture because the engines' own "did anything happen" bool
// is a different fact: it reports whether the configuration changed, and a
// targetless internal transition answers false after running its actions. A
// count keyed off that bool would call a handled event discarded.
//
// Fixture: integration_resources/discarded_event_is_observable/discarded_event_is_observable.scxml
// (canonical, shared with the C++ / C11 / Go / Kotlin / Python channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_discarded_event_is_observable.sh

use std::sync::Arc;

use sce_rust_runtime::{Engine, IScriptEngine};
use sce_rust_tests::integration::discarded_event_is_observable::{
    DiscardedEventIsObservableEvent as Event, DiscardedEventIsObservablePolicy as Policy,
    DiscardedEventIsObservableState as State,
};

fn started() -> (Engine<Policy>, Arc<dyn IScriptEngine>) {
    let script_engine: Arc<dyn IScriptEngine> = Arc::new(sce_rust_lua::LuaEngine::new());
    let mut engine = Engine::new(Policy::new(Arc::clone(&script_engine)));
    engine.initialize();
    (engine, script_engine)
}

/// The fixture's `<assign>`s are the only witness that a handled event ran
/// anything at all — without them a passing run could not be told apart from a
/// run where nothing fired.
fn counter(engine: &Engine<Policy>, script_engine: &Arc<dyn IScriptEngine>, name: &str) -> i64 {
    sce_rust_runtime::helpers::datamodel_read::read_int(
        &**script_engine,
        engine.policy().session_id.as_deref(),
        name,
    )
    .unwrap_or_else(|| panic!("the fixture declares `{name}` in its datamodel"))
}

/// The axis: an event the machine knows but no active state answers is counted.
#[test]
fn an_event_no_active_state_answered_is_counted() {
    let (mut engine, _se) = started();
    assert_eq!(
        engine.discarded_external_events(),
        0,
        "nothing has been discarded before the first event"
    );

    // `settle` is declared in `busy`, so it is in the machine's vocabulary and
    // the host can name it — it just matches nothing while the machine is in
    // `idle`. That is the host-side wiring mistake this count exists for.
    engine.raise_external(Event::Settle, "", "");
    engine.step();

    assert_eq!(
        engine.discarded_external_events(),
        1,
        "`settle` came off the external queue in `idle`, where no transition \
         matches it. W3C SCXML 3.1.2 discards it; the host that queued it has \
         no other way to learn its event went nowhere"
    );
    assert_eq!(
        engine.get_current_state(),
        State::Idle,
        "a discarded event must not move the machine"
    );
}

/// The other half: a handled event must NOT be counted, including the one that
/// changes nothing. A count that is always non-zero is as useless as one that
/// is always zero.
#[test]
fn a_handled_event_is_not_counted() {
    let (mut engine, se) = started();

    engine.raise_external(Event::Poke, "", "");
    engine.step();
    assert_eq!(
        counter(&engine, &se, "pokes"),
        1,
        "`poke`'s self transition did not run, so nothing below is measuring a \
         handled event"
    );
    assert_eq!(
        engine.discarded_external_events(),
        0,
        "`poke` matched a self transition — handled, and the configuration is \
         unchanged only because the transition returns to its own source"
    );

    engine.raise_external(Event::Nudge, "", "");
    engine.step();
    assert_eq!(
        counter(&engine, &se, "nudges"),
        1,
        "`nudge`'s targetless transition did not run"
    );
    assert_eq!(
        engine.discarded_external_events(),
        0,
        "`nudge` matched a targetless internal transition: its actions ran and \
         no state was exited or entered. The engine's own configuration-changed \
         bool is false here, which is why the count cannot be keyed off it"
    );
}

/// Why the query has to exist at all: every pre-existing accessor answers the
/// same for a handled event and a discarded one.
#[test]
fn the_discard_is_not_derivable_from_any_other_accessor() {
    let (mut engine, _se) = started();

    engine.raise_external(Event::Poke, "", "");
    engine.step();
    let handled = (
        engine.get_current_state(),
        engine.get_active_states().to_vec(),
        engine.is_running(),
        engine.is_in_final_state(),
    );

    engine.raise_external(Event::Settle, "", "");
    engine.step();
    let discarded = (
        engine.get_current_state(),
        engine.get_active_states().to_vec(),
        engine.is_running(),
        engine.is_in_final_state(),
    );

    assert_eq!(
        handled, discarded,
        "this fixture exists because these two are indistinguishable through \
         the accessors a host had; if they ever differ, the fixture stopped \
         measuring what it claims"
    );
    assert_eq!(
        engine.discarded_external_events(),
        1,
        "the two are indistinguishable through every other accessor, so the \
         count is the only thing that separates them"
    );
}

/// A count says something went nowhere; a host debugging a stalled supervisor
/// needs to know which event did.
#[test]
fn the_engine_names_the_event_it_discarded() {
    let (mut engine, _se) = started();
    assert_eq!(
        engine.last_discarded_event(),
        None,
        "nothing has been discarded yet"
    );

    engine.raise_external(Event::Settle, "", "");
    engine.step();

    assert_eq!(
        engine.last_discarded_event(),
        Some(Event::Settle),
        "the engine counted a discard but cannot say which event it was"
    );
}

/// The supervisor's actual failure mode: the machine moved on, and the events
/// the host keeps sending no longer match anything.
#[test]
fn an_event_the_machine_has_moved_past_is_counted() {
    let (mut engine, _se) = started();
    engine.raise_external(Event::Go, "", "");
    engine.step();
    assert_eq!(
        engine.get_current_state(),
        State::Busy,
        "`go` should have moved the machine out of `idle`"
    );

    // `poke` is `idle`'s vocabulary; in `busy` it answers nothing.
    engine.raise_external(Event::Poke, "", "");
    engine.step();
    assert_eq!(
        engine.discarded_external_events(),
        1,
        "the machine left `idle`, so `poke` no longer matches — the host that \
         kept sending it is exactly who the count is for"
    );
    assert_eq!(engine.last_discarded_event(), Some(Event::Poke));
}
