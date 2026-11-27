// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The judge of a models manifest (docs/adr/0115, D5 and D9; docs/adr/0146, D1):
//! every TLA+ model beside the manifest checked by TLC as the manifest declares.
//!
//! A run either holds or finds the violation it names, and both are verdicts:
//!
//! - A run that HOLDS explores every reachable state of its configuration and
//!   must explore exactly the number of distinct states the manifest records. A
//!   model whose behaviour shrank (a guard that became false, a variable that no
//!   longer moves) still reports "no error", so the count is what says the model
//!   is the one that was judged.
//! - A run that VIOLATES takes one part of the protocol out, and TLC must find a
//!   violation of the invariant it names, and of no other. A model that cannot
//!   fail shows nothing, and one that fails for another reason shows something
//!   else than it says.
//! - A module with a PROMISE refines an abstract module (ADR 0115, D1 and D2).
//!   Its holding configurations check the refinement, and each run that takes a
//!   part out says whether the promise is broken without it: where it is, TLC
//!   must find a violation of it too, and where it is not, the run says why the
//!   part is there.
//! - A module states its PROGRESS, the temporal properties that hold under its
//!   fairness, or says why it has none, and each has a run that fails without
//!   what it stands on. What must stay POSSIBLE is a state predicate p with
//!   `AG EF p`: from every reachable state a state where p holds can be reached
//!   (ADR 0115, D9). TLA+ cannot say it, so the judge decides it over the whole
//!   graph TLC writes.
//!
//! The manifest is complete in both directions: every configuration and module
//! beside it is declared, every PART of a model (a constant whose value switches
//! a piece of the protocol) has a run that takes it out, and everything a
//! holding run checks has a run that shows it can fail.
//!
//! In shards (ADR 0120, D3), `--shard k/N` checks the k-th of N parts of the runs,
//! spread by their recorded weight ([`shard_runs`]); every part checks the
//! manifest whole, and the parts are every run once.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;
use serde::Deserialize;
use serde_json::Value;

use super::tlc;
use super::toolchain::{self, TlcPin};
use super::FormalError;

/// The surface's stability (docs/adr/0146, D5; `SCE_WIRE_CONTRACTS.md`), which the
/// header of `schemas/sce-formal-models.v1.schema.json` repeats.
pub const MODELS_SCHEMA_STATUS: &str = "pre-release";

const MODULE_KEYS: &[&str] = &[
    "module",
    "decision",
    "parts",
    "runs",
    "promise",
    "refines",
    "abstract",
    "progress",
    "no_progress",
    "possible",
];
const RUN_KEYS: &[&str] = &[
    "config",
    "expect",
    "invariant",
    "property",
    "possible",
    "without",
    "against",
    "states",
    "promise",
    "why",
];

/// A models manifest, as `schemas/sce-formal-models.v1.schema.json` describes it. A key
/// the judge does not know is kept in `other` and refused by [`declared`], with
/// every other problem, rather than failing the read.
#[derive(Debug, Clone, Deserialize)]
pub struct Manifest {
    /// The version of the format; this judge reads 1.
    #[serde(default)]
    pub v: Option<Value>,
    #[serde(default)]
    pub models: Vec<Model>,
    #[serde(flatten)]
    pub other: BTreeMap<String, Value>,
}

/// One module and its runs.
#[derive(Debug, Clone, Deserialize)]
pub struct Model {
    pub module: String,
    #[serde(default)]
    pub decision: Option<String>,
    #[serde(default)]
    pub parts: BTreeMap<String, String>,
    #[serde(default)]
    pub runs: Vec<RunDecl>,
    #[serde(default)]
    pub promise: Option<String>,
    #[serde(default)]
    pub refines: Option<String>,
    #[serde(default, rename = "abstract")]
    pub is_abstract: bool,
    #[serde(default)]
    pub progress: Vec<String>,
    #[serde(default)]
    pub no_progress: Option<String>,
    #[serde(default)]
    pub possible: Vec<String>,
    #[serde(flatten)]
    pub other: BTreeMap<String, Value>,
}

/// One configuration and what TLC must answer of it.
#[derive(Debug, Clone, Deserialize)]
pub struct RunDecl {
    pub config: String,
    pub expect: String,
    #[serde(default)]
    pub invariant: Option<String>,
    #[serde(default)]
    pub property: Option<String>,
    #[serde(default)]
    pub possible: Option<String>,
    #[serde(default)]
    pub without: Option<String>,
    #[serde(default)]
    pub against: Option<String>,
    #[serde(default)]
    pub states: Option<Value>,
    #[serde(default)]
    pub promise: Option<String>,
    #[serde(default)]
    pub why: Option<String>,
    #[serde(flatten)]
    pub other: BTreeMap<String, Value>,
}

impl RunDecl {
    fn holds(&self) -> bool {
        self.expect == "holds"
    }

    fn violates(&self) -> bool {
        self.expect == "violates"
    }

    /// The distinct states a holding run records, when it records a count.
    fn recorded_states(&self) -> Option<u64> {
        self.states.as_ref().and_then(Value::as_u64)
    }
}

/// Read a manifest's text. A text that is no manifest is one problem of the
/// declaration.
pub fn parse(text: &str) -> Result<Manifest, String> {
    serde_json::from_str(text).map_err(|e| format!("not a models manifest: {e}"))
}

static SECTION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(CONSTANTS?|INVARIANTS?|PROPERT(?:Y|IES)|SPECIFICATION|CHECK_DEADLOCK|SYMMETRY|VIEW|(?:ACTION_)?CONSTRAINTS?)\b")
        .expect("a valid expression")
});
static REDUCES: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(SYMMETRY|VIEW|(?:ACTION_)?CONSTRAINTS?)\b").expect("a valid expression")
});
static ASSIGNMENT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(\w+)\s*=\s*(.+)$").expect("a valid expression"));
static PROPERTIES_HEAD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^PROPERT(Y|IES)\b").expect("a valid expression"));
static INVARIANTS_HEAD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^INVARIANTS?\b").expect("a valid expression"));
static CHECKS_HEAD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(INVARIANTS?|PROPERT(Y|IES))\b").expect("a valid expression"));
static UPPER_START: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[A-Z]").expect("a valid expression"));
/// What leaves a step of a proof out, or takes a formula as true.
static TRUSTED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b(OMITTED|AXIOM)\b").expect("a valid expression"));
static BLOCK_COMMENT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)\(\*.*?\*\)").expect("a valid expression"));
static GRAPH_STEP: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(-?\d+) -> (-?\d+)").expect("a valid expression"));
static GRAPH_STATE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^(-?\d+) \[label="(.*)""#).expect("a valid expression"));
static GRAPH_HOLDS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"sce_holds = (TRUE|FALSE)").expect("a valid expression"));

/// A line of a configuration without its comment, trimmed.
fn code_of(line: &str) -> &str {
    line.split("\\*").next().unwrap_or("").trim()
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_default()
}

