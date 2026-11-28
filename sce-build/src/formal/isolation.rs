// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The judge of what changes a check (docs/adr/0115, D11; docs/adr/0146, order
//! step 5): who is asked to make a check green must not be able to do it by
//! changing the check. So what a check says is changed by a commit that changes
//! nothing else and gives its reason. What says it is read out of the commit's
//! PARENT, so that a commit cannot say that something is no check any more:
//!
//! - an abstract module beside the models manifest (a promise), whole;
//! - in any other module, the definition of each property a holding run checks,
//!   and what only those are made of;
//! - in a configuration, the properties it checks; a configuration that is taken
//!   out;
//! - in the models manifest, a model, a run, a part, a promise, a progress, what
//!   must stay possible, and what a run must answer; in the proofs manifest, a
//!   proof, a change it must refuse, and fewer functions verified;
//! - in a file that is proved, the contracts and the checked callers.
//!
//! What is ADDED is no change of a check: a new module, property, run, part, or a
//! file that had no contract. A comment is none either.
//!
//! A commit that changes one of these may carry documents beside it (`docs/**.md`)
//! and nothing more: not the rest of the same module, the counts of states a run
//! must find, a bound of a configuration, the code of the proved file, the
//! judges, or any other file. It names its reason in a bullet of its message that
//! begins `- Spec: `.
//!
//! The judges are among those other files, and are named by no path of their
//! own: the judge is the command a consumer runs over its own commits
//! (docs/adr/0146), and where a repository keeps the scripts and the workflows
//! that run it is that repository's. Nothing but documents may come beside a
//! check, so a judge changed beside one is refused as any file is.
//!
//! Judged commit by commit over the ranges it is given, and only where the
//! commit's parent declares a check (holds the models or the proofs manifest): a
//! rule judges what comes after it. A merge is judged by the commits it brings,
//! and a commit two ranges share is judged once. The manifests are named by their
//! paths in the repository, `docs/formal/models.json` and
//! `docs/formal/proofs.json` unless the caller names others.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::LazyLock;

use regex::Regex;
use serde_json::Value;

use super::models::Report;
use super::FormalError;

/// Where this repository, and a consumer that keeps its checks where this one
/// does, declares its models.
pub const DEFAULT_MODELS: &str = "docs/formal/models.json";

/// Where this repository, and a consumer that keeps its checks where this one
/// does, declares its proofs.
pub const DEFAULT_PROOFS: &str = "docs/formal/proofs.json";

/// The bullet a commit names its reason in.
static REASON: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^- Spec: \S.+").expect("the reason's pattern"));

/// A section of a TLC configuration.
static SECTION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^(CONSTANTS?|INVARIANTS?|PROPERT(?:Y|IES)|SPECIFICATION|CHECK_DEADLOCK|SYMMETRY|VIEW|(?:ACTION_)?CONSTRAINTS?)\b",
    )
    .expect("the section's pattern")
});

/// The start of a definition of a module, and its name.
static DEFINITION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^([A-Za-z]\w*)(\([^)]*\))?\s*==").expect("the definition's pattern")
});

/// The start of anything at the top level of a module.
static TOP_LEVEL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^([A-Za-z]\w*(\([^)]*\))?\s*==|VARIABLES?\b|CONSTANTS?\b|ASSUME\b|THEOREM\b|EXTENDS\b|-{4,}|={4,})",
    )
    .expect("the top level's pattern")
});

/// A block comment of a module.
static BLOCK_COMMENT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)\(\*.*?\*\)").expect("the block comment's pattern"));

/// A line comment of a Rust file.
static LINE_COMMENT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"//[^\n]*").expect("the line comment's pattern"));

/// What opens a contract or a checked caller in a proved file.
static CONTRACT_OPENS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"#\[cfg_attr\(verus_only,|#\[cfg\(verus_only\)\]").expect("the contract's pattern")
});

/// A name in a module.
static NAME: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[A-Za-z]\w*").expect("the name's pattern"));

/// Any run of space.
static SPACE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").expect("the space's pattern"));

/// The manifests a repository declares its checks in, by their paths in the
/// repository.
#[derive(Debug, Clone)]
pub struct Declared {
    pub models: String,
    pub proofs: String,
}

impl Default for Declared {
    fn default() -> Self {
        Self {
            models: DEFAULT_MODELS.to_string(),
            proofs: DEFAULT_PROOFS.to_string(),
        }
    }
}

impl Declared {
    /// The directory of the models manifest, which holds its modules and
    /// configurations, as a prefix of the paths in it.
    fn formal(&self) -> String {
        match self.models.rfind('/') {
            Some(at) => self.models[..=at].to_string(),
            None => String::new(),
        }
    }
}

/// Why a judge refused the ranges it was given, or could not read them.
#[derive(Debug)]
pub enum Failure {
    /// A commit changes what a check says otherwise than by itself and for a
    /// reason (`formal/isolation`).
    Refused(FormalError),
    /// A range could not be read, which is no verdict about it.
    Unreadable(String),
}

/// The commits of one checkout, read through git.
pub struct Repository {
    root: PathBuf,
}

