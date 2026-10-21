// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The replay scenario: the engine-neutral scenario every backend's driver replays
//! against a generated `datamodel="sce-static"` machine (docs/SCE_ACCEPTED_SUBSET.md
//! §2.15, "What holds the scenarios to one meaning").
//!
//! A replay scenario names its `machine` and lists `steps`, each an event sent in
//! (or time moved on) and what the machine must hold after it: its state, its
//! active configuration, its variables as its saved state writes them, what it
//! asked of its host, the child hosts its host was asked for, what it sent its Mesh
//! peers, and whether it ended. It is read by seven drivers in six languages, and a
//! member one of them does not know is ignored by most and refused by the C one: a
//! key that is misspelt (`hostcalls`, `advance`) therefore passes in six engines,
//! which never look at it, and fails in the seventh, and a step that judged nothing
//! is a pass.
//!
//! A scenario of the standard host features (docs/SCE_HOST_FEATURES.md) is the
//! same format replayed against the reference host instead: its steps may give
//! the host directives and expect what the host was asked and holds, and nothing
//! a machine's own drivers read beyond its state and variables ([`Dialect`]).
//!
//! # Why the product owns this
//!
//! The format is SCE's, as the scenario set ([`crate::scenario_set`]) is: a project
//! that writes scenarios for its own machines checks them with `sce-codegen
//! scenarios`, without a runner, and every driver reads the members this module
//! names. This is the one place the vocabulary and the rules between its members
//! are held; `sce-build/tests/every_replay_scenario_keeps_to_its_vocabulary.rs`
//! holds every committed scenario to it.
//!
//! # Findings, not refusals
//!
//! Like a scenario set's, a replay scenario's problems are answered all at once
//! ([`problems`]), each with a JSON pointer to where it is and a code
//! ([`PROBLEM_CODES`]). With the machine's document in hand ([`MachineFacts`]),
//! the names a scenario uses are held to it too: the events it sends, the states
//! and variables it expects ([`machine_problems`]).

use std::collections::BTreeSet;

use serde_json::{Map, Value};

/// The members of a scenario. `router`, when `false`, says the driver registers no
/// Mesh router and no factory for the machine's children, so every send to a peer is
/// refused where it is made (`error.execution`): what a machine does on a device with
/// no link (docs/adr/0046, section 4). Absent or `true` is the router that records.
pub const TOP: &[&str] = &["machine", "about", "router", "steps"];

/// The members of a step. `invoke` sends the step's `event` to the running
/// child session of that invoke, through the Mesh router the child was given,
/// as a peer's event: a path of invoke ids joined by `/` for a child's child.
/// `restore` saves the machine and restores it from the text before the step's
/// event or time (docs/adr/0045).
pub const STEP: &[&str] = &[
    "note",
    "restore",
    "event",
    "data",
    "dropped",
    "invoke",
    "advance_ms",
    "expect",
];

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

/// The members of one entry of `sent`. `invoke` and `start` say which child
/// session sent it, told as `child_hosts` tells a child: the path of invoke ids
/// that started it, and how many children that path had started, from 1. An
/// entry without them is the machine's own.
const SENT: &[&str] = &["target", "event", "data", "invoke", "start"];

/// The members of a scenario of the standard host features
/// ([`Dialect::HostFeatures`]): their drivers give the machine the reference host
/// and no Mesh router to choose, so a scenario of them has no `router`.
pub const HOST_FEATURE_TOP: &[&str] = &["machine", "about", "steps"];

/// The members of a step of a scenario of the standard host features
/// ([`Dialect::HostFeatures`]): `host` sets what the reference host is like before
/// the step, and a step may move time and then send an event, in that order.
pub const HOST_FEATURE_STEP: &[&str] = &["note", "host", "advance_ms", "event", "data", "expect"];

/// The members of a step's `expect` in a scenario of the standard host features:
/// what the host was asked to start in the step (`host_invokes`) and what it holds
/// afterwards (`host_state`), beside the machine's state and variables.
pub const HOST_FEATURE_EXPECT: &[&str] = &["state", "variables", "host_invokes", "host_state"];

/// The directives a step's `host` may give the reference host
/// (docs/SCE_HOST_FEATURES.md): what its clock reads, what the platform and the
/// user say of a permission, the store's quota, the operations it fails, and what
/// its transport answers.
pub const HOST_DIRECTIVES: &[&str] = &[
    "clock",
    "permission",
    "user_decision",
    "quota",
    "fail",
    "http",
];

/// Which drivers replay a scenario, and so which members it may carry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialect {
    /// The seven drivers that replay a machine on its own, its host recording what
    /// it is asked (`sce-build/tests/fixtures/static_datamodel/scenarios`).
    Machine,
    /// The drivers that replay a machine against the reference host of the
    /// standard host features (docs/SCE_HOST_FEATURES.md,
    /// `tests/host_features/scenarios`).
    HostFeatures,
}

impl Dialect {
    /// The dialect `scenario` is written in: that of the standard host features
    /// when a step gives the host a directive or expects what the host was asked
    /// or holds — members no other driver reads — and the machine's otherwise.
    pub fn of(scenario: &Value) -> Self {
        let steps = scenario
            .get("steps")
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or_default();
        let host = steps.iter().any(|step| {
            step.get("host").is_some()
                || step.get("expect").is_some_and(|expect| {
                    expect.get("host_invokes").is_some() || expect.get("host_state").is_some()
                })
        });
        if host {
            Self::HostFeatures
        } else {
            Self::Machine
        }
    }