/// The `Name = value` assignments of a configuration's CONSTANTS section.
pub fn constants_of(text: &str) -> BTreeMap<String, String> {
    let mut found = BTreeMap::new();
    let mut inside = false;
    for line in text.lines().map(code_of) {
        if SECTION.is_match(line) {
            inside = line.starts_with("CONSTANT");
            continue;
        }
        if inside {
            if let Some(assigned) = ASSIGNMENT.captures(line) {
                found.insert(assigned[1].to_string(), assigned[2].trim().to_string());
            }
        }
    }
    found
}

/// The names listed under a section whose head `head` matches.
fn listed_under(text: &str, head: &Regex) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let mut inside = false;
    for line in text.lines().map(code_of) {
        if head.is_match(line) {
            inside = true;
            found.extend(line.split_whitespace().skip(1).map(str::to_string));
        } else if SECTION.is_match(line) {
            inside = false;
        } else if inside && !line.is_empty() {
            found.extend(line.split_whitespace().map(str::to_string));
        }
    }
    found
}

/// The names listed under a configuration's PROPERTIES.
pub fn properties_of(text: &str) -> BTreeSet<String> {
    listed_under(text, &PROPERTIES_HEAD)
}

/// The names listed under a configuration's INVARIANTS.
pub fn invariants_of(text: &str) -> BTreeSet<String> {
    listed_under(text, &INVARIANTS_HEAD)
}

/// The sections of a configuration that take states away from what TLC explores.
pub fn reductions_of(text: &str) -> BTreeSet<String> {
    text.lines()
        .map(code_of)
        .filter_map(|line| REDUCES.captures(line).map(|found| found[1].to_string()))
        .collect()
}

/// The lines of a module, its comments left out, that take something on trust.
pub fn taken_on_trust(name: &str, text: &str) -> Vec<String> {
    let blanked = BLOCK_COMMENT.replace_all(text, |found: &regex::Captures<'_>| {
        "\n".repeat(found[0].matches('\n').count())
    });
    blanked
        .lines()
        .enumerate()
        .filter(|(_, line)| TRUSTED.is_match(line.split("\\*").next().unwrap_or("")))
        .map(|(at, line)| format!("{name}:{}: taken on trust: {}", at + 1, line.trim()))
        .collect()
}

/// `config_text` with its INVARIANTS and PROPERTIES replaced by the promise.
pub fn promise_only(config_text: &str, promise: &str) -> String {
    let mut kept: Vec<&str> = Vec::new();
    let mut skipping = false;
    for line in config_text.lines() {
        if CHECKS_HEAD.is_match(line) {
            skipping = true;
            continue;
        }
        if skipping && UPPER_START.is_match(line) {
            skipping = false;
        }
        if !skipping {
            kept.push(line);
        }
    }
    let promised = format!("    {promise}");
    kept.extend(["PROPERTIES", promised.as_str(), ""]);
    kept.join("\n")
}

/// A module whose states carry, as one more variable, whether `predicate` holds.
///
/// The variable is a function of the state, so the graph is the module's own. A
/// module whose specification is `Init`, `Next` and `vars` is what this reads.
pub fn possible_module(module: &str, predicate: &str) -> String {
    format!(
        "---- MODULE {module}Possible ----\nEXTENDS {module}\nVARIABLE sce_holds\n\
         PossibleInit == Init /\\ sce_holds = ({predicate})\n\
         PossibleNext == Next /\\ sce_holds' = ({predicate})'\n\
         PossibleSpec == PossibleInit /\\ [][PossibleNext]_<<vars, sce_holds>>\n====\n"
    )
}

/// `config_text` with the specification that carries a predicate, and nothing to check.
pub fn possible_config(config_text: &str) -> String {
    let mut kept = vec!["SPECIFICATION PossibleSpec"];
    let mut skipping = false;
    for line in config_text.lines() {
        if SECTION.is_match(line) {
            skipping = !(line.starts_with("CONSTANT") || line.starts_with("CHECK_DEADLOCK"));
        }
        if !skipping {
            kept.push(line);
        }
    }
    kept.push("");
    kept.join("\n")
}

/// A state graph: each state with whether its predicate holds, and the steps.
pub type Graph = (BTreeMap<String, bool>, Vec<(String, String)>);

/// The states of a graph TLC wrote, each with whether its predicate holds, and
/// its steps.
pub fn graph_of(dot: &str) -> Result<Graph, String> {
    let mut holds = BTreeMap::new();
    let mut steps = Vec::new();
    for line in dot.lines() {
        if let Some(step) = GRAPH_STEP.captures(line) {
            steps.push((step[1].to_string(), step[2].to_string()));
            continue;
        }
        if let Some(state) = GRAPH_STATE.captures(line) {
            let Some(value) = GRAPH_HOLDS.captures(&state[2]) else {
                let shown: String = line.chars().take(120).collect();
                return Err(format!(
                    "a state that does not say whether its predicate holds: {shown}"
                ));
            };
            holds.insert(state[1].to_string(), &value[1] == "TRUE");
        }
    }
    Ok((holds, steps))
}

/// `AG EF` of the predicate: every state is in the backward closure of those
/// that hold it.
pub fn always_possible(holds: &BTreeMap<String, bool>, steps: &[(String, String)]) -> bool {
    let mut before: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (origin, target) in steps {
        before
            .entry(target.as_str())
            .or_default()
            .push(origin.as_str());
    }
    let mut reach: Vec<&str> = holds
        .iter()
        .filter(|(_, held)| **held)
        .map(|(state, _)| state.as_str())
        .collect();
    let mut seen: BTreeSet<&str> = reach.iter().copied().collect();
    while let Some(state) = reach.pop() {
        for origin in before.get(state).into_iter().flatten() {
            if seen.insert(origin) {
                reach.push(origin);
            }
        }
    }
    !holds.is_empty() && holds.keys().all(|state| seen.contains(state.as_str()))
}

/// What a holding run checks and no run shows can fail.
///
/// A check that nothing can break says nothing, however true it is (an
/// always-true invariant passes every model). Each part taken out shows what it
/// breaks, so a name nothing breaks is one no part of the protocol stands
/// behind: it is given a run that does, or it is taken out of the module.
pub fn unshown(model: &Model, formal: &Path) -> Vec<String> {
    let present: Vec<&RunDecl> = model
        .runs
        .iter()
        .filter(|run| formal.join(&run.config).is_file())
        .collect();
    let mut checked = BTreeSet::new();
    for run in present.iter().filter(|run| run.holds()) {
        let text = read(&formal.join(&run.config));
        checked.extend(invariants_of(&text));
        checked.extend(properties_of(&text));
    }
    let mut shown: BTreeSet<String> = present
        .iter()
        .flat_map(|run| [&run.invariant, &run.property, &run.possible])
        .flatten()
        .cloned()
        .collect();
    if present
        .iter()
        .any(|run| run.promise.as_deref() == Some("broken"))
    {
        if let Some(promise) = &model.promise {
            shown.insert(promise.clone());
        }
    }
    checked.difference(&shown).cloned().collect()
}

pub(crate) fn unknown_keys(other: &BTreeMap<String, Value>, known: &[&str]) -> Vec<String> {
    other
        .keys()
        .filter(|key| !known.contains(&key.as_str()))
        .cloned()
        .collect()
}

