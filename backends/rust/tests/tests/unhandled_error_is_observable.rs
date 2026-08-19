// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.12.2: the processor MUST signal its own failures by raising
// `error.*` events into the internal queue, and the same paragraph says they
// "are ignored if no transition is found that matches them". Being ignored is
// the clause. Being unable to say it happened is not. Rust AOT path.
//
// `discarded_event_is_observable` asked this for the EXTERNAL queue and stopped
// at its edge on the stated ground that an unmatched internal event "is the
// document's own business, and both ends of it are in the document". That is
// exactly right for an author's `<raise>` and exactly wrong for an error event,
// whose sender is the ENGINE. The host never wrote the document, cannot see the
// failure in the configuration, and is the only party able to act on it — a
// supervisor whose machine fails an `<assign>` every round reads
// `is_running() == true` and a plausible state forever.
//
// Four outcomes the fixture separates, all four leaving the configuration on
// the same state, so no accessor that existed before this fixture tells them
// apart:
//
//   poke               handled, no error       control: proves a run fired
//   whisper            author's <raise>, unmatched  NOT counted
//   boom in `idle`     error, unmatched        COUNTED — the silent failure
//   boom in `guarded`  error, HANDLED          not counted
//
// `boom` is one event name routed to two outcomes by state, so a count cannot
// be keyed off the event or the action — only off what the configuration did
// with the error the engine raised.
//
// The C++ Interpreter answers this through `getLastStateMachineError()` and the
// message it passes to `raiseEvent("error.execution", msg)`; this is the
// generated engines' side of the same question, so a document moving from the
// Interpreter to AOT keeps a signal its host was reading.
//
// Fixture: integration_resources/unhandled_error_is_observable/unhandled_error_is_observable.scxml
// (canonical, shared with the C++ / C11 / Go / Kotlin / Python channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_unhandled_error_is_observable.sh

use std::sync::Arc;

use sce_rust_runtime::{Engine, IScriptEngine};
use sce_rust_tests::integration::unhandled_error_is_observable::{
    UnhandledErrorIsObservableEvent as Event, UnhandledErrorIsObservablePolicy as Policy,
    UnhandledErrorIsObservableState as State,
};

fn started() -> (Engine<Policy>, Arc<dyn IScriptEngine>) {
    let script_engine: Arc<dyn IScriptEngine> = Arc::new(sce_rust_lua::LuaEngine::new());
    let mut engine = Engine::new(Policy::new(Arc::clone(&script_engine)));
    engine.initialize();
    (engine, script_engine)
}

/// The fixture's `<assign>`s are the only witness that a transition ran at all —
/// without them a passing run could not be told apart from one where nothing
/// fired.
fn counter(engine: &Engine<Policy>, script_engine: &Arc<dyn IScriptEngine>, name: &str) -> i64 {
    sce_rust_runtime::helpers::datamodel_read::read_int(
        &**script_engine,
        engine.policy().session_id.as_deref(),
        name,
    )
    .unwrap_or_else(|| panic!("the fixture declares `{name}` in its datamodel"))
}

/// The axis: an error the engine raised that no active state answers is counted.
#[test]
fn an_error_no_transition_answered_is_counted() {
    let (mut engine, se) = started();
    assert_eq!(
        engine.unhandled_error_events(),
        0,
        "no error has gone unhandled before the first event"
    );

    engine.raise_external(Event::Boom, "", "");
    engine.step();

    assert_eq!(
        counter(&engine, &se, "booms"),
        1,
        "`boom`'s transition did not run, so nothing below is measuring an \
         error raised inside a transition that fired"
    );
    assert_eq!(
        engine.unhandled_error_events(),
        1,
        "`boom`'s second <assign> has W3C 5.3's invalid empty location, so the \
         engine raised error.execution — and `idle` declares no transition for \
         it. The host that is driving this machine has no other way to learn \
         its <assign> failed"
    );
    assert_eq!(
        engine.get_current_state(),
        State::Idle,
        "the error must not move the machine on its own"
    );
}

/// The other half: an error the DOCUMENT answered must not be counted. A count
/// that is always non-zero is as useless as one that is always zero.
#[test]
fn an_error_the_document_handled_is_not_counted() {
    let (mut engine, se) = started();

    engine.raise_external(Event::Go, "", "");
    engine.step();
    assert_eq!(
        engine.get_current_state(),
        State::Guarded,
        "`go` should have moved the machine to the state that answers errors"
    );

    engine.raise_external(Event::Boom, "", "");
    engine.step();

    assert_eq!(
        counter(&engine, &se, "caught"),
        1,
        "`guarded`'s error.execution transition did not run, so this test is \
         not measuring a HANDLED error"
    );
    assert_eq!(
        engine.unhandled_error_events(),
        0,
        "the same <assign> failed in `guarded`, where the document does declare \
         a transition for error.execution. The document dealt with it, and its \
         handling is already visible in the configuration — counting it would \
         report the author's own error handling as a silent failure"
    );
    assert_eq!(
        engine.last_unhandled_error(),
        None,
        "nothing went unhandled, so there is no last one to name"
    );
}

