// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A machine that answers an event by sending itself the next one, with no
// target, never lets the external queue empty — Rust AOT path.
//
// Every macrostep of such a machine ends, so `MAX_MACROSTEP_MICROSTEPS` never
// applies, and the main event loop takes the next external event whenever the
// queue is not empty: a host call that drains it did not return. The Rust
// runtime's `run_main_event_loop` had that shape (measured by reading it) until
// it took the budget ARCHITECTURE.md "External-Event Budget" states as one
// contract for every engine. This driver holds this engine to it: the same
// outcomes, the same arithmetic, the same document as the Python one.
//
// The delayed outcomes run on `SceClock::Manual`, so nothing here sleeps and the
// clock moves only where a case moves it.
//
// Fixture: tests/integration/external_chain_is_bounded.scxml. It is outside
// `integration_resources/` for the reason
// `scripts/regen_external_chain_is_bounded.sh` states.
//
// Regeneration (after fixture or template edit):
//   scripts/regen_external_chain_is_bounded.sh

use std::num::NonZeroU32;
use std::sync::Arc;

use sce_rust_runtime::{Engine, IScriptEngine, SceClock};
use sce_rust_tests::integration::external_chain_is_bounded::{
    ExternalChainIsBoundedEvent as Event, ExternalChainIsBoundedPolicy as Policy,
};

/// The default the contract states, spelled here rather than read back from the
/// engine: a test that asked the engine for its own limit would agree with any
/// limit, including one an edit moved by three orders of magnitude.
const DEFAULT_BUDGET: u32 = 10_000;

fn budget(events: u32) -> NonZeroU32 {
    NonZeroU32::new(events).expect("a test budget takes at least one event")
}

fn started() -> (Engine<Policy>, Arc<dyn IScriptEngine>) {
    let script_engine: Arc<dyn IScriptEngine> = Arc::new(sce_rust_lua::LuaEngine::new());
    let mut engine = Engine::new(Policy::new(Arc::clone(&script_engine)));
    engine.set_clock(SceClock::Manual(0));
    engine.initialize();
    (engine, script_engine)
}

/// One host call that delivers `event`: the engine takes it off the queue it was
/// raised on, as the first event of the invocation that runs.
fn send(engine: &mut Engine<Policy>, event: Event) {
    engine.raise_external(event, "", "");
    engine.tick();
}

/// The fixture's `<assign>`s are the only witness of how far a chain got: every
/// outcome leaves the machine in a state the configuration alone cannot tell
/// apart from the others.
fn counter(engine: &Engine<Policy>, script_engine: &Arc<dyn IScriptEngine>, name: &str) -> i64 {
    sce_rust_runtime::helpers::datamodel_read::read_int(
        &**script_engine,
        engine.policy().session_id.as_deref(),
        name,
    )
    .unwrap_or_else(|| panic!("the fixture declares `{name}` in its datamodel"))
}

#[test]
fn the_default_budget_is_the_documented_one() {
    let (engine, _) = started();
    assert_eq!(engine.max_external_events_per_call(), DEFAULT_BUDGET);
    assert_eq!(engine.truncated_event_chains(), 0);
    assert_eq!(engine.last_truncated_event(), None);
}

/// This test returning at all is half the assertion: before the budget the call
/// did not.
#[test]
fn a_chain_that_cannot_end_is_cut_at_the_budget_and_the_call_returns() {
    let (mut engine, se) = started();

    send(&mut engine, Event::Spin);

    assert_eq!(
        engine.truncated_event_chains(),
        1,
        "the call handed control back with an event still queued, and said so; \
         without the count the host sees a machine that is running and has \
         returned, with no sign that anything went wrong"
    );
    // The host's own event is the first of the invocation, so the budget buys
    // the host's event and then `budget - 1` links.
    assert_eq!(
        counter(&engine, &se, "links"),
        i64::from(DEFAULT_BUDGET) - 1,
        "the chain must run exactly as far as the budget allows: fewer means the \
         call was cut early, more means the budget moved"
    );
    assert_eq!(
        engine.last_truncated_event(),
        Some(Event::Link),
        "the count says a call did not reach quiet; this says what it was still taking"
    );
    assert!(
        engine.is_running(),
        "the chain was cut, not the machine: the document is legal, and refusing to \
         run it forever is the engine's decision to report, not a reason to stop a \
         machine whose other states still work"
    );
}

/// The half that makes the count mean something: a chain that ends on its own is
/// not refused, however close to the budget it comes. `bounded` is the host's
/// event and five `lap`s, six in all.
#[test]
fn the_budget_is_exact_for_a_chain_that_ends_by_itself() {
    let (mut exactly, se) = started();
    exactly.set_max_external_events_per_call(budget(6));
    send(&mut exactly, Event::Bounded);
    assert_eq!(counter(&exactly, &se, "laps"), 5);
    assert_eq!(
        exactly.truncated_event_chains(),
        0,
        "a call that takes exactly the budget and empties the queue refused \
         nothing: a long chain is not a runaway"
    );
    assert_eq!(exactly.last_truncated_event(), None);

    let (mut one_short, se) = started();
    one_short.set_max_external_events_per_call(budget(5));
    send(&mut one_short, Event::Bounded);
    assert_eq!(
        counter(&one_short, &se, "laps"),
        4,
        "one `lap` was left queued"
    );
    assert_eq!(one_short.truncated_event_chains(), 1);
    assert_eq!(one_short.last_truncated_event(), Some(Event::Lap));
}