    /// The name a report gives the dialect.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Machine => "machine",
            Self::HostFeatures => "host-features",
        }
    }

    fn top_members(self) -> &'static [&'static str] {
        match self {
            Self::Machine => TOP,
            Self::HostFeatures => HOST_FEATURE_TOP,
        }
    }

    fn step_members(self) -> &'static [&'static str] {
        match self {
            Self::Machine => STEP,
            Self::HostFeatures => HOST_FEATURE_STEP,
        }
    }

    fn expect_members(self) -> &'static [&'static str] {
        match self {
            Self::Machine => EXPECT,
            Self::HostFeatures => HOST_FEATURE_EXPECT,
        }
    }
}

/// Every code a [`Problem`] carries, in the order this module raises them.
pub const PROBLEM_CODES: &[&str] = &[
    "not-an-object",
    "unknown-member",
    "missing-member",
    "wrong-type",
    "no-steps",
    "event-and-time",
    "needs-an-event",
    "dropped-child-event",
    "invoke-path",
    "read-after-end",
    "donedata-without-end",
    "send-target",
    "send-child",
    "start-from-one",
    "candidate-stem",
    "child-twice",
    "no-router",
    "machine-mismatch",
    "event-unmatched",
    "dropped-event-matches",
    "unknown-state",
    "unknown-variable",
];

/// One thing wrong with a scenario: where it is (a JSON pointer into the
/// scenario), which rule it breaks ([`PROBLEM_CODES`]) and why, in words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    pub path: String,
    pub code: &'static str,
    pub detail: String,
}

/// Every problem gathered in one walk, each at the pointer it is under.
struct Found(Vec<Problem>);

impl Found {
    fn push(&mut self, path: &str, code: &'static str, detail: impl Into<String>) {
        debug_assert!(PROBLEM_CODES.contains(&code), "an unlisted code: {code}");
        self.0.push(Problem {
            path: path.to_string(),
            code,
            detail: detail.into(),
        });
    }

    /// `value` as an object, or the problem that it is none.
    fn object<'v>(
        &mut self,
        value: &'v Value,
        path: &str,
        what: &str,
    ) -> Option<&'v Map<String, Value>> {
        let object = value.as_object();
        if object.is_none() {
            self.push(path, "not-an-object", format!("{what} is a JSON object"));
        }
        object
    }

    /// Each member of `object` that is not one of `allowed`.
    fn unknown(&mut self, object: &Map<String, Value>, allowed: &[&str], path: &str, what: &str) {
        for key in object.keys().filter(|key| !allowed.contains(&key.as_str())) {
            self.push(
                &format!("{path}/{}", pointer_token(key)),
                "unknown-member",
                format!(
                    "{what} carries `{key}`, which no driver replays (the members are: {})",
                    allowed.join(", ")
                ),
            );
        }
    }

    /// Whether the member `key` of `object`, when present, is of the kind `holds`
    /// accepts; a problem names `kind` when it is not.
    fn typed(
        &mut self,
        object: &Map<String, Value>,
        key: &str,
        path: &str,
        kind: &str,
        holds: impl Fn(&Value) -> bool,
    ) {
        if let Some(value) = object.get(key) {
            if !holds(value) {
                self.push(
                    &format!("{path}/{}", pointer_token(key)),
                    "wrong-type",
                    format!("`{key}` is {kind}"),
                );
            }
        }
    }
}

/// `key` as one token of a JSON pointer (RFC 6901).
fn pointer_token(key: &str) -> String {
    key.replace('~', "~0").replace('/', "~1")
}

/// Every reason `scenario` is not one every driver of its [`Dialect`] replays the
/// same way, in document order; empty when it is.
pub fn problems(scenario: &Value) -> Vec<Problem> {
    let dialect = Dialect::of(scenario);
    let mut found = Found(Vec::new());
    let Some(top) = found.object(scenario, "", "a scenario") else {
        return found.0;
    };
    found.unknown(top, dialect.top_members(), "", "the scenario");
    match top.get("machine") {
        None => found.push(
            "",
            "missing-member",
            "`machine` names the fixture, or the standard statechart, the scenario drives",
        ),
        Some(machine) if machine.as_str().is_none_or(str::is_empty) => found.push(
            "/machine",
            "wrong-type",
            "`machine` is the name of the fixture, or the standard statechart, the scenario \
             drives",
        ),
        Some(_) => {}
    }
    found.typed(top, "about", "", "text", Value::is_string);
    found.typed(
        top,
        "router",
        "",
        "`true` or `false`, whether the driver gives the machine one",
        Value::is_boolean,
    );
    // A scenario of the standard host features carries no `router` (found above as
    // a member its drivers do not read), so only the machine's dialect runs without one.
    let routed =
        dialect != Dialect::Machine || top.get("router").and_then(Value::as_bool) != Some(false);
    match top.get("steps").map(Value::as_array) {
        None => found.push("", "missing-member", "`steps` is the list of steps"),
        Some(None) => found.push("/steps", "wrong-type", "`steps` is an array"),
        Some(Some(steps)) if steps.is_empty() => found.push(
            "/steps",
            "no-steps",
            "a scenario with no steps judges nothing",
        ),
        Some(Some(steps)) => {
            for (n, step) in steps.iter().enumerate() {
                let path = format!("/steps/{n}");
                step_problems(&mut found, step, &path, dialect);
                if !routed {
                    without_router(&mut found, step, &path);
                }
            }
        }
    }
    found.0
}