pub(crate) fn listed(names: &BTreeSet<String>) -> String {
    let quoted: Vec<String> = names.iter().map(|name| format!("'{name}'")).collect();
    format!("[{}]", quoted.join(", "))
}

/// What is wrong with the declaration itself, each as one line.
pub fn declared(manifest: &Manifest, formal: &Path) -> Vec<String> {
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
    let mut configs: Vec<String> = Vec::new();
    let modules: Vec<&str> = manifest
        .models
        .iter()
        .map(|model| model.module.as_str())
        .collect();
    let mut on_disk_modules: Vec<PathBuf> = files_with(formal, "tla");
    on_disk_modules.sort();
    for module in &on_disk_modules {
        let name = module
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        problems.extend(taken_on_trust(&name, &read(module)));
    }
    for model in &manifest.models {
        let name = &model.module;
        let unknown = unknown_keys(&model.other, MODULE_KEYS);
        if !unknown.is_empty() {
            problems.push(format!(
                "{name}: unknown keys {}",
                listed(&unknown.into_iter().collect())
            ));
        }
        let mut taken_out: BTreeSet<String> = BTreeSet::new();
        if model.is_abstract {
            if !model.runs.is_empty() {
                problems.push(format!(
                    "{name}: an abstract module is run by what refines it, not by itself"
                ));
            }
            if !manifest
                .models
                .iter()
                .any(|m| m.refines.as_deref() == Some(name))
            {
                problems.push(format!("{name}: an abstract module nothing refines"));
            }
        } else if !model.runs.iter().any(RunDecl::holds) {
            problems.push(format!("{name}: no run says the whole protocol holds"));
        }
        if model.promise.is_some() != model.refines.is_some() {
            problems.push(format!(
                "{name}: a promise and what it refines are declared together"
            ));
        }
        if let Some(refines) = &model.refines {
            if !manifest
                .models
                .iter()
                .any(|m| &m.module == refines && m.is_abstract)
            {
                problems.push(format!(
                    "{name}: refines {refines}, which is no abstract module"
                ));
            }
        }
        for run in &model.runs {
            let config = &run.config;
            configs.push(config.clone());
            let stray = unknown_keys(&run.other, RUN_KEYS);
            if !stray.is_empty() {
                problems.push(format!(
                    "{config}: unknown keys {}",
                    listed(&stray.into_iter().collect())
                ));
            }
            let path = formal.join(config);
            if !path.is_file() {
                problems.push(format!("{config}: declared, and no such file"));
                continue;
            }
            let text = read(&path);
            let properties = properties_of(&text);
            // A behaviour cut off by a constraint ends in stuttering, where the
            // fairness it would have met is false, so a temporal property holds
            // of it for nothing (Lamport, Specifying Systems, 14.3.5).
            let reductions = reductions_of(&text);
            if !properties.is_empty() && !reductions.is_empty() {
                problems.push(format!(
                    "{config}: checks a temporal property under {}",
                    listed(&reductions)
                ));
            }
            let held = constants_of(&text);
            let off: BTreeSet<String> = model
                .parts
                .iter()
                .filter(|(part, whole)| held.get(*part) != Some(*whole))
                .map(|(part, _)| part.clone())
                .collect();
            if run.holds() {
                if !off.is_empty() {
                    problems.push(format!("{config}: holds, with {} taken out", listed(&off)));
                }
                if run.recorded_states().is_none() {
                    problems.push(format!(
                        "{config}: a run that holds records its distinct states"
                    ));
                }
                if let Some(promise) = &model.promise {
                    if !properties.contains(promise) {
                        problems.push(format!("{config}: holds, and does not check {promise}"));
                    }
                }
            } else if run.violates() {
                let named = [&run.invariant, &run.property, &run.possible]
                    .iter()
                    .filter(|key| key.is_some())
                    .count();
                if named != 1 {
                    problems.push(format!("{config}: a violation names its invariant, its property or what is not possible"));
                }
                // TLC does not name the temporal property it found violated, so it is
                // the only one.
                if let Some(property) = &run.property {
                    if properties != BTreeSet::from([property.clone()]) {
                        problems.push(format!(
                            "{config}: violates {property}, and checks {}",
                            listed(&properties)
                        ));
                    }
                }
                if run.without.is_some() == run.against.is_some() {
                    problems.push(format!(
                        "{config}: says either what part it takes out or what it goes against"
                    ));
                }
                if let Some(without) = &run.without {
                    taken_out.insert(without.clone());
                    if off != BTreeSet::from([without.clone()]) {
                        let what = if off.is_empty() {
                            "nothing".to_string()
                        } else {
                            listed(&off)
                        };
                        problems.push(format!(
                            "{config}: says it takes out {without}, and takes out {what}"
                        ));
                    }
                } else if !off.is_empty() {
                    problems.push(format!(
                        "{config}: takes out {} without saying so",
                        listed(&off)
                    ));
                }
                if model.promise.is_some() {
                    match run.promise.as_deref() {
                        Some("broken") => {}
                        Some("kept") => {
                            if run.why.as_deref().is_none_or(str::is_empty) {
                                problems.push(format!(
                                    "{config}: the promise is kept without the part, and the run says why it is there"
                                ));
                            }
                        }
                        _ => problems.push(format!(
                            "{config}: says whether the promise is broken or kept"
                        )),
                    }
                }
            } else {
                problems.push(format!(
                    "{config}: expect is 'holds' or 'violates', not '{}'",
                    run.expect
                ));
            }
        }
        for check in unshown(model, formal) {
            problems.push(format!(
                "{name}: checks {check}, and no run shows that it can fail"
            ));
        }
        let present: Vec<&RunDecl> = model
            .runs
            .iter()
            .filter(|run| formal.join(&run.config).is_file())
            .collect();
        let states_progress = !model.progress.is_empty();
        let says_none = model
            .no_progress
            .as_deref()
            .is_some_and(|why| !why.is_empty());
        if !model.is_abstract && states_progress == says_none {
            problems.push(format!(
                "{name}: states its progress, or says why it has none"
            ));
        }
        for possible in &model.possible {
            if !present
                .iter()
                .any(|r| r.holds() && r.possible.as_ref() == Some(possible))
            {
                problems.push(format!("{name}: no run says {possible} always possible"));
            }
            if !present
                .iter()
                .any(|r| r.violates() && r.possible.as_ref() == Some(possible))
            {
                problems.push(format!("{name}: no run shows {possible} lost"));
            }
        }
        let checked_possible: BTreeSet<&String> =
            present.iter().filter_map(|r| r.possible.as_ref()).collect();
        for stray in checked_possible {
            if !model.possible.contains(stray) {
                problems.push(format!(
                    "{name}: {stray} is checked and not declared possible"
                ));
            }
        }
        for progress in &model.progress {
            if !present.iter().any(|r| {
                r.holds() && properties_of(&read(&formal.join(&r.config))).contains(progress)
            }) {
                problems.push(format!("{name}: no run says {progress} holds"));
            }
            if !present
                .iter()
                .any(|r| r.violates() && r.property.as_ref() == Some(progress))
            {
                problems.push(format!("{name}: no run shows {progress} fail"));
            }
        }
        for part in model.parts.keys() {
            if !taken_out.contains(part) {
                problems.push(format!("{name}: no run takes out {part}"));
            }
        }
    }
    let declared_configs: BTreeSet<&str> = configs.iter().map(String::as_str).collect();
    let on_disk_configs: BTreeSet<String> = files_with(formal, "cfg")
        .iter()
        .filter_map(|path| {
            path.file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
        .collect();
    for missing in &on_disk_configs {
        if !declared_configs.contains(missing.as_str()) {
            problems.push(format!(
                "{missing}: a configuration the manifest does not declare"
            ));
        }
    }
    let twice: BTreeSet<&String> = configs
        .iter()
        .filter(|c| configs.iter().filter(|d| d == c).count() > 1)
        .collect();
    for config in twice {
        problems.push(format!("{config}: declared twice"));
    }
    let on_disk_stems: BTreeSet<String> = on_disk_modules
        .iter()
        .filter_map(|path| {
            path.file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
        })
        .collect();
    let declared_modules: BTreeSet<&str> = modules.iter().copied().collect();
    for stem in &on_disk_stems {
        if !declared_modules.contains(stem.as_str()) {
            problems.push(format!(
                "{stem}.tla: a module the manifest does not declare"
            ));
        }
    }
    for module in &declared_modules {
        if !on_disk_stems.contains(*module) {
            problems.push(format!("{module}: a declared module with no .tla"));
        }
    }
    problems
}

pub(crate) fn files_with(dir: &Path, extension: &str) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut found: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && path.extension().is_some_and(|ext| ext == extension))
        .collect();
    found.sort();
    found
}

