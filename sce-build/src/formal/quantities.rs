// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The judge of a quantities manifest (docs/adr/0119; the level of quantity):
//! every JANI model of a document held to the TLA+ model of the same document,
//! so that the second encoding stays an encoding and never becomes a second
//! semantics (D4).
//!
//! - **The model is the generator's.** The JANI model beside the manifest must
//!   be what `sce-codegen model --jani` writes of its document now
//!   ([`super::document_jani`]), byte for byte: it is regenerated, never
//!   edited, and a state of it is read as a state of the TLA+ module by the
//!   encoding that generation gives.
//! - **The graphs are one (D4.1).** With every distribution erased, the
//!   reachable state graph Storm builds of the JANI model equals the graph TLC
//!   writes of the TLA+ module under the configuration's constants, compared
//!   state by state over the module's variables and step by step, with the
//!   initial states, and not up to isomorphism. A model whose graph differs is
//!   refused with the first state or step that differs.
//! - **The runs are accepted by both (D4.2).** Every recorded run a traces
//!   manifest declares of the module (`super::traces`) is replayed over the
//!   explicit state space Storm built, step by step, each step an edge of the
//!   action it names whose target says what the step says, with the
//!   environment's steps between any two; and each altered run is refused at
//!   the step that was altered, so the replay is shown able to refuse.
//!
//! D3's properties and D4.3's configurations that must change a verdict are
//! judged when the properties document carries them; a manifest of today
//! declares neither.

use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;
use serde::Deserialize;
use serde_json::Value as Json;

use super::document_jani::{self, Action, Encoding, Environment, JaniModel, NO_OUTCOME};
use super::document_model;
use super::models::{self, files_with, listed, unknown_keys, Report};
use super::storm::{self, Space};
use super::tla_value::{parse_state, shown, State, Value};
use super::tlc;
use super::toolchain;
use super::traces;
use super::FormalError;
use crate::run_trace::{Fired, Microstep, RunTrace, Step};

/// The surface's stability (docs/adr/0146, D5; `SCE_WIRE_CONTRACTS.md`), which the
/// header of `schemas/sce-formal-quantities.v1.schema.json` repeats.
pub const QUANTITIES_SCHEMA_STATUS: &str = "pre-release";

const MODEL_KEYS: &[&str] = &[
    "jani",
    "document",
    "module",
    "config",
    "w3c_verdict",
    "outside",
    "max_outside",
    "traces",
];

/// A quantities manifest, as `schemas/sce-formal-quantities.v1.schema.json`
/// describes it. A key the judge does not know is kept in `other` and refused
/// by [`declared`], with every other problem, rather than failing the read.
#[derive(Debug, Clone, Deserialize)]
pub struct Manifest {
    /// The version of the format; this judge reads 1.
    #[serde(default)]
    pub v: Option<Json>,
    #[serde(default)]
    pub models: Vec<ModelDecl>,
    #[serde(flatten)]
    pub other: BTreeMap<String, Json>,
}

/// One JANI model, the document it is written from, and the TLA+ model it is
/// held to.
#[derive(Debug, Clone, Deserialize)]
pub struct ModelDecl {
    /// The JANI model, beside the manifest.
    pub jani: String,
    /// The document both models are written from, relative to the manifest.
    pub document: String,
    /// The TLA+ module of the same document, beside the manifest; its name is
    /// the JANI model's.
    pub module: String,
    /// A configuration of that module, whose constants both are judged under.
    pub config: String,
    /// Whether the module is written with the W3C verdict, as its generation
    /// asks (`sce-codegen model --w3c-verdict`).
    #[serde(default)]
    pub w3c_verdict: bool,
    /// The events that may come from outside, which the configuration gives
    /// the module as `Outside`.
    #[serde(default)]
    pub outside: Vec<String>,
    /// The most of them that come, the configuration's `MaxOutside`.
    #[serde(default)]
    pub max_outside: u32,
    /// The traces manifest whose recorded runs of the module are replayed,
    /// relative to the manifest.
    pub traces: String,
    #[serde(flatten)]
    pub other: BTreeMap<String, Json>,
}

impl ModelDecl {
    /// The environment the JANI model is written in.
    pub fn environment(&self) -> Environment {
        Environment {
            outside: self
                .outside
                .iter()
                .map(|event| event.split('.').map(str::to_string).collect())
                .collect(),
            max_outside: self.max_outside,
        }
    }
}

