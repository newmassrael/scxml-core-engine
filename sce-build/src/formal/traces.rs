// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The judge of a traces manifest (docs/adr/0115, D4 and D8; docs/adr/0146, D1):
//! every recorded run of the code a behaviour of the model it names, and every
//! altered run refused where it was altered.
//!
//! A run is a file of the run trace format ([`crate::run_trace`]) a lane's
//! trace port wrote. It is judged by trace validation (Cirstea, Kuppe, Loillier
//! and Merz, SEFM 2024): the judge writes a TLA+ module that holds the run's
//! steps as a sequence and a cursor into it, and whose next-state relation is
//! the model's, constrained to the step the cursor names, which then moves on.
//! Each step is the action of AppendixD it names (`Start`, `Eventless`,
//! `Internal`, `External`, `Stop`), and what the step says must be what the
//! action made of `told`, `configuration`, `currentEvent` and `outcome`. The
//! environment's steps (`Due`, `Arrive`) are not in a run, since no runtime can
//! tell where an event on its external queue came from; they may come between
//! any two steps and leave the cursor where it is, and the external step that
//! takes an event says which.
//!
//! TLC explores that module whole with the invariant that the cursor has not
//! passed the last step. A behaviour that passes it is a behaviour of the model
//! that is the run, so TLC finding the invariant violated is the run ACCEPTED;
//! TLC finding it holds is the run REFUSED, and the furthest the cursor got in
//! the state graph TLC writes names the first step no behaviour explains. Any
//! other answer judged nothing.
//!
//! A trace that says too little is accepted though the run was wrong (D4), so
//! the format makes every step say every field its kind has, and the manifest
//! declares, for every kind of step a recorded run holds, an altered run: one
//! field of one step of that kind given another value. Each must be refused at
//! the very step that was altered; refused later, it was explained where it was
//! altered, and the step says less than the judge needs.
//!
//! The paths of a manifest are read relative to its directory, which holds the
//! models it names (as a models manifest's do), and every file of the directory
//! of a declared run is declared, so a run dropped from the manifest is seen.

use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;
use serde::Deserialize;
use serde_json::Value;

use super::models::{files_with, listed, unknown_keys, Report};
use super::tlc;
use super::toolchain::{self, TlcPin};
use super::FormalError;
use crate::run_trace::{fields_of, Fired, Microstep, RunTrace, Step};

/// The surface's stability (docs/adr/0146, D5; `SCE_WIRE_CONTRACTS.md`), which the
/// header of `schemas/sce-formal-traces.v1.schema.json` repeats.
pub const TRACES_SCHEMA_STATUS: &str = "pre-release";

const TRACE_KEYS: &[&str] = &["trace", "module", "config", "altered"];
const ALTERED_KEYS: &[&str] = &["step", "kind", "field", "value"];

/// The invariant of a trace module that its run has not been consumed whole.
pub const UNFINISHED: &str = "TraceUnfinished";

/// A traces manifest, as `schemas/sce-formal-traces.v1.schema.json` describes
/// it. A key the judge does not know is kept in `other` and refused by
/// [`declared`], with every other problem, rather than failing the read.
#[derive(Debug, Clone, Deserialize)]
pub struct Manifest {
    /// The version of the format; this judge reads 1.
    #[serde(default)]
    pub v: Option<Value>,
    #[serde(default)]
    pub traces: Vec<TraceDecl>,
    #[serde(flatten)]
    pub other: BTreeMap<String, Value>,
}

/// One recorded run, the model it is a run of, and its altered runs.
#[derive(Debug, Clone, Deserialize)]
pub struct TraceDecl {
    /// The run trace's file.
    pub trace: String,
    /// The module the run is of, beside the manifest.
    pub module: String,
    /// A configuration of that module whose constants the run is judged under.
    pub config: String,
    #[serde(default)]
    pub altered: Vec<Altered>,
    #[serde(flatten)]
    pub other: BTreeMap<String, Value>,
}

/// One altered run: the run with one field of one step given another value.
#[derive(Debug, Clone, Deserialize)]
pub struct Altered {
    /// The step's place in the run's `steps`, from 0.
    pub step: usize,
    /// The step's kind, which the step at that place must be.
    pub kind: String,
    /// The field given another value, one the kind says.
    pub field: String,
    /// The value it is given, as the run trace format writes that field.
    pub value: Value,
    #[serde(flatten)]
    pub other: BTreeMap<String, Value>,
}

