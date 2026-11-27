// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The one runner of Storm (docs/adr/0119, D3): a JANI model built into its
//! explicit state space, which Storm writes as JSON with the valuation of every
//! state and the actions of every choice, and that space read back.
//!
//! The space is the model's as Storm builds it, with one thing Storm adds: a
//! state with no choice is given a loop to itself and the label `deadlock`,
//! since every state of a Markov decision process has a choice. That loop is
//! no step of the model, and [`Space::steps`] leaves it out.

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

use serde_json::{Map, Value as Json};

use super::{FormalError, WorkDir};

/// The label Storm gives a state it gave a loop because it had no choice.
pub const DEADLOCK: &str = "deadlock";

/// The label Storm gives the initial states.
pub const INITIAL: &str = "init";

/// The status a program's loader answers when a library it needs is missing:
/// Storm could not run, which says nothing of the model.
const LOADER_FAILED: i32 = 127;

/// One choice of a state: the actions that label it and the states it may
/// lead to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Choice {
    pub actions: BTreeSet<String>,
    pub successors: Vec<u64>,
}

/// One state of the space: Storm's number for it, the valuation of the
/// model's variables, its labels and its choices.
#[derive(Debug, Clone, PartialEq)]
pub struct SpaceState {
    pub id: u64,
    pub valuation: Map<String, Json>,
    pub labels: BTreeSet<String>,
    pub choices: Vec<Choice>,
}

/// The explicit state space of a model.
#[derive(Debug, Clone, PartialEq)]
pub struct Space {
    pub states: Vec<SpaceState>,
}

impl Space {
    /// Every step of the model: each choice's action and each state it leads
    /// to, as (from, action, to), without the loop Storm gives a deadlock.
    pub fn steps(&self) -> Vec<(u64, String, u64)> {
        let mut found = Vec::new();
        for state in &self.states {
            if state.labels.contains(DEADLOCK) {
                continue;
            }
            for choice in &state.choices {
                for &to in &choice.successors {
                    for action in &choice.actions {
                        found.push((state.id, action.clone(), to));
                    }
                }
            }
        }
        found
    }
}

/// The space Storm's JSON export spells: an array of states, each with its
/// `id`, its valuation `s`, its labels `lab` and its choices `c`, each choice
/// with its labels `lab` and its successors `succ` by their `id`.
pub fn read_space(text: &str) -> Result<Space, String> {
    let json: Json =
        serde_json::from_str(text).map_err(|e| format!("Storm's export is not JSON: {e}"))?;
    let entries = json
        .as_array()
        .ok_or("Storm's export is not an array of states")?;
    let strings = |value: Option<&Json>| -> Result<BTreeSet<String>, String> {
        match value {
            None => Ok(BTreeSet::new()),
            Some(Json::Array(items)) => items
                .iter()
                .map(|item| {
                    item.as_str()
                        .map(str::to_string)
                        .ok_or_else(|| format!("a label that is no string: {item}"))
                })
                .collect(),
            Some(other) => Err(format!("labels that are no array: {other}")),
        }
    };
    let mut states = Vec::with_capacity(entries.len());
    for entry in entries {
        let id = entry["id"]
            .as_u64()
            .ok_or_else(|| format!("a state with no number: {entry}"))?;
        let valuation = entry["s"]
            .as_object()
            .cloned()
            .ok_or_else(|| format!("state {id} has no valuation: Storm was not asked for one"))?;
        let labels = strings(entry.get("lab"))?;
        let mut choices = Vec::new();
        for choice in entry["c"].as_array().map(Vec::as_slice).unwrap_or_default() {
            let actions = strings(choice.get("lab"))?;
            let successors = choice["succ"]
                .as_array()
                .ok_or_else(|| format!("a choice of state {id} with no successors"))?
                .iter()
                .map(|successor| {
                    successor["id"]
                        .as_u64()
                        .ok_or_else(|| format!("a successor of state {id} with no number"))
                })
                .collect::<Result<Vec<u64>, String>>()?;
            choices.push(Choice {
                actions,
                successors,
            });
        }
        states.push(SpaceState {
            id,
            valuation,
            labels,
            choices,
        });
    }
    Ok(Space { states })
}