impl Repository {
    /// The repository whose work tree holds `root`.
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
        }
    }

    /// git in the work tree, and not in a repository the environment names: a
    /// hook runs with `GIT_DIR` set to the repository it was called for.
    fn command(&self, args: &[&str]) -> Command {
        let mut command = Command::new("git");
        command
            .current_dir(&self.root)
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .args(args);
        command
    }

    fn git(&self, args: &[&str]) -> Result<String, String> {
        let done = self
            .command(args)
            .output()
            .map_err(|e| format!("git {}: {e}", args.join(" ")))?;
        if !done.status.success() {
            return Err(format!(
                "git {}: {}",
                args.join(" "),
                String::from_utf8_lossy(&done.stderr).trim()
            ));
        }
        Ok(String::from_utf8_lossy(&done.stdout).into_owned())
    }

    /// A file as `commit` holds it, or None.
    fn show(&self, commit: &str, path: &str) -> Option<String> {
        let done = self
            .command(&["show", &format!("{commit}:{path}")])
            .output()
            .ok()?;
        done.status
            .success()
            .then(|| String::from_utf8_lossy(&done.stdout).into_owned())
    }

    /// The commits of `range`, oldest first, without merges.
    fn commits(&self, range: &str) -> Result<Vec<String>, String> {
        Ok(self
            .git(&["rev-list", "--reverse", "--no-merges", range])?
            .split_whitespace()
            .map(str::to_string)
            .collect())
    }

    /// The paths `commit` changes.
    fn changed(&self, commit: &str) -> Result<Vec<String>, String> {
        let listed = self.git(&[
            "diff-tree",
            "--no-commit-id",
            "--name-only",
            "--no-renames",
            "-r",
            "-z",
            commit,
        ])?;
        Ok(listed
            .split('\0')
            .filter(|path| !path.is_empty())
            .map(str::to_string)
            .collect())
    }

    fn message(&self, commit: &str) -> Result<String, String> {
        self.git(&["log", "-1", "--format=%B", commit])
    }
}

/// Parse a range: `<base>..<head>`, both ends named.
pub fn range_spec(text: &str) -> Result<String, String> {
    match text.split_once("..") {
        Some((base, head)) if !base.is_empty() && !head.is_empty() && !head.starts_with('.') => {
            Ok(text.to_string())
        }
        _ => Err(format!("a range is <base>..<head>, got {text:?}")),
    }
}

/// A manifest, or an empty one: a file that does not parse says nothing here,
/// and its judge refuses it.
fn loaded(text: Option<&str>) -> Value {
    match text.map(serde_json::from_str::<Value>) {
        Some(Ok(value)) if value.is_object() => value,
        _ => Value::Object(serde_json::Map::new()),
    }
}

/// The list `value` holds under `key`, or none.
fn listed<'a>(value: &'a Value, key: &str) -> &'a [Value] {
    value
        .get(key)
        .and_then(Value::as_array)
        .map_or(&[][..], Vec::as_slice)
}

/// A value of a manifest as a line names it.
fn shown(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(text)) => text.clone(),
        Some(other) => other.to_string(),
        None => "(none)".to_string(),
    }
}

/// A number of a manifest, or 0.
fn number(value: Option<&Value>) -> f64 {
    value.and_then(Value::as_f64).unwrap_or(0.0)
}

