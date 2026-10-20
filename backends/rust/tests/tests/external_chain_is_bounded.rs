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
// The numbers (the default budget, each outcome's budget and counters) are read
// from tests/external_event_budget/budget_outcomes.json, the one table every
// engine's test of this contract is held to. They are read from that file and
// not from the engine: a test that asked the engine for its own limit would
// agree with any limit, including one an edit moved by three orders of
// magnitude.
//
// Fixture: tests/integration/external_chain_is_bounded.scxml. It is outside
// `integration_resources/` for the reason
// `scripts/regen_external_chain_is_bounded.sh` states.
//
// Regeneration (after fixture or template edit):
//   scripts/regen_external_chain_is_bounded.sh

use std::num::NonZeroU32;
use std::sync::{Arc, OnceLock};

use sce_rust_runtime::{Engine, IScriptEngine, SceClock};
use sce_rust_tests::integration::external_chain_is_bounded::{
    ExternalChainIsBoundedEvent as Event, ExternalChainIsBoundedPolicy as Policy,
};

/// The shared table of outcomes, read once from the repository's `tests/`.
fn table() -> &'static serde_json::Value {
    static TABLE: OnceLock<serde_json::Value> = OnceLock::new();
    TABLE.get_or_init(|| {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../tests/external_event_budget/budget_outcomes.json");
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        serde_json::from_str(&text).expect("the table is JSON")
    })
}

/// One whole number of the table, named by its keys from the root.
fn number(path: &[&str]) -> i64 {
    let mut at = table();
    for key in path {
        at = at
            .get(*key)
            .unwrap_or_else(|| panic!("the table has no `{}`", path.join(".")));
    }
    at.as_i64()
        .unwrap_or_else(|| panic!("`{}` is a whole number in the table", path.join(".")))
}

/// A number of the table that counts events or refusals, as the engine counts.
fn count(path: &[&str]) -> u32 {
    u32::try_from(number(path))
        .unwrap_or_else(|_| panic!("`{}` fits the engine's counter", path.join(".")))
}

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
    assert_eq!(
        engine.max_external_events_per_call(),
        count(&["default_budget"])
    );
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
        count(&["spin", "truncated"]),
        "the call handed control back with an event still queued, and said so; \
         without the count the host sees a machine that is running and has \
         returned, with no sign that anything went wrong"
    );
    // The host's own event is the first of the invocation, so the budget buys
    // the host's event and then `budget - 1` links.
    assert_eq!(
        counter(&engine, &se, "links"),
        number(&["default_budget"]) - 1,
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
    exactly.set_max_external_events_per_call(budget(count(&["exact", "budget"])));
    send(&mut exactly, Event::Bounded);
    assert_eq!(counter(&exactly, &se, "laps"), number(&["exact", "laps"]));
    assert_eq!(
        exactly.truncated_event_chains(),
        count(&["exact", "truncated"]),
        "a call that takes exactly the budget and empties the queue refused \
         nothing: a long chain is not a runaway"
    );
    assert_eq!(exactly.last_truncated_event(), None);

    let (mut one_short, se) = started();
    one_short.set_max_external_events_per_call(budget(count(&["one_short", "budget"])));
    send(&mut one_short, Event::Bounded);
    assert_eq!(
        counter(&one_short, &se, "laps"),
        number(&["one_short", "laps"]),
        "one `lap` was left queued"
    );
    assert_eq!(
        one_short.truncated_event_chains(),
        count(&["one_short", "truncated"])
    );
    assert_eq!(one_short.last_truncated_event(), Some(Event::Lap));
}

/// What the refusal did with the events it would not take: it left them queued.
/// An engine that dropped the queue stops short and never finishes; one that ran
/// the chain anyway finishes it in the first call.
#[test]
fn a_refused_call_leaves_the_queue_so_the_next_call_finishes_the_chain() {
    let (mut engine, se) = started();
    engine.set_max_external_events_per_call(budget(count(&["resume", "budget"])));

    send(&mut engine, Event::Resume);
    assert_eq!(
        engine.truncated_event_chains(),
        count(&["resume", "truncated"])
    );
    assert_eq!(
        counter(&engine, &se, "beats"),
        number(&["resume", "beats_after_first_call"]),
        "the host's event and nineteen beats"
    );

    send(&mut engine, Event::Poke);
    assert_eq!(
        counter(&engine, &se, "beats"),
        number(&["resume", "beats_after_second_call"]),
        "the second call took the beats the first left on the queue, each in a \
         budget of its own, and finished"
    );
    assert_eq!(
        counter(&engine, &se, "pokes"),
        number(&["resume", "pokes"]),
        "and the host's second event was heard"
    );
    assert_eq!(
        engine.truncated_event_chains(),
        count(&["resume", "truncated"]),
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
    engine.set_max_external_events_per_call(budget(count(&["static_zero", "budget"])));

    send(&mut engine, Event::Zero);

    assert!(engine.truncated_event_chains() >= count(&["static_zero", "truncated"]));
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
    engine.set_max_external_events_per_call(budget(count(&["zero_expr", "budget"])));

    send(&mut engine, Event::ZeroExpr);
    assert_eq!(
        engine.truncated_event_chains(),
        count(&["zero_expr", "truncated_after_entry"]),
        "entering the state arms one entry and pops none: nothing has been cut yet"
    );

    engine.tick();

    assert_eq!(
        engine.truncated_event_chains(),
        count(&["zero_expr", "truncated_after_tick"]),
        "the tick popped entries due at its own reading until the budget, left the \
         due one waiting and said so"
    );
    assert_eq!(
        counter(&engine, &se, "exprs"),
        number(&["zero_expr", "exprs"]),
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
    let pulses = number(&["timed", "pulses"]);
    let interval_ms = number(&["timed", "interval_ms"]);
    let step_ms = u64::try_from(interval_ms).expect("a clock step is not negative");
    // Each pulse is due the table's interval after the last, so one jump over
    // all of them is that many milliseconds for each pulse.
    let jump_ms = u64::try_from(pulses * interval_ms).expect("a clock jump is not negative");
    let timed_budget = budget(count(&["timed", "budget"]));

    let (mut walked, se) = started();
    walked.set_max_external_events_per_call(timed_budget);

    send(&mut walked, Event::Timed);
    for _ in 0..pulses {
        walked.advance_time_ms(step_ms);
    }
    assert_eq!(counter(&walked, &se, "pulses"), pulses);
    assert_eq!(
        walked.truncated_event_chains(),
        count(&["timed", "truncated"]),
        "each pulse came in a tick of its own, at an instant of its own"
    );

    let (mut jumped, js) = started();
    jumped.set_max_external_events_per_call(timed_budget);
    send(&mut jumped, Event::Timed);
    jumped.advance_time_ms(jump_ms);
    assert_eq!(counter(&jumped, &js, "pulses"), pulses);
    assert_eq!(
        jumped.truncated_event_chains(),
        count(&["timed", "truncated"]),
        "eight entries came due on the way to one reading, seven of them at \
         earlier instants: a clock that moved a long way is bounded by how far it \
         moved, and is not a chain that does not end"
    );
}

#[test]
fn a_host_chooses_the_budget() {
    let (mut engine, _) = started();
    engine.set_max_external_events_per_call(budget(count(&["host", "budget"])));
    assert_eq!(
        engine.max_external_events_per_call(),
        count(&["host", "budget"])
    );
}
