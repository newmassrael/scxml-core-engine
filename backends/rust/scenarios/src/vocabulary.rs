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
    "child_hosts",
    "sent",
];

/// What a step that expects the machine ended may still state: a machine that
/// ended in a top-level `<final>` has no saved state, so no state, configuration
/// or variable can be read from it.
const AFTER_END: &[&str] = &["ended", "donedata", "host_calls", "child_hosts", "sent"];

/// The members of one entry of `host_calls`.
const HOST_CALL: &[&str] = &["action", "args"];

/// The members of one entry of `child_hosts`: the child host the machine's own
/// host was asked for, told by the invoke that started the child (`invoke`) and
/// how many children that invoke had started (`start`, from 1), the document it
/// started when the invoke names one of several (`candidate`), and what the child
/// asked of that host in the step (`host_calls`, as a machine's own are).
const CHILD_HOST: &[&str] = &["invoke", "start", "candidate", "host_calls"];

/// The members of one entry of `sent`.
const SENT: &[&str] = &["target", "event", "data"];

/// Why `scenario` is not one every driver replays the same way, or `Ok` when it is.
pub fn check(scenario: &Value) -> Result<(), String> {
    let scenario = scenario.as_object().ok_or("a scenario is a JSON object")?;
    unknown(scenario, TOP, "the scenario")?;
    scenario
        .get("machine")
        .and_then(Value::as_str)
        .ok_or("`machine` names the fixture, or the standard statechart, the scenario drives")?;
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
        check_host_calls(calls)?;
    }
    if let Some(children) = expect.get("child_hosts") {
        check_child_hosts(children)?;
    }
    if let Some(sends) = expect.get("sent") {
        for send in sends.as_array().ok_or("`sent` is an array")? {
            let send = send.as_object().ok_or("a send is an object")?;
            unknown(send, SENT, "a send")?;
            // A peer is named `#name`; `#_internal`, `#_parent` and a session's
            // `#_scxml_<id>` are the SCXML processor's own, and what goes to them is
            // no host's to see.
            let target = send
                .get("target")
                .and_then(Value::as_str)
                .ok_or("a send names its `target`")?;
            if !target.starts_with('#') || target.len() < 2 || target.starts_with("#_") {
                return Err(format!(
                    "a send's `target` is a peer as the document writes it (`#name`), not `{target}`"
                ));
            }
            send.get("event")
                .and_then(Value::as_str)
                .ok_or("a send names its `event`")?;
        }
    }
    Ok(())
}

/// The calls a host was asked for, as `host_calls` writes them: of the machine's
/// own host, and of each child host in `child_hosts`.
fn check_host_calls(calls: &Value) -> Result<(), String> {
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
    Ok(())
}

/// The child hosts a step states. A child is told by the invoke and the start, so
/// two entries of one step that name the same child are one child stated twice,
/// and the driver, which writes each child once, could never match both.
fn check_child_hosts(children: &Value) -> Result<(), String> {
    let mut seen = Vec::new();
    for child in children.as_array().ok_or("`child_hosts` is an array")? {
        let child = child.as_object().ok_or("a child host is an object")?;
        unknown(child, CHILD_HOST, "a child host")?;
        let invoke = child
            .get("invoke")
            .and_then(Value::as_str)
            .ok_or("a child host names the `invoke` that started its child")?;
        let start = child
            .get("start")
            .and_then(Value::as_u64)
            .filter(|start| *start >= 1)
            .ok_or("a child host counts the `start` of its invoke from 1")?;
        if let Some(candidate) = child.get("candidate") {
            candidate
                .as_str()
                .filter(|stem| !stem.is_empty())
                .ok_or("a child host's `candidate` is the stem of the document it started")?;
        }
        check_host_calls(
            child
                .get("host_calls")
                .ok_or("a child host states the `host_calls` of its child")?,
        )?;
        if seen.contains(&(invoke, start)) {
            return Err(format!(
                "`child_hosts` states the child of `{invoke}` start {start} twice"
            ));
        }
        seen.push((invoke, start));
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