/// `text` with every run of white space one space: where a line breaks says
/// nothing of what a module means.
fn layout_free(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A TLA+ module without its comments and the space they left. A line that
/// was indented keeps one space before it, so a definition inside a LET or a
/// conjunction is not read as one of the module's.
fn uncommented(module: &str) -> String {
    let text = BLOCK_COMMENT.replace_all(module, " ");
    text.lines()
        .map(|line| {
            let code = line.split("\\*").next().unwrap_or("");
            let body = SPACE.replace_all(code.trim(), " ");
            if !body.is_empty() && code.starts_with(char::is_whitespace) {
                format!(" {body}")
            } else {
                body.to_string()
            }
        })
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Each top-level definition of a module without comments, by its name.
fn definitions(module: &str) -> BTreeMap<String, String> {
    let mut found: BTreeMap<String, String> = BTreeMap::new();
    let mut name: Option<String> = None;
    for line in uncommented(module).lines() {
        if TOP_LEVEL.is_match(line) {
            name = DEFINITION
                .captures(line)
                .map(|started| started[1].to_string());
        }
        if let Some(name) = &name {
            let text = found.entry(name.clone()).or_default();
            text.push_str(line);
            text.push('\n');
        }
    }
    found
}

/// A module without comments and without the definitions `names`.
fn rest_of(module: &str, names: &BTreeSet<String>) -> String {
    let mut kept: Vec<&str> = Vec::new();
    let mut skipping = false;
    let text = uncommented(module);
    for line in text.lines() {
        if TOP_LEVEL.is_match(line) {
            skipping = DEFINITION
                .captures(line)
                .is_some_and(|started| names.contains(&started[1]));
        }
        if !skipping {
            kept.push(line);
        }
    }
    kept.join("\n")
}

/// The definitions that say what `checked` says: those, and what only they
/// are made of.
///
/// A definition the protocol's own steps use too is the model's: changing it
/// changes what is judged, and the runs that take a part out are what see a
/// check it made hollow. The steps are `Init`, `Next` and `Fair`, and the
/// behaviour `behaviour` names: the formula a configuration gives as its
/// SPECIFICATION (or INIT and NEXT), whose variables tuple and fairness are the
/// model's, as a variable added to the model is added to them.
fn said_by(
    module: &str,
    checked: &BTreeSet<String>,
    behaviour: &BTreeSet<String>,
) -> BTreeSet<String> {
    let found = definitions(module);
    let uses: BTreeMap<&str, BTreeSet<&str>> = found
        .iter()
        .map(|(name, text)| {
            let used = NAME
                .find_iter(text)
                .map(|m| m.as_str())
                .filter(|used| *used != name.as_str() && found.contains_key(*used))
                .collect();
            (name.as_str(), used)
        })
        .collect();
    let from_checks: Vec<&str> = checked.iter().map(String::as_str).collect();
    let by_checks = reached(&found, &uses, &from_checks);
    let steps: Vec<&str> = ["Init", "Next", "Fair"]
        .into_iter()
        .chain(behaviour.iter().map(String::as_str))
        .collect();
    let by_steps = reached(&found, &uses, &steps);
    let mut said: BTreeSet<String> = by_checks.difference(&by_steps).cloned().collect();
    said.extend(
        checked
            .iter()
            .filter(|name| found.contains_key(name.as_str()))
            .cloned(),
    );
    said
}

/// Every definition of `found` that `start` names, and every one those use,
/// through `uses`.
fn reached(
    found: &BTreeMap<String, String>,
    uses: &BTreeMap<&str, BTreeSet<&str>>,
    start: &[&str],
) -> BTreeSet<String> {
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut left: Vec<&str> = start
        .iter()
        .copied()
        .filter(|name| found.contains_key(*name))
        .collect();
    while let Some(name) = left.pop() {
        if seen.insert(name.to_string()) {
            if let Some(used) = uses.get(name) {
                left.extend(used.iter().copied());
            }
        }
    }
    seen
}

/// The properties a configuration checks, and what is left of it.
fn checks_of(config: &str) -> (BTreeSet<String>, String) {
    let mut checks: BTreeSet<String> = BTreeSet::new();
    let mut rest: Vec<&str> = Vec::new();
    let mut inside = false;
    for line in config.lines() {
        let line = line.split("\\*").next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let names: Vec<&str> = match SECTION.captures(line) {
            Some(section) => {
                inside = section[1].starts_with("INVARIANT") || section[1].starts_with("PROPERT");
                if inside {
                    line.split_whitespace().skip(1).collect()
                } else {
                    Vec::new()
                }
            }
            None if inside => line.split_whitespace().collect(),
            None => Vec::new(),
        };
        if inside {
            checks.extend(names.into_iter().map(str::to_string));
        } else {
            rest.push(line);
        }
    }
    (checks, rest.join("\n"))
}

/// The behaviour a configuration explores: the formula its SPECIFICATION names,
/// or its INIT and NEXT.
fn behaviour_of(config: &str) -> BTreeSet<String> {
    let mut named: BTreeSet<String> = BTreeSet::new();
    let mut inside = false;
    for line in config.lines() {
        let line = line.split("\\*").next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let mut words = line.split_whitespace();
        let first = words.next().unwrap_or("");
        if matches!(first, "SPECIFICATION" | "INIT" | "NEXT") {
            inside = true;
            named.extend(words.map(str::to_string));
        } else if SECTION.is_match(line) {
            inside = false;
        } else if inside {
            named.extend(line.split_whitespace().map(str::to_string));
        }
    }
    named
}

/// Just past the bracket that closes the one at byte `start` of `text`, or
/// the end of the text.
fn balanced(text: &str, start: usize) -> usize {
    let mut depth = 0i64;
    for (at, byte) in text.bytes().enumerate().skip(start) {
        match byte {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => {
                depth -= 1;
                if depth == 0 {
                    return at + 1;
                }
            }
            _ => {}
        }
    }
    text.len()
}

/// A proved file's contracts and checked callers, and its code; each without
/// comments and space.
fn contract_of(source: &str) -> (String, String) {
    let stripped = LINE_COMMENT.replace_all(source, "");
    let text: &str = &stripped;
    let mut contract: Vec<&str> = Vec::new();
    let mut code: Vec<&str> = Vec::new();
    let mut at = 0;
    while at < text.len() {
        let Some(opened) = CONTRACT_OPENS.find_at(text, at) else {
            code.push(&text[at..]);
            break;
        };
        code.push(&text[at..opened.start()]);
        let mut end = balanced(text, opened.start() + 1);
        if opened.as_str().starts_with("#[cfg(verus_only)") {
            // The item the attribute is on: to the end of its block, or of its
            // statement, or of the file when it has neither.
            let brace = text[end..].find('{').map(|found| end + found);
            let stop = text[end..].find(';').map(|found| end + found);
            end = match (brace, stop) {
                (Some(brace), Some(stop)) if brace < stop => balanced(text, brace),
                (Some(brace), None) => balanced(text, brace),
                (_, Some(stop)) => stop + 1,
                (None, None) => text.len(),
            };
        }
        contract.push(&text[opened.start()..end]);
        at = end;
    }
    let squeezed = |parts: &[&str]| SPACE.replace_all(&parts.join(" "), " ").trim().to_string();
    (squeezed(&contract), squeezed(&code))
}

/// What the models manifest says less or otherwise, and what a run must find
/// that moved.
fn models_changed(before: &Value, after: &Value, name_of: &str) -> (Vec<String>, Vec<String>) {
    let mut spec = Vec::new();
    let mut judged = Vec::new();
    let now = listed(after, "models");
    for model in listed(before, "models") {
        let name = shown(model.get("module"));
        let Some(new) = now.iter().find(|m| m.get("module") == model.get("module")) else {
            spec.push(format!("{name_of}: {name} is taken out"));
            continue;
        };
        if let Some(fields) = model.as_object() {
            for (key, value) in fields {
                if ["runs", "parts", "progress", "possible"].contains(&key.as_str()) {
                    continue;
                }
                if Some(value) != new.get(key) {
                    spec.push(format!("{name_of}: {name}: {key}"));
                }
            }
        }
        if let Some(parts) = model.get("parts").and_then(Value::as_object) {
            for (part, whole) in parts {
                if new.get("parts").and_then(|p| p.get(part)) != Some(whole) {
                    spec.push(format!("{name_of}: {name}: part {part}"));
                }
            }
        }
        for key in ["progress", "possible"] {
            let kept = listed(new, key);
            for lost in listed(model, key).iter().filter(|p| !kept.contains(p)) {
                spec.push(format!("{name_of}: {name}: {key} {}", shown(Some(lost))));
            }
        }
        let runs = listed(new, "runs");
        for run in listed(model, "runs") {
            let config = shown(run.get("config"));
            let Some(now_run) = runs.iter().find(|r| r.get("config") == run.get("config")) else {
                spec.push(format!("{name_of}: {config} is taken out"));
                continue;
            };
            let keys: BTreeSet<&String> = run
                .as_object()
                .into_iter()
                .chain(now_run.as_object())
                .flat_map(|fields| fields.keys())
                .filter(|key| key.as_str() != "states")
                .collect();
            for key in keys {
                if run.get(key) != now_run.get(key) {
                    spec.push(format!("{name_of}: {config}: {key}"));
                }
            }
            if run.get("states") != now_run.get("states") {
                judged.push(format!("{name_of}: {config}: the states it must find"));
            }
        }
    }
    (spec, judged)
}

/// What the proofs manifest says less.
fn proofs_changed(before: &Value, after: &Value, name_of: &str) -> Vec<String> {
    let mut spec = Vec::new();
    let now = listed(after, "proofs");
    for proof in listed(before, "proofs") {
        let name = shown(proof.get("file"));
        let Some(new) = now.iter().find(|p| p.get("file") == proof.get("file")) else {
            spec.push(format!("{name_of}: {name} is taken out"));
            continue;
        };
        if number(new.get("verified")) < number(proof.get("verified")) {
            spec.push(format!("{name_of}: {name}: fewer functions verified"));
        }
        let kept: Vec<Option<&Value>> = listed(new, "mutants")
            .iter()
            .map(|m| m.get("breaks"))
            .collect();
        for mutant in listed(proof, "mutants") {
            if !kept.contains(&mutant.get("breaks")) {
                spec.push(format!(
                    "{name_of}: {name}: no longer refuses: {}",
                    shown(mutant.get("breaks"))
                ));
            }
        }
    }
    spec
}

/// The last part of a path, as a line names a manifest.
fn file_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// What is wrong with one commit, each as one line; none when it isolates what
/// it changes of a check.
fn judged_commit(
    repository: &Repository,
    declared: &Declared,
    commit: &str,
) -> Result<Vec<String>, String> {
    let parent = format!("{commit}^");
    let models_text = repository.show(&parent, &declared.models);
    let proofs_text = repository.show(&parent, &declared.proofs);
    if models_text.is_none() && proofs_text.is_none() {
        return Ok(Vec::new());
    }
    let formal = declared.formal();
    let models = loaded(models_text.as_deref());
    let proofs = loaded(proofs_text.as_deref());
    let mut checked: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut behaviour: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut proof: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for model in listed(&models, "models") {
        let module = shown(model.get("module"));
        for run in listed(model, "runs") {
            let Some(config) = run.get("config").and_then(Value::as_str) else {
                continue;
            };
            if run.get("expect").and_then(Value::as_str) != Some("holds") {
                continue;
            }
            if let Some(text) = repository.show(&parent, &format!("{formal}{config}")) {
                checked
                    .entry(module.clone())
                    .or_default()
                    .extend(checks_of(&text).0);
                behaviour
                    .entry(module.clone())
                    .or_default()
                    .extend(behaviour_of(&text));
            }
        }
        // Level 3 (docs/adr/0115, D3): the theorems an inductive invariant implies
        // are the specification, as a configuration's properties are. The
        // invariant and the types its runs start from are the proof of them, not
        // the specification: Apalache checks that the invariant holds at the start,
        // is kept by every step and implies each theorem, so no edit of the proof
        // passes a theorem that is false. A proof may change beside the model.
        for stated in listed(model, "inductive") {
            checked.entry(module.clone()).or_default().extend(
                listed(stated, "implies")
                    .iter()
                    .filter_map(|name| name.as_str())
                    .map(str::to_string),
            );
            proof.entry(module.clone()).or_default().extend(
                ["invariant", "types"]
                    .into_iter()
                    .filter_map(|key| stated.get(key).and_then(Value::as_str))
                    .map(str::to_string),
            );
        }
    }
    let abstract_module = |module: &str| {
        listed(&models, "models").iter().any(|model| {
            model.get("module").and_then(Value::as_str) == Some(module)
                && model.get("abstract") == Some(&Value::Bool(true))
        })
    };
    let proved: BTreeSet<&str> = listed(&proofs, "proofs")
        .iter()
        .filter_map(|proof| proof.get("file").and_then(Value::as_str))
        .collect();
    let no_checks = BTreeSet::new();

    let mut spec: Vec<String> = Vec::new();
    let mut beside: Vec<String> = Vec::new();
    for path in repository.changed(commit)? {
        let old = repository.show(&parent, &path);
        let new = repository.show(commit, &path);
        let new_text = new.as_deref().unwrap_or("");
        let module = Path::new(&path)
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("");
        let Some(old) = old else {
            // What is new says nothing less than before. A new file of code is
            // still beside.
            if !path.starts_with("docs/") {
                beside.push(path);
            }
            continue;
        };
        if path == declared.models {
            let (less, moved) = models_changed(
                &loaded(Some(&old)),
                &loaded(new.as_deref()),
                file_name(&declared.models),
            );
            spec.extend(less);
            beside.extend(moved);
        } else if path == declared.proofs {
            spec.extend(proofs_changed(
                &loaded(Some(&old)),
                &loaded(new.as_deref()),
                file_name(&declared.proofs),
            ));
        } else if path.starts_with(&formal) && path.ends_with(".tla") {
            if abstract_module(module) {
                if layout_free(&uncommented(&old)) != layout_free(&uncommented(new_text)) {
                    spec.push(format!("{path}: a promise"));
                }
                continue;
            }
            let checks = checked.get(module).unwrap_or(&no_checks);
            let explored = behaviour.get(module).unwrap_or(&no_checks);
            let names = said_by(&old, checks, explored);
            // A definition the change adds that only the checks use is the
            // checks', as the ones they used before are.
            let proved = proof.get(module).unwrap_or(&no_checks);
            let all_said: BTreeSet<String> = names
                .iter()
                .chain(said_by(new_text, checks, explored).iter())
                .chain(said_by(&old, proved, explored).iter())
                .chain(said_by(new_text, proved, explored).iter())
                .cloned()
                .collect();
            let (before, after) = (definitions(&old), definitions(new_text));
            for name in names.iter().filter(|name| before.contains_key(*name)) {
                if before.get(name).map(|text| layout_free(text))
                    != after.get(name).map(|text| layout_free(text))
                {
                    spec.push(format!("{path}: {name}"));
                }
            }
            if layout_free(&rest_of(&old, &all_said)) != layout_free(&rest_of(new_text, &all_said))
            {
                beside.push(format!("{path}: the model itself"));
            }
        } else if path.starts_with(&formal) && path.ends_with(".cfg") {
            if new.is_none() {
                spec.push(format!("{path}: taken out"));
                continue;
            }
            let ((had, bounds), (has, now)) = (checks_of(&old), checks_of(new_text));
            for name in had.difference(&has) {
                spec.push(format!("{path}: no longer checks {name}"));
            }
            if bounds != now {
                beside.push(format!("{path}: what it explores"));
            }
        } else if proved.contains(path.as_str()) {
            let ((contract, code), (new_contract, new_code)) =
                (contract_of(&old), contract_of(new_text));
            if contract != new_contract {
                spec.push(format!("{path}: its contracts"));
            }
            if code != new_code {
                beside.push(format!("{path}: its code"));
            }
        } else if !(path.starts_with("docs/") && path.ends_with(".md")) {
            beside.push(path);
        }
    }

    if spec.is_empty() {
        return Ok(Vec::new());
    }
    let mut problems: Vec<String> = Vec::new();
    let said = |problems: &mut Vec<String>| {
        problems.push("changes what is checked:".to_string());
        problems.extend(spec.iter().map(|line| format!("  {line}")));
    };
    if !beside.is_empty() {
        said(&mut problems);
        problems.push("together with:".to_string());
        problems.extend(beside.iter().take(12).map(|line| format!("  {line}")));
        if beside.len() > 12 {
            problems.push(format!("  and {} more", beside.len() - 12));
        }
    }
    if !REASON.is_match(&repository.message(commit)?) {
        if beside.is_empty() {
            said(&mut problems);
        }
        problems.push("and gives no reason in a bullet `- Spec: <why>`".to_string());
    }
    Ok(problems)
}

/// One commit the judge refused: its line, and what is wrong with it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    /// The commit's short name and the subject of its message.
    pub commit: String,
    /// What it changes of a check, and what beside it, one line each.
    pub problems: Vec<String>,
}

/// Every commit of `ranges` the judge refuses, oldest first in each range, a
/// commit two ranges share judged once.
pub fn judged_ranges(
    repository: &Repository,
    declared: &Declared,
    ranges: &[String],
) -> Result<Vec<Refusal>, String> {
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut refused = Vec::new();
    for range in ranges {
        for commit in repository.commits(range)? {
            if !seen.insert(commit.clone()) {
                continue;
            }
            let problems = judged_commit(repository, declared, &commit)?;
            if problems.is_empty() {
                continue;
            }
            let message = repository.message(&commit)?;
            let subject = message.lines().next().unwrap_or("");
            let short = commit.get(..11).unwrap_or(commit.as_str());
            refused.push(Refusal {
                commit: format!("{short} {subject}"),
                problems,
            });
        }
    }
    Ok(refused)
}

/// Judge every commit of `ranges` in the work tree that holds `root`. Each refused
/// commit's lines go to `report`, and the judge leaves with
/// `formal/isolation` when there is one.
pub fn check(
    root: &Path,
    declared: &Declared,
    ranges: &[String],
    report: &mut Report<'_>,
) -> Result<(), Failure> {
    let repository = Repository::new(root);
    let refused = judged_ranges(&repository, declared, ranges).map_err(Failure::Unreadable)?;
    let range = ranges.join(" ");
    if refused.is_empty() {
        (report.out)(&format!(
            "every commit of {range} changes what a check says by itself and for a reason"
        ));
        return Ok(());
    }
    for refusal in &refused {
        (report.out)(&format!("FAIL {}", refusal.commit));
        for line in &refusal.problems {
            (report.out)(&format!("  {line}"));
        }
    }
    (report.detail)(
        "Change the check in a commit of its own, with nothing beside it but documents, \
         and name the reason in a bullet of its message: - Spec: <why>",
    );
    Err(Failure::Refused(FormalError::Isolation {
        range,
        commits: refused.into_iter().map(|refusal| refusal.commit).collect(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formal::WorkDir;

    const PROMISE: &str = "---- MODULE S ----\nVARIABLES x\nSpec == x = 0\n====\n";
    const MODEL: &str = "---- MODULE M ----\nVARIABLES x\nInit == x = 0\n\\* what is promised\nSafe == x < 3\nNext == x' = x\n====\n";
    const CONFIG: &str = "SPECIFICATION Spec\nCONSTANTS\n    N = 2\nINVARIANTS\n    Safe\n";
    // Split where the proofs judge looks for a contract (proofs::CONTRACT_MARK), so
    // this source is no file the proofs manifest must declare.
    const PROVED: &str = concat!(
        "#[cfg(verus_only)]\nuse vstd::prelude::*;\n",
        "#[cfg_attr(verus",
        "_only, verus_spec(r => ensures r <= a))]\npub fn low(a: u64) -> u64 { a }\n",
        "#[cfg(verus_only)]\nverus! { proof fn l() ensures true {} }\n",
    );
    const MODELS: &str = r#"{"models": [{"module": "M", "parts": {"P": "TRUE"}, "progress": ["Ends"], "runs": [{"config": "M.cfg", "expect": "holds", "states": 7}]}, {"module": "S", "abstract": true, "runs": []}]}"#;
    const PROOFS: &str = r#"{"proofs": [{"file": "rules.rs", "verified": 2, "mutants": [{"breaks": "b", "from": "a", "to": "c"}]}]}"#;
    const JUDGE_FILE: &str = "sce-build/src/formal/isolation.rs";
    const REASONED: &str = "docs: X\n\n- Spec: it said too much";
    const PLAIN: &str = "feat: X\n\n- y";

    /// A repository made here, its commits each judged.
    struct Fixture {
        dir: WorkDir,
        repository: Repository,
        declared: Declared,
    }

    impl Fixture {
        fn new() -> Self {
            let dir = WorkDir::new("isolation-test").expect("a directory for the repository");
            let repository = Repository::new(dir.path());
            let fixture = Self {
                dir,
                repository,
                declared: Declared::default(),
            };
            let hooks = fixture.root().join("no-hooks");
            let hooks = hooks.to_str().expect("the directory is named in UTF-8");
            let setup: [&[&str]; 5] = [
                &["init", "-q"],
                &["config", "user.name", "t"],
                &["config", "user.email", "t@example.com"],
                &["config", "commit.gpgsign", "false"],
                &["config", "core.hooksPath", hooks],
            ];
            for args in setup {
                fixture
                    .repository
                    .git(args)
                    .expect("git sets the repository up");
            }
            fixture.put(JUDGE_FILE, "");
            fixture.put("docs/formal/S.tla", PROMISE);
            fixture.put("docs/formal/M.tla", MODEL);
            fixture.put("docs/formal/M.cfg", CONFIG);
            fixture.put(DEFAULT_MODELS, MODELS);
            fixture.put("rules.rs", PROVED);
            fixture.put(DEFAULT_PROOFS, PROOFS);
            fixture.commit("feat: Begin");
            fixture
        }

        fn root(&self) -> &Path {
            self.dir.path()
        }

        fn put(&self, path: &str, text: &str) {
            let target = self.root().join(path);
            std::fs::create_dir_all(target.parent().expect("a file has a directory"))
                .expect("the directory is made");
            std::fs::write(target, text).expect("the file is written");
        }

        fn remove(&self, path: &str) {
            std::fs::remove_file(self.root().join(path)).expect("the file is removed");
        }

        fn swap(&self, path: &str, old: &str, new: &str) {
            let text = std::fs::read_to_string(self.root().join(path)).expect("the file is read");
            assert!(text.contains(old), "{path} holds no {old:?}");
            self.put(path, &text.replace(old, new));
        }

        fn commit(&self, message: &str) -> String {
            self.repository.git(&["add", "-A"]).expect("git adds");
            self.repository
                .git(&["commit", "-q", "--no-verify", "-m", message])
                .expect("git commits");
            self.repository
                .git(&["rev-parse", "HEAD"])
                .expect("git names HEAD")
                .trim()
                .to_string()
        }

        fn judged(&self, message: &str) -> String {
            let commit = self.commit(message);
            judged_commit(&self.repository, &self.declared, &commit)
                .expect("the commit is read")
                .join("\n")
        }
    }

    #[test]
    fn a_model_changed_by_itself_is_clean() {
        let f = Fixture::new();
        f.swap("docs/formal/M.tla", "x' = x", "x' = x + 1");
        f.put("code.rs", "fn f() {}");
        assert_eq!(f.judged(PLAIN), "");
    }

    #[test]
    fn a_property_changed_by_itself_for_a_reason_is_clean() {
        let f = Fixture::new();
        f.swap("docs/formal/M.tla", "x < 3", "x < 9");
        f.put("docs/adr/1.md", "why");
        assert_eq!(f.judged(REASONED), "");
    }

    #[test]
    fn a_property_changed_without_a_reason_is_refused() {
        let f = Fixture::new();
        f.swap("docs/formal/M.tla", "x < 3", "x < 9");
        assert!(f.judged(PLAIN).contains("gives no reason"));
    }

    #[test]
    fn a_property_changed_with_its_model_is_refused() {
        let f = Fixture::new();
        f.swap("docs/formal/M.tla", "x < 3", "x < 9");
        f.swap("docs/formal/M.tla", "x' = x", "x' = x + 1");
        let found = f.judged(REASONED);
        assert!(found.contains("M.tla: Safe"), "{found}");
        assert!(found.contains("the model itself"), "{found}");
    }

    #[test]
    fn a_property_changed_with_code_is_refused() {
        let f = Fixture::new();
        f.swap("docs/formal/M.tla", "x < 3", "x < 9");
        f.put("code.rs", "fn f() {}");
        assert!(f.judged(REASONED).contains("code.rs"));
    }

    #[test]
    fn a_property_changed_with_a_judge_is_refused() {
        let f = Fixture::new();
        f.swap("docs/formal/M.tla", "x < 3", "x < 9");
        f.put("sce-build/src/formal/models.rs", "pass");
        f.commit("chore: judge");
        f.swap("docs/formal/M.tla", "x < 9", "x < 11");
        f.swap("sce-build/src/formal/models.rs", "pass", "pass  // all");
        assert!(f
            .judged(REASONED)
            .contains("sce-build/src/formal/models.rs"));
    }

    #[test]
    fn a_property_taken_out_is_seen() {
        let f = Fixture::new();
        f.swap("docs/formal/M.tla", "Safe == x < 3\n", "");
        assert!(f.judged(PLAIN).contains("M.tla: Safe"));
    }

    #[test]
    fn what_only_a_property_is_made_of_is_a_check() {
        let f = Fixture::new();
        f.swap(
            "docs/formal/M.tla",
            "Safe == x < 3\n",
            "Bound == 3\nSafe == x < Bound\n",
        );
        f.commit("docs: X\n\n- Spec: name the bound");
        f.swap("docs/formal/M.tla", "Bound == 3", "Bound == 9");
        assert!(f.judged(PLAIN).contains("M.tla: Bound"));
    }

    #[test]
    fn what_the_steps_use_too_is_the_model() {
        let f = Fixture::new();
        f.swap(
            "docs/formal/M.tla",
            "Safe == x < 3\n",
            "Bound == 3\nSafe == x < Bound\n",
        );
        f.swap("docs/formal/M.tla", "x' = x", "x' = Bound");
        f.commit(PLAIN);
        f.swap("docs/formal/M.tla", "Bound == 3", "Bound == 9");
        f.put("code.rs", "fn f() {}");
        assert_eq!(f.judged(PLAIN), "");
    }

    #[test]
    fn a_comment_of_a_property_is_no_change() {
        let f = Fixture::new();
        f.swap(
            "docs/formal/M.tla",
            "what is promised",
            "what a user relies on",
        );
        f.put("code.rs", "fn f() {}");
        assert_eq!(f.judged(PLAIN), "");
    }

    #[test]
    fn a_promise_changed_with_anything_is_refused() {
        let f = Fixture::new();
        f.swap("docs/formal/S.tla", "x = 0", "TRUE");
        f.put("code.rs", "fn f() {}");
        assert!(f.judged(REASONED).contains("S.tla: a promise"));
    }

    #[test]
    fn a_new_promise_beside_its_model_is_clean() {
        let f = Fixture::new();
        f.put("docs/formal/T.tla", PROMISE);
        f.put("docs/formal/T.cfg", CONFIG);
        f.put("code.rs", "fn f() {}");
        assert_eq!(f.judged(PLAIN), "");
    }

    #[test]
    fn a_check_taken_out_of_a_configuration_is_seen() {
        let f = Fixture::new();
        f.swap("docs/formal/M.cfg", "    Safe\n", "");
        assert!(f.judged(PLAIN).contains("no longer checks Safe"));
    }

    #[test]
    fn a_check_added_is_no_change() {
        let f = Fixture::new();
        f.swap("docs/formal/M.cfg", "    Safe\n", "    Safe\n    Other\n");
        f.put("code.rs", "fn f() {}");
        assert_eq!(f.judged(PLAIN), "");
    }

    #[test]
    fn a_configuration_narrowed_with_a_property_is_refused() {
        let f = Fixture::new();
        f.swap("docs/formal/M.tla", "x < 3", "x < 9");
        f.swap("docs/formal/M.cfg", "N = 2", "N = 1");
        assert!(f.judged(REASONED).contains("what it explores"));
    }

    #[test]
    fn a_configuration_taken_out_is_seen() {
        let f = Fixture::new();
        f.remove("docs/formal/M.cfg");
        assert!(f.judged(PLAIN).contains("M.cfg: taken out"));
    }

    #[test]
    fn what_a_run_must_answer_is_a_check() {
        let f = Fixture::new();
        f.swap(DEFAULT_MODELS, r#""holds""#, r#""violates""#);
        assert!(f.judged(PLAIN).contains("M.cfg: expect"));
    }

    #[test]
    fn a_count_moved_alone_is_no_change_of_a_check() {
        let f = Fixture::new();
        f.swap(DEFAULT_MODELS, "7", "9");
        f.swap("docs/formal/M.tla", "x' = x", "x' = x + 1");
        assert_eq!(f.judged(PLAIN), "");
    }

    #[test]
    fn a_count_moved_with_a_property_is_refused() {
        let f = Fixture::new();
        f.swap(DEFAULT_MODELS, "7", "9");
        f.swap("docs/formal/M.tla", "x < 3", "x < 9");
        assert!(f.judged(REASONED).contains("the states it must find"));
    }

    #[test]
    fn a_part_and_a_progress_taken_out_are_seen() {
        let f = Fixture::new();
        f.swap(DEFAULT_MODELS, r#"{"P": "TRUE"}"#, "{}");
        f.swap(DEFAULT_MODELS, r#"["Ends"]"#, "[]");
        let found = f.judged(PLAIN);
        assert!(found.contains("part P"), "{found}");
        assert!(found.contains("progress Ends"), "{found}");
    }

    #[test]
    fn a_model_taken_out_of_the_manifest_is_seen() {
        let f = Fixture::new();
        f.swap(
            DEFAULT_MODELS,
            r#", {"module": "S", "abstract": true, "runs": []}"#,
            "",
        );
        assert!(f.judged(PLAIN).contains("models.json: S is taken out"));
    }

    #[test]
    fn a_contract_changed_with_its_code_is_refused() {
        let f = Fixture::new();
        f.swap("rules.rs", "r <= a", "true");
        f.swap("rules.rs", "{ a }", "{ a + 1 }");
        let found = f.judged(REASONED);
        assert!(found.contains("its contracts"), "{found}");
        assert!(found.contains("its code"), "{found}");
    }

    #[test]
    fn a_contract_changed_by_itself_for_a_reason_is_clean() {
        let f = Fixture::new();
        f.swap("rules.rs", "r <= a", "r < a");
        assert_eq!(f.judged(REASONED), "");
    }

    #[test]
    fn a_checked_caller_is_a_contract() {
        let f = Fixture::new();
        f.swap("rules.rs", "proof fn l() ensures true {}", "");
        assert!(f.judged(PLAIN).contains("its contracts"));
    }

    #[test]
    fn code_of_a_proved_file_changes_by_itself() {
        let f = Fixture::new();
        f.swap("rules.rs", "{ a }", "{ a + 0 }");
        f.put("code.rs", "fn f() {}");
        assert_eq!(f.judged(PLAIN), "");
    }

    #[test]
    fn a_mutant_taken_out_is_seen() {
        let f = Fixture::new();
        f.swap(
            DEFAULT_PROOFS,
            r#"[{"breaks": "b", "from": "a", "to": "c"}]"#,
            "[]",
        );
        assert!(f.judged(PLAIN).contains("no longer refuses: b"));
    }

    #[test]
    fn a_mutant_that_follows_the_code_is_no_change() {
        let f = Fixture::new();
        f.swap(DEFAULT_PROOFS, r#""from": "a""#, r#""from": "a + 0""#);
        f.swap("rules.rs", "{ a }", "{ a + 0 }");
        assert_eq!(f.judged(PLAIN), "");
    }

    #[test]
    fn fewer_functions_verified_is_seen() {
        let f = Fixture::new();
        f.swap(DEFAULT_PROOFS, r#""verified": 2"#, r#""verified": 1"#);
        assert!(f.judged(PLAIN).contains("fewer functions"));
    }

    #[test]
    fn a_repository_that_declares_no_check_is_not_judged() {
        let f = Fixture::new();
        f.remove(DEFAULT_MODELS);
        f.remove(DEFAULT_PROOFS);
        f.commit("chore: without\n\n- Spec: no checks are declared here any more");
        f.swap("docs/formal/M.tla", "x < 3", "x < 9");
        f.swap("docs/formal/M.tla", "x' = x", "x' = x + 1");
        assert_eq!(f.judged(PLAIN), "");
    }

    #[test]
    fn taking_the_manifests_out_with_code_is_refused() {
        let f = Fixture::new();
        f.remove(DEFAULT_MODELS);
        f.put("code.rs", "fn f() {}");
        let found = f.judged(REASONED);
        assert!(found.contains("models.json: M is taken out"), "{found}");
        assert!(found.contains("code.rs"), "{found}");
    }

    #[test]
    fn taking_the_judge_out_with_a_check_is_refused() {
        let f = Fixture::new();
        f.remove(JUDGE_FILE);
        f.swap("docs/formal/M.tla", "x < 3", "x < 9");
        assert!(f.judged(REASONED).contains(JUDGE_FILE));
    }

    #[test]
    fn manifests_kept_elsewhere_are_read_where_they_are_named() {
        let f = Fixture::new();
        let declared = Declared {
            models: "formal/models.json".to_string(),
            proofs: "formal/proofs.json".to_string(),
        };
        f.put("formal/M.tla", MODEL);
        f.put("formal/M.cfg", CONFIG);
        f.put("formal/models.json", MODELS);
        f.put("formal/proofs.json", PROOFS);
        f.commit("feat: Elsewhere");
        f.swap("formal/M.tla", "x < 3", "x < 9");
        let commit = f.commit(PLAIN);
        let found = judged_commit(&f.repository, &declared, &commit)
            .expect("the commit is read")
            .join("\n");
        assert!(found.contains("formal/M.tla: Safe"), "{found}");
    }

    #[test]
    fn a_range_names_each_commit_and_a_shared_commit_once() {
        let f = Fixture::new();
        let base = f.commit_head();
        f.swap("docs/formal/M.tla", "x < 3", "x < 9");
        f.commit("feat: First");
        f.swap("docs/formal/M.tla", "x < 9", "x < 11");
        f.commit("feat: Second\n\n- Spec: a reason");
        let ranges = [format!("{base}..HEAD"), format!("{base}..HEAD")];
        let refused =
            judged_ranges(&f.repository, &f.declared, &ranges).expect("the ranges are read");
        assert_eq!(refused.len(), 1, "{refused:?}");
        assert!(refused[0].commit.ends_with(" feat: First"), "{refused:?}");
    }

    #[test]
    fn a_range_git_cannot_read_is_no_verdict() {
        let f = Fixture::new();
        let ranges = ["0000000000000000000000000000000000000001..HEAD".to_string()];
        assert!(judged_ranges(&f.repository, &f.declared, &ranges).is_err());
    }

    #[test]
    fn a_range_names_both_ends() {
        assert_eq!(range_spec("a..b"), Ok("a..b".to_string()));
        for refused in ["a", "..b", "a..", "a...b"] {
            assert!(range_spec(refused).is_err(), "{refused}");
        }
    }

    impl Fixture {
        fn commit_head(&self) -> String {
            self.repository
                .git(&["rev-parse", "HEAD"])
                .expect("git names HEAD")
                .trim()
                .to_string()
        }
    }

    #[test]
    fn the_variables_of_the_behaviour_a_configuration_explores_are_the_models() {
        let module = "---- MODULE M ----\nVARIABLES x, y\nvars == <<x, y>>\n\
            Init == x = 0 /\\ y = 0\nNext == x' = x /\\ y' = y\n\
            Spec == Init /\\ [][Next]_vars\nSafe == x < 3\nSteady == [][x' = x]_vars\n====\n";
        let checks: BTreeSet<String> = ["Safe", "Steady"].map(String::from).into();
        let unexplored = said_by(module, &checks, &BTreeSet::new());
        assert!(unexplored.contains("vars"), "{unexplored:?}");
        let explored = behaviour_of(CONFIG);
        assert_eq!(explored, ["Spec".to_string()].into());
        let said = said_by(module, &checks, &explored);
        assert!(!said.contains("vars"), "{said:?}");
        assert!(said.contains("Safe") && said.contains("Steady"), "{said:?}");
    }

    #[test]
    fn a_declaration_laid_out_again_is_no_change() {
        let old = "---- MODULE S ----\nCONSTANTS A, B\nVARIABLES x, \\* the count\n    y\nSpec == x = 0\n====\n";
        let new = "---- MODULE S ----\nCONSTANTS\n    \\* @type: Int;\n    A,\n    B\nVARIABLES\n    x,\n    y\nSpec ==\n    x = 0\n====\n";
        assert_eq!(
            layout_free(&uncommented(old)),
            layout_free(&uncommented(new))
        );
        let changed = new.replace("x = 0", "x = 1");
        assert_ne!(
            layout_free(&uncommented(old)),
            layout_free(&uncommented(&changed))
        );
    }

    #[test]
    fn a_name_defined_in_a_let_is_no_definition_of_the_module() {
        let module =
            "---- MODULE M ----\nPair(e) == LET x == e\n    y == <<x, x>>\n    IN y\n====\n";
        let found = definitions(module);
        assert!(found.contains_key("Pair"), "{found:?}");
        assert!(!found.contains_key("y"), "{found:?}");
    }
}