/// Read a manifest's text. A text that is no manifest is one problem of the
/// declaration.
pub fn parse(text: &str) -> Result<Manifest, String> {
    serde_json::from_str(text).map_err(|e| format!("not a quantities manifest: {e}"))
}

/// The recorded runs a traces manifest declares of `module`, with the
/// directory their paths are read from.
fn runs_of(traces_path: &Path, module: &str) -> Result<(PathBuf, Vec<traces::TraceDecl>), String> {
    let text = std::fs::read_to_string(traces_path)
        .map_err(|e| format!("{} cannot be read: {e}", traces_path.display()))?;
    let manifest = traces::parse(&text)?;
    let dir = traces_path
        .parent()
        .filter(|dir| !dir.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
        .to_path_buf();
    let runs = manifest
        .traces
        .into_iter()
        .filter(|decl| decl.module == module)
        .collect();
    Ok((dir, runs))
}

/// What is wrong with the declaration itself, each as one line.
pub fn declared(manifest: &Manifest, dir: &Path) -> Vec<String> {
    let mut problems = Vec::new();
    if manifest.v != Some(Json::from(1)) {
        let given = manifest
            .v
            .as_ref()
            .map_or("none".to_string(), Json::to_string);
        problems.push(format!(
            "the manifest is of version {given}, and this judge reads version 1 (\"v\": 1)"
        ));
    }
    for key in manifest.other.keys() {
        problems.push(format!(
            "the manifest names {key:?}, which it does not declare: a tool is pinned by the judge (docs/adr/0146, D3)"
        ));
    }
    if manifest.models.is_empty() {
        problems.push("the manifest declares no model, which judges nothing".to_string());
    }
    let mut declared_models: BTreeSet<PathBuf> = BTreeSet::new();
    for decl in &manifest.models {
        let name = &decl.jani;
        let unknown = unknown_keys(&decl.other, MODEL_KEYS);
        if !unknown.is_empty() {
            problems.push(format!(
                "{name}: unknown keys {}",
                listed(&unknown.into_iter().collect())
            ));
        }
        let path = dir.join(name);
        if !declared_models.insert(path.clone()) {
            problems.push(format!("{name}: declared twice"));
        }
        if !path.is_file() {
            problems.push(format!(
                "{name}: no such model: sce-codegen model {} --module {} --jani writes it",
                decl.document, decl.module
            ));
        }
        if !dir.join(&decl.document).is_file() {
            problems.push(format!(
                "{name}: its document {} is not there",
                decl.document
            ));
        }
        if !dir.join(format!("{}.tla", decl.module)).is_file() {
            problems.push(format!(
                "{name}: its TLA+ model {} has no .tla",
                decl.module
            ));
        }
        match std::fs::read_to_string(dir.join(&decl.config)) {
            Ok(config) if traces::constants_section(&config).trim().is_empty() => problems.push(
                format!("{name}: {} gives the model no constants", decl.config),
            ),
            Ok(_) => {}
            Err(e) => problems.push(format!("{name}: {} cannot be read: {e}", decl.config)),
        }
        for event in &decl.outside {
            if event.split('.').any(str::is_empty) {
                problems.push(format!(
                    "{name}: the event {event:?} from outside has an empty token"
                ));
            }
        }
        match runs_of(&dir.join(&decl.traces), &decl.module) {
            Ok((_, runs)) if runs.is_empty() => problems.push(format!(
                "{name}: {} declares no recorded run of {}, so no run is replayed (docs/adr/0119, D4.2)",
                decl.traces, decl.module
            )),
            Ok(_) => {}
            Err(problem) => problems.push(format!("{name}: {problem}")),
        }
    }
    for file in files_with(dir, "jani") {
        if !declared_models.contains(&file) {
            problems.push(format!(
                "{}: a JANI model the manifest does not declare",
                file.display()
            ));
        }
    }
    problems
}

/// A state graph over the states of the TLA+ module: the states, the steps
/// between them and the initial ones, each state by its value.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Graph {
    pub states: BTreeSet<State>,
    pub steps: BTreeSet<(State, State)>,
    pub initial: BTreeSet<State>,
}

static DOT_STATE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^(-?\d+) \[label="(.*)""#).expect("a valid expression"));
static DOT_STEP: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(-?\d+) -> (-?\d+)").expect("a valid expression"));

/// A label of TLC's graph with its escapes undone: TLC doubles each backslash,
/// escapes each quote and writes each line break as `\n`.
fn unescaped(label: &str) -> String {
    let mut text = String::with_capacity(label.len());
    let mut chars = label.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            text.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => text.push('\n'),
            Some(other) => text.push(other),
            None => text.push('\\'),
        }
    }
    text
}

