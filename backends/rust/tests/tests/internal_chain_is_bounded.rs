// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.13 ends a macrostep at a configuration where nothing is enabled
// by NULL AND the internal queue is empty. Appendix D's Principles and
// Constraints then say that end need not exist: "A microstep always
// terminates. A macrostep may not. ... This is currently allowed." Rust AOT
// path.
//
// `eventless_macrostep_is_bounded` owns the half of that clause built from
// transitions that need no event. This one owns the other half: a `<raise>`
// answered by a transition that raises again. Measured 2026-08-20 before the
// ceiling reached this branch, `step()` on the fixture's `spin` document did
// not return on this engine — the internal drain had no budget at all, and
// `check_eventless_transitions`' hundred was spent on the branch that was not
// running.
//
// Fixture: integration_resources/internal_chain_is_bounded/internal_chain_is_bounded.scxml
// (canonical, shared with the C++ / C11 / Go / Kotlin / Python channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_internal_chain_is_bounded.sh

use std::sync::Arc;

use sce_rust_runtime::{Engine, IScriptEngine};
use sce_rust_tests::integration::internal_chain_is_bounded::{
    InternalChainIsBoundedEvent as Event, InternalChainIsBoundedPolicy as Policy,
    InternalChainIsBoundedState as State,
};

/// The ceiling the engine applies, spelled here rather than read back from it.
/// A test that asked the engine for its own limit would agree with any limit,
/// including one an edit moved by three orders of magnitude.
const MAX_MICROSTEPS: i64 = 1000;

/// One lap of the alternating chain is two microsteps — one internal event,
/// one eventless transition — and only the internal half is counted, so a
/// chain run to the shared ceiling records half.
const ALTERNATING_LAPS_AT_CEILING: i64 = MAX_MICROSTEPS / 2;

fn started() -> (Engine<Policy>, Arc<dyn IScriptEngine>) {
    let script_engine: Arc<dyn IScriptEngine> = Arc::new(sce_rust_lua::LuaEngine::new());
    let mut engine = Engine::new(Policy::new(Arc::clone(&script_engine)));
    engine.initialize();
    (engine, script_engine)
}

/// The fixture's `<assign>`s are the only witness of how far a chain got —
/// every outcome leaves the machine in a state the configuration alone cannot
/// tell apart from the others.
fn counter(engine: &Engine<Policy>, script_engine: &Arc<dyn IScriptEngine>, name: &str) -> i64 {
    sce_rust_runtime::helpers::datamodel_read::read_int(
        &**script_engine,
        engine.policy().session_id.as_deref(),
        name,
    )
    .unwrap_or_else(|| panic!("the fixture declares `{name}` in its datamodel"))
}

