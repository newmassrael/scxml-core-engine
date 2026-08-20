// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.13 says a macrostep is a chain of microsteps ending in a
// configuration where nothing is enabled by NULL. Appendix D's Principles and
// Constraints then say the chain need not exist: "A microstep always
// terminates. A macrostep may not. ... This is currently allowed." Rust AOT
// path.
//
// So a cyclic eventless document is not malformed, and an engine that runs it
// to the letter never returns. This one does not run it to the letter — and
// that decision is invisible from every other reading. `get_current_state`
// answers, `is_running` is true, and the call returned in microseconds; the
// configuration behind those answers is simply not the stable one the clause
// promises. `truncated_macrosteps` is the only place the difference shows.
//
// `error_cascade_is_bounded` owns the chain built from errors; this one owns
// the chain built from transitions that need no event at all. The fixture
// separates a chain that stops on its own — a HUNDRED microsteps, exactly the
// ceiling, which is where an off-by-one lands — from one that cannot stop.
//
// Fixture: integration_resources/eventless_macrostep_is_bounded/eventless_macrostep_is_bounded.scxml
// (canonical, shared with the C++ / C11 / Go / Kotlin / Python channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_eventless_macrostep_is_bounded.sh

use std::sync::Arc;

use sce_rust_runtime::{Engine, IScriptEngine};
use sce_rust_tests::integration::eventless_macrostep_is_bounded::{
    EventlessMacrostepIsBoundedEvent as Event, EventlessMacrostepIsBoundedPolicy as Policy,
    EventlessMacrostepIsBoundedState as State,
};

/// The ceiling the engine applies, spelled here rather than read back from it.
/// A test that asked the engine for its own limit would agree with any limit,
/// including one an edit moved by three orders of magnitude.
const MAX_MICROSTEPS: i64 = 1000;

/// One lap of either chain is two microsteps (`_a` to `_b`, then back), and
/// only the `_a` edge counts, so a chain run to the ceiling records half.
const LAPS_AT_CEILING: i64 = MAX_MICROSTEPS / 2;

fn started() -> (Engine<Policy>, Arc<dyn IScriptEngine>) {
    let script_engine: Arc<dyn IScriptEngine> = Arc::new(sce_rust_lua::LuaEngine::new());
    let mut engine = Engine::new(Policy::new(Arc::clone(&script_engine)));
    engine.initialize();
    (engine, script_engine)
}

/// The fixture's `<assign>`s are the only witness of how far a chain got —
/// the configuration alone cannot tell a chain that stopped from one that was
/// stopped.
fn counter(engine: &Engine<Policy>, script_engine: &Arc<dyn IScriptEngine>, name: &str) -> i64 {
    sce_rust_runtime::helpers::datamodel_read::read_int(
        &**script_engine,
        engine.policy().session_id.as_deref(),
        name,
    )
    .unwrap_or_else(|| panic!("the fixture declares `{name}` in its datamodel"))
}

/// The axis: a macrostep whose eventless chain cannot end is stopped, and the
/// host is told that it was.
///
/// This test returning at all is half the assertion. On the Python engine,
/// which had no ceiling at all, the equivalent call did not return: measured
/// 2026-08-20, `initialize()` on a two-state document ran until the harness
/// killed it.
#[test]
fn a_macrostep_that_cannot_end_is_stopped() {
    let (mut engine, se) = started();
    assert_eq!(
        engine.truncated_macrosteps(),
        0,
        "nothing has been refused before the machine has done anything"
    );

    engine.raise_external(Event::Spin, "", "");
    engine.step();

    assert_eq!(
        counter(&engine, &se, "spins"),
        LAPS_AT_CEILING,
        "the chain must run exactly as far as the engine allows — fewer means \
         the document was cut off early, more means the ceiling moved"
    );
    assert_eq!(
        engine.truncated_macrosteps(),
        1,
        "the microstep past the budget was enabled and was not taken. \
         Without this count the host sees a machine that is running, in a \
         state the document names, having returned in microseconds — and no \
         way to learn that the configuration it is reading is not a stable one"
    );
    assert_eq!(
        engine.last_truncated_macrostep_state(),
        Some(State::SpinA),
        "an eventless cycle is a closed walk through the state graph, and the \
         count alone does not say which walk. This names a state on it, which \
         is where an author looks first"
    );
    assert!(
        engine.is_running(),
        "the chain was cut, not the machine. §scxml-D allows the document; \
         refusing to run it forever is the engine's decision to report, not a \
         reason to stop a machine whose other states still work"
    );
}