/// The graph TLC wrote in the dot form (`-dump dot`): each state a line with
/// its label, the initial ones filled, and each step a line of two states.
pub fn tlc_graph(dot: &str) -> Result<Graph, String> {
    let mut by_id: BTreeMap<String, State> = BTreeMap::new();
    let mut graph = Graph::default();
    let mut steps = Vec::new();
    for line in dot.lines() {
        if let Some(step) = DOT_STEP.captures(line) {
            steps.push((step[1].to_string(), step[2].to_string()));
            continue;
        }
        if let Some(found) = DOT_STATE.captures(line) {
            let state = parse_state(&unescaped(&found[2])).map_err(|why| {
                format!("TLC wrote a state the judge cannot read ({why}): {line}")
            })?;
            if line.contains("style = filled") {
                graph.initial.insert(state.clone());
            }
            if !graph.states.insert(state.clone()) {
                return Err(format!("TLC wrote one state twice: {}", shown(&state)));
            }
            by_id.insert(found[1].to_string(), state);
        }
    }
    for (from, to) in steps {
        let (Some(from), Some(to)) = (by_id.get(&from), by_id.get(&to)) else {
            return Err(format!(
                "TLC wrote a step {from} -> {to} of a state it did not write"
            ));
        };
        graph.steps.insert((from.clone(), to.clone()));
    }
    Ok(graph)
}

/// The space Storm built of a JANI model, read as a graph over the TLA+
/// module's states by the model's encoding.
pub fn storm_graph(space: &Space, encoding: &Encoding) -> Result<Graph, String> {
    let decoded = decoded(space, encoding)?;
    let mut graph = Graph::default();
    for state in &space.states {
        let value = &decoded[&state.id];
        if !graph.states.insert(value.clone()) {
            return Err(format!(
                "Storm built two states that are one state of the TLA+ module: {}",
                shown(value)
            ));
        }
        if state.labels.contains(storm::INITIAL) {
            graph.initial.insert(value.clone());
        }
    }
    for (from, _, to) in space.steps() {
        let (Some(from), Some(to)) = (decoded.get(&from), decoded.get(&to)) else {
            return Err(format!(
                "Storm built a step {from} -> {to} of a state it did not export"
            ));
        };
        graph.steps.insert((from.clone(), to.clone()));
    }
    Ok(graph)
}

/// Each state of the space by Storm's number, read as the TLA+ module's.
fn decoded(space: &Space, encoding: &Encoding) -> Result<BTreeMap<u64, State>, String> {
    space
        .states
        .iter()
        .map(|state| {
            encoding
                .state(&state.valuation)
                .map(|decoded| (state.id, decoded))
                .map_err(|why| format!("state {} of Storm's space: {why}", state.id))
        })
        .collect()
}

/// The first state, step or initial state of one graph that the other does
/// not have, said of whose it is; none when the two are one.
pub fn first_difference(tlc: &Graph, jani: &Graph) -> Option<String> {
    if let Some(state) = tlc.states.difference(&jani.states).next() {
        return Some(format!(
            "TLC reaches a state the JANI model does not: {}",
            shown(state)
        ));
    }
    if let Some(state) = jani.states.difference(&tlc.states).next() {
        return Some(format!(
            "the JANI model reaches a state TLC does not: {}",
            shown(state)
        ));
    }
    let step = |(from, to): &(State, State)| format!("{}  ->  {}", shown(from), shown(to));
    if let Some(missing) = tlc.steps.difference(&jani.steps).next() {
        return Some(format!(
            "TLC takes a step the JANI model does not: {}",
            step(missing)
        ));
    }
    if let Some(extra) = jani.steps.difference(&tlc.steps).next() {
        return Some(format!(
            "the JANI model takes a step TLC does not: {}",
            step(extra)
        ));
    }
    if tlc.initial != jani.initial {
        let shown_all =
            |states: &BTreeSet<State>| states.iter().map(shown).collect::<Vec<_>>().join("; ");
        return Some(format!(
            "the initial states differ: TLC's {}, the JANI model's {}",
            shown_all(&tlc.initial),
            shown_all(&jani.initial)
        ));
    }
    None
}