/// Read a manifest's text. A text that is no manifest is one problem of the
/// declaration.
pub fn parse(text: &str) -> Result<Manifest, String> {
    serde_json::from_str(text).map_err(|e| format!("not a traces manifest: {e}"))
}

/// The head of a section of a configuration.
static SECTION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^(CONSTANTS?|INIT|NEXT|SPECIFICATION|INVARIANTS?|PROPERT(?:Y|IES)|CHECK_DEADLOCK|SYMMETRY|VIEW|ALIAS|POSTCONDITION|(?:ACTION_)?CONSTRAINTS?)\b",
    )
    .expect("a valid expression")
});

/// A state of the graph TLC writes, by the cursor of the trace module.
static CURSOR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"traceCursor = (\d+)").expect("a valid expression"));

/// The CONSTANTS section of a configuration, its head and its lines, and
/// nothing else of it: what the trace module of the same module is given.
pub fn constants_section(config_text: &str) -> String {
    let mut kept = Vec::new();
    let mut inside = false;
    for line in config_text.lines() {
        if SECTION.is_match(line) {
            inside = line.starts_with("CONSTANT");
        }
        if inside {
            kept.push(line);
        }
    }
    kept.join("\n")
}

/// The configuration a trace module is checked under: the constants of the
/// model's own configuration, the trace module's initial predicate and
/// next-state relation, and the invariant whose violation is the run accepted.
pub fn trace_config(config_text: &str) -> String {
    format!(
        "{}\nINIT TraceInit\nNEXT TraceNext\nINVARIANT {UNFINISHED}\nCHECK_DEADLOCK FALSE\n",
        constants_section(config_text)
    )
}

/// A TLA+ string literal of `text`.
fn tla_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        if c == '"' || c == '\\' {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('"');
    out
}

fn tla_sequence(items: impl Iterator<Item = String>) -> String {
    let items: Vec<String> = items.collect();
    if items.is_empty() {
        "<<>>".to_string()
    } else {
        format!("<< {} >>", items.join(", "))
    }
}

fn tla_names(names: &[String]) -> String {
    tla_sequence(names.iter().map(|name| tla_string(name)))
}

fn tla_set(names: &[String]) -> String {
    let items: Vec<String> = names.iter().map(|name| tla_string(name)).collect();
    format!("{{{}}}", items.join(", "))
}

/// An event's name as AppendixD holds it: the sequence of its tokens.
fn tla_event(name: &str) -> String {
    tla_sequence(name.split('.').map(tla_string))
}

fn tla_fired(fired: &[Fired]) -> String {
    tla_sequence(
        fired
            .iter()
            .map(|row| format!("<< {}, {} >>", tla_string(&row.source), row.index)),
    )
}

/// One step as a record of the trace module, with every field, so the
/// sequence is of one shape; a field the step's kind does not say holds a
/// value the trace module never reads.
fn tla_step(step: &Step) -> String {
    let none = || "None".to_string();
    let (event, microstep, entered, configuration, exited, outcome) = match step {
        Step::Start {
            entered,
            configuration,
        } => (
            "NoEvent".to_string(),
            None,
            tla_names(entered),
            tla_set(configuration),
            "<<>>".to_string(),
            none(),
        ),
        Step::Eventless(said) => (
            "NoEvent".to_string(),
            Some(said),
            String::new(),
            String::new(),
            String::new(),
            none(),
        ),
        Step::Internal { event, microstep } | Step::External { event, microstep } => (
            tla_event(event),
            Some(microstep),
            String::new(),
            String::new(),
            String::new(),
            none(),
        ),
        Step::Stop { exited, outcome } => (
            "NoEvent".to_string(),
            None,
            "<<>>".to_string(),
            "{}".to_string(),
            tla_names(exited),
            outcome.as_deref().map_or_else(none, tla_string),
        ),
    };
    let (fired, exited, entered, configuration) = match microstep {
        Some(Microstep {
            fired,
            exited,
            entered,
            configuration,
        }) => (
            tla_fired(fired),
            tla_names(exited),
            tla_names(entered),
            tla_set(configuration),
        ),
        None => ("<<>>".to_string(), exited, entered, configuration),
    };
    format!(
        "[kind |-> {}, event |-> {event}, fired |-> {fired}, exited |-> {exited}, \
         entered |-> {entered}, configuration |-> {configuration}, outcome |-> {outcome}]",
        tla_string(step.kind())
    )
}