/// What the refusal did with the events it would not take: it left them queued.
/// An engine that dropped the queue stops short and never finishes; one that ran
/// the chain anyway finishes it in the first call.
#[test]
fn a_refused_call_leaves_the_queue_so_the_next_call_finishes_the_chain() {
    let (mut engine, se) = started();
    engine.set_max_external_events_per_call(budget(20));

    send(&mut engine, Event::Resume);
    assert_eq!(engine.truncated_event_chains(), 1);
    assert_eq!(
        counter(&engine, &se, "beats"),
        19,
        "the host's event and nineteen beats"
    );

    send(&mut engine, Event::Poke);
    assert_eq!(
        counter(&engine, &se, "beats"),
        30,
        "the second call took the beats the first left on the queue, each in a \
         budget of its own, and finished"
    );
    assert_eq!(
        counter(&engine, &se, "pokes"),
        1,
        "and the host's second event was heard"
    );
    assert_eq!(
        engine.truncated_event_chains(),
        1,
        "the second call ended the way the clause says: nothing more is counted"
    );
}

/// `delay="0ms"` is due at the instant being processed. This engine reads a
/// STATIC zero delay as undelayed and queues it straight to the external queue,
/// so the invocation bound holds it. What is held is that the call returns and
/// says it was cut; how many blinks were handled is not asserted.
#[test]
fn a_chain_through_a_static_delay_of_zero_does_not_keep_the_call_from_returning() {
    let (mut engine, se) = started();
    engine.set_max_external_events_per_call(budget(50));

    send(&mut engine, Event::Zero);

    assert!(engine.truncated_event_chains() >= 1);
    assert!(counter(&engine, &se, "blinks") >= 1);
    assert!(engine.is_running());
}

/// The same chain through `delayexpr="'0ms'"`, which has no static value this
/// engine could read as undelayed, so it goes through the scheduler: a due entry
/// is popped, its handler arms another that is due at the same reading, and the
/// tick that is popping finds it. Each pass takes one event, so a budget on the
/// drain alone never trips — this is the case ARCHITECTURE.md rule 5 exists for.
/// This test returning at all is the assertion.
#[test]
fn a_chain_through_a_delay_expression_that_is_zero_does_not_keep_the_tick_from_returning() {
    let (mut engine, se) = started();
    engine.set_max_external_events_per_call(budget(50));

    send(&mut engine, Event::ZeroExpr);
    assert_eq!(
        engine.truncated_event_chains(),
        0,
        "entering the state arms one entry and pops none: nothing has been cut yet"
    );

    engine.tick();

    assert_eq!(
        engine.truncated_event_chains(),
        1,
        "the tick popped entries due at its own reading until the budget, left the \
         due one waiting and said so"
    );
    assert_eq!(
        counter(&engine, &se, "exprs"),
        50,
        "the budget of pops at one reading, no more and no fewer"
    );
    assert_eq!(engine.last_truncated_event(), Some(Event::Blink));
    assert!(engine.is_running());
}

/// Eight pulses, each due one millisecond after the last. They are due at later
/// instants, so a legitimate time-driven workload is not a runaway however small
/// the budget: three here, against eight events.
#[test]
fn a_chain_that_is_finite_because_the_clock_is_is_not_refused() {
    let (mut walked, se) = started();
    walked.set_max_external_events_per_call(budget(3));

    send(&mut walked, Event::Timed);
    for _ in 0..8 {
        walked.advance_time_ms(1);
    }
    assert_eq!(counter(&walked, &se, "pulses"), 8);
    assert_eq!(
        walked.truncated_event_chains(),
        0,
        "each pulse came in a tick of its own, at an instant of its own"
    );

    let (mut jumped, js) = started();
    jumped.set_max_external_events_per_call(budget(3));
    send(&mut jumped, Event::Timed);
    jumped.advance_time_ms(8);
    assert_eq!(counter(&jumped, &js, "pulses"), 8);
    assert_eq!(
        jumped.truncated_event_chains(),
        0,
        "eight entries came due on the way to one reading, seven of them at \
         earlier instants: a clock that moved a long way is bounded by how far it \
         moved, and is not a chain that does not end"
    );
}

#[test]
fn a_host_chooses_the_budget() {
    let (mut engine, _) = started();
    engine.set_max_external_events_per_call(budget(7));
    assert_eq!(engine.max_external_events_per_call(), 7);
}