/// The first of [`problems`], in words, or `Ok` when there is none — for a caller
/// that holds a scenario to the vocabulary and stops at the first reason.
pub fn check(scenario: &Value) -> Result<(), String> {
    match problems(scenario).into_iter().next() {
        Some(problem) => Err(format!(
            "{}: {}",
            display_path(&problem.path),
            problem.detail
        )),
        None => Ok(()),
    }
}

/// A pointer as a reader of a message names the place: `the scenario` for the
/// whole, and `step 3` where the pointer is under a step.
fn display_path(path: &str) -> String {
    if path.is_empty() {
        return "the scenario".to_string();
    }
    match path.strip_prefix("/steps/") {
        Some(rest) => {
            let (n, below) = rest.split_once('/').unwrap_or((rest, ""));
            if below.is_empty() {
                format!("step {n}")
            } else {
                format!("step {n} (/{below})")
            }
        }
        None => path.to_string(),
    }
}

/// What a step of a scenario run with no router may not state: a send to a peer is
/// refused before anything records it, so a `sent` would judge nothing, and a child
/// is given no router for a step's `invoke` to reach it through.
fn without_router(found: &mut Found, step: &Value, path: &str) {
    if step.get("invoke").is_some() {
        found.push(
            &format!("{path}/invoke"),
            "no-router",
            "a scenario with no router gives no child one, so no step reaches a child by `invoke`",
        );
    }
    if step
        .get("expect")
        .and_then(|expect| expect.get("sent"))
        .is_some()
    {
        found.push(
            &format!("{path}/expect/sent"),
            "no-router",
            "a scenario with no router records no send, so no step states what was `sent`",
        );
    }
}

fn step_problems(found: &mut Found, step: &Value, path: &str, dialect: Dialect) {
    let Some(step) = found.object(step, path, "a step") else {
        return;
    };
    found.unknown(step, dialect.step_members(), path, "the step");
    if let Some(directives) = step.get("host") {
        let at = format!("{path}/host");
        if let Some(directives) = found.object(directives, &at, "`host`") {
            found.unknown(directives, HOST_DIRECTIVES, &at, "`host`");
        }
    }
    found.typed(step, "note", path, "text", Value::is_string);
    // A step that does not restore leaves the member out: `false` would read as a
    // choice a driver has to make, and a driver that skipped the member would
    // make it by accident.
    found.typed(step, "restore", path, "`true`, or left out", |v| {
        v.as_bool() == Some(true)
    });
    found.typed(step, "event", path, "an event name", |v| {
        v.as_str().is_some_and(|name| !name.is_empty())
    });
    found.typed(step, "dropped", path, "true or false", Value::is_boolean);
    found.typed(
        step,
        "advance_ms",
        path,
        "a whole number of milliseconds",
        Value::is_u64,
    );
    // The host-features drivers move time and then send the event, all of them;
    // the machine's drivers do not agree on an order.
    if dialect == Dialect::Machine && step.contains_key("event") && step.contains_key("advance_ms")
    {
        found.push(
            path,
            "event-and-time",
            "a step sends an event or moves time, not both: the drivers do not agree on which \
             comes first",
        );
    }
    for key in ["data", "dropped", "invoke"] {
        if step.contains_key(key) && !step.contains_key("event") {
            found.push(
                &format!("{path}/{key}"),
                "needs-an-event",
                format!("`{key}` belongs to a step that sends an event"),
            );
        }
    }
    if let Some(invoke) = step.get("invoke") {
        invoke_path(
            found,
            invoke,
            &format!("{path}/invoke"),
            "the step's `invoke`",
        );
        // Whether an event is dropped is a question of the machine's own events;
        // a child's are not the machine's to judge.
        if step.contains_key("dropped") {
            found.push(
                &format!("{path}/dropped"),
                "dropped-child-event",
                "`dropped` belongs to an event the machine itself is sent",
            );
        }
    }
    match step.get("expect") {
        None => found.push(
            path,
            "missing-member",
            "a step states what it expects in `expect`",
        ),
        Some(expect) => expect_problems(found, expect, &format!("{path}/expect"), dialect),
    }
}