/// The trace module of `trace`, a run of `module`: its steps, a cursor into
/// them, and `module`'s next-state relation constrained to the step the cursor
/// names, with the environment's steps between any two.
pub fn trace_module(module: &str, trace: &RunTrace) -> String {
    let steps: Vec<String> = trace.steps.iter().map(tla_step).collect();
    format!(
        r#"---- MODULE {module}Trace ----
(***************************************************************************)
(* A run of the lane {lane} as a behaviour {module} must have (docs/adr/0115, *)
(* D4 and D8), written by sce-codegen formal traces and not kept: each step *)
(* is the action of AppendixD it names, and what it says is what that      *)
(* action makes of told, configuration, currentEvent and outcome.          *)
(***************************************************************************)
EXTENDS {module}

VARIABLE traceCursor

TraceSteps == <<
    {steps}
>>

\* The row of a transition a step names by its source and its position among
\* that source's own transitions (docs/adr/0089); 0, which is no row, when the
\* source has no such transition.
TraceRow(f) ==
    IF f[2] + 1 <= Len(TransitionsOf(f[1])) THEN TransitionsOf(f[1])[f[2] + 1] ELSE 0

RECURSIVE TraceRows(_)
TraceRows(fs) == IF fs = <<>> THEN <<>> ELSE <<TraceRow(Head(fs))>> \o TraceRows(Tail(fs))

TraceTold(s) == [exited |-> s.exited, fired |-> TraceRows(s.fired), entered |-> s.entered]

TraceInit == Init /\ traceCursor = 1

TraceStep(s) ==
    \/ /\ s.kind = "start"
       /\ Start
       /\ told'.entered = s.entered
       /\ configuration' = s.configuration
    \/ /\ s.kind = "eventless"
       /\ Eventless
       /\ told' = TraceTold(s)
       /\ configuration' = s.configuration
    \/ /\ s.kind = "internal"
       /\ Internal
       /\ currentEvent' = s.event
       /\ told' = TraceTold(s)
       /\ configuration' = s.configuration
    \/ /\ s.kind = "external"
       /\ External
       /\ currentEvent' = s.event
       /\ told' = TraceTold(s)
       /\ configuration' = s.configuration
    \/ /\ s.kind = "stop"
       /\ Stop
       /\ told'.exited = s.exited
       /\ outcome' = s.outcome

\* A step of the run, or a step of the environment, which no run says.
TraceNext ==
    \/ /\ traceCursor <= Len(TraceSteps)
       /\ TraceStep(TraceSteps[traceCursor])
       /\ traceCursor' = traceCursor + 1
    \/ /\ \/ \E i \in 1..Len(delayed) : Due(i)
          \/ \E e \in Outside : Arrive(e)
       /\ UNCHANGED traceCursor

\* Violated by a behaviour that is the whole run: the run accepted.
{UNFINISHED} == traceCursor <= Len(TraceSteps)
====
"#,
        lane = trace.lane,
        steps = steps.join(",\n    "),
    )
}

/// What TLC made of one trace module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Judged {
    /// A behaviour of the model is the whole run.
    Accepted,
    /// No behaviour explains the step at this place of the run's `steps`.
    RefusedAt(usize),
    /// TLC did not judge: what it answered.
    NoVerdict(String),
}

/// The furthest place of the run a behaviour reached, from the graph TLC
/// wrote: the place of the first step it did not explain.
pub fn first_unexplained(dot: &str) -> Option<usize> {
    CURSOR
        .captures_iter(dot)
        .filter_map(|found| found[1].parse::<usize>().ok())
        .max()
        .map(|cursor| cursor.saturating_sub(1))
}

/// What TLC's answer to a trace module says of the run.
pub fn judged(answer: &tlc::Answer) -> Judged {
    match answer.status {
        tlc::SAFETY if tlc::violated_invariants(&answer.output) == [UNFINISHED] => Judged::Accepted,
        tlc::HOLDS => match answer.graph.as_deref().and_then(first_unexplained) {
            Some(at) => Judged::RefusedAt(at),
            None => Judged::NoVerdict("TLC wrote no graph of the run's states".to_string()),
        },
        status => Judged::NoVerdict(format!(
            "TLC answered {status}, which judges nothing of a run"
        )),
    }
}