/// What is wrong with TLC's answer to `run`, or None where it is as declared.
pub fn judged(run: &RunDecl, status: i32, output: &str) -> Option<String> {
    if run.holds() {
        if status != tlc::HOLDS {
            let named = if output.contains("Temporal properties were violated") {
                "Temporal properties were violated".to_string()
            } else {
                tlc::violated_invariants(output)
                    .first()
                    .map_or("no violation named".to_string(), |name| {
                        format!("Invariant {name} is violated")
                    })
            };
            return Some(format!(
                "expected no violation, and TLC answered {status} ({named})"
            ));
        }
        let Some(count) = tlc::distinct_states(output) else {
            return Some("TLC reported no state count".to_string());
        };
        let recorded = run.recorded_states();
        if recorded != Some(count) {
            let recorded = run
                .states
                .as_ref()
                .map_or("none".to_string(), Value::to_string);
            return Some(format!(
                "{count} distinct states, and the manifest records {recorded}"
            ));
        }
        return None;
    }
    if status != tlc::SAFETY && status != tlc::LIVENESS {
        return Some(format!("expected a violation, and TLC answered {status}"));
    }
    if let Some(property) = &run.property {
        return (status != tlc::LIVENESS)
            .then(|| format!("expected a violation of {property}, and TLC answered {status}"));
    }
    let violated = tlc::violated_invariants(output);
    let expected = run.invariant.clone().unwrap_or_default();
    if violated != [expected.clone()] {
        let found = if violated.is_empty() {
            "no invariant".to_string()
        } else {
            format!("{violated:?}")
        };
        return Some(format!(
            "expected a violation of {expected}, and TLC found {found}"
        ));
    }
    None
}

/// What is wrong with TLC's answer to the promise of a run that takes a part out.
pub fn promise_judged(run: &RunDecl, status: i32) -> Option<String> {
    match run.promise.as_deref() {
        Some("broken") if status != tlc::LIVENESS => Some(format!(
            "the promise is said to be broken, and TLC answered {status}"
        )),
        Some("kept") if status != tlc::HOLDS => Some(format!(
            "the promise is said to be kept, and TLC answered {status}"
        )),
        _ => None,
    }
}

/// What is wrong with whether the predicate of `run` is always possible.
pub fn possible_judged(run: &RunDecl, possible: bool) -> Option<String> {
    if possible == run.holds() {
        return None;
    }
    let predicate = run.possible.as_deref().unwrap_or("");
    Some(format!(
        "{predicate} is {} possible",
        if possible { "always" } else { "not always" }
    ))
}

/// Parse `k/N` with `1 <= k <= N`.
pub fn shard_spec(text: &str) -> Result<(usize, usize), String> {
    let refused = || format!("a shard is k/N with 1 <= k <= N, got {text:?}");
    let (index, count) = text.split_once('/').ok_or_else(refused)?;
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    if !digits(index) || !digits(count) {
        return Err(refused());
    }
    let index: usize = index.parse().map_err(|_| refused())?;
    let count: usize = count.parse().map_err(|_| refused())?;
    if !(1 <= index && index <= count) {
        return Err(refused());
    }
    Ok((index, count))
}

/// The runs of each of `count` shards, as (module, configuration) pairs.
///
/// Largest first, each to the shard whose load after taking it is least; of
/// equals, the one holding the fewest runs, then the lowest-numbered (Graham's
/// rule, as `scripts/lib/ctest_shards.py` spreads the lanes' units, so ADR 0120's
/// plan of this lane did not move with the judge). A holding run weighs the
/// distinct states its configuration records, in thousands, once more for each
/// second pass over the same states (a promise, a graph for what must stay
/// possible); a run that violates stops at its first violation and weighs one.
/// Measured on job 138489726305 (2026-11-24): a holding run's time followed its
/// count, Principals.cfg 248 s for 8.7 million states and its promise, and
/// BroadcastLog_r1.cfg 116 s for 15.4 million states alone.
pub fn shard_runs(manifest: &Manifest, count: usize) -> Result<Vec<Vec<(String, String)>>, String> {
    if count < 1 {
        return Err(format!("a plan of {count} shard(s)"));
    }
    let mut units: Vec<(u64, String, (String, String))> = Vec::new();
    for model in &manifest.models {
        for run in &model.runs {
            let mut weight = 1;
            if run.holds() {
                weight = (run.recorded_states().unwrap_or(0) / 1000).max(1);
                weight *= 1 + u64::from(run.promise.is_some()) + u64::from(run.possible.is_some());
            }
            let key = format!("{}/{}", model.module, run.config);
            units.push((weight, key, (model.module.clone(), run.config.clone())));
        }
    }
    units.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    let mut loads = vec![0u64; count];
    let mut held = vec![0usize; count];
    let mut shards: Vec<Vec<(String, String)>> = vec![Vec::new(); count];
    for (weight, _, unit) in units {
        let best = (0..count)
            .min_by_key(|&s| (loads[s] + weight, held[s], s))
            .expect("at least one shard");
        loads[best] += weight;
        held[best] += 1;
        shards[best].push(unit);
    }
    Ok(shards)
}

/// Where the lines of a judge's report go: one line per run and per problem on
/// `out`, and what TLC printed of a run that is not as declared on `detail`.
pub struct Report<'a> {
    pub out: &'a mut dyn FnMut(&str),
    pub detail: &'a mut dyn FnMut(&str),
}

