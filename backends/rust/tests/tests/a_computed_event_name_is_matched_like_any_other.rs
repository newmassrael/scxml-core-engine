// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A computed event name is matched like any other (§scxml-3.12.1, §scxml-5.10) —
// Rust AOT.
//
// A `<send eventexpr>` names its event at run time, so the document cannot have
// written the name: it is delivered as the event the document's names resolve it
// to (its own, the longest token prefix of it the document writes, or its
// wildcard), and `_event.name` is the whole name. The document sends six, over
// the external and the internal queue, now and after a delay, and takes each only
// when it is told the whole name.
//
// Fixture: integration_resources/a_computed_event_name_is_matched_like_any_other/a_computed_event_name_is_matched_like_any_other.scxml
// (canonical, shared with the C++ / Go / Kotlin / Python / C11 channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_a_computed_event_name_is_matched_like_any_other.sh

use std::sync::{Arc, Mutex};
use std::time::Duration;

use sce_rust_runtime::{Engine, TransitionTaken};
use sce_rust_tests::integration::a_computed_event_name_is_matched_like_any_other::{
    AComputedEventNameIsMatchedLikeAnyOtherPolicy, AComputedEventNameIsMatchedLikeAnyOtherState,
    AComputedEventNameIsMatchedLikeAnyOtherTransition,
};

type State = AComputedEventNameIsMatchedLikeAnyOtherState;
type Seen = Arc<Mutex<Vec<(State, u32)>>>;

fn engine() -> Engine<AComputedEventNameIsMatchedLikeAnyOtherPolicy> {
    let script_engine: Arc<dyn sce_rust_runtime::IScriptEngine> =
        Arc::new(sce_rust_lua::LuaEngine::new());
    Engine::new(AComputedEventNameIsMatchedLikeAnyOtherPolicy::new(
        script_engine,
    ))
}

/// An engine whose transition observer (docs/adr/0089) keeps each row it is
/// told in `seen`.
fn observed(seen: &Seen) -> Engine<AComputedEventNameIsMatchedLikeAnyOtherPolicy> {
    let mut engine = engine();
    let sink = Arc::clone(seen);
    engine.set_transition_observer(move |taken: TransitionTaken<State>| {
        sink.lock()
            .expect("the observer's list is not poisoned")
            .push((taken.source, taken.index));
    });
    engine
}

#[test]
fn a_computed_name_is_matched_by_token_and_told_whole() {
    let mut engine = engine();
    engine
        .initialize()
        .expect("the machine's script session is established");

    let completed = engine.run_until_completion(Duration::from_secs(2), Duration::from_millis(10));
    assert!(
        completed,
        "a_computed_event_name_is_matched_like_any_other stopped before its final state: a \
         computed name was dropped, or matched but told shorter than the name it was sent \
         under (`request.new`, `other.thing`, `request.again`, `last.one`)"
    );

    assert_eq!(
        engine.terminal_state(),
        Some(AComputedEventNameIsMatchedLikeAnyOtherState::Pass)
    );
}

/// The document declares its transition table, and a run that reaches `pass`
/// takes each of its six transitions once, in document order: the observer is
/// told exactly those rows, each by the `(from, index)` its row of the table
/// carries (docs/adr/0089).
#[test]
fn the_observer_is_told_each_row_of_the_table_the_run_takes() {
    let seen = Seen::default();
    let mut engine = observed(&seen);
    engine
        .initialize()
        .expect("the machine's script session is established");
    let completed = engine.run_until_completion(Duration::from_secs(2), Duration::from_millis(10));
    assert!(completed, "the observed run stopped before its final state");
    assert_eq!(engine.terminal_state(), Some(State::Pass));

    let seen = seen.lock().expect("not poisoned").clone();
    assert_eq!(
        seen,
        vec![
            (State::S1, 0),
            (State::S2, 0),
            (State::S3, 0),
            (State::S4, 0),
            (State::S5, 0),
            (State::S6, 0),
        ],
        "the rows taken, in the order the run took them"
    );
    let rows: Vec<(State, u32)> = AComputedEventNameIsMatchedLikeAnyOtherTransition::ALL
        .iter()
        .map(|row| (row.from, row.index))
        .collect();
    assert!(
        seen.iter().all(|taken| rows.contains(taken)),
        "every pair the observer is told is a row of the table: {seen:?} against {rows:?}"
    );
    assert!(
        rows.iter().all(|row| seen.contains(row)),
        "the run takes every row of the table: {rows:?} against {seen:?}"
    );
}

/// An observer set and then cleared is told nothing, and the run is the one an
/// engine that never had one makes.
#[test]
fn a_cleared_observer_is_told_nothing_and_the_run_is_the_same() {
    let seen = Seen::default();
    let mut engine = observed(&seen);
    engine.clear_transition_observer();
    engine
        .initialize()
        .expect("the machine's script session is established");
    let completed = engine.run_until_completion(Duration::from_secs(2), Duration::from_millis(10));

    assert!(completed, "the run stopped before its final state");
    assert_eq!(engine.terminal_state(), Some(State::Pass));
    assert!(seen.lock().expect("not poisoned").is_empty());
}
