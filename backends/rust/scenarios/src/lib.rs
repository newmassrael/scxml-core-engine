// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! Replay of an engine-neutral scenario against the Rust engine
//! (`datamodel="sce-static"`, docs/SCE_ACCEPTED_SUBSET.md §2.15).
//!
//! A scenario (`sce-build/tests/fixtures/static_datamodel/scenarios/*.json`) is
//! data: a list of steps, each an external event with its payload and what the
//! machine must hold after it runs to quiescence — its current state, the set of
//! its active states (`configuration`), any of its variables and what it asked of
//! its host (`host_calls`). What the
//! machine holds is read from its saved state, the text
//! every backend saves byte for byte, so one scenario judges every backend by the
//! same answer and needs no per-type glue.

pub mod hosts;
pub mod integration;
pub mod machines;
pub mod vocabulary;
#[cfg(target_arch = "wasm32")]
pub mod wasm;

use sce_rust_runtime::saved_state::SavedState;
use sce_rust_runtime::{Engine, StatePolicy};
use serde_json::Value;

/// How many rounds a host gives a machine that needs `tick` after an event
/// before it reads the machine back. The engine reports no point at which it has
/// settled, so a host runs rounds: a child session takes an event the parent
/// forwarded in one round and its end reaches the parent in the next, which is
/// two for each level of child. Five is what the other backends' suites give a
/// machine, and a round that finds nothing to do changes nothing.
const SETTLE_TICKS: usize = 5;

/// Give the machine the rounds it needs before it is read back. The generator
/// says which call a machine needs: `step` drains the queues and nothing else,
/// and a machine with a delayed send or a child session is driven by `tick`,
/// which also runs those.
fn settle<P: StatePolicy>(engine: &mut Engine<P>) {
    if P::NEEDS_EVENT_SCHEDULER {
        for _ in 0..SETTLE_TICKS {
            engine.tick();
        }
    } else {
        engine.step();
    }
}

/// Replay `scenario` against `engine`, reading the machine back with
/// `save` after every step. An event name no event of the machine matches
/// fails the replay: the engine drops one silently, which would otherwise
/// pass a misspelt step without running it — unless the step says it expects
/// the drop (`"dropped": true`), which is then what is held. A name matches an
/// event of the machine as the engine delivers it
/// ([`StatePolicy::resolve_event_by_name`], §scxml-3.12.1): the document need
/// not write it.
///
/// A mismatch panics with the step and what differed: a test fails on it, and
/// on a target where a panic stops the module the message is what says why.
pub fn replay<P: StatePolicy>(
    engine: Engine<P>,
    save: impl Fn(&Engine<P>) -> SavedState,
    scenario: &str,
) {
    replay_recording(engine, save, None, scenario);
}

/// [`replay`] for a machine whose `<sce:action>`s go to a host that records them.
///
/// `host_calls` answers what the machine asked of its host since the last time
/// it was asked, oldest first, each as the scenario writes it
/// (`{"action": "showAttempts", "args": [0, false]}`, the arguments in the order
/// the document gives them). A step's `expect.host_calls` holds the calls made in
/// THAT step, the machine's start included in the first, and a step that states
/// none is not judged on them: they are not carried into the next step.
pub fn replay_with_host<P: StatePolicy>(
    engine: Engine<P>,
    save: impl Fn(&Engine<P>) -> SavedState,
    mut host_calls: impl FnMut(&Engine<P>) -> Vec<Value>,
    scenario: &str,
) {
    replay_recording(engine, save, Some(&mut host_calls), scenario);
}

/// What a driver offers to read what the machine asked of its host.
type HostCalls<'a, P> = Option<&'a mut dyn FnMut(&Engine<P>) -> Vec<Value>>;