fn expect_problems(found: &mut Found, expect: &Value, path: &str, dialect: Dialect) {
    let Some(expect) = found.object(expect, path, "`expect`") else {
        return;
    };
    found.unknown(expect, dialect.expect_members(), path, "`expect`");
    found.typed(
        expect,
        "host_invokes",
        path,
        "an array of the requests the host was asked to start",
        Value::is_array,
    );
    found.typed(
        expect,
        "host_state",
        path,
        "an object of what the host holds",
        Value::is_object,
    );
    found.typed(expect, "state", path, "a state id", Value::is_string);
    found.typed(
        expect,
        "configuration",
        path,
        "an array of state ids",
        |v| {
            v.as_array()
                .is_some_and(|ids| ids.iter().all(Value::is_string))
        },
    );
    found.typed(
        expect,
        "variables",
        path,
        "an object of variable values",
        Value::is_object,
    );
    found.typed(expect, "ended", path, "true or false", Value::is_boolean);
    if expect.get("ended").and_then(Value::as_bool) == Some(true) {
        for key in expect
            .keys()
            .filter(|key| !AFTER_END.contains(&key.as_str()))
            .filter(|key| EXPECT.contains(&key.as_str()))
        {
            found.push(
                &format!("{path}/{}", pointer_token(key)),
                "read-after-end",
                format!("a machine that ended has no `{key}` to read"),
            );
        }
    } else if expect.contains_key("donedata") {
        found.push(
            &format!("{path}/donedata"),
            "donedata-without-end",
            "`donedata` belongs to a step that expects `ended`",
        );
    }
    if let Some(calls) = expect.get("host_calls") {
        host_call_problems(found, calls, &format!("{path}/host_calls"));
    }
    if let Some(children) = expect.get("child_hosts") {
        child_host_problems(found, children, &format!("{path}/child_hosts"));
    }
    if let Some(sends) = expect.get("sent") {
        sent_problems(found, sends, &format!("{path}/sent"));
    }
}

fn sent_problems(found: &mut Found, sends: &Value, path: &str) {
    let Some(sends) = sends.as_array() else {
        found.push(path, "wrong-type", "`sent` is an array");
        return;
    };
    for (n, send) in sends.iter().enumerate() {
        let at = format!("{path}/{n}");
        let Some(send) = found.object(send, &at, "a send") else {
            continue;
        };
        found.unknown(send, SENT, &at, "a send");
        // A peer is named `#name`; `#_internal`, `#_parent` and a session's
        // `#_scxml_<id>` are the SCXML processor's own, and what goes to them is
        // no host's to see.
        match send.get("target").map(Value::as_str) {
            None => found.push(&at, "missing-member", "a send names its `target`"),
            Some(Some(target))
                if target.starts_with('#') && target.len() >= 2 && !target.starts_with("#_") => {}
            Some(target) => found.push(
                &format!("{at}/target"),
                "send-target",
                format!(
                    "a send's `target` is a peer as the document writes it (`#name`), not `{}`",
                    target.unwrap_or("a value that is no text")
                ),
            ),
        }
        match send.get("event") {
            None => found.push(&at, "missing-member", "a send names its `event`"),
            Some(event) if !event.is_string() => found.push(
                &format!("{at}/event"),
                "wrong-type",
                "`event` is an event name",
            ),
            Some(_) => {}
        }
        // A child's send is told by the path that started it and the start; one
        // without the other names no child.
        match (send.get("invoke"), send.get("start")) {
            (None, None) => {}
            (Some(invoke), Some(start)) => {
                invoke_path(found, invoke, &format!("{at}/invoke"), "a send's `invoke`");
                if start.as_u64().is_none_or(|start| start < 1) {
                    found.push(
                        &format!("{at}/start"),
                        "start-from-one",
                        "a send's `start` counts the starts of its invoke from 1",
                    );
                }
            }
            _ => found.push(
                &at,
                "send-child",
                "a child's send states both its `invoke` and its `start`",
            ),
        }
    }
}

/// A path of invoke ids, joined by `/` for a child's child: an XML id cannot
/// contain `/`, so the join is unambiguous, and no id in it is empty.
fn invoke_path(found: &mut Found, path: &Value, at: &str, what: &str) {
    match path.as_str() {
        None => found.push(at, "invoke-path", format!("{what} is a path of invoke ids")),
        Some(text) if text.split('/').any(str::is_empty) => found.push(
            at,
            "invoke-path",
            format!("{what} `{text}` is invoke ids joined by `/`, none of them empty"),
        ),
        Some(_) => {}
    }
}

/// The calls a host was asked for, as `host_calls` writes them: of the machine's
/// own host, and of each child host in `child_hosts`.
fn host_call_problems(found: &mut Found, calls: &Value, path: &str) {
    let Some(calls) = calls.as_array() else {
        found.push(path, "wrong-type", "`host_calls` is an array");
        return;
    };
    for (n, call) in calls.iter().enumerate() {
        let at = format!("{path}/{n}");
        let Some(call) = found.object(call, &at, "a host call") else {
            continue;
        };
        found.unknown(call, HOST_CALL, &at, "a host call");
        if call.get("action").and_then(Value::as_str).is_none() {
            found.push(&at, "missing-member", "a host call names its `action`");
        }
        if call.get("args").and_then(Value::as_array).is_none() {
            found.push(
                &at,
                "missing-member",
                "a host call carries its `args` as an array",
            );
        }
    }
}