/// The axis: a macrostep whose `<raise>` chain cannot end is stopped, and the
/// host is told that it was.
///
/// This test returning at all is half the assertion. Before the ceiling
/// reached this branch it did not: the same call ran until the harness killed
/// it.
#[test]
fn a_raise_chain_that_cannot_end_is_stopped() {
    let (mut engine, se) = started();
    assert_eq!(
        engine.truncated_macrosteps(),
        0,
        "nothing has been refused before the machine has done anything"
    );

    engine.raise_external(Event::Spin, "", "");
    engine.step();

    assert_eq!(
        counter(&engine, &se, "links"),
        MAX_MICROSTEPS,
        "the chain must run exactly as far as the engine allows — fewer means \
         the document was cut off early, more means the ceiling moved"
    );
    assert_eq!(
        engine.truncated_macrosteps(),
        1,
        "the microstep past the budget was queued and was not taken. \
         Without this count the host sees a machine that is running, in a \
         state the document names, having returned in microseconds — and no \
         way to learn that the configuration it is reading is not a stable one"
    );
    assert_eq!(
        engine.last_truncated_macrostep_state(),
        Some(State::Spin),
        "the count alone says a document somewhere cannot settle; this says \
         where to look"
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
/// The fixture's bounded chain is exactly `MAX_MICROSTEPS` links for this
/// reason. A ceiling that counted loop turns rather than microsteps taken, or
/// that tested `>=` where it meant `>`, reports this ordinary document as a
/// runaway.
#[test]
fn a_raise_chain_that_ends_at_the_ceiling_is_not_refused() {
    let (mut engine, se) = started();

    engine.raise_external(Event::Bounded, "", "");
    engine.step();

    assert_eq!(
        counter(&engine, &se, "laps"),
        MAX_MICROSTEPS,
        "the guard `laps < 999` stops matching at the thousandth link, which \
         raises nothing — so the queue empties and the chain stops by itself"
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
}

/// A dequeue that selected nothing is not a microstep, so it spends no budget.
///
/// Appendix D takes a microstep for a transition that was SELECTED; a dequeue
/// that matched none is the loop turn the clause does not count. The fixture's
/// `unanswered` chain is `bounded` with one unmatched event added per link, so
/// the two differ in exactly that and must cost the same.
///
/// Measured 2026-08-21: this claim had no witness. The mutation that spends the
/// budget on every dequeue SURVIVED all five outcomes, because every other
/// chain in the fixture answers every event it raises — an engine that
/// over-counted would report this settling document as a runaway at half its
/// length, and nothing here could see it.
#[test]
fn a_dequeue_that_selected_nothing_spends_no_budget() {
    let (mut engine, se) = started();

    engine.raise_external(Event::Unanswered, "", "");
    engine.step();

    assert_eq!(
        counter(&engine, &se, "ignores"),
        MAX_MICROSTEPS,
        "the chain is the same length as `bounded`; the unmatched events \
         between its links are dequeues that selected nothing, and those are \
         not microsteps"
    );
    assert_eq!(
        engine.truncated_macrosteps(),
        0,
        "a thousand microsteps and a thousand discards is a thousand \
         microsteps: an engine that counted the discards refuses this document \
         at link five hundred and reports a runaway that is not one"
    );
    assert_eq!(
        engine.last_truncated_macrostep_state(),
        None,
        "and nothing names a state, because nothing was stopped"
    );
    assert!(engine.is_running(), "the document settled on its own");
}

/// The case a per-branch budget lets through: a chain that alternates one
/// `<raise>` with one eventless transition.
///
/// Neither branch of §scxml-D's inner loop reaches the ceiling on its own here
/// — each takes every other microstep — so an engine that gives each branch a
/// counter of its own runs this document forever with both ceilings half
/// spent. One of the seven shipped exactly that pair of counters. The witness
/// is arithmetic: fifty laps, not a hundred and not forever.
#[test]
fn an_alternating_chain_spends_one_shared_budget() {
    let (mut engine, se) = started();

    engine.raise_external(Event::Alternate, "", "");
    engine.step();

    assert_eq!(
        counter(&engine, &se, "alts"),
        ALTERNATING_LAPS_AT_CEILING,
        "the two branches share one budget, so a chain that alternates them \
         gets five hundred laps out of a thousand microsteps. A thousand here \
         would mean the internal branch had a ceiling of its own"
    );
    assert_eq!(
        engine.truncated_macrosteps(),
        1,
        "and the refusal is reported once, whichever branch was holding the \
         budget when it ran out"
    );
    assert_eq!(
        engine.last_truncated_macrostep_state(),
        Some(State::Alt),
        "named the same way as any other chain that could not settle"
    );
}

/// What the refusal did with the links it would not run: it left them queued.
///
/// The fixture's `resume` chain is half again as long as the ceiling, so the
/// first macrostep is refused with five hundred links still to go and the
/// second one finishes them. An engine that dropped the queue stops at a
/// thousand and never finishes; one that ran the chain anyway finishes it in
/// the first macrostep. Neither is distinguishable from the correct answer by
/// any other outcome in this fixture.
///
/// The event driving the second macrostep is `poke`, and what it does is
/// deliberately not asserted: §scxml-3.13 gives internal events priority, so
/// this engine reaches it only after the chain, while the C++ AOT engine's
/// `processEvent` takes the host's event first. That divergence is its own
/// debt and not what this fixture measures — the counters below are the same
/// on both.
#[test]
fn a_refused_chain_is_left_queued_for_the_next_macrostep() {
    let (mut engine, se) = started();

    engine.raise_external(Event::Resume, "", "");
    engine.step();
    assert_eq!(
        counter(&engine, &se, "beats"),
        MAX_MICROSTEPS,
        "the first macrostep spends the whole budget on the chain"
    );
    assert_eq!(engine.truncated_macrosteps(), 1);

    engine.raise_external(Event::Poke, "", "");
    engine.step();

    assert_eq!(
        counter(&engine, &se, "beats"),
        MAX_MICROSTEPS + MAX_MICROSTEPS / 2,
        "the second macrostep picked the chain up where the first was cut and \
         ran it to its end — the refused links were left on the queue, not \
         dropped"
    );
    assert_eq!(
        engine.truncated_macrosteps(),
        1,
        "and nothing was refused this time: the chain ended on its own inside \
         the budget, which is an ordinary macrostep however long the document \
         took to get there"
    );
    assert!(engine.is_running());
}

/// The control: an ordinary document is untouched by any of this.
///
/// Without it, an engine that refused every macrostep would pass the
/// assertions above and fail nothing.
#[test]
fn an_ordinary_macrostep_is_not_counted() {
    let (mut engine, se) = started();

    engine.raise_external(Event::Poke, "", "");
    engine.step();

    assert_eq!(
        counter(&engine, &se, "pokes"),
        1,
        "the run happened: a counter of zero cannot tell an engine that did \
         nothing from one that was never asked"
    );
    assert_eq!(
        engine.truncated_macrosteps(),
        0,
        "and one transition is not a chain that cannot end"
    );
    assert_eq!(engine.last_truncated_macrostep_state(), None);
    assert_eq!(engine.get_current_state(), State::Idle);
}