/// What a replay of a run over a JANI model's space made of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Replayed {
    /// A path of the space is the whole run.
    Accepted,
    /// No path of the space explains the step at this place of the run's
    /// `steps`.
    RefusedAt(usize),
}

/// What a step of a run must find in the state its edge reaches, as the trace
/// module of `super::traces` constrains the TLA+ module's step (`TraceStep`).
fn says(step: &Step, state: &State, encoding: &Encoding) -> bool {
    let names = |names: &[String]| Value::strings(names.iter().map(String::as_str));
    let set = |names: &[String]| Value::Set(names.iter().map(|name| Value::string(name)).collect());
    let rows = |fired: &[Fired]| {
        Value::Seq(
            fired
                .iter()
                .map(|row| Value::Int(encoding.trace_row(&row.source, row.index)))
                .collect(),
        )
    };
    let told_of = |said: &Microstep| {
        Value::record([
            ("exited", names(&said.exited)),
            ("fired", rows(&said.fired)),
            ("entered", names(&said.entered)),
        ])
    };
    let told = state.get("told");
    let told_field = |name: &str| told.and_then(|told| told.field(name));
    let configuration = state.get("configuration");
    match step {
        Step::Start {
            entered,
            configuration: said,
        } => told_field("entered") == Some(&names(entered)) && configuration == Some(&set(said)),
        Step::Eventless(said) => {
            told == Some(&told_of(said)) && configuration == Some(&set(&said.configuration))
        }
        Step::Internal { event, microstep } | Step::External { event, microstep } => {
            state.get("currentEvent") == Some(&Value::strings(event.split('.')))
                && told == Some(&told_of(microstep))
                && configuration == Some(&set(&microstep.configuration))
        }
        Step::Stop { exited, outcome } => {
            told_field("exited") == Some(&names(exited))
                && state.get("outcome")
                    == Some(&Value::string(outcome.as_deref().unwrap_or(NO_OUTCOME)))
        }
    }
}

/// Replay `trace` over `space`, the space Storm built of a JANI model with
/// `encoding`: from the initial states, each step of the run an edge of the
/// action it names whose target says what the step says, with any number of
/// the environment's steps before it, which no run says.
pub fn replay(space: &Space, encoding: &Encoding, trace: &RunTrace) -> Result<Replayed, String> {
    let decoded = decoded(space, encoding)?;
    let mut leaving: BTreeMap<u64, Vec<(Action, u64)>> = BTreeMap::new();
    for (from, action, to) in space.steps() {
        let action = Action::named(&action).ok_or_else(|| {
            format!("Storm built a step of the action {action:?}, which no model writes")
        })?;
        leaving.entry(from).or_default().push((action, to));
    }
    let mut frontier: BTreeSet<u64> = space
        .states
        .iter()
        .filter(|state| state.labels.contains(storm::INITIAL))
        .map(|state| state.id)
        .collect();
    for (at, step) in trace.steps.iter().enumerate() {
        let mut reached = frontier.clone();
        let mut pending: Vec<u64> = frontier.into_iter().collect();
        while let Some(state) = pending.pop() {
            for &(action, to) in leaving.get(&state).into_iter().flatten() {
                if action.of_environment() && reached.insert(to) {
                    pending.push(to);
                }
            }
        }
        let wanted = Action::named(step.kind()).ok_or_else(|| {
            format!(
                "steps[{at}] is of the kind {}, which no model writes",
                step.kind()
            )
        })?;
        frontier = reached
            .iter()
            .flat_map(|state| leaving.get(state).into_iter().flatten())
            .filter(|(action, to)| {
                *action == wanted
                    && decoded
                        .get(to)
                        .is_some_and(|state| says(step, state, encoding))
            })
            .map(|&(_, to)| to)
            .collect();
        if frontier.is_empty() {
            return Ok(Replayed::RefusedAt(at));
        }
    }
    Ok(Replayed::Accepted)
}

fn tail_lines(output: &str, count: usize) -> String {
    let lines: Vec<&str> = output.lines().collect();
    lines[lines.len().saturating_sub(count)..].join("\n")
}

/// The tools a check runs: TLC on the pinned Java runtime, and Storm.
struct Tools<'a> {
    tlc: tlc::Tools<'a>,
    storm: &'a Path,
    workers: NonZeroUsize,
}