/// The child hosts a step states. A child is told by the invoke and the start, so
/// two entries of one step that name the same child are one child stated twice,
/// and the driver, which writes each child once, could never match both.
fn child_host_problems(found: &mut Found, children: &Value, path: &str) {
    let Some(children) = children.as_array() else {
        found.push(path, "wrong-type", "`child_hosts` is an array");
        return;
    };
    let mut seen: Vec<(&str, u64)> = Vec::new();
    for (n, child) in children.iter().enumerate() {
        let at = format!("{path}/{n}");
        let Some(child) = found.object(child, &at, "a child host") else {
            continue;
        };
        found.unknown(child, CHILD_HOST, &at, "a child host");
        let invoke = child.get("invoke").and_then(Value::as_str);
        if invoke.is_none() {
            found.push(
                &at,
                "missing-member",
                "a child host names the `invoke` that started its child",
            );
        }
        let start = child
            .get("start")
            .and_then(Value::as_u64)
            .filter(|start| *start >= 1);
        if start.is_none() {
            found.push(
                &at,
                "start-from-one",
                "a child host counts the `start` of its invoke from 1",
            );
        }
        if let Some(candidate) = child.get("candidate") {
            if candidate.as_str().is_none_or(str::is_empty) {
                found.push(
                    &format!("{at}/candidate"),
                    "candidate-stem",
                    "a child host's `candidate` is the stem of the document it started",
                );
            }
        }
        match child.get("host_calls") {
            None => found.push(
                &at,
                "missing-member",
                "a child host states the `host_calls` of its child",
            ),
            Some(calls) => host_call_problems(found, calls, &format!("{at}/host_calls")),
        }
        if let (Some(invoke), Some(start)) = (invoke, start) {
            if seen.contains(&(invoke, start)) {
                found.push(
                    &at,
                    "child-twice",
                    format!("`child_hosts` states the child of `{invoke}` start {start} twice"),
                );
            }
            seen.push((invoke, start));
        }
    }
}

/// What a scenario's names are held to, read from the machine's document: the
/// name the scenario must give it, its states, its variables, and the event
/// descriptors of its transitions.
#[derive(Debug, Clone, Default)]
pub struct MachineFacts {
    /// The name a scenario gives the machine, its last `/`-separated segment
    /// compared: the document's stem, which is what every driver reads the
    /// machine under.
    pub name: String,
    /// Every state id of the document, compound, parallel and final ones
    /// included.
    pub states: BTreeSet<String>,
    /// Every variable the document declares, top-level and per state: what a
    /// saved state writes and a step's `variables` reads.
    pub variables: BTreeSet<String>,
    /// Every event name the document writes — what a generated machine's table of
    /// its events holds.
    pub names: BTreeSet<String>,
    /// The `event` attribute of every transition, as written: a descriptor a name
    /// the machine is sent is matched against, `*` included.
    pub events: Vec<String>,
}

impl MachineFacts {
    /// The facts of `model`, a statechart read from a document whose stem is `name`.
    pub fn of(model: &crate::model::SCXMLModel, name: &str) -> Self {
        Self {
            name: name.to_string(),
            states: model.states.keys().cloned().collect(),
            variables: model
                .variables
                .iter()
                .chain(model.states.values().flat_map(|s| s.datamodel.iter()))
                .map(|v| v.id.clone())
                .collect(),
            names: model
                .events
                .iter()
                .filter(|event| !event.is_empty())
                .cloned()
                .collect(),
            events: model
                .states
                .values()
                .flat_map(|s| s.transitions.iter())
                .map(|t| t.event.clone())
                .filter(|event| !event.trim().is_empty())
                .collect(),
        }
    }

    /// Whether an event of this name is one the machine takes, as a generated
    /// engine delivers it: a name the document writes, or the longest token prefix
    /// of it that it writes, or one a transition's descriptor matches, its
    /// wildcard included. Anything else is dropped.
    fn matches(&self, event: &str) -> bool {
        let mut candidate = event;
        loop {
            if self.names.contains(candidate) {
                return true;
            }
            match candidate.rfind('.') {
                Some(at) => candidate = &candidate[..at],
                None => break,
            }
        }
        self.events
            .iter()
            .any(|attribute| crate::event_descriptor::attribute_matches(attribute, event))
    }
}