/// The other half, and the one that makes the count mean something: a chain
/// that ends on its own is not refused, however long it is.
///
/// The fixture's bounded chain is exactly `MAX_MICROSTEPS` microsteps for this
/// reason. A ceiling that counted loop turns rather than microsteps taken, or
/// that tested `>=` where it meant `>`, reports this ordinary document as a
/// runaway — two engines in this repository did, and one of them stopped the
/// machine over it.
#[test]
fn a_chain_that_ends_at_the_ceiling_is_not_refused() {
    let (mut engine, se) = started();

    engine.raise_external(Event::Bounded, "", "");
    engine.step();

    assert_eq!(
        counter(&engine, &se, "laps"),
        LAPS_AT_CEILING,
        "the guard `laps < 500` closes after five hundred laps, so the chain \
         is a thousand microsteps long and then stops by itself"
    );
    assert_eq!(
        engine.truncated_macrosteps(),
        0,
        "nothing was refused: the macrostep reached the stable configuration \
         §scxml-3.13 describes, using every microstep it was allowed. A long \
         chain is not a runaway"
    );
    assert_eq!(
        engine.last_truncated_macrostep_state(),
        None,
        "and nothing names a state, because nothing was stopped"
    );
    assert!(
        engine.is_running(),
        "a document that settles on its own must not be reported dead by an \
         engine that just finished running it correctly"
    );
    assert_eq!(
        engine.get_current_state(),
        State::BoundedA,
        "the chain rests where its guard closed"
    );
}

/// A count, not a flag: a second unbounded macrostep is refused the same way
/// the first was, and costs the document the same budget.
///
/// An engine that recorded the truncation once and then treated the machine
/// as known-broken would report the same number forever, and a host polling it
/// could not tell a machine that spun once from one spinning every macrostep.
#[test]
fn a_second_truncated_macrostep_counts_again() {
    let (mut engine, se) = started();

    engine.raise_external(Event::Spin, "", "");
    engine.step();
    assert_eq!(engine.truncated_macrosteps(), 1);

    // `reset` is the fixture's way back out of the cycle, and it moves the
    // machine on purpose: the two C++ engines complete a macrostep only after
    // a transition that does.
    engine.raise_external(Event::Reset, "", "");
    engine.step();
    assert_eq!(engine.get_current_state(), State::Idle);

    engine.raise_external(Event::Spin, "", "");
    engine.step();

    assert_eq!(
        engine.truncated_macrosteps(),
        2,
        "the second macrostep hit the same ceiling and was counted again"
    );
    assert_eq!(
        counter(&engine, &se, "spins"),
        2 * LAPS_AT_CEILING,
        "and it really bought the document a full budget again rather than \
         refusing on sight — the ceiling bounds a macrostep, it does not \
         condemn a machine"
    );
}

/// The control: an ordinary document is untouched by any of this.
///
/// Without it, an engine that refused every macrostep would pass the two
/// assertions above and fail nothing.
#[test]
fn an_ordinary_macrostep_is_not_counted() {
    let (mut engine, se) = started();

    engine.raise_external(Event::Poke, "", "");
    engine.step();

    assert_eq!(counter(&engine, &se, "pokes"), 1, "the run fired");
    assert_eq!(
        engine.truncated_macrosteps(),
        0,
        "a macrostep of one microstep ends the way the clause says it does"
    );
    assert_eq!(engine.last_truncated_macrostep_state(), None);
    assert_eq!(engine.get_current_state(), State::Idle);
}
