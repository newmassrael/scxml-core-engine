// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! Replay of an engine-neutral scenario against the Rust engine
//! (`datamodel="sce-static"`, docs/SCE_ACCEPTED_SUBSET.md §2.15).
//!
//! A scenario (`sce-build/tests/fixtures/static_datamodel/scenarios/*.json`) is
//! data: a list of steps, each an external event with its payload and what the
//! machine must hold after it runs to quiescence — its current state and any of
//! its variables. What the machine holds is read from its saved state, the text
//! every backend saves byte for byte, so one scenario judges every backend by the
//! same answer and needs no per-type glue.

pub mod integration;
pub mod machines;
#[cfg(target_arch = "wasm32")]
pub mod wasm;

use sce_rust_runtime::saved_state::SavedState;
use sce_rust_runtime::{Engine, StatePolicy};
use serde_json::Value;

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
    mut engine: Engine<P>,
    save: impl Fn(&Engine<P>) -> SavedState,
    scenario: &str,
) {
    let scenario: Value = serde_json::from_str(scenario).expect("a scenario is JSON");
    let steps = scenario["steps"].as_array().expect("steps");
    assert!(!steps.is_empty(), "a scenario with no steps judges nothing");
    engine.initialize();
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
            engine.step();
        }
        let expect = &step["expect"];
        let note = step.get("note").and_then(Value::as_str).unwrap_or("");
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