/// The JANI model of `decl`'s document as the generator writes it now, or why
/// it writes none.
fn generated(dir: &Path, decl: &ModelDecl) -> Result<Result<JaniModel, String>, FormalError> {
    let path = dir.join(&decl.document);
    let shown_path = path.display().to_string();
    let mut model = match crate::parser::SCXMLParser::new().parse_file(&shown_path) {
        Ok(model) => model,
        Err(e) => return Ok(Err(format!("{} cannot be read: {e}", decl.document))),
    };
    crate::analyzer::analyze(&mut model, &shown_path);
    let options = document_model::Options {
        module: Some(&decl.module),
        w3c_verdict: decl.w3c_verdict,
    };
    match document_jani::write(&model, &decl.document, options, &decl.environment()) {
        Ok(written) => Ok(Ok(written)),
        Err(FormalError::NoTranslation { refusals, .. }) => Ok(Err(format!(
            "{} has no JANI model: {}",
            decl.document,
            refusals.join("; ")
        ))),
        Err(other) => Err(other),
    }
}

/// D4.1 for one model: Storm's space of it, once its graph is the TLA+
/// module's; or why one of the two could not be had, or how they differ.
fn same_graph(
    tools: &Tools<'_>,
    dir: &Path,
    decl: &ModelDecl,
    written: &JaniModel,
    report: &mut Report<'_>,
) -> Result<Result<Space, String>, FormalError> {
    let config_text = models::read(&dir.join(&decl.config));
    let dump = format!(
        "SPECIFICATION Spec\n{}CHECK_DEADLOCK FALSE\n",
        models::constants_config(&config_text)
    );
    let answer = tlc::run(&tlc::Run {
        tools: tools.tlc,
        sources: dir,
        module: &decl.module,
        config_name: &format!("{}Graph.cfg", decl.module),
        config_text: &dump,
        extra: &[],
        workers: tools.workers,
        write_graph: true,
    })?;
    if answer.status != tlc::HOLDS {
        (report.detail)(&tail_lines(&answer.output, 25));
        return Ok(Err(format!(
            "TLC answered {} where it was to write the graph of {}",
            answer.status, decl.module
        )));
    }
    let Some(dot) = &answer.graph else {
        return Ok(Err(format!("TLC wrote no graph of {}", decl.module)));
    };
    let tlc_side = match tlc_graph(dot) {
        Ok(graph) => graph,
        Err(why) => return Ok(Err(why)),
    };
    let found = answer.distinct_states();
    if found != u64::try_from(tlc_side.states.len()).ok() {
        return Ok(Err(format!(
            "TLC found {} states and wrote {}",
            found.map_or("no".to_string(), |n| n.to_string()),
            tlc_side.states.len()
        )));
    }
    let space = match storm::build(tools.storm, &written.text)? {
        Ok(space) => space,
        Err(why) => return Ok(Err(why)),
    };
    let jani_side = match storm_graph(&space, &written.encoding) {
        Ok(graph) => graph,
        Err(why) => return Ok(Err(why)),
    };
    if let Some(difference) = first_difference(&tlc_side, &jani_side) {
        return Ok(Err(format!(
            "its graph is not the graph of {} under {}: {difference}",
            decl.module, decl.config
        )));
    }
    (report.out)(&format!(
        "ok   {}: its graph is the graph of {} under {} ({} states, {} steps)",
        decl.jani,
        decl.module,
        decl.config,
        tlc_side.states.len(),
        tlc_side.steps.len()
    ));
    Ok(Ok(space))
}

