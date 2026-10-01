// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The judge: what a scenario set expects, set against what an engine driver
//! observed.
//!
//! A driver is a separate program. It takes a design and a scenario set, runs
//! each scenario, and writes an [observation trace](Trace): for every step of
//! every scenario, what the machine sent outward, whether it had finished,
//! which states were active, and the data it could read. This module reads
//! that trace and the set and says, scenario by scenario, whether the machine
//! did what the examples say. It runs nothing. The comparison lives here, once,
//! so a second driver on a second engine is a thin writer of traces and cannot
//! disagree with the first about what a result means.
//!
//! # Four verdicts, and why `not-judged` is not `fail`
//!
//! ```text
//!   pass          every check the scenario makes was observed and held
//!   fail          at least one check was observed and did not hold
//!   not-judged    nothing observed disagrees, but something the scenario
//!                 asks about could not be seen
//!   blocked /     the scenario's own status: it waits on a fact, or on a
//!   awaiting-     question for the owner. Never judged.
//!   decision
//! ```
//!
//! A driver that cannot see a channel must not make a design look wrong, and
//! must not make it look right. So a scenario that asks about a channel the
//! driver declares unavailable (`observes`) is `not-judged`, with the reason
//! the driver gave, and a mismatch on a channel that WAS observed is `fail`
//! even when other checks of the same scenario could not be judged: a failure
//! is conclusive, a gap is not.
//!
//! # What a pass says
//!
//! That the machine behaved as these examples say, on this engine, over these
//! inputs. Not that the design is right, not that the examples are the owner's
//! (`origin` says who wrote them and is repeated), and for a scenario that
//! carries a `bound`, only up to that bound.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{json, Value};

use crate::scenario_set::{ExpectedOutbound, Problem, Scalar, ScenarioSet, Status};

/// The stability status of `schemas/sce-observation-trace.v1.schema.json`,
/// held to the schema file's own header by `schema_file_declares_status`.
/// `SCE_WIRE_CONTRACTS.md` requires one commit to move both.
pub const TRACE_SCHEMA_STATUS: &str = "pre-release";

/// What the `record` field must say.
pub const TRACE_RECORD_KIND: &str = "sce-observation-trace";

/// The wire version this build reads.
pub const TRACE_VERSION: u32 = 1;

/// Why a file could not be read as an observation trace.
#[derive(Debug)]
pub enum TraceError {
    Read {
        path: String,
        source: std::io::Error,
    },
    Parse {
        path: String,
        source: serde_json::Error,
    },
    NotATrace {
        path: String,
        found: String,
    },
    UnsupportedVersion {
        path: String,
        found: String,
    },
    Shape {
        path: String,
        source: serde_json::Error,
    },
}

impl TraceError {
    /// Which refusal this is, without the path (see
    /// [`crate::scenario_set::ScenarioSetError::kind`]).
    pub fn kind(&self) -> &'static str {
        match self {
            TraceError::Read { .. } => "read",
            TraceError::Parse { .. } => "parse",
            TraceError::NotATrace { .. } => "not-an-observation-trace",
            TraceError::UnsupportedVersion { .. } => "unsupported-version",
            TraceError::Shape { .. } => "shape",
        }
    }
}

impl std::fmt::Display for TraceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TraceError::Read { path, source } => write!(f, "{path}: cannot be read: {source}"),
            TraceError::Parse { path, source } => write!(f, "{path}: is not JSON: {source}"),
            TraceError::NotATrace { path, found } => write!(
                f,
                "{path}: is not an observation trace: its `record` is {found}, and a trace's is \
                 \"{TRACE_RECORD_KIND}\""
            ),
            TraceError::UnsupportedVersion { path, found } => write!(
                f,
                "{path}: is an observation trace of version {found}, and this build reads \
                 version {TRACE_VERSION}"
            ),
            TraceError::Shape { path, source } => write!(
                f,
                "{path}: is an observation trace this build cannot read, so none of it is used \
                 rather than part of it: {source}"
            ),
        }
    }
}

impl std::error::Error for TraceError {}

/// The channel is observed. Only `true` is a value: a driver that cannot see
/// something says why, with [`Channel::Unavailable`], and never `false`.
#[derive(Debug, Clone, Copy)]
pub struct Seen;

impl<'de> Deserialize<'de> for Seen {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        if bool::deserialize(deserializer)? {
            Ok(Seen)
        } else {
            Err(serde::de::Error::custom(
                "a channel is observed (`true`) or unavailable with a reason \
                 (`{\"unavailable\": ...}`); `false` says neither",
            ))
        }
    }
}

/// A reason: text with something in it. An empty reason would tell a scenario
/// that a channel cannot be seen and not why.
#[derive(Debug, Clone)]
pub struct Reason(pub String);

