// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.12.2 says an error event nothing matches is ignored. It says
// nothing about an error event something DOES match, answered by a handler
// that fails the same way every time: the failure raises `error.execution`,
// the same transition answers it, and the drain never empties. Rust AOT path.
//
// That is not a hang, which is what makes it worth an accessor. Measured
// 2026-08-19 on the Python engine and a two-line document: 37,000 links a
// second, configuration unmoved, `is_running` true — the reading an
// unattended supervisor takes as a healthy idle machine while a core is
// pinned. `unhandled_error_is_observable` owns the error nobody answered;
// this owns the error answered by a handler that cannot handle it.
//
// The fixture separates a chain that STOPS by itself (`settle`, three links,
// then its guard stops matching) from one that cannot (`spin`). Both are runs
// of errors, and only the second is a defect — a ceiling that could not tell
// them apart would report every document that fails often as broken.
//
// Fixture: integration_resources/error_cascade_is_bounded/error_cascade_is_bounded.scxml
// (canonical, shared with the C++ / C11 / Go / Kotlin / Python channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_error_cascade_is_bounded.sh

use std::sync::Arc;

use sce_rust_runtime::{Engine, IScriptEngine};
use sce_rust_tests::integration::error_cascade_is_bounded::{
    ErrorCascadeIsBoundedEvent as Event, ErrorCascadeIsBoundedPolicy as Policy,
    ErrorCascadeIsBoundedState as State,
};

/// The ceiling the engine applies, spelled here rather than read back from it.
/// A test that asked the engine for its own limit would agree with any limit,
/// including one an edit moved by three orders of magnitude — and the number
/// is exactly what this fixture exists to pin.
const MAX_LINKS: i64 = 100;

fn started() -> (Engine<Policy>, Arc<dyn IScriptEngine>) {
    let script_engine: Arc<dyn IScriptEngine> = Arc::new(sce_rust_lua::LuaEngine::new());
    let mut engine = Engine::new(Policy::new(Arc::clone(&script_engine)));
    engine.initialize();
    (engine, script_engine)
}

/// The fixture's `<assign>`s are the only witness that a handler ran at all —
/// every outcome here leaves the configuration where it was.
fn counter(engine: &Engine<Policy>, script_engine: &Arc<dyn IScriptEngine>, name: &str) -> i64 {
    sce_rust_runtime::helpers::datamodel_read::read_int(
        &**script_engine,
        engine.policy().session_id.as_deref(),
        name,
    )
    .unwrap_or_else(|| panic!("the fixture declares `{name}` in its datamodel"))
}

/// The axis: a handler that answers its own failure with the same failure is
/// stopped, and the host is told.
///
/// This test returning at all is half the assertion. Before the ceiling
/// existed it did not: the same call ran until the harness was killed.
#[test]
fn a_handler_that_cannot_handle_its_error_is_stopped() {
    let (mut engine, se) = started();
    assert_eq!(
        engine.error_cascade_events(),
        0,
        "nothing has been refused before the machine has done anything"
    );

    engine.raise_external(Event::Spin, "", "");
    engine.step();

    assert_eq!(
        counter(&engine, &se, "runs"),
        MAX_LINKS,
        "`runaway`'s handler must run exactly as many times as the engine \
         allows links in a chain — fewer means the document was cut off early, \
         more means the ceiling moved"
    );
    assert_eq!(
        counter(&engine, &se, "ticks"),
        MAX_LINKS,
        "every link's handler also raises the author's own `tick`, and every \
         one of them was delivered. An engine that counted those as links \
         would refuse at half the depth; one that let them end the chain would \
         never refuse at all — and a handler that logs before it fails is an \
         ordinary document, not a corner case"
    );
    assert_eq!(
        engine.error_cascade_events(),
        1,
        "the handler's <assign> failed again on the last allowed link, and the \
         error it raised is the one the engine refused to queue. Without that \
         count the host sees a machine that is running, in a plausible state, \
         with nothing to say about the core it is burning"
    );
    assert_eq!(
        engine.last_error_cascade_event(),
        Some(Event::ErrorExecution),
        "a count alone does not name the repair: `error.execution` is a \
         handler whose own content fails, `error.communication` one that \
         answers an unreachable target by talking to it again"
    );
    assert!(
        engine.is_running(),
        "the chain was cut, not the machine — refusing to feed a broken \
         handler is not a reason to stop running a document whose other \
         states still work"
    );
    assert_eq!(
        engine.get_current_state(),
        State::Runaway,
        "the handler is targetless, so nothing here may move the machine"
    );
}