/// How to check a manifest.
pub struct Options<'a> {
    pub only: Option<&'a str>,
    pub shard: Option<(usize, usize)>,
    pub workers: &'a str,
    pub record: bool,
}

fn tail_lines(output: &str, count: usize) -> String {
    let lines: Vec<&str> = output.lines().collect();
    lines[lines.len().saturating_sub(count)..].join("\n")
}

/// Whether `AG EF predicate` holds of `module` in `config`.
fn run_possible(
    jar: &Path,
    formal: &Path,
    module: &str,
    config_text: &str,
    predicate: &str,
    workers: &str,
) -> Result<Result<bool, String>, FormalError> {
    let carried = format!("{module}Possible");
    let extra = vec![(format!("{carried}.tla"), possible_module(module, predicate))];
    let answer = tlc::run(&tlc::Run {
        jar,
        sources: formal,
        module: &carried,
        config_name: &format!("{carried}.cfg"),
        config_text: &possible_config(config_text),
        extra: &extra,
        workers,
        write_graph: true,
    })?;
    if answer.status != tlc::HOLDS {
        let shown: String = answer
            .output
            .chars()
            .rev()
            .take(1500)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        return Ok(Err(format!(
            "TLC answered {} where it was to write the graph:\n{shown}",
            answer.status
        )));
    }
    let Some(dot) = &answer.graph else {
        return Ok(Err("TLC wrote no graph".to_string()));
    };
    let (holds, steps) = match graph_of(dot) {
        Ok(graph) => graph,
        Err(problem) => return Ok(Err(problem)),
    };
    let count = answer.distinct_states();
    if count != Some(holds.len() as u64) {
        let found = count.map_or("no".to_string(), |n| n.to_string());
        return Ok(Err(format!(
            "TLC found {found} states and wrote {}",
            holds.len()
        )));
    }
    Ok(Ok(always_possible(&holds, &steps)))
}