fn replay_recording<P: StatePolicy>(
    mut engine: Engine<P>,
    save: impl Fn(&Engine<P>) -> SavedState,
    mut host_calls: HostCalls<'_, P>,
    scenario: &str,
) {
    let scenario: Value = serde_json::from_str(scenario).expect("a scenario is JSON");
    let steps = scenario["steps"].as_array().expect("steps");
    assert!(!steps.is_empty(), "a scenario with no steps judges nothing");
    engine.initialize();
    // A machine that starts a child session in its first state has that child
    // running, and maybe ended, before any event arrives; it is read as the
    // events leave it, after the rounds that take.
    if P::NEEDS_EVENT_SCHEDULER {
        settle(&mut engine);
    }
    for (n, step) in steps.iter().enumerate() {
        // A step that moves the machine's time on, for a scenario of a delayed
        // send: the engine is handed a manual clock by its caller, so the wait
        // is the one the step names and not the one the test happened to take.
        if let Some(ms) = step.get("advance_ms").and_then(Value::as_u64) {
            engine.advance_time_ms(ms);
        }
        if let Some(event) = step.get("event").and_then(Value::as_str) {
            let dropped = step.get("dropped").and_then(Value::as_bool) == Some(true);
            assert_eq!(
                P::resolve_event_by_name(event).is_none(),
                dropped,
                "step {n}: the machine's events {} `{event}`",
                if dropped { "match" } else { "do not match" }
            );
            let data = step.get("data").map(Value::to_string).unwrap_or_default();
            engine.raise_external_by_name(event, &data);
            settle(&mut engine);
        }
        let expect = &step["expect"];
        let note = step.get("note").and_then(Value::as_str).unwrap_or("");
        // What the machine asked of its host in this step. Read on every step so
        // that a step which states nothing does not hand its calls to the next.
        let asked = host_calls.as_mut().map(|read| read(&engine));
        match (expect.get("host_calls"), asked) {
            (Some(want), Some(got)) => assert_eq!(
                &Value::Array(got),
                want,
                "step {n} ({note}): what the machine asked of its host"
            ),
            (Some(_), None) => panic!(
                "step {n} ({note}): this driver records no host calls, so it cannot hold a step to them"
            ),
            (None, _) => {}
        }
        // A machine that ended in a top-level <final> has no saved state to
        // read — the save refuses one — so what a scenario can say of it is
        // that it ended, and that is the whole of the step.
        if expect.get("ended").and_then(Value::as_bool) == Some(true) {
            assert!(
                expect.get("state").is_none() && expect.get("variables").is_none(),
                "step {n} ({note}): an ended machine has no state or variables to read"
            );
            assert!(
                engine.is_in_final_state(),
                "step {n} ({note}): the machine ended in a top-level <final>"
            );
            // The data its <donedata> left for the invoking parent, as the
            // JSON the done event carries; compared as a value, so the order
            // the pairs were written in is not part of the answer.
            if let Some(want) = expect.get("donedata") {
                let got: Value = serde_json::from_str(engine.donedata_at_final())
                    .expect("the data of a done event is JSON");
                assert_eq!(
                    &got, want,
                    "step {n} ({note}): the data the final's done event carries"
                );
            }
            continue;
        }
        let saved: Value =
            serde_json::from_str(&save(&engine).to_json()).expect("a saved state is JSON");
        if let Some(state) = expect.get("state") {
            assert_eq!(
                &saved["current"], state,
                "step {n} ({note}): the current state"
            );
        }
        // Every active state, a compound or a parallel one with the atomic ones
        // below it. A set: the order the machine lists them in is not part of the
        // answer.
        if let Some(want) = expect.get("configuration") {
            assert_eq!(
                id_set(&saved["configuration"]),
                id_set(want),
                "step {n} ({note}): the active states"
            );
        }
        if let Some(variables) = expect.get("variables").and_then(Value::as_object) {
            for (name, want) in variables {
                let got = &saved["variables"][name];
                assert!(
                    holds(got, want),
                    "step {n} ({note}): variable `{name}` is {got}, not {want}"
                );
            }
        }
    }
}

/// The state ids of a JSON array, as a set.
fn id_set(ids: &Value) -> std::collections::BTreeSet<&str> {
    ids.as_array()
        .expect("a configuration is an array of state ids")
        .iter()
        .map(|id| id.as_str().expect("a state id is text"))
        .collect()
}

/// Whether a saved value is the scenario's. The saved state writes a 64-bit
/// integer as its decimal text (a JSON number past 2^53 is not exact
/// everywhere), so an expected number matches that text of the same
/// integer; everything else must be equal as written.
fn holds(got: &Value, want: &Value) -> bool {
    match (got, want) {
        (Value::String(text), Value::Number(n)) => {
            (n.is_i64() || n.is_u64()) && *text == n.to_string()
        }
        _ => got == want,
    }
}
