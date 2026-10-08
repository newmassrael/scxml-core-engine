// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The members a scenario may carry, and the rules between them.
//!
//! A scenario is read by seven drivers written in six languages, and a member one
//! of them does not know is ignored by most and refused by the C one. A key that is
//! misspelt (`hostcalls`, `advance`) therefore passes in six engines, which never
//! look at it, and fails in the seventh: a step that judged nothing is a pass.
//! [`check`] is the one place the answer is held, and the table test of this crate
//! holds every committed scenario to it, so the vocabulary SCE_ACCEPTED_SUBSET.md
//! §2.15 describes is the one the files keep to.

use serde_json::{Map, Value};

/// The members of a scenario.
pub const TOP: &[&str] = &["machine", "about", "steps"];

/// The members of a step.
pub const STEP: &[&str] = &["note", "event", "data", "dropped", "advance_ms", "expect"];

/// The members of a step's `expect`.
pub const EXPECT: &[&str] = &[
    "state",
    "configuration",
    "variables",
    "ended",
    "donedata",
    "host_calls",
];

/// What a step that expects the machine ended may still state: a machine that
/// ended in a top-level `<final>` has no saved state, so no state, configuration
/// or variable can be read from it.
const AFTER_END: &[&str] = &["ended", "donedata", "host_calls"];

/// The members of one entry of `host_calls`.
const HOST_CALL: &[&str] = &["action", "args"];

/// Why `scenario` is not one every driver replays the same way, or `Ok` when it is.
pub fn check(scenario: &Value) -> Result<(), String> {
    let scenario = scenario.as_object().ok_or("a scenario is a JSON object")?;
    unknown(scenario, TOP, "the scenario")?;
    scenario
        .get("machine")
        .and_then(Value::as_str)
        .ok_or("`machine` names the fixture the scenario drives")?;
    let steps = scenario
        .get("steps")
        .and_then(Value::as_array)
        .ok_or("`steps` is an array")?;
    if steps.is_empty() {
        return Err("a scenario with no steps judges nothing".into());
    }
    for (n, step) in steps.iter().enumerate() {
        check_step(step).map_err(|why| format!("step {n}: {why}"))?;
    }
    Ok(())
}

fn check_step(step: &Value) -> Result<(), String> {
    let step = step.as_object().ok_or("a step is an object")?;
    unknown(step, STEP, "the step")?;
    if step.contains_key("event") && step.contains_key("advance_ms") {
        return Err(
            "a step sends an event or moves time, not both: the drivers do not \
                    agree on which comes first"
                .into(),
        );
    }
    for key in ["data", "dropped"] {
        if step.contains_key(key) && !step.contains_key("event") {
            return Err(format!("`{key}` belongs to a step that sends an event"));
        }
    }
    let expect = step
        .get("expect")
        .and_then(Value::as_object)
        .ok_or("a step states what it expects in `expect`")?;
    check_expect(expect)
}

fn check_expect(expect: &Map<String, Value>) -> Result<(), String> {
    unknown(expect, EXPECT, "`expect`")?;
    if expect.get("ended").and_then(Value::as_bool) == Some(true) {
        if let Some(key) = expect.keys().find(|key| !AFTER_END.contains(&key.as_str())) {
            return Err(format!("a machine that ended has no `{key}` to read"));
        }
    } else if expect.contains_key("donedata") {
        return Err("`donedata` belongs to a step that expects `ended`".into());
    }
    if let Some(calls) = expect.get("host_calls") {
        for call in calls.as_array().ok_or("`host_calls` is an array")? {
            let call = call.as_object().ok_or("a host call is an object")?;
            unknown(call, HOST_CALL, "a host call")?;
            call.get("action")
                .and_then(Value::as_str)
                .ok_or("a host call names its `action`")?;
            call.get("args")
                .and_then(Value::as_array)
                .ok_or("a host call carries its `args` as an array")?;
        }
    }
    Ok(())
}

/// A member of `object` that is not one of `allowed`, as the reason to refuse it.
fn unknown(object: &Map<String, Value>, allowed: &[&str], what: &str) -> Result<(), String> {
    match object.keys().find(|key| !allowed.contains(&key.as_str())) {
        Some(key) => Err(format!(
            "{what} carries `{key}`, which no driver replays (the members are: {})",
            allowed.join(", ")
        )),
        None => Ok(()),
    }
}
