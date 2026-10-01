// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The scenario set: examples of what a specification says a machine does,
//! stated in the owner's words and judged against what an engine observes.
//!
//! # Why the product owns this
//!
//! Requirement closure answers *is there a node that claims this id*. That is
//! evidence only for a requirement met by something existing. A requirement
//! met by something NOT happening has no honest answer among its outcomes, and
//! [`crate::requirement_manifest::Outcome::NeedsScenario`] says what would
//! carry it: a scenario asserting the thing does not occur, and passing. This
//! file is that column's input. The product owns the format, as it owns the
//! requirement list, so that every engine and every client reads one thing.
//!
//! # What this module is, and is not
//!
//! It reads a scenario set and says whether it is a usable one: well formed,
//! every name inside the interface it declares, every quote word for word in
//! the specification, and no scenario that would pass by asserting nothing.
//! It runs nothing and says nothing about a design. Which engine drives a
//! machine is not decided here; the observations an engine emits and the
//! judgement of them against these expectations are the next layer.
//!
//! # Findings, not refusals
//!
//! A file that is not a scenario set at all (not JSON, another record, another
//! version, a field this build does not know) is REFUSED, as a requirement
//! list is. A scenario set that is one but has things wrong with it is
//! answered with every problem at once ([`ScenarioSet::problems`]): the client
//! that wrote it fixes them in one round, and a first-error refusal would cost
//! it one round per mistake.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::{Deserialize, Serialize};

/// The stability status of `schemas/sce-scenario-set.v1.schema.json`, held to
/// the schema file's own header by `schema_file_declares_status`.
/// `SCE_WIRE_CONTRACTS.md` requires one commit to move both.
pub const SCENARIO_SET_SCHEMA_STATUS: &str = "pre-release";

/// What the `record` field must say.
pub const RECORD_KIND: &str = "sce-scenario-set";

/// The wire version this build reads.
pub const SCENARIO_SET_VERSION: u32 = 1;

/// Why a file could not be read as a scenario set.
#[derive(Debug)]
pub enum ScenarioSetError {
    Read {
        path: String,
        source: std::io::Error,
    },
    /// Not JSON.
    Parse {
        path: String,
        source: serde_json::Error,
    },
    /// JSON, but not a scenario set: another record, or none.
    NotAScenarioSet { path: String, found: String },
    /// A scenario set of a version this build does not read.
    UnsupportedVersion { path: String, found: String },
    /// A scenario set whose shape this build does not know: a field it does
    /// not have, a value of the wrong kind, a field missing.
    Shape {
        path: String,
        source: serde_json::Error,
    },
}

impl ScenarioSetError {
    /// Which refusal this is, in words that do not carry the path, so a
    /// diagnostic keyed on it does not change identity with the directory the
    /// file was checked out into.
    pub fn kind(&self) -> &'static str {
        match self {
            ScenarioSetError::Read { .. } => "read",
            ScenarioSetError::Parse { .. } => "parse",
            ScenarioSetError::NotAScenarioSet { .. } => "not-a-scenario-set",
            ScenarioSetError::UnsupportedVersion { .. } => "unsupported-version",
            ScenarioSetError::Shape { .. } => "shape",
        }
    }
}

impl std::fmt::Display for ScenarioSetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScenarioSetError::Read { path, source } => {
                write!(f, "{path}: cannot be read: {source}")
            }
            ScenarioSetError::Parse { path, source } => {
                write!(f, "{path}: is not JSON: {source}")
            }
            ScenarioSetError::NotAScenarioSet { path, found } => write!(
                f,
                "{path}: is not a scenario set: its `record` is {found}, and a scenario set's is \
                 \"{RECORD_KIND}\""
            ),
            ScenarioSetError::UnsupportedVersion { path, found } => write!(
                f,
                "{path}: is a scenario set of version {found}, and this build reads version \
                 {SCENARIO_SET_VERSION}"
            ),
            ScenarioSetError::Shape { path, source } => write!(
                f,
                "{path}: is a scenario set this build cannot read, so none of it is used rather \
                 than part of it: {source}"
            ),
        }
    }
}

impl std::error::Error for ScenarioSetError {}

/// Who wrote the examples. A declaration only: nothing here can verify it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Origin {
    AiProposed,
    OwnerWritten,
}

impl Origin {
    pub fn as_str(self) -> &'static str {
        match self {
            Origin::AiProposed => "ai-proposed",
            Origin::OwnerWritten => "owner-written",
        }
    }
}

/// Whether a scenario can be run as written.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    #[default]
    Runnable,
    AwaitingDecision,
    Blocked,
}

impl Status {
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Runnable => "runnable",
            Status::AwaitingDecision => "awaiting-decision",
            Status::Blocked => "blocked",
        }
    }
}

/// What a payload field holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    Integer,
    Number,
    Text,
    Boolean,
}

/// The sha256 of `bytes`, as 64 lowercase hex digits: how a scenario set and an
/// observation trace name each other.
pub fn digest_of(bytes: &[u8]) -> String {
    crate::generator_witness::hex_encode(&crate::generator_witness::sha256_bytes(bytes))
}

impl FieldType {
    fn name(self) -> &'static str {
        match self {
            FieldType::Integer => "integer",
            FieldType::Number => "number",
            FieldType::Text => "text",
            FieldType::Boolean => "boolean",
        }
    }

    /// Whether `value` is something a field of this type holds. A `number`
    /// field holds a whole number too; an `integer` field does not hold a
    /// fraction.
    fn holds(self, value: &Scalar) -> bool {
        matches!(
            (self, value),
            (FieldType::Integer, Scalar::Integer(_))
                | (FieldType::Number, Scalar::Integer(_) | Scalar::Number(_))
                | (FieldType::Text, Scalar::Text(_))
                | (FieldType::Boolean, Scalar::Boolean(_))
        )
    }
}

/// A value an example gives a payload field or a data item. Never null, never
/// a list or an object: those do not deserialize, which is the refusal.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Scalar {
    Boolean(bool),
    Integer(i64),
    Number(f64),
    Text(String),
}