impl<'de> Deserialize<'de> for Reason {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        if text.trim().is_empty() {
            Err(serde::de::Error::custom(
                "a reason is text, and this is empty",
            ))
        } else {
            Ok(Reason(text))
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Unavailable {
    pub unavailable: Reason,
}

/// Whether a driver can see one kind of thing at all.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum Channel {
    Observed(Seen),
    Unavailable(Unavailable),
}

impl Channel {
    /// The driver's reason when the channel cannot be seen.
    fn unavailable(&self) -> Option<&str> {
        match self {
            Channel::Observed(_) => None,
            Channel::Unavailable(gone) => Some(gone.unavailable.0.as_str()),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Engine {
    pub name: String,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Design {
    pub path: Option<String>,
    pub sha256: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetRef {
    pub sha256: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observes {
    pub outbound: Channel,
    pub finished: Channel,
    pub configuration: Channel,
    pub data: Channel,
}

/// An event the machine sent to something outside itself.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sent {
    pub event: String,
    pub payload: Option<BTreeMap<String, Scalar>>,
}

/// What was seen at the end of one step. A key is present exactly when its
/// channel is observed.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub outbound: Option<Vec<Sent>>,
    pub finished: Option<bool>,
    pub configuration: Option<Vec<String>>,
    pub data: Option<BTreeMap<String, Scalar>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Refused {
    pub why: Reason,
}

/// One scenario as a driver ran it, or declined to.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Run {
    pub scenario: String,
    pub observations: Option<Vec<Observation>>,
    pub refused: Option<Refused>,
}

/// An observation trace that has been read.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Trace {
    pub record: String,
    pub v: u32,
    pub engine: Engine,
    pub design: Option<Design>,
    pub scenario_set: Option<SetRef>,
    pub observes: Observes,
    /// Data names the driver knows it cannot read, each with why. A name left
    /// out of an observation's `data` is a gap either way; this is what lets
    /// the gap say a generator gave the item no reader, and not only that it
    /// was missed.
    #[serde(default)]
    pub unreadable: BTreeMap<String, Reason>,
    pub runs: Vec<Run>,
}

impl Trace {
    /// Read and shape-check an observation trace from a file.
    pub fn load(path: &Path) -> Result<Self, TraceError> {
        let display = path.display().to_string();
        let raw = std::fs::read_to_string(path).map_err(|source| TraceError::Read {
            path: display.clone(),
            source,
        })?;
        Self::from_json(&raw, &display)
    }

    /// The reading half, without the filesystem. The record and the version
    /// are looked at before the typed read, so a file of another kind is told
    /// it is of another kind.
    pub fn from_json(raw: &str, label: &str) -> Result<Self, TraceError> {
        let tree: Value = serde_json::from_str(raw).map_err(|source| TraceError::Parse {
            path: label.to_string(),
            source,
        })?;
        match tree.get("record").and_then(Value::as_str) {
            Some(TRACE_RECORD_KIND) => {}
            Some(other) => {
                return Err(TraceError::NotATrace {
                    path: label.to_string(),
                    found: format!("\"{other}\""),
                })
            }
            None => {
                return Err(TraceError::NotATrace {
                    path: label.to_string(),
                    found: "missing".to_string(),
                })
            }
        }
        match tree.get("v").and_then(Value::as_u64) {
            Some(v) if v == u64::from(TRACE_VERSION) => {}
            other => {
                return Err(TraceError::UnsupportedVersion {
                    path: label.to_string(),
                    found: other.map_or_else(|| "missing".to_string(), |v| v.to_string()),
                })
            }
        }
        serde_json::from_value(tree).map_err(|source| TraceError::Shape {
            path: label.to_string(),
            source,
        })
    }
}

/// The verdict on one scenario.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Verdict {
    Pass,
    Fail,
    NotJudged,
    Blocked,
    AwaitingDecision,
}

impl Verdict {
    pub fn as_str(self) -> &'static str {
        match self {
            Verdict::Pass => "pass",
            Verdict::Fail => "fail",
            Verdict::NotJudged => "not-judged",
            Verdict::Blocked => "blocked",
            Verdict::AwaitingDecision => "awaiting-decision",
        }
    }
}

/// A check that was observed and did not hold.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Failure {
    pub scenario: String,
    pub step: usize,
    /// `outbound`, `finished`, `condition` or `data`.
    pub check: &'static str,
    pub expected: Value,
    pub observed: Value,
}

/// A check that could not be judged, and why.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Gap {
    pub scenario: String,
    pub step: Option<usize>,
    pub check: Option<&'static str>,
    pub why: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ScenarioVerdict {
    pub id: String,
    pub verdict: Verdict,
    pub requirements: Vec<String>,
    /// Cycles a run went to, when the sentence claims something no run can
    /// show. A pass here holds up to that bound.
    pub bound: Option<u32>,
    /// Why, for a verdict that is not a pass or a fail.
    pub reason: Option<String>,
}

/// Everything the judge found.
#[derive(Debug, Clone, PartialEq)]
pub struct Judgement {
    /// Whether any scenario was judged. False when the set has problems or
    /// the trace was taken against another set: then there are no verdicts.
    pub judged: bool,
    pub set_problems: Vec<Problem>,
    pub trace_problems: Vec<Problem>,
    pub verdicts: Vec<ScenarioVerdict>,
    pub failures: Vec<Failure>,
    pub gaps: Vec<Gap>,
}

impl Judgement {
    /// How many scenarios have each verdict.
    pub fn count(&self, wanted: Verdict) -> usize {
        self.verdicts.iter().filter(|v| v.verdict == wanted).count()
    }
}

fn trace_problem(
    scenario: Option<&str>,
    path: String,
    code: &'static str,
    detail: String,
) -> Problem {
    Problem {
        scenario: scenario.map(str::to_string),
        path,
        code,
        detail,
    }
}

/// Every problem code this module reports about a trace.
pub const TRACE_PROBLEM_CODES: [&str; 6] = [
    "trace-for-another-set",
    "unknown-scenario",
    "duplicate-run",
    "run-shape",
    "observation-count",
    "observation-missing",
];

/// Judge `trace` against `set`. `set_digest` is the sha256 of the set's file,
/// compared with the one the trace names.
pub fn judge(set: &ScenarioSet, set_digest: &str, trace: &Trace) -> Judgement {
    let mut judgement = Judgement {
        judged: false,
        set_problems: set.problems(),
        trace_problems: Vec::new(),
        verdicts: Vec::new(),
        failures: Vec::new(),
        gaps: Vec::new(),
    };
    // A verdict from a set with problems is a verdict about nothing.
    if !judgement.set_problems.is_empty() {
        return judgement;
    }
    if let Some(named) = &trace.scenario_set {
        if named.sha256 != set_digest {
            judgement.trace_problems.push(trace_problem(
                None,
                "scenario_set.sha256".to_string(),
                "trace-for-another-set",
                format!(
                    "the trace was taken against a scenario set with digest {}, and the set \
                     given has digest {set_digest}",
                    named.sha256
                ),
            ));
            return judgement;
        }
    }
    judgement.judged = true;

    let known: BTreeSet<&str> = set.scenarios.iter().map(|s| s.id.as_str()).collect();
    let mut runs: BTreeMap<&str, &Run> = BTreeMap::new();
    for (index, run) in trace.runs.iter().enumerate() {
        let path = format!("runs[{index}]");
        if !known.contains(run.scenario.as_str()) {
            judgement.trace_problems.push(trace_problem(
                Some(&run.scenario),
                path,
                "unknown-scenario",
                format!("the set has no scenario called `{}`", run.scenario),
            ));
        } else if run.observations.is_some() == run.refused.is_some() {
            judgement.trace_problems.push(trace_problem(
                Some(&run.scenario),
                path,
                "run-shape",
                "a run carries its observations, or says why it was refused, and not both or \
                 neither"
                    .to_string(),
            ));
        } else if runs.contains_key(run.scenario.as_str()) {
            judgement.trace_problems.push(trace_problem(
                Some(&run.scenario),
                path,
                "duplicate-run",
                format!(
                    "`{}` has more than one run; the first is used",
                    run.scenario
                ),
            ));
        } else {
            runs.insert(run.scenario.as_str(), run);
        }
    }

    for scenario in &set.scenarios {
        let run = runs.get(scenario.id.as_str()).copied();
        let verdict = scenario_verdict(scenario, run, trace, &mut judgement);
        judgement.verdicts.push(verdict);
    }
    judgement
}

fn scenario_verdict(
    scenario: &crate::scenario_set::Scenario,
    run: Option<&Run>,
    trace: &Trace,
    judgement: &mut Judgement,
) -> ScenarioVerdict {
    let verdict = |verdict: Verdict, reason: Option<String>| ScenarioVerdict {
        id: scenario.id.clone(),
        verdict,
        requirements: scenario.requirements.clone(),
        bound: scenario.bound.as_ref().map(|bound| bound.cycles),
        reason,
    };
    // A scenario that cannot run is never judged, whatever the trace holds.
    match scenario.status {
        Status::Blocked => {
            return verdict(Verdict::Blocked, scenario.blocked_by.clone());
        }
        Status::AwaitingDecision => {
            return verdict(
                Verdict::AwaitingDecision,
                scenario.decision.as_ref().map(|d| d.question.clone()),
            );
        }
        Status::Runnable => {}
    }
    let not_judged = |judgement: &mut Judgement, why: String| {
        judgement.gaps.push(Gap {
            scenario: scenario.id.clone(),
            step: None,
            check: None,
            why: why.clone(),
        });
        verdict(Verdict::NotJudged, Some(why))
    };
    let Some(run) = run else {
        return not_judged(
            judgement,
            "the trace has no run for this scenario".to_string(),
        );
    };
    if let Some(refused) = &run.refused {
        return not_judged(
            judgement,
            format!("the driver did not run it: {}", refused.why.0),
        );
    }
    let observations = run.observations.as_deref().unwrap_or_default();
    if observations.len() != scenario.steps.len() {
        judgement.trace_problems.push(trace_problem(
            Some(&scenario.id),
            format!("runs[{}].observations", run_index(trace, &scenario.id)),
            "observation-count",
            format!(
                "the scenario has {} step(s) and the run carries {} observation(s): one per step",
                scenario.steps.len(),
                observations.len()
            ),
        ));
        return not_judged(
            judgement,
            "the run does not carry one observation per step".to_string(),
        );
    }

    let failures_before = judgement.failures.len();
    let gaps_before = judgement.gaps.len();
    for (index, (step, observed)) in scenario.steps.iter().zip(observations).enumerate() {
        let Some(expect) = &step.expect else { continue };
        let mut at = StepCheck {
            scenario: &scenario.id,
            step: index,
            observed,
            trace,
            judgement: &mut *judgement,
            run_at: run_index(trace, &scenario.id),
        };
        if let Some(expected) = &expect.outbound {
            at.outbound(expected);
        }
        if let Some(expected) = expect.finished {
            at.finished(expected);
        }
        if let Some(expected) = &expect.condition {
            at.condition(expected);
        }
        if let Some(expected) = &expect.data {
            at.data(expected);
        }
    }
    if judgement.failures.len() > failures_before {
        verdict(Verdict::Fail, None)
    } else if judgement.gaps.len() > gaps_before {
        verdict(
            Verdict::NotJudged,
            Some("a check could not be judged".to_string()),
        )
    } else {
        verdict(Verdict::Pass, None)
    }
}

/// Where in the trace a scenario's run is, for the path a problem names.
fn run_index(trace: &Trace, scenario: &str) -> usize {
    trace
        .runs
        .iter()
        .position(|run| run.scenario == scenario)
        .unwrap_or_default()
}

/// The checks of one step against what was seen at its end.
struct StepCheck<'a, 'j> {
    scenario: &'a str,
    step: usize,
    observed: &'a Observation,
    trace: &'a Trace,
    judgement: &'j mut Judgement,
    run_at: usize,
}

impl StepCheck<'_, '_> {
    fn gap(&mut self, check: &'static str, why: String) {
        self.judgement.gaps.push(Gap {
            scenario: self.scenario.to_string(),
            step: Some(self.step),
            check: Some(check),
            why,
        });
    }