/// The boundary the count is drawn at: an author's own unmatched `<raise>` is
/// not an unhandled error. Both ends of that event are inside the document,
/// which is the reason `discarded_external_events` stops at the external queue
/// — and the reason this count does not stop there.
#[test]
fn an_authors_unmatched_raise_is_not_an_unhandled_error() {
    let (mut engine, se) = started();

    engine.raise_external(Event::Whisper, "", "");
    engine.step();

    assert_eq!(
        engine.unhandled_error_events(),
        0,
        "`whisper` raises `unheard` and `retry.error.execution`, neither of \
         which any state answers. Both are discarded exactly as an unmatched \
         error is, and neither is one: the author wrote the raises and the \
         absent handlers, and can read both in the document. \
         `retry.error.execution` is the sharper half — it CONTAINS `error.` \
         without starting with it, and §scxml-3.12.2 reserves the prefix, not \
         the substring"
    );
    assert_eq!(
        counter(&engine, &se, "heards"),
        1,
        "`whisper`'s third raise, `heard`, does match — and the transition it \
         matches did not run. The count above is a byproduct of this drain, \
         never its job: an implementation that only selects transitions for \
         error events stops running the document for everything else, and this \
         is the assertion that notices"
    );
    assert_eq!(
        engine.discarded_external_events(),
        0,
        "`whisper` itself was handled, so the external-queue count stays put — \
         the internal events it raised are not on that queue at all"
    );
}

/// Why the query has to exist: every pre-existing accessor answers the same for
/// a run that failed silently and one that did not fail at all.
#[test]
fn the_unhandled_error_is_not_derivable_from_any_other_accessor() {
    let (mut engine, _se) = started();

    engine.raise_external(Event::Poke, "", "");
    engine.step();
    let clean = (
        engine.get_current_state(),
        engine.get_active_states().to_vec(),
        engine.is_running(),
        engine.is_in_final_state(),
        engine.discarded_external_events(),
        engine.last_discarded_event(),
    );

    engine.raise_external(Event::Boom, "", "");
    engine.step();
    let failed = (
        engine.get_current_state(),
        engine.get_active_states().to_vec(),
        engine.is_running(),
        engine.is_in_final_state(),
        engine.discarded_external_events(),
        engine.last_discarded_event(),
    );

    assert_eq!(
        clean, failed,
        "this fixture exists because these two are indistinguishable through \
         every accessor a host had — including layer three's discard count, \
         which never sees the internal queue. If they ever differ, the fixture \
         stopped measuring what it claims"
    );
    assert_eq!(
        engine.unhandled_error_events(),
        1,
        "the two are indistinguishable through every other accessor, so this \
         count is the only thing that separates a silent failure from a clean run"
    );
}

/// A count says something failed; a host repairing it needs the class of error.
#[test]
fn the_engine_names_the_error_it_dropped() {
    let (mut engine, _se) = started();
    assert_eq!(
        engine.last_unhandled_error(),
        None,
        "nothing has gone unhandled yet"
    );

    engine.raise_external(Event::Boom, "", "");
    engine.step();

    assert_eq!(
        engine.last_unhandled_error(),
        Some(Event::ErrorExecution),
        "`error.execution` is the document's own executable content failing; \
         `error.communication` would be a <send> that could not reach its \
         target. Two different repairs, and a bare count separates neither"
    );
}

/// The supervisor's actual failure mode: every round fails the same way and
/// nothing in the configuration ever changes.
#[test]
fn a_machine_failing_every_round_is_counted_every_round() {
    let (mut engine, se) = started();

    for round in 1..=3 {
        engine.raise_external(Event::Boom, "", "");
        engine.step();
        assert_eq!(
            engine.unhandled_error_events(),
            round,
            "round {round} did not add to the count; a supervisor polling this \
             number is exactly who learns the loop is not making progress"
        );
        assert_eq!(
            engine.get_current_state(),
            State::Idle,
            "the machine looks identical on every round, which is the problem"
        );
    }
    assert_eq!(
        counter(&engine, &se, "booms"),
        3,
        "all three rounds ran their transition"
    );
}