/// D4.2 for one model: each recorded run of its module accepted by the
/// replay, and each altered run refused where it was altered. The failures,
/// each as one line.
fn runs_replayed(
    dir: &Path,
    decl: &ModelDecl,
    written: &JaniModel,
    space: &Space,
    report: &mut Report<'_>,
) -> Vec<String> {
    let mut failures = Vec::new();
    let (runs_dir, runs) = match runs_of(&dir.join(&decl.traces), &decl.module) {
        Ok(found) => found,
        Err(problem) => return vec![format!("{}: {problem}", decl.jani)],
    };
    for run in &runs {
        let name = format!("{} over {}", run.trace, decl.jani);
        let text = models::read(&runs_dir.join(&run.trace));
        let trace = match RunTrace::from_json(&text) {
            Ok(trace) => trace,
            Err(e) => {
                failures.push(format!("{name}: {e}"));
                continue;
            }
        };
        match replay(space, &written.encoding, &trace) {
            Ok(Replayed::Accepted) => (report.out)(&format!(
                "ok   {name}: a path of the model ({} steps)",
                trace.steps.len()
            )),
            Ok(Replayed::RefusedAt(at)) => failures.push(format!(
                "{name}: the run TLC accepts is no path of the model: nothing explains steps[{at}]"
            )),
            Err(why) => failures.push(format!("{name}: {why}")),
        }
        for altered in &run.altered {
            let shown_alteration = format!(
                "{name} with steps[{}] {:?} altered",
                altered.step, altered.field
            );
            let verdict = traces::altered_run(&text, altered)
                .and_then(|altered_trace| replay(space, &written.encoding, &altered_trace));
            match verdict {
                Ok(Replayed::RefusedAt(at)) if at == altered.step => {
                    (report.out)(&format!("ok   {shown_alteration}: refused there"))
                }
                Ok(Replayed::RefusedAt(at)) => failures.push(format!(
                    "{shown_alteration}: refused at steps[{at}] and not where it was altered"
                )),
                Ok(Replayed::Accepted) => failures.push(format!(
                    "{shown_alteration}: accepted, so the replay takes a run the recorded one is not"
                )),
                Err(why) => failures.push(format!("{shown_alteration}: {why}")),
            }
        }
    }
    failures
}