/// Every name `scenario` uses that `machine` does not have: a `machine` that names
/// another document, an event sent to the machine itself that no transition of it
/// matches (unless the step expects it `dropped`, or one it matches that the step
/// expects dropped), and a state or a variable the document does not declare.
/// The shape of the scenario is [`problems`]'s, and a member of the wrong kind is
/// passed over here.
pub fn machine_problems(scenario: &Value, machine: &MachineFacts) -> Vec<Problem> {
    let mut found = Found(Vec::new());
    if let Some(name) = scenario.get("machine").and_then(Value::as_str) {
        let last = name.rsplit('/').next().unwrap_or(name);
        if last != machine.name {
            found.push(
                "/machine",
                "machine-mismatch",
                format!(
                    "the scenario drives `{name}`, and the machine it is checked against is \
                     `{}`",
                    machine.name
                ),
            );
        }
    }
    let steps = scenario
        .get("steps")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();
    for (n, step) in steps.iter().enumerate() {
        let path = format!("/steps/{n}");
        if let Some(event) = step.get("event").and_then(Value::as_str) {
            // An event a step hands a child is the child's, not the machine's.
            if step.get("invoke").is_none() {
                let dropped = step.get("dropped").and_then(Value::as_bool) == Some(true);
                match (machine.matches(event), dropped) {
                    (false, false) => found.push(
                        &format!("{path}/event"),
                        "event-unmatched",
                        format!(
                            "no transition of `{}` matches `{event}`, so the machine drops it \
                             and the step judges nothing it did: name an event the machine \
                             takes, or say the step expects it dropped (`\"dropped\": true`)",
                            machine.name
                        ),
                    ),
                    (true, true) => found.push(
                        &format!("{path}/dropped"),
                        "dropped-event-matches",
                        format!(
                            "a transition of `{}` matches `{event}`, so it is not dropped",
                            machine.name
                        ),
                    ),
                    _ => {}
                }
            }
        }
        let Some(expect) = step.get("expect").and_then(Value::as_object) else {
            continue;
        };
        let mut state = |at: String, id: &str| {
            if !machine.states.contains(id) {
                found.push(
                    &at,
                    "unknown-state",
                    format!("`{}` declares no state `{id}`", machine.name),
                );
            }
        };
        if let Some(id) = expect.get("state").and_then(Value::as_str) {
            state(format!("{path}/expect/state"), id);
        }
        if let Some(ids) = expect.get("configuration").and_then(Value::as_array) {
            for (i, id) in ids.iter().enumerate() {
                if let Some(id) = id.as_str() {
                    state(format!("{path}/expect/configuration/{i}"), id);
                }
            }
        }
        if let Some(variables) = expect.get("variables").and_then(Value::as_object) {
            for name in variables.keys() {
                if !machine.variables.contains(name) {
                    found.push(
                        &format!("{path}/expect/variables/{}", pointer_token(name)),
                        "unknown-variable",
                        format!("`{}` declares no variable `{name}`", machine.name),
                    );
                }
            }
        }
    }
    found.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// The codes `scenario` is found to have, in order.
    fn codes(scenario: &Value) -> Vec<&'static str> {
        problems(scenario).into_iter().map(|p| p.code).collect()
    }

    /// The rules are held by what they refuse: each case is a scenario that
    /// breaks one, and a check that accepted everything would fail every one.
    #[test]
    fn each_rule_of_the_vocabulary_refuses_what_it_is_for() {
        let step = |extra: Value| {
            let mut base = json!({"expect": {"state": "idle"}});
            base.as_object_mut()
                .expect("an object")
                .extend(extra.as_object().expect("an object").clone());
            json!({"machine": "m", "steps": [base]})
        };
        let cases = [
            (
                json!({"machine": "m", "steps": [{"expect": {}}], "extra": 1}),
                "the scenario carries `extra`",
            ),
            (
                json!({"machine": "m", "steps": []}),
                "no steps judges nothing",
            ),
            (
                json!({"machine": "m", "steps": [{"expect": {"hostcalls": []}}]}),
                "`expect` carries `hostcalls`",
            ),
            (step(json!({"advance": 5})), "the step carries `advance`"),
            (
                step(json!({"restore": false})),
                "`restore` is `true`, or left out",
            ),
            (step(json!({"event": "go", "advance_ms": 5})), "not both"),
            (
                step(json!({"dropped": true})),
                "`dropped` belongs to a step that sends an event",
            ),
            (
                json!({"machine": "m", "steps": [{"event": "go"}]}),
                "states what it expects",
            ),
            (
                json!({"machine": "m", "steps": [{"expect": {"ended": true, "state": "s"}}]}),
                "a machine that ended has no `state`",
            ),
            (
                json!({"machine": "m", "steps": [{"expect": {"donedata": {}}}]}),
                "`donedata` belongs to a step that expects `ended`",
            ),
            (
                json!({"machine": "m", "steps": [{"expect": {"host_calls": [{"action": "a"}]}}]}),
                "carries its `args`",
            ),
            (
                json!({"machine": "m", "steps": [{"expect": {"host_calls": [{"action": "a", "args": [], "x": 1}]}}]}),
                "a host call carries `x`",
            ),
            (
                json!({"machine": "m", "steps": [{"expect": {"child_hosts": {}}}]}),
                "`child_hosts` is an array",
            ),
            (
                json!({"machine": "m", "steps": [{"expect": {"child_hosts": [{"start": 1, "host_calls": []}]}}]}),
                "names the `invoke`",
            ),
            (
                json!({"machine": "m", "steps": [{"expect": {"child_hosts": [{"invoke": "w", "host_calls": []}]}}]}),
                "counts the `start`",
            ),
            (
                json!({"machine": "m", "steps": [{"expect": {"child_hosts": [{"invoke": "w", "start": 0, "host_calls": []}]}}]}),
                "counts the `start`",
            ),
            (
                json!({"machine": "m", "steps": [{"expect": {"child_hosts": [{"invoke": "w", "start": 1}]}}]}),
                "states the `host_calls`",
            ),
            (
                json!({"machine": "m", "steps": [{"expect": {"child_hosts": [{"invoke": "w", "start": 1, "candidate": "", "host_calls": []}]}}]}),
                "the stem of the document",
            ),
            (
                json!({"machine": "m", "steps": [{"expect": {"child_hosts": [{"invoke": "w", "start": 1, "host_calls": [{"action": "a"}]}]}}]}),
                "carries its `args`",
            ),
            (
                json!({"machine": "m", "steps": [{"expect": {"child_hosts": [{"invoke": "w", "start": 1, "host_calls": [], "hosts": []}]}}]}),
                "a child host carries `hosts`",
            ),
            (
                json!({"machine": "m", "steps": [{"expect": {"child_hosts": [
                    {"invoke": "w", "start": 1, "host_calls": []},
                    {"invoke": "w", "start": 1, "host_calls": []}
                ]}}]}),
                "twice",
            ),
            (
                json!({"machine": "m", "steps": [{"expect": {"sent": {}}}]}),
                "`sent` is an array",
            ),
            (
                json!({"machine": "m", "steps": [{"expect": {"sent": [{"event": "e"}]}}]}),
                "a send names its `target`",
            ),
            (
                json!({"machine": "m", "steps": [{"expect": {"sent": [{"target": "#peer"}]}}]}),
                "a send names its `event`",
            ),
            (
                json!({"machine": "m", "steps": [{"expect": {"sent": [{"target": "peer", "event": "e"}]}}]}),
                "a peer as the document writes it",
            ),
            (
                json!({"machine": "m", "steps": [{"expect": {"sent": [{"target": "#_internal", "event": "e"}]}}]}),
                "a peer as the document writes it",
            ),
            (
                json!({"machine": "m", "steps": [{"expect": {"sent": [{"target": "#peer", "event": "e", "delay": 5}]}}]}),
                "a send carries `delay`",
            ),
            (
                step(json!({"invoke": "list"})),
                "`invoke` belongs to a step that sends an event",
            ),
            (
                step(json!({"event": "e", "invoke": "peer//inner"})),
                "none of them empty",
            ),
            (
                step(json!({"event": "e", "invoke": "list", "dropped": true})),
                "`dropped` belongs to an event the machine itself is sent",
            ),
            (
                json!({"machine": "m", "steps": [{"expect": {"sent": [{"target": "#peer", "event": "e", "invoke": "list"}]}}]}),
                "both its `invoke` and its `start`",
            ),
            (
                json!({"machine": "m", "steps": [{"expect": {"sent": [{"target": "#peer", "event": "e", "invoke": "list", "start": 0}]}}]}),
                "counts the starts of its invoke from 1",
            ),
            (
                step(json!({"advance_ms": "5"})),
                "a whole number of milliseconds",
            ),
            (
                json!({"machine": "m", "steps": [{"expect": {"configuration": "idle"}}]}),
                "an array of state ids",
            ),
            (
                json!({"steps": [{"expect": {}}]}),
                "`machine` names the fixture",
            ),
            (
                json!({"machine": "m", "router": "none", "steps": [{"expect": {}}]}),
                "`router` is `true` or `false`",
            ),
            (
                json!({"machine": "m", "router": false, "steps": [{"expect": {"sent": []}}]}),
                "records no send",
            ),
            (
                json!({"machine": "m", "router": false, "steps": [{"event": "e", "invoke": "list", "expect": {}}]}),
                "gives no child one",
            ),
        ];
        for (scenario, reason) in cases {
            match check(&scenario) {
                Ok(()) => {
                    panic!("accepted a scenario that should be refused for `{reason}`: {scenario}")
                }
                Err(why) => assert!(
                    why.contains(reason),
                    "refused for `{why}`, not for `{reason}`: {scenario}"
                ),
            }
        }
    }

    /// A scenario of the standard host features is held to the members its own
    /// drivers read: the host's directives and what the host was asked and holds,
    /// and a step that moves time and sends an event, which those drivers order.
    #[test]
    fn a_host_features_scenario_keeps_to_its_own_members() {
        let fits = json!({
            "machine": "static_host_kv",
            "steps": [
                {"host": {"quota": 2}, "advance_ms": 5, "event": "put", "data": {"k": "a"},
                 "expect": {"state": "idle", "host_invokes": [], "host_state": {"store": {}}}}
            ]
        });
        assert_eq!(Dialect::of(&fits), Dialect::HostFeatures);
        assert_eq!(problems(&fits), Vec::new());
        let unknown_directive = json!({
            "machine": "static_host_kv",
            "steps": [{"host": {"weather": "rain"}, "expect": {"host_invokes": []}}]
        });
        assert!(check(&unknown_directive)
            .unwrap_err()
            .contains("`host` carries `weather`"));
        let machine_member = json!({
            "machine": "static_host_kv",
            "steps": [{"host": {}, "expect": {"sent": []}}]
        });
        assert!(check(&machine_member)
            .unwrap_err()
            .contains("`expect` carries `sent`"));
    }

    /// A scenario run with no router (docs/adr/0046) is refused only what it could
    /// not judge, at the place it states it; one that keeps the router may state
    /// both, and a step that restores (docs/adr/0045) says so with `true`.
    #[test]
    fn a_scenario_with_no_router_states_no_send_and_reaches_no_child() {
        let offline = json!({
            "machine": "m",
            "router": false,
            "steps": [
                {"expect": {"state": "idle"}},
                {"restore": true, "event": "go", "expect": {"state": "idle"}}
            ]
        });
        assert_eq!(problems(&offline), Vec::new());
        let routed = json!({
            "machine": "m",
            "router": true,
            "steps": [{"event": "e", "invoke": "list", "expect": {"sent": []}}]
        });
        assert_eq!(problems(&routed), Vec::new());
        let unrouted = json!({
            "machine": "m",
            "router": false,
            "steps": [{"event": "e", "invoke": "list", "expect": {"sent": []}}]
        });
        let found: Vec<(&str, String)> = problems(&unrouted)
            .into_iter()
            .map(|p| (p.code, p.path))
            .collect();
        assert_eq!(
            found,
            vec![
                ("no-router", "/steps/0/invoke".to_string()),
                ("no-router", "/steps/0/expect/sent".to_string()),
            ]
        );
        // The drivers of the standard host features choose no router.
        let host = json!({
            "machine": "static_host_kv",
            "router": false,
            "steps": [{"host": {}, "expect": {"host_invokes": []}}]
        });
        assert_eq!(Dialect::of(&host), Dialect::HostFeatures);
        assert!(check(&host)
            .unwrap_err()
            .contains("the scenario carries `router`"));
    }

    #[test]
    fn every_problem_is_reported_not_only_the_first() {
        let scenario = json!({
            "machine": "m",
            "steps": [
                {"event": "go", "advance_ms": 5, "expect": {"hostcalls": []}},
                {"expect": {"sent": [{"target": "peer"}]}}
            ]
        });
        assert_eq!(
            codes(&scenario),
            vec![
                "event-and-time",
                "unknown-member",
                "send-target",
                "missing-member"
            ]
        );
        let paths: Vec<String> = problems(&scenario).into_iter().map(|p| p.path).collect();
        assert_eq!(
            paths,
            vec![
                "/steps/0",
                "/steps/0/expect/hostcalls",
                "/steps/1/expect/sent/0/target",
                "/steps/1/expect/sent/0"
            ]
        );
    }

    #[test]
    fn every_problem_code_is_made_by_an_input() {
        // Each code is raised by at least one case above or below: a code no input
        // makes is a rule nobody could break, or one this list lost track of.
        let machine = MachineFacts {
            name: "m".to_string(),
            states: ["idle".to_string()].into(),
            variables: ["count".to_string()].into(),
            names: ["go".to_string()].into(),
            events: vec!["go".to_string()],
        };
        let inputs = [
            json!(1),
            json!({"machine": "m", "steps": [{"expect": {}}], "x": 1}),
            json!({"steps": [{"expect": {}}]}),
            json!({"machine": 3, "steps": [{"expect": {}}]}),
            json!({"machine": "m", "steps": []}),
            json!({"machine": "m", "steps": [{"event": "go", "advance_ms": 1, "expect": {}}]}),
            json!({"machine": "m", "steps": [{"data": {}, "expect": {}}]}),
            json!({"machine": "m", "steps": [{"event": "e", "invoke": "c", "dropped": true, "expect": {}}]}),
            json!({"machine": "m", "steps": [{"event": "e", "invoke": "", "expect": {}}]}),
            json!({"machine": "m", "steps": [{"expect": {"ended": true, "state": "idle"}}]}),
            json!({"machine": "m", "steps": [{"expect": {"donedata": 1}}]}),
            json!({"machine": "m", "steps": [{"expect": {"sent": [{"target": "x", "event": "e"}]}}]}),
            json!({"machine": "m", "steps": [{"expect": {"sent": [{"target": "#p", "event": "e", "start": 1}]}}]}),
            json!({"machine": "m", "steps": [{"expect": {"child_hosts": [{"invoke": "w", "start": 0, "host_calls": []}]}}]}),
            json!({"machine": "m", "steps": [{"expect": {"child_hosts": [{"invoke": "w", "start": 1, "candidate": "", "host_calls": []}]}}]}),
            json!({"machine": "m", "steps": [{"expect": {"child_hosts": [
                {"invoke": "w", "start": 1, "host_calls": []},
                {"invoke": "w", "start": 1, "host_calls": []}]}}]}),
            json!({"machine": "m", "router": false, "steps": [{"expect": {"sent": []}}]}),
        ];
        let against_machine = [
            json!({"machine": "other", "steps": [{"expect": {}}]}),
            json!({"machine": "m", "steps": [{"event": "stop", "expect": {}}]}),
            json!({"machine": "m", "steps": [{"event": "go", "dropped": true, "expect": {}}]}),
            json!({"machine": "m", "steps": [{"expect": {"state": "busy"}}]}),
            json!({"machine": "m", "steps": [{"expect": {"variables": {"total": 1}}}]}),
        ];
        let made: BTreeSet<&str> = inputs
            .iter()
            .flat_map(problems)
            .chain(
                against_machine
                    .iter()
                    .flat_map(|s| machine_problems(s, &machine)),
            )
            .map(|p| p.code)
            .collect();
        for code in PROBLEM_CODES {
            assert!(made.contains(code), "no input makes `{code}`");
        }
    }

    #[test]
    fn the_names_a_scenario_uses_are_held_to_its_machine() {
        // `report` is a name the machine writes only where it sends it; `tick` and
        // `error.*` are its transitions' descriptors.
        let machine = MachineFacts {
            name: "static_counter".to_string(),
            states: ["counting".to_string(), "done".to_string()].into(),
            variables: ["count".to_string()].into(),
            names: ["tick".to_string(), "report".to_string()].into(),
            events: vec!["tick error.*".to_string()],
        };
        let fits = json!({
            "machine": "static_counter",
            "steps": [
                {"expect": {"state": "counting", "variables": {"count": 0}}},
                {"event": "tick", "expect": {"configuration": ["counting"]}},
                {"event": "error.execution", "expect": {}},
                {"event": "report.now", "expect": {}},
                {"event": "unknown", "dropped": true, "expect": {}}
            ]
        });
        assert_eq!(machine_problems(&fits, &machine), Vec::new());
        // A standard statechart is named by its library name, and read under its
        // last segment.
        let standard = MachineFacts {
            name: "record_list".to_string(),
            ..MachineFacts::default()
        };
        assert_eq!(
            machine_problems(
                &json!({"machine": "sce:std/shared/record_list", "steps": []}),
                &standard
            ),
            Vec::new()
        );
    }
}