/// `trace_text` with `altered` applied, read back as a run; or why it is no
/// altered run: a place past the run, a step of another kind, a field the kind
/// does not say, a value that changes nothing, or one that leaves no run.
pub fn altered_run(trace_text: &str, altered: &Altered) -> Result<RunTrace, String> {
    let mut document: Value =
        serde_json::from_str(trace_text).map_err(|e| format!("not a run trace: {e}"))?;
    let at = altered.step;
    let Some(step) = document
        .get_mut("steps")
        .and_then(Value::as_array_mut)
        .and_then(|steps| steps.get_mut(at))
    else {
        return Err(format!("the run has no steps[{at}]"));
    };
    let kind = step.get("kind").and_then(Value::as_str).unwrap_or_default();
    if kind != altered.kind {
        return Err(format!(
            "steps[{at}] is a {kind} step, and the alteration is of a {} step",
            altered.kind
        ));
    }
    if !fields_of(kind).is_some_and(|fields| fields.contains(&altered.field.as_str())) {
        return Err(format!("a {kind} step does not say `{}`", altered.field));
    }
    let recorded = step.get(&altered.field).cloned().unwrap_or(Value::Null);
    if recorded == altered.value || same_set(&altered.field, &recorded, &altered.value) {
        return Err(format!(
            "steps[{at}] `{}` is given what the run says: the alteration changes nothing",
            altered.field
        ));
    }
    step[altered.field.as_str()] = altered.value.clone();
    RunTrace::from_json(&document.to_string())
        .map_err(|e| format!("the altered run is no run: {e}"))
}

/// Whether two values of a field are the same configuration in another order:
/// a configuration is a set.
fn same_set(field: &str, a: &Value, b: &Value) -> bool {
    if field != "configuration" {
        return false;
    }
    let as_set = |value: &Value| -> Option<BTreeSet<String>> {
        value
            .as_array()?
            .iter()
            .map(|item| item.as_str().map(str::to_string))
            .collect()
    };
    matches!((as_set(a), as_set(b)), (Some(a), Some(b)) if a == b)
}

/// Where a path the manifest names is: relative to the manifest's directory.
fn beside(dir: &Path, rel: &str) -> PathBuf {
    dir.join(rel)
}

/// What is wrong with the declaration itself, each as one line.
pub fn declared(manifest: &Manifest, dir: &Path) -> Vec<String> {
    let mut problems = Vec::new();
    if manifest.v != Some(Value::from(1)) {
        let given = manifest
            .v
            .as_ref()
            .map_or("none".to_string(), Value::to_string);
        problems.push(format!(
            "the manifest is of version {given}, and this judge reads version 1 (\"v\": 1)"
        ));
    }
    for key in manifest.other.keys() {
        problems.push(format!(
            "the manifest names {key:?}, which it does not declare: a tool is pinned by the judge (docs/adr/0146, D3)"
        ));
    }
    if manifest.traces.is_empty() {
        problems.push("the manifest declares no run, which judges nothing".to_string());
    }
    let mut declared_runs: BTreeSet<PathBuf> = BTreeSet::new();
    let mut run_dirs: BTreeSet<PathBuf> = BTreeSet::new();
    for decl in &manifest.traces {
        let name = &decl.trace;
        let unknown = unknown_keys(&decl.other, TRACE_KEYS);
        if !unknown.is_empty() {
            problems.push(format!(
                "{name}: unknown keys {}",
                listed(&unknown.into_iter().collect())
            ));
        }
        let path = beside(dir, name);
        if !declared_runs.insert(path.clone()) {
            problems.push(format!("{name}: declared twice"));
        }
        if let Some(parent) = path.parent() {
            run_dirs.insert(parent.to_path_buf());
        }
        if !beside(dir, &format!("{}.tla", decl.module)).is_file() {
            problems.push(format!("{name}: its model {} has no .tla", decl.module));
        }
        match std::fs::read_to_string(beside(dir, &decl.config)) {
            Ok(config) if constants_section(&config).trim().is_empty() => problems.push(format!(
                "{name}: {} gives the model no constants",
                decl.config
            )),
            Ok(_) => {}
            Err(e) => problems.push(format!("{name}: {} cannot be read: {e}", decl.config)),
        }
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) => {
                problems.push(format!("{name}: cannot be read: {e}"));
                continue;
            }
        };
        let trace = match RunTrace::from_json(&text) {
            Ok(trace) => trace,
            Err(e) => {
                problems.push(format!("{name}: {e}"));
                continue;
            }
        };
        let kinds: BTreeSet<&str> = trace.steps.iter().map(Step::kind).collect();
        let altered_kinds: BTreeSet<&str> = decl
            .altered
            .iter()
            .map(|altered| altered.kind.as_str())
            .collect();
        for kind in kinds.difference(&altered_kinds) {
            problems.push(format!(
                "{name}: no altered run of a {kind} step, so nothing shows the judge refuses a \
                 wrong one (docs/adr/0115, D4)"
            ));
        }
        for altered in &decl.altered {
            let unknown = unknown_keys(&altered.other, ALTERED_KEYS);
            if !unknown.is_empty() {
                problems.push(format!(
                    "{name}: steps[{}] altered: unknown keys {}",
                    altered.step,
                    listed(&unknown.into_iter().collect())
                ));
            }
            if let Err(problem) = altered_run(&text, altered) {
                problems.push(format!("{name}: {problem}"));
            }
        }
    }
    for run_dir in &run_dirs {
        for file in files_with(run_dir, "json") {
            if !declared_runs.contains(&file) {
                problems.push(format!(
                    "{}: a recorded run the manifest does not declare",
                    file.display()
                ));
            }
        }
    }
    problems
}