/// Check every JANI model `manifest_path` declares, against the TLA+ models
/// beside it, as it declares them. Each line goes to `report` as it is judged.
/// Each run of TLC is given the `workers` the judge was, one after another.
pub fn check(
    manifest_path: &Path,
    workers: NonZeroUsize,
    report: &mut Report<'_>,
) -> Result<(), FormalError> {
    let shown_manifest = manifest_path.display().to_string();
    let text = std::fs::read_to_string(manifest_path).map_err(|e| FormalError::Declaration {
        manifest: shown_manifest.clone(),
        problems: vec![format!("cannot be read: {e}")],
    })?;
    let manifest = parse(&text).map_err(|problem| FormalError::Declaration {
        manifest: shown_manifest.clone(),
        problems: vec![problem],
    })?;
    let dir = manifest_path
        .parent()
        .filter(|dir| !dir.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let problems = declared(&manifest, dir);
    if !problems.is_empty() {
        for problem in &problems {
            (report.out)(&format!("FAIL {problem}"));
        }
        return Err(FormalError::Declaration {
            manifest: shown_manifest,
            problems,
        });
    }
    let pins = toolchain::pinned();
    let jar = toolchain::admitted_tlc(&pins.tlc)?;
    // TLC runs on the pinned Java runtime, never the machine's.
    let java = toolchain::java_program(&toolchain::admitted_java(&pins.java)?);
    let library = if models::modules_extend_apalache(dir) {
        Some(toolchain::admitted_apalache_modules(&pins.apalache)?)
    } else {
        None
    };
    let storm_program = toolchain::admitted_storm(&pins.storm)?;
    let tools = Tools {
        tlc: tlc::Tools {
            java: &java,
            jar: &jar,
            library: library.as_deref(),
        },
        storm: &storm_program,
        workers,
    };
    let mut failures = Vec::new();
    for decl in &manifest.models {
        let written = match generated(dir, decl)? {
            Ok(written) => written,
            Err(why) => {
                (report.out)(&format!("FAIL {}: {why}", decl.jani));
                failures.push(format!("{}: {why}", decl.jani));
                continue;
            }
        };
        if models::read(&dir.join(&decl.jani)) != written.text {
            let why = format!(
                "not what sce-codegen model {} --module {} --jani writes of it: regenerate it, \
                 never edit it",
                decl.document, decl.module
            );
            (report.out)(&format!("FAIL {}: {why}", decl.jani));
            failures.push(format!("{}: {why}", decl.jani));
            continue;
        }
        let space = match same_graph(&tools, dir, decl, &written, report)? {
            Ok(space) => space,
            Err(why) => {
                (report.out)(&format!("FAIL {}: {why}", decl.jani));
                failures.push(format!("{}: {why}", decl.jani));
                continue;
            }
        };
        for failure in runs_replayed(dir, decl, &written, &space, report) {
            (report.out)(&format!("FAIL {failure}"));
            failures.push(failure);
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(FormalError::Verdict {
            manifest: shown_manifest,
            runs: manifest.models.len(),
            failures,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::SCXMLParser;
    use serde_json::json;

    const DELAYED: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
            datamodel="null" initial="a">
        <state id="a">
            <onentry><send event="t" delay="1s"/></onentry>
            <transition event="t" target="pass"/>
        </state>
        <final id="pass"/>
    </scxml>"#;

    fn jani() -> JaniModel {
        let mut model = SCXMLParser::new()
            .parse_string(DELAYED, "doc")
            .expect("parse");
        crate::analyzer::analyze(&mut model, "");
        document_jani::write(
            &model,
            "doc.scxml",
            document_model::Options::default(),
            &Environment::default(),
        )
        .expect("a model")
    }

    /// A space of states numbered as the model's controls, each valued by its
    /// control, with the steps given; a state with none is a deadlock Storm
    /// gave a loop.
    fn space(count: u64, steps: &[(u64, &str, u64)]) -> Space {
        let states = (0..count)
            .map(|id| {
                let mut choices: Vec<storm::Choice> = steps
                    .iter()
                    .filter(|(from, _, _)| *from == id)
                    .map(|(_, action, to)| storm::Choice {
                        actions: [action.to_string()].into_iter().collect(),
                        successors: vec![*to],
                    })
                    .collect();
                let mut labels = BTreeSet::new();
                if id == 0 {
                    labels.insert(storm::INITIAL.to_string());
                }
                if choices.is_empty() {
                    labels.insert(storm::DEADLOCK.to_string());
                    choices.push(storm::Choice {
                        actions: BTreeSet::new(),
                        successors: vec![id],
                    });
                }
                storm::SpaceState {
                    id,
                    valuation: [(document_jani::CONTROL.to_string(), json!(id))]
                        .into_iter()
                        .collect(),
                    labels,
                    choices,
                }
            })
            .collect();
        Space { states }
    }

    const STEPS: [(u64, &str, u64); 4] = [
        (0, "start", 1),
        (1, "due", 2),
        (2, "external", 3),
        (3, "stop", 4),
    ];

    fn run() -> RunTrace {
        RunTrace {
            lane: "rust".to_string(),
            steps: vec![
                Step::Start {
                    entered: vec!["a".to_string()],
                    configuration: vec!["a".to_string()],
                },
                Step::External {
                    event: "t".to_string(),
                    microstep: Microstep {
                        fired: vec![Fired {
                            source: "a".to_string(),
                            index: 0,
                        }],
                        exited: vec!["a".to_string()],
                        entered: vec!["pass".to_string()],
                        configuration: vec!["pass".to_string()],
                    },
                },
                Step::Stop {
                    exited: vec!["pass".to_string()],
                    outcome: Some("pass".to_string()),
                },
            ],
        }
    }

    /// Turns red if the replay stops letting the environment's steps come
    /// between two of the run's, takes a step of another action, or lets a
    /// step that says other than its target through.
    #[test]
    fn a_run_is_a_path_and_an_altered_run_is_refused_where_altered() {
        let model = jani();
        assert_eq!(model.encoding.control_count(), 5);
        let built = space(5, &STEPS);
        assert_eq!(
            replay(&built, &model.encoding, &run()),
            Ok(Replayed::Accepted)
        );
        let mut other_event = run();
        if let Step::External { event, .. } = &mut other_event.steps[1] {
            *event = "u".to_string();
        }
        assert_eq!(
            replay(&built, &model.encoding, &other_event),
            Ok(Replayed::RefusedAt(1))
        );
        let mut no_outcome = run();
        no_outcome.steps[2] = Step::Stop {
            exited: vec!["pass".to_string()],
            outcome: None,
        };
        assert_eq!(
            replay(&built, &model.encoding, &no_outcome),
            Ok(Replayed::RefusedAt(2))
        );
        let without_due = space(5, &[(0, "start", 1), (2, "external", 3), (3, "stop", 4)]);
        assert_eq!(
            replay(&without_due, &model.encoding, &run()),
            Ok(Replayed::RefusedAt(1)),
            "the event the run takes is on its queue only once the send comes due"
        );
    }

    /// A state as TLC writes it in a label of its graph.
    fn dot_label(state: &State) -> String {
        state
            .iter()
            .map(|(name, value)| format!("/\\ {name} = {value}"))
            .collect::<Vec<_>>()
            .join("\n")
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
    }

    /// The graph TLC would write of the states and steps given, as its
    /// DotStateWriter writes one.
    fn dot(states: &[State], steps: &[(usize, usize)]) -> String {
        let mut text =
            String::from("strict digraph DiskGraph {\nnodesep=0.35;\nsubgraph cluster_graph {\n");
        text.push_str(&format!(
            "-90 [label=\"{}\",style = filled]\n",
            dot_label(&states[0])
        ));
        for (from, to) in steps {
            text.push_str(&format!("-9{from} -> -9{to};\n"));
            text.push_str(&format!(
                "-9{to} [label=\"{}\"];\n",
                dot_label(&states[*to])
            ));
        }
        text.push_str("{rank = same; -90;}\n}\n}");
        text
    }

    /// Turns red if a graph TLC writes is read otherwise than as the states it
    /// labels, or the comparison passes two graphs that differ by a step.
    #[test]
    fn the_graphs_are_compared_state_by_state_and_step_by_step() {
        let model = jani();
        let states: Vec<State> = (0..5)
            .map(|control| {
                model
                    .encoding
                    .state(
                        &[(document_jani::CONTROL.to_string(), json!(control))]
                            .into_iter()
                            .collect(),
                    )
                    .expect("a state")
            })
            .collect();
        let tlc_side =
            tlc_graph(&dot(&states, &[(0, 1), (1, 2), (2, 3), (3, 4)])).expect("TLC's graph");
        assert_eq!(tlc_side.states.len(), 5);
        assert_eq!(tlc_side.steps.len(), 4);
        assert_eq!(tlc_side.initial.len(), 1);
        let jani_side = storm_graph(&space(5, &STEPS), &model.encoding).expect("Storm's graph");
        assert_eq!(first_difference(&tlc_side, &jani_side), None);
        let fewer = tlc_graph(&dot(&states, &[(0, 1), (1, 2), (2, 3)])).expect("TLC's graph");
        let difference = first_difference(&fewer, &jani_side).expect("a difference");
        assert!(
            difference.starts_with("the JANI model reaches a state TLC does not"),
            "{difference}"
        );
        let mut skipping = jani_side.clone();
        skipping
            .steps
            .insert((states[1].clone(), states[3].clone()));
        let difference = first_difference(&tlc_side, &skipping).expect("a difference");
        assert!(
            difference.starts_with("the JANI model takes a step TLC does not"),
            "{difference}"
        );
    }

    /// Turns red if a label's escapes are undone otherwise than TLC writes
    /// them.
    #[test]
    fn a_label_is_read_with_its_escapes_undone() {
        assert_eq!(
            unescaped(r#"/\\ s = {\"a\"}\n/\\ x = 1"#),
            "/\\ s = {\"a\"}\n/\\ x = 1"
        );
    }

    /// Turns red if a declaration with a model, a document, a module or a run
    /// missing, a key the judge does not know, or a model it does not declare,
    /// passes.
    #[test]
    fn a_declaration_not_as_the_tree_is_refused_with_every_problem() {
        let dir = super::super::WorkDir::new("test-quantities").expect("a working directory");
        std::fs::write(dir.path().join("Stray.jani"), "{}").expect("written");
        std::fs::write(dir.path().join("traces.json"), r#"{"v": 1, "traces": []}"#)
            .expect("written");
        let manifest = parse(
            r#"{"v": 2, "storm": {}, "models": [{"jani": "M.jani", "document": "m.scxml",
                "module": "M", "config": "M.cfg", "traces": "traces.json", "note": "x"}]}"#,
        )
        .expect("a manifest");
        let problems = declared(&manifest, dir.path()).join("\n");
        for expected in [
            "this judge reads version 1",
            "the manifest names \"storm\"",
            "M.jani: unknown keys ['note']",
            "M.jani: no such model",
            "M.jani: its document m.scxml is not there",
            "M.jani: its TLA+ model M has no .tla",
            "M.jani: M.cfg cannot be read",
            "declares no recorded run of M",
            "Stray.jani: a JANI model the manifest does not declare",
        ] {
            assert!(
                problems.contains(expected),
                "missing {expected:?} in\n{problems}"
            );
        }
    }

    /// Turns red if the status this judge states of its surface leaves the
    /// one its schema's header states.
    #[test]
    fn the_status_of_the_surface_is_the_schemas() {
        let schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../schemas/sce-formal-quantities.v1.schema.json"
        ))
        .expect("the schema is JSON");
        assert_eq!(
            schema["x-sce-schema-status"],
            serde_json::Value::from(QUANTITIES_SCHEMA_STATUS)
        );
    }
}