/// The other half, and the one that makes the count mean something: a chain
/// that ends by itself must pass through untouched.
#[test]
fn a_chain_that_ends_on_its_own_is_not_refused() {
    let (mut engine, se) = started();

    engine.raise_external(Event::Settle, "", "");
    engine.step();

    assert_eq!(
        counter(&engine, &se, "repairs"),
        3,
        "`settling`'s handler repairs three times and then its `repairs < 3` \
         guard stops matching. Three links is what a real repair strategy \
         looks like, and the engine must not have interrupted it"
    );
    assert_eq!(
        engine.error_cascade_events(),
        0,
        "nothing was refused: the chain ended on the document's own terms. A \
         ceiling that fired here would report every document that fails often \
         as one that cannot stop failing"
    );
    assert_eq!(
        engine.last_error_cascade_event(),
        None,
        "nothing was refused, so there is no last one to name"
    );
    assert_eq!(
        engine.unhandled_error_events(),
        1,
        "the fourth error found no matching transition once the guard closed, \
         which is the ordinary clause — the two counts answer different \
         questions and this document produces exactly one of each"
    );
}

/// A single failure with nobody to answer it is not a chain. The chain is
/// measured handler-to-handler, not failure-to-failure.
#[test]
fn one_error_nobody_answered_is_not_a_chain() {
    let (mut engine, _se) = started();

    for _ in 0..5 {
        engine.raise_external(Event::Boom, "", "");
        engine.step();
    }

    assert_eq!(
        engine.unhandled_error_events(),
        5,
        "five failures, none of them answered — the clause's own case"
    );
    assert_eq!(
        engine.error_cascade_events(),
        0,
        "no handler ran, so no handler raised anything: a count keyed off how \
         OFTEN a document fails would already be at five here"
    );
}

/// The machine is still a machine afterwards. Cutting the chain must not cost
/// the document the states that work.
#[test]
fn the_machine_still_answers_after_its_chain_is_cut() {
    let (mut engine, se) = started();

    engine.raise_external(Event::Spin, "", "");
    engine.step();
    assert_eq!(
        engine.error_cascade_events(),
        1,
        "precondition: this test is about what happens AFTER a refusal"
    );

    engine.raise_external(Event::Poke, "", "");
    engine.step();

    assert_eq!(
        counter(&engine, &se, "pokes"),
        1,
        "`runaway` answers `poke` with a targetless transition, and it ran — \
         an engine that stopped the machine to end the chain would leave the \
         host with a dead document instead of a bounded one"
    );
    assert_eq!(
        engine.error_cascade_events(),
        1,
        "`poke` raises nothing, so the count that was already there is all \
         there is: the refusal is a fact about the past, not a mode"
    );
}

/// A second chain starts from zero. The depth is a property of the chain, not
/// of the machine's whole life: an engine that never reset it would refuse the
/// second chain on its first link, and the host would read a machine that has
/// stopped trying rather than one that is still failing.
#[test]
fn a_second_chain_starts_from_zero() {
    let (mut engine, se) = started();

    engine.raise_external(Event::Spin, "", "");
    engine.step();
    engine.raise_external(Event::Reset, "", "");
    engine.step();
    assert_eq!(
        engine.get_current_state(),
        State::Idle,
        "`reset` is the fixture's way back out of the chain"
    );

    engine.raise_external(Event::Spin, "", "");
    engine.step();

    assert_eq!(
        counter(&engine, &se, "runs"),
        2 * MAX_LINKS,
        "the second entry into `runaway` must buy the document a full chain \
         again. A depth carried across the drains would stop this one at its \
         first link and leave the counter at {MAX_LINKS}"
    );
    assert_eq!(
        engine.error_cascade_events(),
        2,
        "two chains, two refusals — a count that saturates at one would read \
         as a machine that recovered"
    );
}