/// Judge `trace`, a run of `decl`'s model, by TLC.
fn judge_run(
    jar: &Path,
    library: Option<&Path>,
    dir: &Path,
    decl: &TraceDecl,
    config_text: &str,
    trace: &RunTrace,
    workers: NonZeroUsize,
) -> Result<(Judged, String), FormalError> {
    let carried = format!("{}Trace", decl.module);
    let extra = vec![(format!("{carried}.tla"), trace_module(&decl.module, trace))];
    let answer = tlc::run(&tlc::Run {
        jar,
        library,
        sources: dir,
        module: &carried,
        config_name: &format!("{carried}.cfg"),
        config_text: &trace_config(config_text),
        extra: &extra,
        workers,
        write_graph: true,
    })?;
    Ok((judged(&answer), answer.output))
}

fn tail_lines(output: &str, count: usize) -> String {
    let lines: Vec<&str> = output.lines().collect();
    lines[lines.len().saturating_sub(count)..].join("\n")
}

/// What a step of `trace` is, for a line of the report.
fn step_named(trace: &RunTrace, at: usize) -> String {
    match trace.steps.get(at) {
        Some(Step::Internal { event, .. } | Step::External { event, .. }) => {
            format!("steps[{at}] ({} {event})", trace.steps[at].kind())
        }
        Some(step) => format!("steps[{at}] ({})", step.kind()),
        None => format!("steps[{at}]"),
    }
}