/// Check every model `manifest_path` declares, the models beside it, as it declares
/// them. Each run's line goes to `report` as it is judged.
pub fn check(
    manifest_path: &Path,
    options: &Options<'_>,
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
    let formal = manifest_path
        .parent()
        .filter(|dir| !dir.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let problems = declared(&manifest, formal);
    if !problems.is_empty() && !options.record {
        for problem in &problems {
            (report.out)(&format!("FAIL {problem}"));
        }
        return Err(FormalError::Declaration {
            manifest: shown,
            problems,
        });
    }
    let mine: Option<BTreeSet<(String, String)>> = match options.shard {
        Some((index, count)) => {
            let plan =
                shard_runs(&manifest, count).map_err(|problem| FormalError::Declaration {
                    manifest: shown.clone(),
                    problems: vec![problem],
                })?;
            let part: BTreeSet<(String, String)> = plan[index - 1].iter().cloned().collect();
            (report.out)(&format!("shard {index} of {count}, {} run(s)", part.len()));
            Some(part)
        }
        None => None,
    };
    let pin: TlcPin = toolchain::pinned().tlc;
    let jar = toolchain::admitted_tlc(&pin)?;
    let mut failures = Vec::new();
    let mut judged_runs = 0usize;
    let mut recorded: Vec<(String, u64)> = Vec::new();
    for model in &manifest.models {
        if options.only.is_some_and(|only| only != model.module) {
            continue;
        }
        for run in &model.runs {
            if mine
                .as_ref()
                .is_some_and(|part| !part.contains(&(model.module.clone(), run.config.clone())))
            {
                continue;
            }
            judged_runs += 1;
            let config_text = read(&formal.join(&run.config));
            let tlc_run = |config: &str| {
                tlc::run(&tlc::Run {
                    jar: &jar,
                    sources: formal,
                    module: &model.module,
                    config_name: &run.config,
                    config_text: config,
                    extra: &[],
                    workers: options.workers,
                    write_graph: false,
                })
            };
            let answer = tlc_run(&config_text)?;
            let mut output = answer.output.clone();
            if options.record && run.holds() && answer.status == tlc::HOLDS {
                if let Some(count) = answer.distinct_states() {
                    recorded.push((run.config.clone(), count));
                }
            }
            let mut wrong = if run.violates() && run.possible.is_some() {
                None
            } else {
                judged(run, answer.status, &answer.output)
            };
            if wrong.is_none() {
                if let Some(predicate) = &run.possible {
                    wrong = match run_possible(
                        &jar,
                        formal,
                        &model.module,
                        &config_text,
                        predicate,
                        options.workers,
                    )? {
                        Ok(possible) => possible_judged(run, possible),
                        Err(problem) => Some(problem),
                    };
                }
            }
            if wrong.is_none() && run.promise.is_some() {
                if let Some(promise) = &model.promise {
                    let promised = tlc_run(&promise_only(&config_text, promise))?;
                    output = promised.output.clone();
                    wrong = promise_judged(run, promised.status);
                }
            }
            match &wrong {
                None => (report.out)(&format!("ok   {}: {}", run.config, run.expect)),
                Some(why) => {
                    (report.out)(&format!("FAIL {}: {why}", run.config));
                    (report.detail)(&tail_lines(&output, 25));
                    failures.push(format!("{}: {why}", run.config));
                }
            }
        }
    }
    if options.record {
        let written =
            record_counts(&text, &recorded).map_err(|problem| FormalError::Declaration {
                manifest: shown.clone(),
                problems: vec![problem],
            })?;
        std::fs::write(manifest_path, written).map_err(|e| FormalError::ToolUnavailable {
            tool: "the manifest's file".to_string(),
            detail: format!("cannot write {shown}: {e}"),
        })?;
    }
    if !failures.is_empty() {
        return Err(FormalError::Verdict {
            manifest: shown,
            runs: judged_runs,
            failures,
        });
    }
    (report.out)("every model is as declared");
    Ok(())
}

/// `text` with the distinct states of each named holding run set to the count
/// TLC found, and nothing else of it moved: the count is replaced where the run
/// records one, and written after its `"expect": "holds"` where it records none.
/// What is written is read back and must be the manifest with those counts.
pub fn record_counts(text: &str, counts: &[(String, u64)]) -> Result<String, String> {
    let mut written = text.to_string();
    for (config, count) in counts {
        let key = format!("\"config\": \"{config}\"");
        let at = match written
            .match_indices(&key)
            .map(|(at, _)| at)
            .collect::<Vec<_>>()
            .as_slice()
        {
            [at] => *at,
            found => {
                return Err(format!(
                    "{config}: named {} times in the manifest's text",
                    found.len()
                ))
            }
        };
        let end = written[at..]
            .find('}')
            .map(|end| at + end)
            .ok_or_else(|| format!("{config}: its run does not close"))?;
        let object = &written[at..end];
        if let Some(states) = object.find("\"states\": ") {
            let digits_at = at + states + "\"states\": ".len();
            let digits = written[digits_at..]
                .chars()
                .take_while(char::is_ascii_digit)
                .count();
            written.replace_range(digits_at..digits_at + digits, &count.to_string());
        } else {
            let expect = object
                .find("\"expect\": \"holds\"")
                .ok_or_else(|| format!("{config}: a count is recorded only of a run that holds"))?;
            let expect_at = at + expect;
            let line_start = written[..expect_at].rfind('\n').map_or(0, |n| n + 1);
            let indent = written[line_start..expect_at].to_string();
            let after = expect_at + "\"expect\": \"holds\"".len();
            written.insert_str(after, &format!(",\n{indent}\"states\": {count}"));
        }
    }
    let mut expected: Value =
        serde_json::from_str(text).map_err(|e| format!("not a models manifest: {e}"))?;
    let read_back: Value = serde_json::from_str(&written)
        .map_err(|e| format!("the counts written leave no manifest: {e}"))?;
    for (config, count) in counts {
        let runs = expected["models"]
            .as_array_mut()
            .into_iter()
            .flatten()
            .flat_map(|model| model["runs"].as_array_mut().into_iter().flatten());
        for run in runs {
            if run["config"] == config.as_str() {
                run["states"] = Value::from(*count);
            }
        }
    }
    if read_back != expected {
        return Err("the counts written moved something else of the manifest".to_string());
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// The checks that judge a model are judged here, with no TLC.
    struct Fixture {
        dir: crate::formal::WorkDir,
    }

    impl Fixture {
        fn new() -> Self {
            let fixture = Self {
                dir: crate::formal::WorkDir::new("test-models").expect("a working directory"),
            };
            fixture.put("M.tla", "");
            fixture.write("M.cfg", &[("Part", "TRUE")]);
            fixture.write("M_without_part.cfg", &[("Part", "FALSE")]);
            fixture
        }

        fn formal(&self) -> &Path {
            self.dir.path()
        }

        fn put(&self, name: &str, text: &str) {
            std::fs::write(self.formal().join(name), text).expect("written");
        }

        fn append(&self, name: &str, text: &str) {
            let before = read(&self.formal().join(name));
            self.put(name, &format!("{before}{text}"));
        }

        fn write(&self, name: &str, constants: &[(&str, &str)]) {
            let body: Vec<String> = constants
                .iter()
                .map(|(k, v)| format!("    {k} = {v}"))
                .collect();
            self.put(
                name,
                &format!(
                    "SPECIFICATION Spec\nCONSTANTS\n{}\nINVARIANTS\n    Safe\n",
                    body.join("\n")
                ),
            );
        }

        fn manifest(&self) -> Value {
            json!({"v": 1, "models": [{
                "module": "M", "parts": {"Part": "TRUE"}, "no_progress": "a model of one step",
                "runs": [
                    {"config": "M.cfg", "expect": "holds", "states": 7},
                    {"config": "M_without_part.cfg", "expect": "violates", "invariant": "Safe", "without": "Part"},
                ],
            }]})
        }

        fn declared(&self, manifest: &Value) -> Vec<String> {
            declared(
                &serde_json::from_value(manifest.clone()).expect("a manifest"),
                self.formal(),
            )
        }

        fn finds(&self, manifest: &Value, needle: &str) -> bool {
            self.declared(manifest)
                .iter()
                .any(|problem| problem.contains(needle))
        }

        fn promising(&self) -> Value {
            let mut manifest = self.manifest();
            manifest["models"][0]["promise"] = json!("Refines");
            manifest["models"][0]["refines"] = json!("S");
            manifest["models"]
                .as_array_mut()
                .expect("models")
                .push(json!({"module": "S", "abstract": true, "runs": []}));
            self.put("S.tla", "");
            self.append("M.cfg", "PROPERTIES\n    Refines\n");
            manifest["models"][0]["runs"][1]["promise"] = json!("broken");
            manifest
        }

        fn progressing(&self) -> Value {
            let mut manifest = self.manifest();
            manifest["models"][0]["progress"] = json!(["Ends"]);
            manifest["models"][0]
                .as_object_mut()
                .expect("model")
                .remove("no_progress");
            self.append("M.cfg", "PROPERTIES\n    Ends\n");
            self.put(
                "M_unfair.cfg",
                "SPECIFICATION Spec\nCONSTANTS\n    Part = TRUE\nPROPERTIES\n    Ends\n",
            );
            manifest["models"][0]["runs"].as_array_mut().expect("runs").push(
                json!({"config": "M_unfair.cfg", "expect": "violates", "property": "Ends", "against": "fairness"}),
            );
            manifest
        }
    }

    fn run(value: Value) -> RunDecl {
        serde_json::from_value(value).expect("a run")
    }

    #[test]
    fn a_whole_declaration_is_clean() {
        let f = Fixture::new();
        assert_eq!(f.declared(&f.manifest()), Vec::<String>::new());
    }

    #[test]
    fn a_manifest_of_another_version_is_refused() {
        let f = Fixture::new();
        let mut manifest = f.manifest();
        manifest["v"] = json!(2);
        assert!(f.finds(&manifest, "of version 2"));
        manifest.as_object_mut().expect("manifest").remove("v");
        assert!(f.finds(&manifest, "of version none"));
    }

    #[test]
    fn a_tool_named_in_the_manifest_is_refused() {
        let f = Fixture::new();
        let mut manifest = f.manifest();
        manifest["tlc"] = json!({"version": "1.7.4"});
        assert!(f.finds(&manifest, "pinned by the judge"));
    }

    #[test]
    fn what_is_taken_on_trust_is_found() {
        let f = Fixture::new();
        for line in ["<1>1. OMITTED", "AXIOM Good == TRUE"] {
            f.put(
                "M.tla",
                &format!("(* no\nOMITTED step *)\n\\* AXIOM none\n{line}\n"),
            );
            let found: Vec<String> = f
                .declared(&f.manifest())
                .into_iter()
                .filter(|p| p.contains("taken on trust"))
                .collect();
            assert_eq!(found.len(), 1, "{line}");
            assert!(found[0].contains("M.tla:4"), "{found:?}");
        }
    }

    #[test]
    fn a_check_no_run_shows_to_fail_is_found() {
        let f = Fixture::new();
        f.append("M.cfg", "    Always\n");
        assert!(f.finds(&f.manifest(), "checks Always, and no run shows"));
    }

    #[test]
    fn a_property_no_run_shows_to_fail_is_found() {
        let f = Fixture::new();
        f.append("M.cfg", "PROPERTIES\n    Lives\n");
        assert!(f.finds(&f.manifest(), "checks Lives, and no run shows"));
    }

    #[test]
    fn a_configuration_nobody_declared_is_found() {
        let f = Fixture::new();
        f.write("M_stray.cfg", &[("Part", "TRUE")]);
        assert!(f.finds(&f.manifest(), "does not declare"));
    }

    #[test]
    fn a_module_nobody_declared_is_found() {
        let f = Fixture::new();
        f.put("N.tla", "");
        assert!(f.finds(&f.manifest(), "N.tla"));
    }

    #[test]
    fn a_declared_configuration_that_is_not_there_is_found() {
        let f = Fixture::new();
        std::fs::remove_file(f.formal().join("M.cfg")).expect("removed");
        assert!(f.finds(&f.manifest(), "no such file"));
    }

    #[test]
    fn a_part_no_run_takes_out_is_found() {
        let f = Fixture::new();
        let mut manifest = f.manifest();
        manifest["models"][0]["parts"]["Other"] = json!("TRUE");
        assert!(f.finds(&manifest, "no run takes out Other"));
    }

    #[test]
    fn a_run_that_takes_out_nothing_is_found() {
        let f = Fixture::new();
        f.write("M_without_part.cfg", &[("Part", "TRUE")]);
        assert!(f.finds(&f.manifest(), "takes out nothing"));
    }

    #[test]
    fn a_holding_run_with_a_part_out_is_found() {
        let f = Fixture::new();
        f.write("M.cfg", &[("Part", "FALSE")]);
        assert!(f.finds(&f.manifest(), "holds, with"));
    }

    #[test]
    fn a_holding_run_without_a_count_is_found() {
        let f = Fixture::new();
        let mut manifest = f.manifest();
        manifest["models"][0]["runs"][0]
            .as_object_mut()
            .expect("run")
            .remove("states");
        assert!(f.finds(&manifest, "records its distinct states"));
    }

    #[test]
    fn an_unknown_key_is_found() {
        let f = Fixture::new();
        let mut manifest = f.manifest();
        manifest["models"][0]["owner"] = json!("someone");
        manifest["models"][0]["runs"][0]["note"] = json!("x");
        assert!(f.finds(&manifest, "M: unknown keys ['owner']"));
        assert!(f.finds(&manifest, "M.cfg: unknown keys ['note']"));
    }

    #[test]
    fn a_state_space_that_moved_is_found() {
        let holding = run(json!({"config": "M.cfg", "expect": "holds", "states": 7}));
        assert_eq!(judged(&holding, 0, "7 distinct states found"), None);
        assert!(judged(&holding, 0, "6 distinct states found").is_some());
        assert!(judged(&holding, 0, "nothing").is_some());
        assert!(judged(
            &holding,
            12,
            "Invariant Safe is violated. 7 distinct states found"
        )
        .is_some());
    }

    #[test]
    fn an_abstract_module_nothing_refines_is_found() {
        let f = Fixture::new();
        let mut manifest = f.manifest();
        f.put("S.tla", "");
        manifest["models"]
            .as_array_mut()
            .expect("models")
            .push(json!({"module": "S", "abstract": true, "runs": []}));
        assert!(f.finds(&manifest, "nothing refines"));
    }

    #[test]
    fn a_promise_that_refines_no_abstract_module_is_found() {
        let f = Fixture::new();
        let mut manifest = f.promising();
        manifest["models"][0]["refines"] = json!("Nobody");
        assert!(f.finds(&manifest, "no abstract module"));
    }

    #[test]
    fn a_promise_that_is_checked_and_broken_is_clean() {
        let f = Fixture::new();
        let manifest = f.promising();
        assert_eq!(f.declared(&manifest), Vec::<String>::new());
    }

    #[test]
    fn a_holding_run_that_does_not_check_the_promise_is_found() {
        let f = Fixture::new();
        let manifest = f.promising();
        f.write("M.cfg", &[("Part", "TRUE")]);
        assert!(f.finds(&manifest, "does not check Refines"));
    }

    #[test]
    fn a_part_taken_out_that_says_nothing_of_the_promise_is_found() {
        let f = Fixture::new();
        let mut manifest = f.promising();
        manifest["models"][0]["runs"][1]
            .as_object_mut()
            .expect("run")
            .remove("promise");
        assert!(f.finds(&manifest, "broken or kept"));
    }

    #[test]
    fn a_kept_promise_without_a_reason_is_found() {
        let f = Fixture::new();
        let mut manifest = f.promising();
        manifest["models"][0]["runs"][1]["promise"] = json!("kept");
        assert!(f.finds(&manifest, "says why"));
    }

    #[test]
    fn a_progress_that_holds_and_can_fail_is_clean() {
        let f = Fixture::new();
        let manifest = f.progressing();
        assert_eq!(f.declared(&manifest), Vec::<String>::new());
    }

    #[test]
    fn a_progress_that_never_fails_is_found() {
        let f = Fixture::new();
        let mut manifest = f.progressing();
        manifest["models"][0]["runs"]
            .as_array_mut()
            .expect("runs")
            .pop();
        std::fs::remove_file(f.formal().join("M_unfair.cfg")).expect("removed");
        assert!(f.finds(&manifest, "shows Ends fail"));
    }

    #[test]
    fn a_progress_no_run_checks_is_found() {
        let f = Fixture::new();
        let manifest = f.progressing();
        f.write("M.cfg", &[("Part", "TRUE")]);
        assert!(f.finds(&manifest, "says Ends holds"));
    }

    #[test]
    fn a_temporal_property_under_a_constraint_is_found() {
        for section in ["CONSTRAINT", "ACTION_CONSTRAINT", "SYMMETRY", "VIEW"] {
            let f = Fixture::new();
            let manifest = f.progressing();
            f.append("M.cfg", &format!("{section} Small\n"));
            assert!(f.finds(&manifest, "under"), "{section}");
        }
    }

    #[test]
    fn a_violated_property_beside_another_is_found() {
        let f = Fixture::new();
        let manifest = f.progressing();
        f.append("M_unfair.cfg", "    Other\n");
        assert!(f.finds(&manifest, "and checks"));
    }

    #[test]
    fn a_model_that_states_no_progress_and_gives_no_reason_is_found() {
        let f = Fixture::new();
        let mut manifest = f.manifest();
        manifest["models"][0]
            .as_object_mut()
            .expect("model")
            .remove("no_progress");
        assert!(f.finds(&manifest, "why it has none"));
    }

    #[test]
    fn what_is_possible_is_shown_and_can_be_lost() {
        let f = Fixture::new();
        let mut manifest = f.manifest();
        manifest["models"][0]["possible"] = json!(["Reset"]);
        let found = f.declared(&manifest);
        assert!(
            found.iter().any(|p| p.contains("always possible"))
                && found.iter().any(|p| p.contains("lost"))
        );
        manifest["models"][0]["runs"][0]["possible"] = json!("Reset");
        f.write("M_lost.cfg", &[("Part", "FALSE")]);
        manifest["models"][0]["runs"].as_array_mut().expect("runs").push(
            json!({"config": "M_lost.cfg", "expect": "violates", "possible": "Reset", "without": "Part"}),
        );
        assert_eq!(f.declared(&manifest), Vec::<String>::new());
        manifest["models"][0]["runs"][0]["possible"] = json!("Other");
        assert!(f.finds(&manifest, "not declared possible"));
    }

    #[test]
    fn a_graph_is_read_and_its_possibility_decided() {
        let dot = "strict digraph {\n1 [label=\"/\\\\ x = 1\\n/\\\\ sce_holds = FALSE\",style = filled]\n\
                   1 -> 2 [label=\"\",color=\"black\"];\n-2 [label=\"sce_holds = FALSE\"];\n2 [label=\"sce_holds = TRUE\"];\n\
                   1 -> -2 [label=\"\"];\n";
        let (holds, steps) = graph_of(dot).expect("a graph");
        let expected: BTreeMap<String, bool> = [("1", false), ("-2", false), ("2", true)]
            .into_iter()
            .map(|(s, h)| (s.to_string(), h))
            .collect();
        assert_eq!(holds, expected);
        let pair = |a: &str, b: &str| (a.to_string(), b.to_string());
        assert_eq!(steps, vec![pair("1", "2"), pair("1", "-2")]);
        assert!(!always_possible(&holds, &steps));
        let mut back = steps.clone();
        back.push(pair("-2", "1"));
        assert!(always_possible(&holds, &back));
        let alone: BTreeMap<String, bool> = [("1".to_string(), false)].into_iter().collect();
        assert!(!always_possible(&alone, &[pair("1", "1")]));
        assert!(!always_possible(&BTreeMap::new(), &[]));
        assert!(graph_of("1 [label=\"x = 1\"];").is_err());
    }

    #[test]
    fn a_possibility_judged_against_what_it_says() {
        let holding = run(json!({"config": "c", "expect": "holds", "possible": "P"}));
        let violating = run(json!({"config": "c", "expect": "violates", "possible": "P"}));
        assert_eq!(possible_judged(&holding, true), None);
        assert!(possible_judged(&holding, false).is_some());
        assert_eq!(possible_judged(&violating, false), None);
        assert!(possible_judged(&violating, true).is_some());
    }

    #[test]
    fn a_possible_configuration_keeps_its_constants_alone() {
        let text = "SPECIFICATION Spec\nCHECK_DEADLOCK FALSE\nCONSTANTS\n    A = 1\nINVARIANTS\n    Safe\nPROPERTIES\n    Ends\n";
        assert_eq!(
            possible_config(text),
            "SPECIFICATION PossibleSpec\nCHECK_DEADLOCK FALSE\nCONSTANTS\n    A = 1\n"
        );
    }

    #[test]
    fn a_property_judged_by_its_own_status() {
        let violating = run(json!({"config": "c", "expect": "violates", "property": "Ends"}));
        assert_eq!(
            judged(&violating, 13, "Temporal properties were violated."),
            None
        );
        assert!(judged(&violating, 12, "Invariant Safe is violated.").is_some());
        assert!(judged(&violating, 0, "7 distinct states found").is_some());
    }

    #[test]
    fn a_promise_judged_against_what_it_says() {
        let broken = run(json!({"config": "c", "expect": "violates", "promise": "broken"}));
        let kept = run(json!({"config": "c", "expect": "violates", "promise": "kept"}));
        assert_eq!(promise_judged(&broken, 13), None);
        assert!(promise_judged(&broken, 0).is_some());
        assert_eq!(promise_judged(&kept, 0), None);
        assert!(promise_judged(&kept, 13).is_some());
    }

    #[test]
    fn a_promise_only_configuration_keeps_its_constants() {
        let text = "SPECIFICATION Spec\nCONSTANTS\n    A = 1\nINVARIANTS\n    Safe\nPROPERTIES\n    Ends\n";
        assert_eq!(
            promise_only(text, "Refines"),
            "SPECIFICATION Spec\nCONSTANTS\n    A = 1\nPROPERTIES\n    Refines\n"
        );
    }

    #[test]
    fn a_violation_of_another_invariant_is_found() {
        let violating = run(json!({"config": "c", "expect": "violates", "invariant": "Safe"}));
        assert_eq!(judged(&violating, 12, "Invariant Safe is violated."), None);
        assert!(judged(&violating, 12, "Invariant Other is violated.").is_some());
        assert!(judged(&violating, 0, "7 distinct states found").is_some());
        assert!(judged(&violating, 150, "parse error").is_some());
    }

    #[test]
    fn the_shards_are_every_run_once() {
        let manifest: Manifest = serde_json::from_value(json!({"models": [
            {"module": "A", "runs": [
                {"config": "A.cfg", "expect": "holds", "states": 9000, "promise": "kept"},
                {"config": "A_without.cfg", "expect": "violates", "invariant": "Safe"}]},
            {"module": "B", "runs": [
                {"config": "B.cfg", "expect": "holds", "states": 4000},
                {"config": "B_live.cfg", "expect": "holds", "states": 3000},
                {"config": "B_without.cfg", "expect": "violates", "invariant": "Safe"}]},
        ]}))
        .expect("a manifest");
        let mut every: Vec<(String, String)> = manifest
            .models
            .iter()
            .flat_map(|m| m.runs.iter().map(|r| (m.module.clone(), r.config.clone())))
            .collect();
        every.sort();
        for count in [1, 2, 3, 6] {
            let parts = shard_runs(&manifest, count).expect("a plan");
            assert_eq!(parts.len(), count);
            let mut all: Vec<(String, String)> = parts.iter().flatten().cloned().collect();
            all.sort();
            assert_eq!(all, every, "{count}");
            assert_eq!(
                parts,
                shard_runs(&manifest, count).expect("a plan"),
                "the same manifest, another plan"
            );
        }
        // A.cfg weighs its 9000 states twice, more than the rest together.
        let plan = shard_runs(&manifest, 2).expect("a plan");
        let heaviest = plan
            .iter()
            .find(|part| part.contains(&("A".to_string(), "A.cfg".to_string())))
            .expect("placed");
        assert_eq!(heaviest, &vec![("A".to_string(), "A.cfg".to_string())]);
        assert!(shard_runs(&manifest, 0).is_err());
    }

    #[test]
    fn a_shard_is_k_of_n() {
        assert_eq!(shard_spec("2/3"), Ok((2, 3)));
        for refused in ["0/3", "4/3", "1", "a/b", "/3", "1/"] {
            assert!(shard_spec(refused).is_err(), "{refused}");
        }
    }

    #[test]
    fn a_count_is_recorded_and_nothing_else_moves() {
        let text = "{\n  \"models\": [\n    {\n      \"module\": \"M\",\n      \"runs\": [\n        {\n          \"config\": \"M.cfg\",\n          \"expect\": \"holds\",\n          \"states\": 7\n        },\n        {\n          \"config\": \"M_new.cfg\",\n          \"expect\": \"holds\"\n        }\n      ]\n    }\n  ]\n}\n";
        let written = record_counts(
            text,
            &[("M.cfg".to_string(), 9), ("M_new.cfg".to_string(), 4)],
        )
        .expect("recorded");
        assert!(written.contains("\"states\": 9\n"), "{written}");
        assert!(
            written.contains("\"expect\": \"holds\",\n          \"states\": 4\n"),
            "{written}"
        );
        assert_eq!(
            written
                .replace("\"states\": 9", "\"states\": 7")
                .replace(",\n          \"states\": 4", ""),
            text
        );
        assert!(record_counts(text, &[("Nowhere.cfg".to_string(), 1)]).is_err());
    }

    #[test]
    fn the_status_of_the_surface_is_the_schemas() {
        let schema: Value = serde_json::from_str(include_str!(
            "../../../schemas/sce-formal-models.v1.schema.json"
        ))
        .expect("the schema is JSON");
        assert_eq!(
            schema["x-sce-schema-status"],
            Value::from(MODELS_SCHEMA_STATUS)
        );
    }
}