impl Scalar {
    fn kind(&self) -> &'static str {
        match self {
            Scalar::Boolean(_) => "boolean",
            Scalar::Integer(_) => "integer",
            Scalar::Number(_) => "number",
            Scalar::Text(_) => "text",
        }
    }

    /// Whether two values are the same one. Numbers are compared as numbers,
    /// so `3` and `3.0` are the same value; text and booleans are compared as
    /// they are, and a number is never a piece of text.
    pub fn same_value(&self, other: &Scalar) -> bool {
        match (self, other) {
            (Scalar::Boolean(a), Scalar::Boolean(b)) => a == b,
            (Scalar::Text(a), Scalar::Text(b)) => a == b,
            (Scalar::Integer(a), Scalar::Integer(b)) => a == b,
            (Scalar::Number(a), Scalar::Number(b)) => a == b,
            (Scalar::Integer(a), Scalar::Number(b)) | (Scalar::Number(b), Scalar::Integer(a)) => {
                (*a as f64) == *b
            }
            _ => false,
        }
    }
}

type Payload = BTreeMap<String, Scalar>;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Specification {
    pub doc_id: String,
    pub rev: String,
}

/// How an output leaves the machine. An output with none is one whose route
/// nobody has decided, and a run cannot observe it.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Route {
    #[serde(rename = "type")]
    pub processor_type: String,
    pub target: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    pub name: String,
    #[serde(default)]
    pub payload: BTreeMap<String, FieldType>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Output {
    pub name: String,
    #[serde(default)]
    pub payload: BTreeMap<String, FieldType>,
    pub via: Option<Route>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Interface {
    pub why: Option<String>,
    pub inputs: Vec<Input>,
    pub outputs: Vec<Output>,
    #[serde(default)]
    pub conditions: Vec<String>,
    #[serde(default)]
    pub data: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct World {
    pub why: Option<String>,
    pub source: String,
    pub values: Option<serde_json::Map<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExpectedOutbound {
    pub event: String,
    pub payload: Option<Payload>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Expect {
    pub condition: Option<String>,
    pub outbound: Option<Vec<ExpectedOutbound>>,
    pub finished: Option<bool>,
    pub data: Option<Payload>,
}

impl Expect {
    /// How many things this expectation checks.
    pub fn checks(&self) -> usize {
        usize::from(self.condition.is_some())
            + usize::from(self.outbound.is_some())
            + usize::from(self.finished.is_some())
            + usize::from(self.data.as_ref().is_some_and(|d| !d.is_empty()))
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Step {
    pub send: Option<String>,
    pub payload: Option<Payload>,
    pub advance_ms: Option<u64>,
    pub expect: Option<Expect>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Decision {
    pub question: String,
    #[serde(default)]
    pub alternatives: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bound {
    pub cycles: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    pub id: String,
    pub quote: String,
    #[serde(default)]
    pub requirements: Vec<String>,
    #[serde(default)]
    pub status: Status,
    pub blocked_by: Option<String>,
    pub decision: Option<Decision>,
    #[serde(default)]
    pub assumes: Vec<String>,
    pub note: Option<String>,
    pub bound: Option<Bound>,
    pub steps: Vec<Step>,
}

impl Scenario {
    /// How many things the scenario checks, over all its steps.
    pub fn expectations(&self) -> usize {
        self.steps
            .iter()
            .filter_map(|step| step.expect.as_ref())
            .map(Expect::checks)
            .sum()
    }
}

/// A scenario set that has been read: a file of the right record and version
/// and a shape this build knows. It may still have problems
/// ([`ScenarioSet::problems`]).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioSet {
    pub record: String,
    pub v: u32,
    pub specification: Specification,
    pub origin: Origin,
    pub interface: Interface,
    pub world: Option<World>,
    pub scenarios: Vec<Scenario>,
}

/// One thing wrong with a scenario set that was otherwise readable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Problem {
    /// The scenario it is about, when it is about one.
    pub scenario: Option<String>,
    /// Where in the file, as a JSON path.
    pub path: String,
    /// Which problem, in a word that does not change with the text.
    pub code: &'static str,
    /// What is wrong, for a person.
    pub detail: String,
}

fn problem(scenario: Option<&str>, path: String, code: &'static str, detail: String) -> Problem {
    Problem {
        scenario: scenario.map(str::to_string),
        path,
        code,
        detail,
    }
}

impl ScenarioSet {
    /// Read and shape-check a scenario set from a file.
    pub fn load(path: &Path) -> Result<Self, ScenarioSetError> {
        Self::load_with_digest(path).map(|(set, _)| set)
    }

    /// The same, and the sha256 of the file's bytes as written. What identifies
    /// a scenario set is its digest, which is what an observation trace names
    /// to say which set it was taken against.
    pub fn load_with_digest(path: &Path) -> Result<(Self, String), ScenarioSetError> {
        let display = path.display().to_string();
        let bytes = std::fs::read(path).map_err(|source| ScenarioSetError::Read {
            path: display.clone(),
            source,
        })?;
        let raw = String::from_utf8(bytes.clone()).map_err(|source| ScenarioSetError::Read {
            path: display.clone(),
            source: std::io::Error::new(std::io::ErrorKind::InvalidData, source),
        })?;
        let digest = digest_of(&bytes);
        Ok((Self::from_json(&raw, &display)?, digest))
    }

    /// The reading half, without the filesystem.
    ///
    /// The record and the version are looked at BEFORE the typed read, so a
    /// file of another kind is told it is of another kind, and not that it
    /// has a field this build does not know.
    pub fn from_json(raw: &str, label: &str) -> Result<Self, ScenarioSetError> {
        let tree: serde_json::Value =
            serde_json::from_str(raw).map_err(|source| ScenarioSetError::Parse {
                path: label.to_string(),
                source,
            })?;
        match tree.get("record").and_then(serde_json::Value::as_str) {
            Some(RECORD_KIND) => {}
            Some(other) => {
                return Err(ScenarioSetError::NotAScenarioSet {
                    path: label.to_string(),
                    found: format!("\"{other}\""),
                })
            }
            None => {
                return Err(ScenarioSetError::NotAScenarioSet {
                    path: label.to_string(),
                    found: "missing".to_string(),
                })
            }
        }
        match tree.get("v").and_then(serde_json::Value::as_u64) {
            Some(v) if v == u64::from(SCENARIO_SET_VERSION) => {}
            other => {
                return Err(ScenarioSetError::UnsupportedVersion {
                    path: label.to_string(),
                    found: other.map_or_else(|| "missing".to_string(), |v| v.to_string()),
                })
            }
        }
        serde_json::from_value(tree).map_err(|source| ScenarioSetError::Shape {
            path: label.to_string(),
            source,
        })
    }

    /// How many scenarios are in each status: runnable, awaiting a decision,
    /// blocked.
    pub fn counts(&self) -> (usize, usize, usize) {
        let count = |wanted: Status| self.scenarios.iter().filter(|s| s.status == wanted).count();
        (
            count(Status::Runnable),
            count(Status::AwaitingDecision),
            count(Status::Blocked),
        )
    }

    /// Every thing wrong with the set that does not need the specification.
    ///
    /// All of them, in file order. The first would be enough to refuse the
    /// file, and would leave its author one mistake per round.
    pub fn problems(&self) -> Vec<Problem> {
        let mut found = Vec::new();
        if self.scenarios.is_empty() {
            found.push(problem(
                None,
                "scenarios".to_string(),
                "no-scenarios",
                "a scenario set with no scenario is an empty claim: it would be usable and \
                 judge nothing"
                    .to_string(),
            ));
        }
        self.interface_problems(&mut found);
        if let Some(world) = &self.world {
            if world.source.trim().is_empty() {
                found.push(problem(
                    None,
                    "world.source".to_string(),
                    "world-without-source",
                    "a `world` says where its facts come from, or that nobody has decided \
                     (`undecided`)"
                        .to_string(),
                ));
            }
        }
        let mut seen: BTreeSet<&str> = BTreeSet::new();
        let ids: BTreeSet<&str> = self.scenarios.iter().map(|s| s.id.as_str()).collect();
        for (index, scenario) in self.scenarios.iter().enumerate() {
            if !seen.insert(scenario.id.as_str()) {
                found.push(problem(
                    Some(scenario.id.as_str()),
                    format!("scenarios[{index}].id"),
                    "duplicate-scenario-id",
                    format!("another scenario is already called `{}`", scenario.id),
                ));
            }
            self.scenario_problems(index, scenario, &ids, &mut found);
        }
        found
    }

    fn interface_problems(&self, found: &mut Vec<Problem>) {
        let mut inputs = BTreeSet::new();
        for (index, input) in self.interface.inputs.iter().enumerate() {
            if !inputs.insert(input.name.as_str()) {
                found.push(problem(
                    None,
                    format!("interface.inputs[{index}].name"),
                    "duplicate-input",
                    format!("the input `{}` is declared twice", input.name),
                ));
            }
        }
        let mut outputs = BTreeSet::new();
        for (index, output) in self.interface.outputs.iter().enumerate() {
            if !outputs.insert(output.name.as_str()) {
                found.push(problem(
                    None,
                    format!("interface.outputs[{index}].name"),
                    "duplicate-output",
                    format!("the output `{}` is declared twice", output.name),
                ));
            }
            if let Some(route) = &output.via {
                if route.processor_type.trim().is_empty() || route.target.trim().is_empty() {
                    found.push(problem(
                        None,
                        format!("interface.outputs[{index}].via"),
                        "route-incomplete",
                        format!(
                            "the route of `{}` names a processor type and a target, both \
                             non-empty; with neither decided, leave `via` out",
                            output.name
                        ),
                    ));
                }
            }
        }
        for (field, names) in [
            ("conditions", &self.interface.conditions),
            ("data", &self.interface.data),
        ] {
            let mut seen = BTreeSet::new();
            for (index, name) in names.iter().enumerate() {
                if !seen.insert(name.as_str()) {
                    found.push(problem(
                        None,
                        format!("interface.{field}[{index}]"),
                        "duplicate-name",
                        format!("`{name}` is listed twice under `{field}`"),
                    ));
                }
            }
        }
    }

    fn scenario_problems(
        &self,
        index: usize,
        scenario: &Scenario,
        ids: &BTreeSet<&str>,
        found: &mut Vec<Problem>,
    ) {
        let at = |rest: &str| format!("scenarios[{index}]{rest}");
        let id = Some(scenario.id.as_str());

        if scenario.steps.is_empty() {
            found.push(problem(
                id,
                at(".steps"),
                "no-steps",
                "a scenario with no step observes nothing".to_string(),
            ));
        }
        self.status_problems(index, scenario, ids, found);
        if scenario
            .bound
            .as_ref()
            .is_some_and(|bound| bound.cycles == 0)
        {
            found.push(problem(
                id,
                at(".bound.cycles"),
                "bound-zero",
                "a bound of zero cycles is a run that did nothing".to_string(),
            ));
        }
        // A scenario that is meant to run and asserts nothing passes without
        // measuring anything, which reads as success and is worth none.
        if scenario.status == Status::Runnable
            && !scenario.steps.is_empty()
            && scenario.expectations() == 0
        {
            found.push(problem(
                id,
                at(".steps"),
                "runnable-asserts-nothing",
                "a runnable scenario states at least one expectation; one that states none \
                 would pass whatever the machine did"
                    .to_string(),
            ));
        }
        for (step_index, step) in scenario.steps.iter().enumerate() {
            self.step_problems(index, step_index, step, scenario, found);
        }
    }

    fn status_problems(
        &self,
        index: usize,
        scenario: &Scenario,
        ids: &BTreeSet<&str>,
        found: &mut Vec<Problem>,
    ) {
        let at = |rest: &str| format!("scenarios[{index}]{rest}");
        let id = Some(scenario.id.as_str());
        let blocked_by = scenario
            .blocked_by
            .as_deref()
            .is_some_and(|text| !text.trim().is_empty());
        match scenario.status {
            Status::Blocked => {
                if !blocked_by {
                    found.push(problem(
                        id,
                        at(".blocked_by"),
                        "blocked-without-reason",
                        "a blocked scenario says what it waits on".to_string(),
                    ));
                }
                if scenario.decision.is_some() {
                    found.push(problem(
                        id,
                        at(".decision"),
                        "status-conflict",
                        "a blocked scenario waits on a fact; a question for the owner makes it \
                         `awaiting-decision`"
                            .to_string(),
                    ));
                }
            }
            Status::AwaitingDecision => {
                if scenario.decision.is_none() {
                    found.push(problem(
                        id,
                        at(".decision"),
                        "awaiting-without-decision",
                        "a scenario that awaits a decision states the question".to_string(),
                    ));
                }
                if scenario.blocked_by.is_some() {
                    found.push(problem(
                        id,
                        at(".blocked_by"),
                        "status-conflict",
                        "a scenario that awaits a decision is not also blocked on a fact"
                            .to_string(),
                    ));
                }
            }
            Status::Runnable => {
                if scenario.blocked_by.is_some() || scenario.decision.is_some() {
                    found.push(problem(
                        id,
                        at(".status"),
                        "runnable-with-blocker",
                        "a scenario that names what it waits on is not runnable; give it the \
                         status that says so"
                            .to_string(),
                    ));
                }
            }
        }
        if let Some(decision) = &scenario.decision {
            for (alt_index, alternative) in decision.alternatives.iter().enumerate() {
                let path = at(&format!(".decision.alternatives[{alt_index}]"));
                if alternative == &scenario.id {
                    found.push(problem(
                        id,
                        path,
                        "alternative-is-itself",
                        "the alternatives of a decision are the OTHER scenarios".to_string(),
                    ));
                } else if !ids.contains(alternative.as_str()) {
                    found.push(problem(
                        id,
                        path,
                        "unknown-alternative",
                        format!("no scenario is called `{alternative}`"),
                    ));
                }
            }
        }
    }

    fn step_problems(
        &self,
        index: usize,
        step_index: usize,
        step: &Step,
        scenario: &Scenario,
        found: &mut Vec<Problem>,
    ) {
        let at = |rest: &str| format!("scenarios[{index}].steps[{step_index}]{rest}");
        let id = Some(scenario.id.as_str());

        if step.send.is_some() && step.advance_ms.is_some() {
            found.push(problem(
                id,
                at(""),
                "step-acts-twice",
                "a step sends an event or lets time pass, not both; they are two steps".to_string(),
            ));
        }
        if step.payload.is_some() && step.send.is_none() {
            found.push(problem(
                id,
                at(".payload"),
                "payload-without-send",
                "a payload belongs to an event that is sent".to_string(),
            ));
        }
        if step.send.is_none()
            && step.advance_ms.is_none()
            && step.payload.is_none()
            && step.expect.is_none()
        {
            found.push(problem(
                id,
                at(""),
                "empty-step",
                "a step does something or observes something".to_string(),
            ));
        }
        if step.advance_ms == Some(0) {
            found.push(problem(
                id,
                at(".advance_ms"),
                "advance-not-positive",
                "time that passes is at least one millisecond".to_string(),
            ));
        }
        if let Some(name) = &step.send {
            self.send_problems(name, step, scenario, &at(""), found);
        }
        if let Some(expect) = &step.expect {
            self.expect_problems(expect, scenario, &at(".expect"), found);
        }
    }

    fn send_problems(
        &self,
        name: &str,
        step: &Step,
        scenario: &Scenario,
        base: &str,
        found: &mut Vec<Problem>,
    ) {
        let id = Some(scenario.id.as_str());
        let Some(input) = self
            .interface
            .inputs
            .iter()
            .find(|input| input.name == name)
        else {
            found.push(problem(
                id,
                format!("{base}.send"),
                "unknown-input",
                format!(
                    "`{name}` is not an input of the interface (declared: {})",
                    listed(
                        self.interface
                            .inputs
                            .iter()
                            .map(|input| input.name.as_str())
                    )
                ),
            ));
            return;
        };
        let given = step.payload.clone().unwrap_or_default();
        for (field, value) in &given {
            match input.payload.get(field) {
                None => found.push(problem(
                    id,
                    format!("{base}.payload.{field}"),
                    "unknown-payload-field",
                    format!(
                        "`{name}` carries no field `{field}` (declared: {})",
                        listed(input.payload.keys().map(String::as_str))
                    ),
                )),
                Some(declared) if !declared.holds(value) => found.push(problem(
                    id,
                    format!("{base}.payload.{field}"),
                    "payload-type",
                    format!(
                        "`{field}` of `{name}` is {}, and the value is {}",
                        declared.name(),
                        value.kind()
                    ),
                )),
                Some(_) => {}
            }
        }
        // A sent event is concrete: an example that leaves a declared field
        // out is not an example of any event the machine could receive.
        for field in input.payload.keys() {
            if !given.contains_key(field) {
                found.push(problem(
                    id,
                    format!("{base}.payload"),
                    "payload-field-missing",
                    format!("`{name}` carries `{field}` and the step does not give it"),
                ));
            }
        }
    }

    fn expect_problems(
        &self,
        expect: &Expect,
        scenario: &Scenario,
        base: &str,
        found: &mut Vec<Problem>,
    ) {
        let id = Some(scenario.id.as_str());
        if expect.checks() == 0 {
            found.push(problem(
                id,
                base.to_string(),
                "empty-expectation",
                "an expectation checks something; leave it out if it checks nothing".to_string(),
            ));
        }
        if let Some(condition) = &expect.condition {
            if !self
                .interface
                .conditions
                .iter()
                .any(|name| name == condition)
            {
                found.push(problem(
                    id,
                    format!("{base}.condition"),
                    "unknown-condition",
                    format!(
                        "`{condition}` is not a condition the specification names (declared: {})",
                        listed(self.interface.conditions.iter().map(String::as_str))
                    ),
                ));
            }
        }
        if let Some(data) = &expect.data {
            for name in data.keys() {
                if !self.interface.data.iter().any(|declared| declared == name) {
                    found.push(problem(
                        id,
                        format!("{base}.data.{name}"),
                        "unknown-data",
                        format!(
                            "`{name}` is not a data item the specification names (declared: {})",
                            listed(self.interface.data.iter().map(String::as_str))
                        ),
                    ));
                }
            }
        }
        for (index, outbound) in expect.outbound.iter().flatten().enumerate() {
            let path = format!("{base}.outbound[{index}]");
            let Some(output) = self
                .interface
                .outputs
                .iter()
                .find(|output| output.name == outbound.event)
            else {
                found.push(problem(
                    id,
                    format!("{path}.event"),
                    "unknown-output",
                    format!(
                        "`{}` is not an output of the interface (declared: {})",
                        outbound.event,
                        listed(
                            self.interface
                                .outputs
                                .iter()
                                .map(|output| output.name.as_str())
                        )
                    ),
                ));
                continue;
            };
            for (field, value) in outbound.payload.iter().flatten() {
                match output.payload.get(field) {
                    None => found.push(problem(
                        id,
                        format!("{path}.payload.{field}"),
                        "unknown-payload-field",
                        format!(
                            "`{}` carries no field `{field}` (declared: {})",
                            outbound.event,
                            listed(output.payload.keys().map(String::as_str))
                        ),
                    )),
                    Some(declared) if !declared.holds(value) => found.push(problem(
                        id,
                        format!("{path}.payload.{field}"),
                        "payload-type",
                        format!(
                            "`{field}` of `{}` is {}, and the value is {}",
                            outbound.event,
                            declared.name(),
                            value.kind()
                        ),
                    )),
                    Some(_) => {}
                }
            }
        }
    }

    /// The scenarios whose quote is not in `specification`, word for word.
    ///
    /// Runs of white space are one space on both sides, so a sentence a
    /// specification wraps across lines is still found; nothing else is
    /// forgiven. A quote is the anchor that lets an owner read an example
    /// against their own words, and one that is a paraphrase has none.
    pub fn quote_problems(&self, specification: &str) -> Vec<Problem> {
        let text = squeeze(specification);
        self.scenarios
            .iter()
            .enumerate()
            .filter_map(|(index, scenario)| {
                let quote = squeeze(&scenario.quote);
                if !quote.is_empty() && text.contains(&quote) {
                    return None;
                }
                Some(problem(
                    Some(scenario.id.as_str()),
                    format!("scenarios[{index}].quote"),
                    "quote-not-found",
                    if quote.is_empty() {
                        "the quote is empty".to_string()
                    } else {
                        "the quote is not in the specification word for word".to_string()
                    },
                ))
            })
            .collect()
    }
}

/// White space collapsed to single spaces and trimmed.
fn squeeze(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Names for a sentence: `a, b, c`, or `none` when there are none.
fn listed<'a>(names: impl Iterator<Item = &'a str>) -> String {
    let names: Vec<&str> = names.collect();
    if names.is_empty() {
        "none".to_string()
    } else {
        names.join(", ")
    }
}

/// Every problem code this module reports, in the order the checks run. The
/// tests make each one from an input, so a code listed here that nothing can
/// produce, or a code produced that is not listed, fails.
pub const PROBLEM_CODES: [&str; 29] = [
    "no-scenarios",
    "duplicate-input",
    "duplicate-output",
    "route-incomplete",
    "duplicate-name",
    "world-without-source",
    "duplicate-scenario-id",
    "no-steps",
    "blocked-without-reason",
    "status-conflict",
    "awaiting-without-decision",
    "runnable-with-blocker",
    "unknown-alternative",
    "alternative-is-itself",
    "bound-zero",
    "runnable-asserts-nothing",
    "step-acts-twice",
    "payload-without-send",
    "empty-step",
    "advance-not-positive",
    "unknown-input",
    "unknown-payload-field",
    "payload-type",
    "payload-field-missing",
    "empty-expectation",
    "unknown-condition",
    "unknown-data",
    "unknown-output",
    "quote-not-found",
];

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    const SCHEMA: &str = include_str!("../../schemas/sce-scenario-set.v1.schema.json");

    /// One change to the base set, named for what it does to it.
    type Case = (&'static str, Box<dyn Fn(&mut Value)>);

    /// The four sets written by hand before any code, each with the
    /// specification its quotes are taken from.
    const FIXTURES: [(&str, &str, &str); 4] = [
        (
            "door-with-auto-close",
            include_str!("../tests/fixtures/scenario_sets/door-with-auto-close.scenarios.json"),
            include_str!("../tests/fixtures/scenario_sets/door-with-auto-close.spec.txt"),
        ),
        (
            "connection-keeper",
            include_str!("../tests/fixtures/scenario_sets/connection-keeper.scenarios.json"),
            include_str!("../tests/fixtures/scenario_sets/connection-keeper.spec.txt"),
        ),
        (
            "vending-controller",
            include_str!("../tests/fixtures/scenario_sets/vending-controller.scenarios.json"),
            include_str!("../tests/fixtures/scenario_sets/vending-controller.spec.txt"),
        ),
        (
            "retry-client",
            include_str!("../tests/fixtures/scenario_sets/retry-client.scenarios.json"),
            include_str!("../tests/fixtures/scenario_sets/retry-client.spec.txt"),
        ),
    ];

    fn schema() -> Value {
        serde_json::from_str(SCHEMA).expect("scenario set schema is JSON")
    }

    fn violations(instance: &Value) -> Vec<String> {
        let schema = schema();
        let validator = jsonschema::JSONSchema::options()
            .with_draft(jsonschema::Draft::Draft7)
            .compile(&schema)
            .expect("scenario set schema compiles");
        let outcome = validator.validate(instance);
        match outcome {
            Ok(()) => Vec::new(),
            Err(errors) => errors.map(|e| e.to_string()).collect(),
        }
    }

    /// The smallest set that has nothing wrong with it, and uses every kind
    /// of expectation once. Every case below changes one thing in it.
    fn base() -> Value {
        json!({
            "record": "sce-scenario-set",
            "v": 1,
            "specification": {"doc_id": "d", "rev": "1"},
            "origin": "ai-proposed",
            "interface": {
                "inputs": [{"name": "go", "payload": {"n": "integer"}}],
                "outputs": [{"name": "done", "payload": {"n": "integer"}}],
                "conditions": ["idle"],
                "data": ["count"]
            },
            "scenarios": [{
                "id": "S1",
                "quote": "The machine goes.",
                "steps": [{
                    "send": "go",
                    "payload": {"n": 1},
                    "expect": {
                        "outbound": [{"event": "done", "payload": {"n": 1}}],
                        "condition": "idle",
                        "data": {"count": 1}
                    }
                }]
            }]
        })
    }

    fn read(value: &Value) -> ScenarioSet {
        ScenarioSet::from_json(&value.to_string(), "test").expect("the set is readable")
    }

    fn codes(value: &Value) -> Vec<&'static str> {
        read(value).problems().iter().map(|p| p.code).collect()
    }

    /// The value at `pointer`, made if it is the last key of an object that is
    /// not there yet, so a case can set a key the base set does not carry.
    fn at<'a>(value: &'a mut Value, pointer: &str) -> &'a mut Value {
        let (parent, last) = pointer
            .rsplit_once('/')
            .expect("a pointer has a last token");
        let parent = value
            .pointer_mut(parent)
            .unwrap_or_else(|| panic!("no {parent} in the base set"));
        match parent {
            Value::Object(map) => map.entry(last.to_string()).or_insert(Value::Null),
            Value::Array(items) => &mut items[last.parse::<usize>().expect("an index")],
            other => panic!("{pointer}: its parent is {other}, not an object or a list"),
        }
    }

    #[test]
    fn schema_file_declares_status() {
        assert_eq!(
            schema()["x-sce-schema-status"].as_str(),
            Some(SCENARIO_SET_SCHEMA_STATUS),
            "schemas/sce-scenario-set.v1.schema.json x-sce-schema-status disagrees with \
             SCENARIO_SET_SCHEMA_STATUS; SCE_WIRE_CONTRACTS.md requires one commit to move both",
        );
    }

    #[test]
    fn schema_version_and_record_match_the_reader() {
        let schema = schema();
        assert_eq!(
            schema["properties"]["v"]["const"].as_u64(),
            Some(u64::from(SCENARIO_SET_VERSION))
        );
        assert_eq!(
            schema["properties"]["record"]["const"].as_str(),
            Some(RECORD_KIND)
        );
    }

    #[test]
    fn the_base_set_is_valid_and_has_nothing_wrong() {
        let base = base();
        assert_eq!(violations(&base), Vec::<String>::new());
        assert_eq!(codes(&base), Vec::<&str>::new());
    }

    /// Every set the product ships is valid against the wire schema, is read,
    /// has nothing wrong, and quotes its specification word for word. The
    /// counts are pinned so a fixture that quietly lost scenarios, or a reader
    /// that quietly skipped them, fails here instead of passing with less.
    #[test]
    fn every_set_the_product_reads_validates_against_the_wire_schema() {
        // (runnable, awaiting a decision, blocked)
        let expected = [
            ("door-with-auto-close", (5, 2, 0)),
            ("connection-keeper", (8, 0, 0)),
            ("vending-controller", (6, 1, 6)),
            ("retry-client", (6, 0, 0)),
        ];
        let mut scenarios = 0;
        for (name, text, specification) in FIXTURES {
            let instance: Value = serde_json::from_str(text).expect("fixture is JSON");
            assert_eq!(violations(&instance), Vec::<String>::new(), "{name}");
            let set = ScenarioSet::from_json(text, name).expect("fixture is readable");
            assert_eq!(set.problems(), Vec::new(), "{name}");
            assert_eq!(set.quote_problems(specification), Vec::new(), "{name}");
            let want = expected.iter().find(|(n, _)| *n == name).expect("pinned").1;
            assert_eq!(set.counts(), want, "{name}");
            scenarios += set.scenarios.len();
        }
        assert_eq!(
            scenarios, 34,
            "the four fixtures hold 34 scenarios between them"
        );
    }

    /// What the schema refuses the reader must not accept silently. A
    /// validator that accepted everything would pass the sweep above, so the
    /// negative case is what shows the schema refusing something.
    #[test]
    fn the_wire_schema_rejects_what_the_reader_does_not_accept() {
        let cases: Vec<Case> = vec![
            (
                "a key the format does not have",
                Box::new(|v| {
                    v["surprise"] = json!(true);
                }),
            ),
            (
                "a key the scenario does not have",
                Box::new(|v| {
                    v["scenarios"][0]["owner_says"] = json!("yes");
                }),
            ),
            (
                "another record",
                Box::new(|v| {
                    v["record"] = json!("sce-authoring-profile");
                }),
            ),
            (
                "another version",
                Box::new(|v| {
                    v["v"] = json!(2);
                }),
            ),
            (
                "a scenario with no quote",
                Box::new(|v| {
                    v["scenarios"][0].as_object_mut().unwrap().remove("quote");
                }),
            ),
            (
                "a scenario with no steps",
                Box::new(|v| {
                    v["scenarios"][0]["steps"] = json!([]);
                }),
            ),
            (
                "a step that sends and lets time pass",
                Box::new(|v| {
                    v["scenarios"][0]["steps"][0]["advance_ms"] = json!(5);
                }),
            ),
            (
                "a step that lets no time pass",
                Box::new(|v| {
                    v["scenarios"][0]["steps"][0] =
                        json!({"advance_ms": 0, "expect": {"finished": true}});
                }),
            ),
            (
                "a payload with nothing sent",
                Box::new(|v| {
                    v["scenarios"][0]["steps"][0] =
                        json!({"payload": {"n": 1}, "expect": {"finished": true}});
                }),
            ),
            (
                "a null payload value",
                Box::new(|v| {
                    v["scenarios"][0]["steps"][0]["payload"]["n"] = Value::Null;
                }),
            ),
            (
                "a list as a payload value",
                Box::new(|v| {
                    v["scenarios"][0]["steps"][0]["payload"]["n"] = json!([1]);
                }),
            ),
            (
                "an expectation that checks nothing",
                Box::new(|v| {
                    v["scenarios"][0]["steps"][0]["expect"] = json!({});
                }),
            ),
            (
                "a blocked scenario that says nothing of what blocks it",
                Box::new(|v| {
                    v["scenarios"][0]["status"] = json!("blocked");
                }),
            ),
            (
                "a scenario awaiting a decision with no question",
                Box::new(|v| {
                    v["scenarios"][0]["status"] = json!("awaiting-decision");
                }),
            ),
            (
                "a status the format does not have",
                Box::new(|v| {
                    v["scenarios"][0]["status"] = json!("skipped");
                }),
            ),
            (
                "an origin the format does not have",
                Box::new(|v| {
                    v["origin"] = json!("someone");
                }),
            ),
            (
                "a payload type the format does not have",
                Box::new(|v| {
                    v["interface"]["inputs"][0]["payload"]["n"] = json!("float");
                }),
            ),
            (
                "a route with no target",
                Box::new(|v| {
                    v["interface"]["outputs"][0]["via"] = json!({"type": "x"});
                }),
            ),
            (
                "a condition listed twice",
                Box::new(|v| {
                    v["interface"]["conditions"] = json!(["idle", "idle"]);
                }),
            ),
            (
                "an empty set of scenarios",
                Box::new(|v| {
                    v["scenarios"] = json!([]);
                }),
            ),
        ];
        // Each case starts from a set shown valid and changes one thing, so
        // the refusal is pinned to that change.
        assert_eq!(violations(&base()), Vec::<String>::new());
        let mut refused = 0;
        for (what, mutate) in &cases {
            let mut value = base();
            mutate(&mut value);
            assert!(!violations(&value).is_empty(), "the schema accepted {what}",);
            let reader = ScenarioSet::from_json(&value.to_string(), "test");
            let answer = match &reader {
                Err(_) => true,
                Ok(set) => !set.problems().is_empty(),
            };
            assert!(
                answer,
                "the schema refuses {what}, and the reader read it and found nothing wrong",
            );
            refused += 1;
        }
        assert_eq!(refused, 20, "every case ran");
    }

    #[test]
    fn a_file_that_is_not_a_scenario_set_is_refused_by_kind() {
        let refusals: Vec<(&str, String)> = vec![
            ("parse", "{ not json".to_string()),
            (
                "not-a-scenario-set",
                json!({"record": "sce-authoring-profile", "v": 1}).to_string(),
            ),
            ("not-a-scenario-set", json!({"v": 1}).to_string()),
            (
                "unsupported-version",
                json!({"record": RECORD_KIND, "v": 2}).to_string(),
            ),
            (
                "unsupported-version",
                json!({"record": RECORD_KIND}).to_string(),
            ),
            ("shape", {
                let mut value = base();
                value["surprise"] = json!(1);
                value.to_string()
            }),
            ("shape", {
                let mut value = base();
                value["scenarios"][0]["steps"][0]["payload"]["n"] = Value::Null;
                value.to_string()
            }),
            ("shape", {
                let mut value = base();
                value["scenarios"][0].as_object_mut().unwrap().remove("id");
                value.to_string()
            }),
        ];
        for (kind, text) in &refusals {
            let error = ScenarioSet::from_json(text, "f.json").expect_err("refused");
            assert_eq!(error.kind(), *kind, "{error}");
            assert!(error.to_string().starts_with("f.json:"), "{error}");
        }
    }

    /// One mutation of the base set for each problem code, so a code is only
    /// listed in [`PROBLEM_CODES`] if an input produces it.
    #[test]
    fn every_problem_code_is_made_by_an_input() {
        let cases: Vec<Case> = vec![
            (
                "no-scenarios",
                Box::new(|v| {
                    *at(v, "/scenarios") = json!([]);
                }),
            ),
            (
                "duplicate-input",
                Box::new(|v| {
                    let first = at(v, "/interface/inputs/0").clone();
                    at(v, "/interface/inputs")
                        .as_array_mut()
                        .unwrap()
                        .push(first);
                }),
            ),
            (
                "duplicate-output",
                Box::new(|v| {
                    let first = at(v, "/interface/outputs/0").clone();
                    at(v, "/interface/outputs")
                        .as_array_mut()
                        .unwrap()
                        .push(first);
                }),
            ),
            (
                "route-incomplete",
                Box::new(|v| {
                    *at(v, "/interface/outputs/0/via") = json!({"type": " ", "target": "t"});
                }),
            ),
            (
                "duplicate-name",
                Box::new(|v| {
                    *at(v, "/interface/conditions") = json!(["idle", "idle"]);
                }),
            ),
            (
                "world-without-source",
                Box::new(|v| {
                    v["world"] = json!({"source": "  "});
                }),
            ),
            (
                "duplicate-scenario-id",
                Box::new(|v| {
                    let first = at(v, "/scenarios/0").clone();
                    at(v, "/scenarios").as_array_mut().unwrap().push(first);
                }),
            ),
            (
                "no-steps",
                Box::new(|v| {
                    *at(v, "/scenarios/0/steps") = json!([]);
                }),
            ),
            (
                "blocked-without-reason",
                Box::new(|v| {
                    *at(v, "/scenarios/0/status") = json!("blocked");
                }),
            ),
            (
                "status-conflict",
                Box::new(|v| {
                    *at(v, "/scenarios/0/status") = json!("blocked");
                    v["scenarios"][0]["blocked_by"] = json!("a fact");
                    v["scenarios"][0]["decision"] = json!({"question": "q"});
                }),
            ),
            (
                "awaiting-without-decision",
                Box::new(|v| {
                    *at(v, "/scenarios/0/status") = json!("awaiting-decision");
                }),
            ),
            (
                "runnable-with-blocker",
                Box::new(|v| {
                    v["scenarios"][0]["blocked_by"] = json!("a fact");
                }),
            ),
            (
                "unknown-alternative",
                Box::new(|v| {
                    *at(v, "/scenarios/0/status") = json!("awaiting-decision");
                    v["scenarios"][0]["decision"] =
                        json!({"question": "q", "alternatives": ["nope"]});
                }),
            ),
            (
                "alternative-is-itself",
                Box::new(|v| {
                    *at(v, "/scenarios/0/status") = json!("awaiting-decision");
                    v["scenarios"][0]["decision"] =
                        json!({"question": "q", "alternatives": ["S1"]});
                }),
            ),
            (
                "bound-zero",
                Box::new(|v| {
                    v["scenarios"][0]["bound"] = json!({"cycles": 0});
                }),
            ),
            (
                "runnable-asserts-nothing",
                Box::new(|v| {
                    at(v, "/scenarios/0/steps/0")
                        .as_object_mut()
                        .unwrap()
                        .remove("expect");
                }),
            ),
            (
                "step-acts-twice",
                Box::new(|v| {
                    v["scenarios"][0]["steps"][0]["advance_ms"] = json!(5);
                }),
            ),
            (
                "payload-without-send",
                Box::new(|v| {
                    *at(v, "/scenarios/0/steps/0") =
                        json!({"payload": {"n": 1}, "expect": {"finished": true}});
                }),
            ),
            (
                "empty-step",
                Box::new(|v| {
                    let steps = at(v, "/scenarios/0/steps").as_array_mut().unwrap();
                    steps.push(json!({}));
                }),
            ),
            (
                "advance-not-positive",
                Box::new(|v| {
                    *at(v, "/scenarios/0/steps/0") =
                        json!({"advance_ms": 0, "expect": {"finished": true}});
                }),
            ),
            (
                "unknown-input",
                Box::new(|v| {
                    *at(v, "/scenarios/0/steps/0/send") = json!("stop");
                }),
            ),
            (
                "unknown-payload-field",
                Box::new(|v| {
                    v["scenarios"][0]["steps"][0]["payload"]["m"] = json!(2);
                }),
            ),
            (
                "payload-type",
                Box::new(|v| {
                    *at(v, "/scenarios/0/steps/0/payload/n") = json!("one");
                }),
            ),
            (
                "payload-field-missing",
                Box::new(|v| {
                    at(v, "/scenarios/0/steps/0")
                        .as_object_mut()
                        .unwrap()
                        .remove("payload");
                }),
            ),
            (
                "empty-expectation",
                Box::new(|v| {
                    *at(v, "/scenarios/0/steps/0/expect") = json!({});
                }),
            ),
            (
                "unknown-condition",
                Box::new(|v| {
                    *at(v, "/scenarios/0/steps/0/expect/condition") = json!("broken");
                }),
            ),
            (
                "unknown-data",
                Box::new(|v| {
                    v["scenarios"][0]["steps"][0]["expect"]["data"] = json!({"nothing": 0});
                }),
            ),
            (
                "unknown-output",
                Box::new(|v| {
                    *at(v, "/scenarios/0/steps/0/expect/outbound/0/event") = json!("vanished");
                }),
            ),
        ];
        let mut produced = BTreeSet::new();
        for (code, mutate) in &cases {
            let mut value = base();
            mutate(&mut value);
            let found = codes(&value);
            assert!(
                found.contains(code),
                "the input made for `{code}` produced {found:?}"
            );
            produced.insert(*code);
        }
        // The one code that needs the specification.
        let set = read(&base());
        let quotes = set.quote_problems("A different sentence.");
        assert_eq!(
            quotes.iter().map(|p| p.code).collect::<Vec<_>>(),
            vec!["quote-not-found"]
        );
        produced.insert("quote-not-found");
        let listed: BTreeSet<&str> = PROBLEM_CODES.iter().copied().collect();
        assert_eq!(listed.len(), PROBLEM_CODES.len(), "a code is listed twice");
        assert_eq!(produced, listed);
    }

    /// A payload type error is reported on the outbound side too, not only on
    /// the event sent in.
    #[test]
    fn an_expected_output_is_held_to_its_declared_payload() {
        let mut value = base();
        value["scenarios"][0]["steps"][0]["expect"]["outbound"][0]["payload"] =
            json!({"n": "one", "extra": 1});
        let found = codes(&value);
        assert!(found.contains(&"payload-type"), "{found:?}");
        assert!(found.contains(&"unknown-payload-field"), "{found:?}");
    }

    /// An integer field holds a whole number, a number field holds either, and
    /// neither holds text.
    #[test]
    fn a_payload_value_must_be_of_the_declared_type() {
        for (declared, given, ok) in [
            ("integer", json!(3), true),
            ("integer", json!(3.5), false),
            ("number", json!(3), true),
            ("number", json!(3.5), true),
            ("number", json!("3"), false),
            ("text", json!("a"), true),
            ("text", json!(1), false),
            ("boolean", json!(true), true),
            ("boolean", json!(1), false),
        ] {
            let mut value = base();
            value["interface"]["inputs"][0]["payload"]["n"] = json!(declared);
            value["scenarios"][0]["steps"][0]["payload"]["n"] = given.clone();
            // the expectation that would also be mistyped is dropped
            value["scenarios"][0]["steps"][0]["expect"]["outbound"][0]
                .as_object_mut()
                .unwrap()
                .remove("payload");
            let found = codes(&value);
            assert_eq!(
                !found.contains(&"payload-type"),
                ok,
                "{declared} given {given}: {found:?}"
            );
        }
    }

    #[test]
    fn a_wrapped_sentence_is_found_and_a_paraphrase_is_not() {
        let mut value = base();
        value["scenarios"][0]["quote"] = json!("The machine\n   goes.");
        let set = read(&value);
        assert_eq!(
            set.quote_problems("Before.  The machine goes.\nAfter."),
            Vec::new()
        );
        assert_eq!(
            set.quote_problems("The machine moves.")
                .iter()
                .map(|p| p.code)
                .collect::<Vec<_>>(),
            vec!["quote-not-found"]
        );
        // case is not forgiven, and neither is a missing word
        assert_eq!(set.quote_problems("the machine goes.").len(), 1);
        assert_eq!(set.quote_problems("The machine.").len(), 1);
    }

    #[test]
    fn an_empty_quote_is_never_found() {
        let mut value = base();
        value["scenarios"][0]["quote"] = json!("  ");
        let set = read(&value);
        let problems = set.quote_problems("anything at all");
        assert_eq!(problems.len(), 1);
        assert_eq!(problems[0].detail, "the quote is empty");
    }

    #[test]
    fn every_problem_in_a_set_is_reported_not_only_the_first() {
        let mut value = base();
        *at(&mut value, "/scenarios/0/steps/0/send") = json!("stop");
        *at(&mut value, "/scenarios/0/steps/0/expect/condition") = json!("broken");
        value["scenarios"][0]["bound"] = json!({"cycles": 0});
        let found = codes(&value);
        for code in ["unknown-input", "unknown-condition", "bound-zero"] {
            assert!(found.contains(&code), "{code} missing from {found:?}");
        }
    }

    #[test]
    fn a_problem_names_the_scenario_and_where() {
        let mut value = base();
        *at(&mut value, "/scenarios/0/steps/0/expect/condition") = json!("broken");
        let problems = read(&value).problems();
        assert_eq!(problems.len(), 1);
        assert_eq!(problems[0].scenario.as_deref(), Some("S1"));
        assert_eq!(problems[0].path, "scenarios[0].steps[0].expect.condition");
        assert!(
            problems[0].detail.contains("declared: idle"),
            "{}",
            problems[0].detail
        );
    }

    /// A scenario that is not runnable may say nothing about what it expects:
    /// it is a question, or it waits on a fact, and neither asserts anything
    /// yet. Only a runnable scenario that asserts nothing is wrong.
    #[test]
    fn only_a_runnable_scenario_must_assert_something() {
        let mut value = base();
        at(&mut value, "/scenarios/0/steps/0")
            .as_object_mut()
            .unwrap()
            .remove("expect");
        assert!(codes(&value).contains(&"runnable-asserts-nothing"));
        *at(&mut value, "/scenarios/0/status") = json!("blocked");
        value["scenarios"][0]["blocked_by"] = json!("a fact");
        assert_eq!(codes(&value), Vec::<&str>::new());
        *at(&mut value, "/scenarios/0/status") = json!("awaiting-decision");
        value["scenarios"][0]
            .as_object_mut()
            .unwrap()
            .remove("blocked_by");
        value["scenarios"][0]["decision"] = json!({"question": "q"});
        assert_eq!(codes(&value), Vec::<&str>::new());
    }

    /// An output may carry a route. One that names a type and a target is
    /// fine, and one that names neither is how a route nobody decided is said.
    #[test]
    fn an_output_may_name_the_route_it_leaves_by() {
        let mut value = base();
        value["interface"]["outputs"][0]["via"] = json!({"type": "x-caller", "target": "outside"});
        assert_eq!(violations(&value), Vec::<String>::new());
        assert_eq!(codes(&value), Vec::<&str>::new());
        let set = read(&value);
        let route = set.interface.outputs[0].via.as_ref().expect("a route");
        assert_eq!(
            (route.processor_type.as_str(), route.target.as_str()),
            ("x-caller", "outside")
        );
    }
}