/// Check every run `manifest_path` declares, against the models beside it, as
/// it declares them. Each run's line goes to `report` as it is judged. Each
/// run of TLC is given the `workers` the judge was, one run after another.
pub fn check(
    manifest_path: &Path,
    workers: NonZeroUsize,
    report: &mut Report<'_>,
) -> Result<(), FormalError> {
    let shown = manifest_path.display().to_string();
    let text = std::fs::read_to_string(manifest_path).map_err(|e| FormalError::Declaration {
        manifest: shown.clone(),
        problems: vec![format!("cannot be read: {e}")],
    })?;
    let manifest = parse(&text).map_err(|problem| FormalError::Declaration {
        manifest: shown.clone(),
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
            manifest: shown,
            problems,
        });
    }
    let pins = toolchain::pinned();
    let pin: TlcPin = pins.tlc.clone();
    let jar = toolchain::admitted_tlc(&pin)?;
    // A model beside the traces that extends Apalache is read with the pinned
    // Apalache's module, as the models judge reads it.
    let library = if super::models::modules_extend_apalache(dir) {
        Some(toolchain::admitted_apalache_modules(&pins.apalache)?)
    } else {
        None
    };
    let mut failures = Vec::new();
    let mut judged_runs = 0usize;
    for decl in &manifest.traces {
        let name = &decl.trace;
        let trace_text = std::fs::read_to_string(beside(dir, name)).unwrap_or_default();
        let config_text = std::fs::read_to_string(beside(dir, &decl.config)).unwrap_or_default();
        let trace = RunTrace::from_json(&trace_text).map_err(|e| FormalError::Declaration {
            manifest: shown.clone(),
            problems: vec![format!("{name}: {e}")],
        })?;
        judged_runs += 1;
        let (verdict, output) = judge_run(
            &jar,
            library.as_deref(),
            dir,
            decl,
            &config_text,
            &trace,
            workers,
        )?;
        let wrong = match verdict {
            Judged::Accepted => None,
            Judged::RefusedAt(at) => Some(format!(
                "the run of the lane {} is no behaviour of {}: no step of the model explains {}",
                trace.lane,
                decl.module,
                step_named(&trace, at)
            )),
            Judged::NoVerdict(why) => Some(why),
        };
        match wrong {
            None => (report.out)(&format!(
                "ok   {name}: a behaviour of {} ({} steps)",
                decl.module,
                trace.steps.len()
            )),
            Some(why) => {
                (report.out)(&format!("FAIL {name}: {why}"));
                (report.detail)(&tail_lines(&output, 25));
                failures.push(format!("{name}: {why}"));
            }
        }
        for altered in &decl.altered {
            judged_runs += 1;
            let shown_alteration = format!(
                "{name} with steps[{}] `{}` altered",
                altered.step, altered.field
            );
            let altered_trace =
                altered_run(&trace_text, altered).map_err(|problem| FormalError::Declaration {
                    manifest: shown.clone(),
                    problems: vec![format!("{name}: {problem}")],
                })?;
            let (verdict, output) = judge_run(
                &jar,
                library.as_deref(),
                dir,
                decl,
                &config_text,
                &altered_trace,
                workers,
            )?;
            let wrong = match verdict {
                Judged::RefusedAt(at) if at == altered.step => None,
                Judged::RefusedAt(at) => Some(format!(
                    "refused at {} and not where it was altered: the altered step is explained, \
                     so the step says less than the judge needs (docs/adr/0115, D4)",
                    step_named(&altered_trace, at)
                )),
                Judged::Accepted => Some(
                    "accepted: the judge takes a run the recorded one is not as a behaviour of \
                     the model"
                        .to_string(),
                ),
                Judged::NoVerdict(why) => Some(why),
            };
            match wrong {
                None => (report.out)(&format!("ok   {shown_alteration}: refused there")),
                Some(why) => {
                    (report.out)(&format!("FAIL {shown_alteration}: {why}"));
                    (report.detail)(&tail_lines(&output, 25));
                    failures.push(format!("{shown_alteration}: {why}"));
                }
            }
        }
    }
    if !failures.is_empty() {
        return Err(FormalError::Verdict {
            manifest: shown,
            runs: judged_runs,
            failures,
        });
    }
    (report.out)("every run is a behaviour of its model, and every altered run is refused");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const RUN: &str = r#"{
  "version": 1,
  "lane": "rust",
  "steps": [
    {"kind": "start", "entered": ["s0", "s01"], "configuration": ["s0", "s01"]},
    {"kind": "internal", "event": "go.on", "fired": [{"source": "s0", "index": 1}], "exited": ["s01", "s0"], "entered": ["pass"], "configuration": ["pass"]},
    {"kind": "stop", "exited": ["pass"], "outcome": "pass"}
  ]
}
"#;

    const CONFIG: &str = "SPECIFICATION Spec\nCONSTANTS\n    Root = \"scxml\"\n    States <- DocStates\nINVARIANTS\n    LegalConfiguration\nPROPERTIES\n    Refines\nCHECK_DEADLOCK FALSE\n";

    /// The declaration's checks, judged here with no TLC.
    struct Fixture {
        dir: crate::formal::WorkDir,
    }

    impl Fixture {
        fn new() -> Self {
            let fixture = Self {
                dir: crate::formal::WorkDir::new("test-traces").expect("a working directory"),
            };
            fixture.put("M.tla", "");
            fixture.put("M.cfg", CONFIG);
            std::fs::create_dir_all(fixture.dir.path().join("traces")).expect("a directory");
            fixture.put("traces/M.rust.json", RUN);
            fixture
        }

        fn put(&self, name: &str, text: &str) {
            std::fs::write(self.dir.path().join(name), text).expect("written");
        }

        fn manifest(&self) -> Value {
            json!({"v": 1, "traces": [{
                "trace": "traces/M.rust.json", "module": "M", "config": "M.cfg",
                "altered": [
                    {"step": 0, "kind": "start", "field": "entered", "value": ["s01", "s0"]},
                    {"step": 1, "kind": "internal", "field": "event", "value": "go"},
                    {"step": 2, "kind": "stop", "field": "outcome", "value": null},
                ],
            }]})
        }

        fn declared(&self, manifest: &Value) -> Vec<String> {
            declared(
                &serde_json::from_value(manifest.clone()).expect("a manifest"),
                self.dir.path(),
            )
        }
    }

    fn one_problem(problems: &[String], phrase: &str) {
        assert!(
            problems.len() == 1 && problems[0].contains(phrase),
            "expected one problem saying {phrase:?}, got {problems:?}"
        );
    }

    #[test]
    fn a_complete_declaration_has_no_problem() {
        let fixture = Fixture::new();
        assert_eq!(fixture.declared(&fixture.manifest()), Vec::<String>::new());
    }

    #[test]
    fn every_kind_a_run_holds_has_an_altered_run() {
        let fixture = Fixture::new();
        let mut manifest = fixture.manifest();
        manifest["traces"][0]["altered"]
            .as_array_mut()
            .expect("a list")
            .remove(1);
        one_problem(
            &fixture.declared(&manifest),
            "no altered run of a internal step",
        );
    }

    #[test]
    fn an_alteration_names_a_step_of_its_kind_and_a_field_of_that_kind() {
        let fixture = Fixture::new();
        let mut manifest = fixture.manifest();
        manifest["traces"][0]["altered"][1]["kind"] = json!("eventless");
        let problems = fixture.declared(&manifest);
        assert!(
            problems.iter().any(|p| p.contains(
                "steps[1] is a internal step, and the alteration is of a eventless step"
            )),
            "{problems:?}"
        );
        let mut manifest = fixture.manifest();
        manifest["traces"][0]["altered"][0]["field"] = json!("event");
        one_problem(
            &fixture.declared(&manifest),
            "a start step does not say `event`",
        );
        let mut manifest = fixture.manifest();
        manifest["traces"][0]["altered"][0]["step"] = json!(9);
        let problems = fixture.declared(&manifest);
        assert!(
            problems
                .iter()
                .any(|p| p.contains("the run has no steps[9]")),
            "{problems:?}"
        );
    }

    #[test]
    fn an_alteration_that_changes_nothing_is_refused() {
        let fixture = Fixture::new();
        let mut manifest = fixture.manifest();
        manifest["traces"][0]["altered"][1]["value"] = json!("go.on");
        one_problem(
            &fixture.declared(&manifest),
            "the alteration changes nothing",
        );
        // A configuration is a set: the same states in another order are the
        // same configuration.
        let mut manifest = fixture.manifest();
        manifest["traces"][0]["altered"][1] =
            json!({"step": 0, "kind": "start", "field": "configuration", "value": ["s01", "s0"]});
        let problems = fixture.declared(&manifest);
        assert!(
            problems
                .iter()
                .any(|p| p.contains("the alteration changes nothing")),
            "{problems:?}"
        );
    }

    #[test]
    fn an_alteration_that_leaves_no_run_is_refused() {
        let fixture = Fixture::new();
        let mut manifest = fixture.manifest();
        manifest["traces"][0]["altered"][1]["value"] = json!("a..b");
        one_problem(&fixture.declared(&manifest), "the altered run is no run");
    }

    #[test]
    fn every_recorded_run_beside_a_declared_one_is_declared() {
        let fixture = Fixture::new();
        fixture.put("traces/Other.rust.json", RUN);
        one_problem(
            &fixture.declared(&fixture.manifest()),
            "a recorded run the manifest does not declare",
        );
    }

    #[test]
    fn the_manifest_is_of_version_1_names_nothing_else_and_a_model_that_is_there() {
        let fixture = Fixture::new();
        let mut manifest = fixture.manifest();
        manifest["v"] = json!(2);
        one_problem(&fixture.declared(&manifest), "this judge reads version 1");
        let mut manifest = fixture.manifest();
        manifest["tlc"] = json!("1.8.0");
        one_problem(
            &fixture.declared(&manifest),
            "a tool is pinned by the judge",
        );
        let mut manifest = fixture.manifest();
        manifest["traces"][0]["note"] = json!("x");
        one_problem(&fixture.declared(&manifest), "unknown keys ['note']");
        let mut manifest = fixture.manifest();
        manifest["traces"][0]["module"] = json!("Missing");
        one_problem(
            &fixture.declared(&manifest),
            "its model Missing has no .tla",
        );
        fixture.put("M.cfg", "SPECIFICATION Spec\n");
        one_problem(
            &fixture.declared(&fixture.manifest()),
            "gives the model no constants",
        );
        let empty = json!({"v": 1, "traces": []});
        one_problem(&fixture.declared(&empty), "declares no run");
    }

    #[test]
    fn a_trace_module_holds_each_step_as_appendix_d_holds_its_values() {
        let trace = RunTrace::from_json(RUN).expect("a run");
        let module = trace_module("M", &trace);
        assert!(module.starts_with("---- MODULE MTrace ----"), "{module}");
        assert!(module.contains("EXTENDS M\n"));
        assert!(module.contains(
            r#"[kind |-> "start", event |-> NoEvent, fired |-> <<>>, exited |-> <<>>, entered |-> << "s0", "s01" >>, configuration |-> {"s0", "s01"}, outcome |-> None]"#
        ), "{module}");
        assert!(module.contains(
            r#"[kind |-> "internal", event |-> << "go", "on" >>, fired |-> << << "s0", 1 >> >>, exited |-> << "s01", "s0" >>, entered |-> << "pass" >>, configuration |-> {"pass"}, outcome |-> None]"#
        ), "{module}");
        assert!(module.contains(
            r#"[kind |-> "stop", event |-> NoEvent, fired |-> <<>>, exited |-> << "pass" >>, entered |-> <<>>, configuration |-> {}, outcome |-> "pass"]"#
        ), "{module}");
        assert!(module.contains(&format!("{UNFINISHED} == traceCursor <= Len(TraceSteps)")));
        assert!(module.trim_end().ends_with("===="));
    }

    #[test]
    fn a_trace_is_judged_under_the_constants_of_its_model_alone() {
        assert_eq!(
            trace_config(CONFIG),
            "CONSTANTS\n    Root = \"scxml\"\n    States <- DocStates\nINIT TraceInit\nNEXT TraceNext\nINVARIANT TraceUnfinished\nCHECK_DEADLOCK FALSE\n"
        );
    }

    #[test]
    fn a_name_is_a_tla_string() {
        assert_eq!(tla_string(r#"a"b\c"#), r#""a\"b\\c""#);
        assert_eq!(tla_event("done.state.p0"), r#"<< "done", "state", "p0" >>"#);
    }

    fn answer(status: i32, output: &str, graph: Option<&str>) -> tlc::Answer {
        tlc::Answer {
            status,
            output: output.to_string(),
            graph: graph.map(str::to_string),
        }
    }

    #[test]
    fn tlcs_answer_is_read_as_a_verdict_on_the_run() {
        assert_eq!(
            judged(&answer(
                tlc::SAFETY,
                "Error: Invariant TraceUnfinished is violated.",
                None
            )),
            Judged::Accepted
        );
        assert!(matches!(
            judged(&answer(
                tlc::SAFETY,
                "Error: Invariant LegalConfiguration is violated.",
                None
            )),
            Judged::NoVerdict(_)
        ));
        let graph = "strict digraph DiskGraph {\n1 [label=\"/\\\\ traceCursor = 1\"]\n2 [label=\"/\\\\ traceCursor = 3\"]\n3 [label=\"/\\\\ traceCursor = 2\"]\n}";
        assert_eq!(first_unexplained(graph), Some(2));
        assert_eq!(
            judged(&answer(tlc::HOLDS, "", Some(graph))),
            Judged::RefusedAt(2)
        );
        assert!(matches!(
            judged(&answer(tlc::HOLDS, "", None)),
            Judged::NoVerdict(_)
        ));
        assert!(matches!(
            judged(&answer(255, "", None)),
            Judged::NoVerdict(_)
        ));
    }

    #[test]
    fn an_altered_run_reads_with_the_one_field_changed() {
        let altered: Altered = serde_json::from_value(
            json!({"step": 1, "kind": "internal", "field": "fired", "value": [{"source": "s0", "index": 0}]}),
        )
        .expect("an alteration");
        let run = altered_run(RUN, &altered).expect("an altered run");
        let recorded = RunTrace::from_json(RUN).expect("a run");
        assert_eq!(run.steps[0], recorded.steps[0]);
        assert_eq!(run.steps[2], recorded.steps[2]);
        let Step::Internal { microstep, .. } = &run.steps[1] else {
            panic!("the altered step keeps its kind");
        };
        assert_eq!(
            microstep.fired,
            vec![Fired {
                source: "s0".to_string(),
                index: 0
            }]
        );
    }

    #[test]
    fn the_status_of_the_surface_is_the_schemas() {
        let schema: Value = serde_json::from_str(include_str!(
            "../../../schemas/sce-formal-traces.v1.schema.json"
        ))
        .expect("the schema is JSON");
        assert_eq!(
            schema["x-sce-schema-status"],
            Value::from(TRACES_SCHEMA_STATUS)
        );
    }
}