/// What one build answered: the space, or why Storm built none of the model.
pub type Built = Result<Space, String>;

/// The arguments Storm is run with: the model, its space exported as JSON with
/// the valuation of each state and the actions of each choice.
fn arguments(model: &Path, export: &Path) -> Vec<std::ffi::OsString> {
    vec![
        "--jani".into(),
        model.as_os_str().to_owned(),
        "--exportbuild".into(),
        export.as_os_str().to_owned(),
        "--buildstateval".into(),
        "--buildchoicelab".into(),
    ]
}

/// Build the model `text` by the pinned Storm at `storm`, in a directory of
/// its own. A model Storm refuses is `Ok(Err)`, a verdict about the model; a
/// Storm that cannot run is `formal/tool-unavailable`.
pub fn build(storm: &Path, text: &str) -> Result<Built, FormalError> {
    let work = WorkDir::new("storm")?;
    let model = work.path().join("model.jani");
    let export = work.path().join("space.json");
    std::fs::write(&model, text).map_err(|e| FormalError::ToolUnavailable {
        tool: "a working directory".to_string(),
        detail: format!("cannot write {}: {e}", model.display()),
    })?;
    let ran = Command::new(storm)
        .args(arguments(&model, &export))
        .current_dir(work.path())
        .output()
        .map_err(|e| FormalError::ToolUnavailable {
            tool: "Storm".to_string(),
            detail: format!("cannot run {}: {e}", storm.display()),
        })?;
    let output = format!(
        "{}{}",
        String::from_utf8_lossy(&ran.stdout),
        String::from_utf8_lossy(&ran.stderr)
    );
    match ran.status.code() {
        Some(0) => {}
        Some(LOADER_FAILED) => {
            return Err(FormalError::ToolUnavailable {
                tool: "Storm".to_string(),
                detail: format!(
                    "{} could not load: {} (the system libraries its package asks for are \
                     the host image's, tools/ci-images/host/Containerfile)",
                    storm.display(),
                    output.trim()
                ),
            })
        }
        status => {
            let tail: Vec<&str> = output.lines().rev().take(20).collect();
            let shown: Vec<&str> = tail.into_iter().rev().collect();
            return Ok(Err(format!(
                "Storm answered {} where it was to build the model:\n{}",
                status.map_or("a signal".to_string(), |code| code.to_string()),
                shown.join("\n")
            )));
        }
    }
    let text = std::fs::read_to_string(&export)
        .map_err(|e| format!("Storm wrote no export {}: {e}", export.display()));
    Ok(text.and_then(|text| read_space(&text)))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Turns red if the export is read otherwise than Storm writes it, or the
    /// loop Storm gives a deadlock is taken as a step of the model.
    #[test]
    fn the_space_is_read_and_the_deadlock_loop_left_out() {
        let text = r#"[
            {"id": 0, "s": {"control": 0, "data_x": 0}, "lab": ["init"],
             "c": [{"id": 0, "lab": ["start"], "succ": [{"id": 1, "prob": "1"}]}]},
            {"id": 1, "s": {"control": 1, "data_x": 1}, "lab": ["deadlock"],
             "c": [{"id": 1, "succ": [{"id": 1, "prob": "1"}]}]}
        ]"#;
        let space = read_space(text).expect("a space");
        assert_eq!(space.states.len(), 2);
        assert!(space.states[0].labels.contains(INITIAL));
        assert_eq!(space.states[1].valuation["data_x"], serde_json::json!(1));
        assert_eq!(space.steps(), vec![(0, "start".to_string(), 1)]);
        assert!(read_space(r#"[{"id": 0, "lab": []}]"#).is_err());
        assert!(read_space("{}").is_err());
    }

    /// Turns red if Storm is asked for less than the judge reads: the
    /// valuations and the actions of the choices, exported as JSON.
    #[test]
    fn storm_is_asked_for_the_valuations_and_the_actions() {
        let args: Vec<String> = arguments(Path::new("m.jani"), Path::new("s.json"))
            .iter()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            args,
            [
                "--jani",
                "m.jani",
                "--exportbuild",
                "s.json",
                "--buildstateval",
                "--buildchoicelab"
            ]
        );
    }
}