    fn fail(&mut self, check: &'static str, expected: Value, observed: Value) {
        self.judgement.failures.push(Failure {
            scenario: self.scenario.to_string(),
            step: self.step,
            check,
            expected,
            observed,
        });
    }

    /// Whether the channel can be judged: the driver sees it, and the
    /// observation carries it. A channel the driver declares observed and an
    /// observation that leaves it out is a defect of the trace, said as such.
    fn channel(
        &mut self,
        check: &'static str,
        name: &str,
        channel: &Channel,
        present: bool,
    ) -> bool {
        if let Some(why) = channel.unavailable() {
            self.gap(check, format!("the driver cannot observe {name}: {why}"));
            return false;
        }
        if !present {
            self.judgement.trace_problems.push(trace_problem(
                Some(self.scenario),
                format!("runs[{}].observations[{}]", self.run_at, self.step),
                "observation-missing",
                format!("the trace declares {name} observed and this step carries none"),
            ));
            self.gap(
                check,
                format!(
                    "the trace declares {name} observed and step {} carries none",
                    self.step
                ),
            );
            return false;
        }
        true
    }

    fn outbound(&mut self, expected: &[ExpectedOutbound]) {
        let (trace, seen) = (self.trace, self.observed);
        let present = seen.outbound.is_some();
        if !self.channel(
            "outbound",
            "events sent outward",
            &trace.observes.outbound,
            present,
        ) {
            return;
        }
        let observed = seen.outbound.as_deref().unwrap_or_default();
        let holds = expected.len() == observed.len()
            && expected.iter().zip(observed).all(|(want, got)| {
                want.event == got.event
                    && want.payload.as_ref().is_none_or(|fields| {
                        fields.iter().all(|(name, value)| {
                            got.payload
                                .as_ref()
                                .and_then(|payload| payload.get(name))
                                .is_some_and(|seen| value.same_value(seen))
                        })
                    })
            });
        if !holds {
            let wanted: Vec<Value> = expected
                .iter()
                .map(|e| event_json(&e.event, e.payload.as_ref()))
                .collect();
            let got: Vec<Value> = observed
                .iter()
                .map(|s| event_json(&s.event, s.payload.as_ref()))
                .collect();
            self.fail("outbound", Value::Array(wanted), Value::Array(got));
        }
    }

    fn finished(&mut self, expected: bool) {
        let (trace, seen) = (self.trace, self.observed);
        let present = seen.finished.is_some();
        if !self.channel(
            "finished",
            "whether the machine finished",
            &trace.observes.finished,
            present,
        ) {
            return;
        }
        if seen.finished != Some(expected) {
            self.fail("finished", json!(expected), json!(seen.finished));
        }
    }

    fn condition(&mut self, expected: &str) {
        let (trace, seen) = (self.trace, self.observed);
        let present = seen.configuration.is_some();
        if !self.channel(
            "condition",
            "the active states",
            &trace.observes.configuration,
            present,
        ) {
            return;
        }
        let observed = seen.configuration.as_deref().unwrap_or_default();
        if !observed.iter().any(|state| state == expected) {
            self.fail("condition", json!(expected), json!(observed));
        }
    }

    fn data(&mut self, expected: &BTreeMap<String, Scalar>) {
        let (trace, seen) = (self.trace, self.observed);
        let present = seen.data.is_some();
        if !self.channel("data", "data values", &trace.observes.data, present) {
            return;
        }
        let empty = BTreeMap::new();
        let observed = seen.data.as_ref().unwrap_or(&empty);
        for (name, want) in expected {
            match observed.get(name) {
                None => {
                    let step = self.step;
                    let why = trace
                        .unreadable
                        .get(name)
                        .map(|reason| format!(": {}", reason.0))
                        .unwrap_or_default();
                    self.gap(
                        "data",
                        format!("the driver could not read `{name}` at step {step}{why}"),
                    );
                }
                Some(got) if !want.same_value(got) => {
                    self.fail("data", one_field(name, want), one_field(name, got));
                }
                Some(_) => {}
            }
        }
    }
}

/// `{name: value}`, for a failure that names the one data item that differs.
fn one_field(name: &str, value: &Scalar) -> Value {
    let mut map = serde_json::Map::new();
    map.insert(
        name.to_string(),
        serde_json::to_value(value).unwrap_or(Value::Null),
    );
    Value::Object(map)
}

fn event_json(event: &str, payload: Option<&BTreeMap<String, Scalar>>) -> Value {
    match payload {
        Some(fields) => json!({ "event": event, "payload": fields }),
        None => json!({ "event": event }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenario_set::digest_of;

    const SCHEMA: &str = include_str!("../../schemas/sce-observation-trace.v1.schema.json");
    const RETRY_SET: &str =
        include_str!("../tests/fixtures/scenario_sets/retry-client.scenarios.json");
    const RETRY_TRACE: &str =
        include_str!("../tests/fixtures/scenario_sets/retry-client.trace.json");

    /// One change to a value, named for what it does.
    type Case = (&'static str, Box<dyn Fn(&mut Value)>);

    /// A change to the observation of `S1`, and the one check it must break:
    /// the step, the kind of check, what was expected and what was observed.
    type Break = (
        &'static str,
        Box<dyn Fn(&mut Value)>,
        usize,
        &'static str,
        Value,
        Value,
    );

    fn schema() -> Value {
        serde_json::from_str(SCHEMA).expect("observation trace schema is JSON")
    }

    fn violations(instance: &Value) -> Vec<String> {
        let schema = schema();
        let validator = jsonschema::JSONSchema::options()
            .with_draft(jsonschema::Draft::Draft7)
            .compile(&schema)
            .expect("observation trace schema compiles");
        let outcome = validator.validate(instance);
        match outcome {
            Ok(()) => Vec::new(),
            Err(errors) => errors.map(|e| e.to_string()).collect(),
        }
    }

    /// A set with a scenario that asserts every kind of thing, one that is
    /// blocked and one that waits on a decision.
    fn small_set() -> Value {
        json!({
            "record": "sce-scenario-set", "v": 1,
            "specification": {"doc_id": "d", "rev": "1"},
            "origin": "ai-proposed",
            "interface": {
                "inputs": [{"name": "go", "payload": {"n": "integer"}}],
                "outputs": [{"name": "done", "payload": {"n": "integer"}}],
                "conditions": ["busy"],
                "data": ["count"]
            },
            "scenarios": [
                {"id": "S1", "quote": "q", "requirements": ["R1"], "steps": [
                    {"send": "go", "payload": {"n": 1}, "expect": {
                        "outbound": [{"event": "done", "payload": {"n": 1}}],
                        "condition": "busy", "finished": false, "data": {"count": 1}}},
                    {"advance_ms": 5, "expect": {"outbound": [], "finished": true}}
                ]},
                {"id": "S2", "quote": "q", "status": "blocked", "blocked_by": "a fact",
                 "steps": [{"send": "go", "payload": {"n": 1}}]},
                {"id": "S3", "quote": "q", "status": "awaiting-decision",
                 "decision": {"question": "which?"},
                 "steps": [{"send": "go", "payload": {"n": 1}}]}
            ]
        })
    }

    /// What a correct driver writes for `S1` of [`small_set`].
    fn base_trace() -> Value {
        json!({
            "record": "sce-observation-trace", "v": 1,
            "engine": {"name": "test-driver"},
            "observes": {"outbound": true, "finished": true, "configuration": true, "data": true},
            "runs": [{"scenario": "S1", "observations": [
                {"outbound": [{"event": "done", "payload": {"n": 1}}], "finished": false,
                 "configuration": ["root", "busy"], "data": {"count": 1}},
                {"outbound": [], "finished": true, "configuration": ["root"], "data": {"count": 1}}
            ]}]
        })
    }

    fn judged_with(set: &Value, trace: &Value) -> Judgement {
        let text = set.to_string();
        let set = ScenarioSet::from_json(&text, "set").expect("the set is readable");
        let trace = Trace::from_json(&trace.to_string(), "trace").expect("the trace is readable");
        judge(&set, &digest_of(text.as_bytes()), &trace)
    }

    fn judged(trace: &Value) -> Judgement {
        judged_with(&small_set(), trace)
    }

    fn verdict_of(judgement: &Judgement, id: &str) -> Verdict {
        judgement
            .verdicts
            .iter()
            .find(|v| v.id == id)
            .unwrap_or_else(|| panic!("no verdict for {id}"))
            .verdict
    }

    fn change(mutate: impl Fn(&mut Value)) -> Judgement {
        let mut trace = base_trace();
        mutate(&mut trace);
        judged(&trace)
    }

    fn only_failure(judgement: &Judgement) -> &Failure {
        assert_eq!(judgement.failures.len(), 1, "{:?}", judgement.failures);
        &judgement.failures[0]
    }

    #[test]
    fn schema_file_declares_status() {
        assert_eq!(
            schema()["x-sce-schema-status"].as_str(),
            Some(TRACE_SCHEMA_STATUS),
            "schemas/sce-observation-trace.v1.schema.json x-sce-schema-status disagrees with \
             TRACE_SCHEMA_STATUS; SCE_WIRE_CONTRACTS.md requires one commit to move both",
        );
    }

    #[test]
    fn schema_version_and_record_match_the_reader() {
        let schema = schema();
        assert_eq!(
            schema["properties"]["v"]["const"].as_u64(),
            Some(u64::from(TRACE_VERSION))
        );
        assert_eq!(
            schema["properties"]["record"]["const"].as_str(),
            Some(TRACE_RECORD_KIND)
        );
    }

    /// A machine that does what the retry specification says, written out by
    /// hand, passes every scenario of the retry set. The counts are pinned: a
    /// judge that quietly skipped scenarios would pass with fewer.
    #[test]
    fn every_trace_the_product_reads_validates_against_the_wire_schema() {
        let instance: Value = serde_json::from_str(RETRY_TRACE).expect("fixture is JSON");
        assert_eq!(violations(&instance), Vec::<String>::new());
        let mut shown = base_trace();
        assert_eq!(violations(&shown), Vec::<String>::new());
        shown["observes"]["data"] = json!({"unavailable": "the design declares no data"});
        assert_eq!(violations(&shown), Vec::<String>::new());

        let set = ScenarioSet::from_json(RETRY_SET, "retry").expect("fixture set");
        let trace = Trace::from_json(RETRY_TRACE, "retry trace").expect("fixture trace");
        let judgement = judge(&set, &digest_of(RETRY_SET.as_bytes()), &trace);
        assert!(judgement.judged);
        assert_eq!(judgement.set_problems, Vec::new());
        assert_eq!(judgement.trace_problems, Vec::new());
        assert_eq!(judgement.failures, Vec::new());
        assert_eq!(judgement.gaps, Vec::new());
        assert_eq!(judgement.verdicts.len(), 6);
        assert_eq!(judgement.count(Verdict::Pass), 6);
    }

    #[test]
    fn a_retry_machine_that_sends_a_fourth_request_fails_the_scenario_that_forbids_it() {
        let set = ScenarioSet::from_json(RETRY_SET, "retry").expect("fixture set");
        let mut trace: Value = serde_json::from_str(RETRY_TRACE).expect("fixture trace");
        // At 600 ms the correct machine reports the timeout; this one retries.
        trace["runs"][2]["observations"][4]["outbound"] = json!([{"event": "SendRequest"}]);
        trace["runs"][2]["observations"][4]["finished"] = json!(false);
        let trace = Trace::from_json(&trace.to_string(), "trace").expect("readable");
        let judgement = judge(&set, &digest_of(RETRY_SET.as_bytes()), &trace);
        assert_eq!(verdict_of(&judgement, "T3-at-most-three"), Verdict::Fail);
        assert_eq!(
            judgement.count(Verdict::Pass),
            5,
            "the other five are untouched"
        );
        let first = &judgement.failures[0];
        assert_eq!(
            (first.scenario.as_str(), first.step, first.check),
            ("T3-at-most-three", 4, "outbound")
        );
        assert_eq!(first.expected, json!([{"event": "TimeoutError"}]));
        assert_eq!(first.observed, json!([{"event": "SendRequest"}]));
    }

    #[test]
    fn a_retry_machine_that_forgets_to_stop_its_timer_fails_the_absence_scenario() {
        let set = ScenarioSet::from_json(RETRY_SET, "retry").expect("fixture set");
        let mut trace: Value = serde_json::from_str(RETRY_TRACE).expect("fixture trace");
        // T5: after the response nothing may be sent however long the clock runs.
        trace["runs"][4]["observations"][2]["outbound"] = json!([{"event": "SendRequest"}]);
        let trace = Trace::from_json(&trace.to_string(), "trace").expect("readable");
        let judgement = judge(&set, &digest_of(RETRY_SET.as_bytes()), &trace);
        assert_eq!(
            verdict_of(&judgement, "T5-response-stops-the-timer"),
            Verdict::Fail
        );
        let failure = only_failure(&judgement);
        assert_eq!((failure.step, failure.check), (2, "outbound"));
        assert_eq!(failure.expected, json!([]));
    }

    #[test]
    fn a_correct_trace_passes_and_a_scenario_that_cannot_run_is_not_judged() {
        let judgement = judged(&base_trace());
        assert!(judgement.judged);
        assert_eq!(verdict_of(&judgement, "S1"), Verdict::Pass);
        assert_eq!(verdict_of(&judgement, "S2"), Verdict::Blocked);
        assert_eq!(verdict_of(&judgement, "S3"), Verdict::AwaitingDecision);
        assert_eq!(judgement.failures, Vec::new());
        assert_eq!(judgement.gaps, Vec::new());
        let s2 = judgement
            .verdicts
            .iter()
            .find(|v| v.id == "S2")
            .expect("S2");
        assert_eq!(s2.reason.as_deref(), Some("a fact"));
        let s3 = judgement
            .verdicts
            .iter()
            .find(|v| v.id == "S3")
            .expect("S3");
        assert_eq!(s3.reason.as_deref(), Some("which?"));
        let s1 = judgement
            .verdicts
            .iter()
            .find(|v| v.id == "S1")
            .expect("S1");
        assert_eq!(s1.requirements, vec!["R1".to_string()]);
    }

    #[test]
    fn a_run_for_a_scenario_that_cannot_run_changes_nothing() {
        let judgement = change(|trace| {
            trace["runs"].as_array_mut().unwrap().push(json!({
                "scenario": "S2",
                "observations": [{"outbound": [], "finished": true, "configuration": [], "data": {}}]
            }));
        });
        assert_eq!(verdict_of(&judgement, "S2"), Verdict::Blocked);
        assert_eq!(judgement.trace_problems, Vec::new());
    }

    /// One change to the observation of S1, and the one check it must break.
    #[test]
    fn each_check_that_does_not_hold_is_a_failure_at_its_step() {
        let cases: Vec<Break> = vec![
            (
                "an event that was not expected is sent too",
                Box::new(|t| {
                    t["runs"][0]["observations"][1]["outbound"] = json!([{"event": "done"}]);
                }),
                1,
                "outbound",
                json!([]),
                json!([{"event": "done"}]),
            ),
            (
                "the expected event is not sent",
                Box::new(|t| {
                    t["runs"][0]["observations"][0]["outbound"] = json!([]);
                }),
                0,
                "outbound",
                json!([{"event": "done", "payload": {"n": 1}}]),
                json!([]),
            ),
            (
                "another event is sent",
                Box::new(|t| {
                    t["runs"][0]["observations"][0]["outbound"] =
                        json!([{"event": "other", "payload": {"n": 1}}]);
                }),
                0,
                "outbound",
                json!([{"event": "done", "payload": {"n": 1}}]),
                json!([{"event": "other", "payload": {"n": 1}}]),
            ),
            (
                "the event carries another value",
                Box::new(|t| {
                    t["runs"][0]["observations"][0]["outbound"][0]["payload"]["n"] = json!(2);
                }),
                0,
                "outbound",
                json!([{"event": "done", "payload": {"n": 1}}]),
                json!([{"event": "done", "payload": {"n": 2}}]),
            ),
            (
                "the event carries no payload at all",
                Box::new(|t| {
                    t["runs"][0]["observations"][0]["outbound"] = json!([{"event": "done"}]);
                }),
                0,
                "outbound",
                json!([{"event": "done", "payload": {"n": 1}}]),
                json!([{"event": "done"}]),
            ),
            (
                "the machine has not finished when it should have",
                Box::new(|t| {
                    t["runs"][0]["observations"][1]["finished"] = json!(false);
                }),
                1,
                "finished",
                json!(true),
                json!(false),
            ),
            (
                "the machine is not in the named condition",
                Box::new(|t| {
                    t["runs"][0]["observations"][0]["configuration"] = json!(["root"]);
                }),
                0,
                "condition",
                json!("busy"),
                json!(["root"]),
            ),
            (
                "a data item holds another value",
                Box::new(|t| {
                    t["runs"][0]["observations"][0]["data"]["count"] = json!(2);
                }),
                0,
                "data",
                json!({"count": 1}),
                json!({"count": 2}),
            ),
            (
                "a data item holds text where a number is expected",
                Box::new(|t| {
                    t["runs"][0]["observations"][0]["data"]["count"] = json!("1");
                }),
                0,
                "data",
                json!({"count": 1}),
                json!({"count": "1"}),
            ),
        ];
        // The base shows the control: nothing in it fails.
        assert_eq!(judged(&base_trace()).failures, Vec::new());
        for (what, mutate, step, check, expected, observed) in &cases {
            let judgement = change(mutate);
            assert_eq!(verdict_of(&judgement, "S1"), Verdict::Fail, "{what}");
            let failure = only_failure(&judgement);
            assert_eq!(failure.scenario, "S1", "{what}");
            assert_eq!((failure.step, failure.check), (*step, *check), "{what}");
            assert_eq!(&failure.expected, expected, "{what}");
            assert_eq!(&failure.observed, observed, "{what}");
        }
        assert_eq!(cases.len(), 9);
    }

    /// An expected event with no payload checks nothing about the payload, an
    /// expected payload is a subset of what was sent, and numbers are compared
    /// as numbers.
    #[test]
    fn a_payload_is_checked_only_as_far_as_the_scenario_states_it() {
        let mut set = small_set();
        set["scenarios"][0]["steps"][0]["expect"]["outbound"] = json!([{"event": "done"}]);
        let mut trace = base_trace();
        trace["runs"][0]["observations"][0]["outbound"] =
            json!([{"event": "done", "payload": {"n": 99}}]);
        assert_eq!(verdict_of(&judged_with(&set, &trace), "S1"), Verdict::Pass);

        // a subset of the fields observed
        let mut trace = base_trace();
        trace["runs"][0]["observations"][0]["outbound"][0]["payload"]["extra"] = json!("seen");
        assert_eq!(verdict_of(&judged(&trace), "S1"), Verdict::Pass);

        // 1 and 1.0 are one value; 1 and "1" are not
        let mut trace = base_trace();
        trace["runs"][0]["observations"][0]["outbound"][0]["payload"]["n"] = json!(1.0);
        trace["runs"][0]["observations"][0]["data"]["count"] = json!(1.0);
        assert_eq!(verdict_of(&judged(&trace), "S1"), Verdict::Pass);
    }

    #[test]
    fn events_are_compared_in_order_and_in_number() {
        let mut set = small_set();
        set["interface"]["outputs"] = json!([{"name": "a"}, {"name": "b"}]);
        set["scenarios"][0]["steps"][0]["expect"] =
            json!({"outbound": [{"event": "a"}, {"event": "b"}]});
        set["scenarios"][0]["steps"][1]["expect"] = json!({"finished": true});
        let mut trace = base_trace();
        trace["runs"][0]["observations"][0]["outbound"] = json!([{"event": "a"}, {"event": "b"}]);
        assert_eq!(verdict_of(&judged_with(&set, &trace), "S1"), Verdict::Pass);
        trace["runs"][0]["observations"][0]["outbound"] = json!([{"event": "b"}, {"event": "a"}]);
        assert_eq!(verdict_of(&judged_with(&set, &trace), "S1"), Verdict::Fail);
        trace["runs"][0]["observations"][0]["outbound"] = json!([{"event": "a"}]);
        assert_eq!(verdict_of(&judged_with(&set, &trace), "S1"), Verdict::Fail);
        trace["runs"][0]["observations"][0]["outbound"] =
            json!([{"event": "a"}, {"event": "b"}, {"event": "a"}]);
        assert_eq!(verdict_of(&judged_with(&set, &trace), "S1"), Verdict::Fail);
    }

    /// A driver that cannot see a channel does not make the design look wrong
    /// or right: what asks about it is not judged, with the driver's reason,
    /// and what does not ask about it is judged as ever.
    #[test]
    fn a_channel_the_driver_cannot_see_is_a_gap_and_not_a_failure() {
        let judgement = change(|t| {
            t["observes"]["outbound"] = json!({"unavailable": "the route of every output is open"});
            for step in t["runs"][0]["observations"].as_array_mut().unwrap() {
                step.as_object_mut().unwrap().remove("outbound");
            }
        });
        assert_eq!(verdict_of(&judgement, "S1"), Verdict::NotJudged);
        assert_eq!(judgement.failures, Vec::new());
        let reasons: Vec<&str> = judgement.gaps.iter().map(|g| g.why.as_str()).collect();
        assert!(
            reasons
                .iter()
                .all(|why| why.contains("the route of every output is open")),
            "{reasons:?}"
        );
        assert!(judgement.gaps.iter().all(|g| g.check == Some("outbound")));
        assert_eq!(
            judgement.trace_problems,
            Vec::new(),
            "declared unavailable is not a defect"
        );

        // a scenario that never asks about outbound events is judged as ever
        let mut set = small_set();
        set["scenarios"][0]["steps"][0]["expect"] = json!({"condition": "busy"});
        set["scenarios"][0]["steps"][1]["expect"] = json!({"finished": true});
        let mut trace = base_trace();
        trace["observes"]["outbound"] = json!({"unavailable": "route open"});
        for step in trace["runs"][0]["observations"].as_array_mut().unwrap() {
            step.as_object_mut().unwrap().remove("outbound");
        }
        assert_eq!(verdict_of(&judged_with(&set, &trace), "S1"), Verdict::Pass);
    }

    /// A failure is conclusive and a gap is not: one check that was observed
    /// and did not hold makes the scenario a fail even when another could not
    /// be judged.
    #[test]
    fn a_failure_outranks_a_gap_in_the_same_scenario() {
        let judgement = change(|t| {
            t["observes"]["configuration"] = json!({"unavailable": "no state names"});
            t["runs"][0]["observations"][0]
                .as_object_mut()
                .unwrap()
                .remove("configuration");
            t["runs"][0]["observations"][1]["finished"] = json!(false);
        });
        assert_eq!(verdict_of(&judgement, "S1"), Verdict::Fail);
        assert_eq!(judgement.failures.len(), 1);
        assert_eq!(judgement.gaps.len(), 1);
    }

    #[test]
    fn a_channel_declared_observed_and_missing_from_a_step_is_a_defect_of_the_trace() {
        let judgement = change(|t| {
            t["runs"][0]["observations"][0]
                .as_object_mut()
                .unwrap()
                .remove("finished");
        });
        assert_eq!(verdict_of(&judgement, "S1"), Verdict::NotJudged);
        assert_eq!(judgement.failures, Vec::new());
        let codes: Vec<&str> = judgement.trace_problems.iter().map(|p| p.code).collect();
        assert_eq!(codes, vec!["observation-missing"]);
        assert_eq!(judgement.trace_problems[0].path, "runs[0].observations[0]");
    }

    #[test]
    fn a_data_item_the_driver_could_not_read_is_a_gap_and_a_wrong_one_is_a_failure() {
        let judgement = change(|t| {
            t["runs"][0]["observations"][0]["data"] = json!({});
        });
        assert_eq!(verdict_of(&judgement, "S1"), Verdict::NotJudged);
        assert!(
            judgement.gaps[0].why.contains("`count`"),
            "{:?}",
            judgement.gaps
        );
        assert_eq!(judgement.failures, Vec::new());
    }

    #[test]
    fn a_data_item_the_driver_says_it_cannot_read_carries_its_reason_into_the_gap() {
        let judgement = change(|t| {
            t["runs"][0]["observations"][0]["data"] = json!({});
            t["unreadable"] = json!({"count": "the generator gave it no reader"});
        });
        assert_eq!(verdict_of(&judgement, "S1"), Verdict::NotJudged);
        assert!(
            judgement.gaps[0]
                .why
                .ends_with(": the generator gave it no reader"),
            "{:?}",
            judgement.gaps
        );
        assert_eq!(judgement.failures, Vec::new());

        // A reason about another name is not this name's.
        let judgement = change(|t| {
            t["runs"][0]["observations"][0]["data"] = json!({});
            t["unreadable"] = json!({"other": "the generator gave it no reader"});
        });
        assert!(
            !judgement.gaps[0].why.contains("no reader"),
            "{:?}",
            judgement.gaps
        );
    }

    #[test]
    fn a_run_the_driver_refused_or_never_made_is_not_judged_and_says_why() {
        let judgement = change(|t| {
            t["runs"][0] = json!({"scenario": "S1", "refused": {"why": "the design has no transition on `go`"}});
        });
        assert_eq!(verdict_of(&judgement, "S1"), Verdict::NotJudged);
        let s1 = judgement
            .verdicts
            .iter()
            .find(|v| v.id == "S1")
            .expect("S1");
        assert!(s1
            .reason
            .as_deref()
            .unwrap()
            .contains("no transition on `go`"));

        let judgement = change(|t| {
            t["runs"] = json!([]);
        });
        assert_eq!(verdict_of(&judgement, "S1"), Verdict::NotJudged);
        assert!(
            judgement.gaps[0].why.contains("no run"),
            "{:?}",
            judgement.gaps
        );
    }

    #[test]
    fn a_scenario_that_is_not_judged_is_never_counted_as_a_pass() {
        let judgement = change(|t| {
            t["runs"] = json!([]);
        });
        assert_eq!(judgement.count(Verdict::Pass), 0);
        assert_eq!(judgement.count(Verdict::NotJudged), 1);
        assert_eq!(judgement.count(Verdict::Blocked), 1);
        assert_eq!(judgement.count(Verdict::AwaitingDecision), 1);
    }

    /// Every problem code the judge reports about a trace is made by an input.
    #[test]
    fn every_trace_problem_code_is_made_by_an_input() {
        let cases: Vec<Case> = vec![
            (
                "trace-for-another-set",
                Box::new(|t| {
                    t["scenario_set"] = json!({"sha256": "0".repeat(64)});
                }),
            ),
            (
                "unknown-scenario",
                Box::new(|t| {
                    t["runs"]
                        .as_array_mut()
                        .unwrap()
                        .push(json!({"scenario": "nope", "refused": {"why": "x"}}));
                }),
            ),
            (
                "duplicate-run",
                Box::new(|t| {
                    let again = t["runs"][0].clone();
                    t["runs"].as_array_mut().unwrap().push(again);
                }),
            ),
            (
                "run-shape",
                Box::new(|t| {
                    t["runs"][0]["refused"] = json!({"why": "and also observations"});
                }),
            ),
            (
                "observation-count",
                Box::new(|t| {
                    t["runs"][0]["observations"].as_array_mut().unwrap().pop();
                }),
            ),
            (
                "observation-missing",
                Box::new(|t| {
                    t["runs"][0]["observations"][0]
                        .as_object_mut()
                        .unwrap()
                        .remove("data");
                }),
            ),
        ];
        let mut produced = BTreeSet::new();
        for (code, mutate) in &cases {
            let judgement = change(mutate);
            let found: Vec<&str> = judgement.trace_problems.iter().map(|p| p.code).collect();
            assert!(
                found.contains(code),
                "the input made for `{code}` produced {found:?}"
            );
            produced.insert(*code);
        }
        let listed: BTreeSet<&str> = TRACE_PROBLEM_CODES.iter().copied().collect();
        assert_eq!(
            listed.len(),
            TRACE_PROBLEM_CODES.len(),
            "a code is listed twice"
        );
        assert_eq!(produced, listed);
    }

    #[test]
    fn the_first_of_two_runs_for_one_scenario_is_the_one_used() {
        let judgement = change(|t| {
            let mut wrong = t["runs"][0].clone();
            wrong["observations"][1]["finished"] = json!(false);
            t["runs"].as_array_mut().unwrap().push(wrong);
        });
        assert_eq!(verdict_of(&judgement, "S1"), Verdict::Pass);
        assert_eq!(judgement.trace_problems.len(), 1);
    }

    #[test]
    fn a_trace_taken_against_another_set_judges_nothing() {
        let judgement = change(|t| {
            t["scenario_set"] = json!({"sha256": "f".repeat(64)});
        });
        assert!(!judgement.judged);
        assert_eq!(judgement.verdicts, Vec::new());
        assert_eq!(judgement.trace_problems[0].code, "trace-for-another-set");

        // naming the right set, or no set at all, is judged
        let text = small_set().to_string();
        let mut trace = base_trace();
        trace["scenario_set"] = json!({"sha256": digest_of(text.as_bytes())});
        assert!(judged(&trace).judged);
        assert!(judged(&base_trace()).judged);
    }

    /// A verdict from a set with problems would be a verdict about nothing.
    #[test]
    fn a_set_with_problems_is_not_judged() {
        let mut set = small_set();
        set["scenarios"][0]["steps"][0]["send"] = json!("not-an-input");
        let judgement = judged_with(&set, &base_trace());
        assert!(!judgement.judged);
        assert_eq!(judgement.verdicts, Vec::new());
        assert_eq!(judgement.set_problems[0].code, "unknown-input");
    }

    #[test]
    fn a_bound_is_carried_to_the_verdict() {
        let mut set = small_set();
        set["scenarios"][0]["bound"] = json!({"cycles": 5});
        let judgement = judged_with(&set, &base_trace());
        let s1 = judgement
            .verdicts
            .iter()
            .find(|v| v.id == "S1")
            .expect("S1");
        assert_eq!((s1.verdict, s1.bound), (Verdict::Pass, Some(5)));
    }

    #[test]
    fn a_file_that_is_not_an_observation_trace_is_refused_by_kind() {
        let refusals: Vec<(&str, String)> = vec![
            ("parse", "{ nope".to_string()),
            (
                "not-an-observation-trace",
                json!({"record": "sce-scenario-set", "v": 1}).to_string(),
            ),
            ("not-an-observation-trace", json!({"v": 1}).to_string()),
            (
                "unsupported-version",
                json!({"record": TRACE_RECORD_KIND, "v": 2}).to_string(),
            ),
            ("shape", {
                let mut trace = base_trace();
                trace["surprise"] = json!(1);
                trace.to_string()
            }),
            ("shape", {
                let mut trace = base_trace();
                trace["observes"]["outbound"] = json!(false);
                trace.to_string()
            }),
            ("shape", {
                let mut trace = base_trace();
                trace["runs"][0]["observations"][0]["data"]["count"] = Value::Null;
                trace.to_string()
            }),
        ];
        for (kind, text) in &refusals {
            let error = Trace::from_json(text, "t.json").expect_err("refused");
            assert_eq!(error.kind(), *kind, "{error}");
            assert!(error.to_string().starts_with("t.json:"), "{error}");
        }
    }

    /// What the schema refuses the reader must not take in silence: it either
    /// refuses the file or the judge finds a problem with it.
    #[test]
    fn the_wire_schema_rejects_what_the_reader_does_not_accept() {
        let cases: Vec<Case> = vec![
            (
                "a key the format does not have",
                Box::new(|t| {
                    t["surprise"] = json!(true);
                }),
            ),
            (
                "another record",
                Box::new(|t| {
                    t["record"] = json!("sce-scenario-set");
                }),
            ),
            (
                "another version",
                Box::new(|t| {
                    t["v"] = json!(2);
                }),
            ),
            (
                "no engine",
                Box::new(|t| {
                    t.as_object_mut().unwrap().remove("engine");
                }),
            ),
            (
                "an engine with no name",
                Box::new(|t| {
                    t["engine"] = json!({});
                }),
            ),
            (
                "a channel that is neither seen nor unavailable",
                Box::new(|t| {
                    t["observes"]["data"] = json!(false);
                }),
            ),
            (
                "an unavailable channel with no reason",
                Box::new(|t| {
                    t["observes"]["data"] = json!({"unavailable": ""});
                }),
            ),
            (
                "no statement about a channel",
                Box::new(|t| {
                    t["observes"].as_object_mut().unwrap().remove("finished");
                }),
            ),
            (
                "a run with both observations and a refusal",
                Box::new(|t| {
                    t["runs"][0]["refused"] = json!({"why": "both"});
                }),
            ),
            (
                "a run with neither",
                Box::new(|t| {
                    t["runs"][0] = json!({"scenario": "S1"});
                }),
            ),
            (
                "an observation key the format does not have",
                Box::new(|t| {
                    t["runs"][0]["observations"][0]["mood"] = json!("fine");
                }),
            ),
            (
                "a sent event with no name",
                Box::new(|t| {
                    t["runs"][0]["observations"][0]["outbound"] = json!([{"payload": {"n": 1}}]);
                }),
            ),
            (
                "a null data value",
                Box::new(|t| {
                    t["runs"][0]["observations"][0]["data"]["count"] = Value::Null;
                }),
            ),
            (
                "a set digest that is not a digest",
                Box::new(|t| {
                    t["scenario_set"] = json!({"sha256": "not hex"});
                }),
            ),
            (
                "a refusal with no reason",
                Box::new(|t| {
                    t["runs"][0] = json!({"scenario": "S1", "refused": {}});
                }),
            ),
            (
                "an unreadable data name with no reason",
                Box::new(|t| {
                    t["unreadable"] = json!({"count": ""});
                }),
            ),
            (
                "unreadable data names given as a list",
                Box::new(|t| {
                    t["unreadable"] = json!(["count"]);
                }),
            ),
        ];
        // Each case starts from a trace shown valid and changes one thing.
        assert_eq!(violations(&base_trace()), Vec::<String>::new());
        for (what, mutate) in &cases {
            let mut trace = base_trace();
            mutate(&mut trace);
            assert!(!violations(&trace).is_empty(), "the schema accepted {what}");
            let set = small_set();
            let handled = match Trace::from_json(&trace.to_string(), "trace") {
                Err(_) => true,
                Ok(read) => {
                    let text = set.to_string();
                    let set = ScenarioSet::from_json(&text, "set").expect("readable");
                    let judgement = judge(&set, &digest_of(text.as_bytes()), &read);
                    !judgement.trace_problems.is_empty() || !judgement.gaps.is_empty()
                }
            };
            assert!(
                handled,
                "the schema refuses {what}, and the reader took it without a word"
            );
        }
        assert_eq!(cases.len(), 17, "every case ran");
    }
}
