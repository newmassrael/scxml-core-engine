// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A lane's replay of a scenario, judged (docs/adr/0115, the converse of D8):
//! the run the lane made when its harness gave the machine the scenario's
//! inputs is the run the model made, step for step.
//!
//! A scenario is the run TLC gave of a violation or a witness
//! ([`super::behaviour`]), written as `<name>.scenario.json`; a lane's run of
//! it is written by that lane's harness, with its trace port on, as
//! `<name>.<lane>.json` beside it. Two runs are the same when they take the
//! same steps: of the same kinds, on the same events, through the same rows,
//! exiting and entering the same states in the same order, and ending each
//! step in the same configuration, which is a set. A lane whose run is another
//! does not conform on that behaviour, which is a defect of the model, the
//! generator or the lane, settled as D8 settles one, and not a verdict about
//! the document. A scenario that ends in a cycle is replayed once.
//!
//! The replays are sound and no more (Tretmans 1996): a lane that passes them
//! conforms on the behaviours they are and on no other.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::run_trace::{Microstep, RunTrace, Step};

/// What a scenario's file is named by after its name.
pub const SCENARIO_SUFFIX: &str = ".scenario.json";

/// Why `run`, a lane's run of `scenario`'s inputs, is not the run the model
/// made: the first step where they part, or the count of steps where one ends
/// first. `None` when it is the same run.
pub fn parting(scenario: &RunTrace, run: &RunTrace) -> Option<String> {
    for (at, (model, lane)) in scenario.steps.iter().zip(&run.steps).enumerate() {
        if !same_step(model, lane) {
            return Some(format!(
                "step {at}: the model took {model:?}, and the lane {} took {lane:?}",
                run.lane
            ));
        }
    }
    (scenario.steps.len() != run.steps.len()).then(|| {
        format!(
            "the model took {} steps, and the lane {} took {}",
            scenario.steps.len(),
            run.lane,
            run.steps.len()
        )
    })
}

/// A configuration, as the set it is.
fn set(states: &[String]) -> BTreeSet<&str> {
    states.iter().map(String::as_str).collect()
}

/// Whether two steps are the same, the configurations as sets.
fn same_step(model: &Step, lane: &Step) -> bool {
    let same_microstep = |a: &Microstep, b: &Microstep| {
        a.fired == b.fired
            && a.exited == b.exited
            && a.entered == b.entered
            && set(&a.configuration) == set(&b.configuration)
    };
    match (model, lane) {
        (
            Step::Start {
                entered: a,
                configuration: ca,
            },
            Step::Start {
                entered: b,
                configuration: cb,
            },
        ) => a == b && set(ca) == set(cb),
        (Step::Eventless(a), Step::Eventless(b)) => same_microstep(a, b),
        (
            Step::Internal {
                event: ea,
                microstep: a,
            },
            Step::Internal {
                event: eb,
                microstep: b,
            },
        )
        | (
            Step::External {
                event: ea,
                microstep: a,
            },
            Step::External {
                event: eb,
                microstep: b,
            },
        ) => ea == eb && same_microstep(a, b),
        (
            Step::Stop {
                exited: a,
                outcome: oa,
            },
            Step::Stop {
                exited: b,
                outcome: ob,
            },
        ) => a == b && oa == ob,
        _ => false,
    }
}

/// Judge every scenario in `dir` and every lane's run of it: each run is
/// the scenario's, each lane of `lanes` replayed each scenario, and every
/// file is a scenario or a run of one. Every problem, by its file; none when
/// all hold.
pub fn judge_dir(dir: &Path, lanes: &[&str]) -> Vec<String> {
    let mut problems = Vec::new();
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) => return vec![format!("{} cannot be read: {e}", dir.display())],
    };
    let mut files: BTreeMap<String, String> = BTreeMap::new();
    for entry in entries {
        let path = match entry {
            Ok(entry) => entry.path(),
            Err(e) => {
                problems.push(format!("{} cannot be read: {e}", dir.display()));
                continue;
            }
        };
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !name.ends_with(".json") {
            continue;
        }
        match std::fs::read_to_string(&path) {
            Ok(text) => {
                files.insert(name.to_string(), text);
            }
            Err(e) => problems.push(format!("{name} cannot be read: {e}")),
        }
    }
    let mut read: BTreeMap<&str, RunTrace> = BTreeMap::new();
    for (name, text) in &files {
        match RunTrace::from_json(text) {
            Ok(run) => {
                read.insert(name.as_str(), run);
            }
            Err(e) => problems.push(format!("{name} is no run trace: {e}")),
        }
    }
    let scenarios: Vec<(&str, &RunTrace)> = read
        .iter()
        .filter_map(|(name, run)| name.strip_suffix(SCENARIO_SUFFIX).map(|stem| (stem, run)))
        .collect();
    if scenarios.is_empty() {
        problems.push(format!(
            "{} holds no scenario (<name>{SCENARIO_SUFFIX})",
            dir.display()
        ));
    }
    for (stem, scenario) in &scenarios {
        if scenario.lane != super::behaviour::MODEL_LANE {
            problems.push(format!(
                "{stem}{SCENARIO_SUFFIX} is a run of the lane {}, and a scenario is the model's",
                scenario.lane
            ));
        }
        for lane in lanes {
            let name = format!("{stem}.{lane}.json");
            match read.get(name.as_str()) {
                None if !files.contains_key(&name) => problems.push(format!(
                    "{name}: the lane {lane} did not replay the scenario {stem}"
                )),
                None => {}
                Some(run) if run.lane != *lane => problems.push(format!(
                    "{name} is a run of the lane {}, and its name says {lane}",
                    run.lane
                )),
                Some(run) if !run.inputs.is_empty() || run.cycle.is_some() => problems.push(
                    format!("{name} says inputs or a cycle, which a lane's run does not"),
                ),
                Some(run) => {
                    if let Some(parted) = parting(scenario, run) {
                        problems.push(format!("{name} is not the run of {stem}: {parted}"));
                    }
                }
            }
        }
    }
    let expected: BTreeSet<String> = scenarios
        .iter()
        .flat_map(|(stem, _)| {
            std::iter::once(format!("{stem}{SCENARIO_SUFFIX}"))
                .chain(lanes.iter().map(move |lane| format!("{stem}.{lane}.json")))
        })
        .collect();
    for name in files.keys() {
        if !expected.contains(name) {
            problems.push(format!(
                "{name} is neither a scenario nor the run of a lane of one"
            ));
        }
    }
    problems
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run_trace::Fired;

    fn scenario() -> RunTrace {
        RunTrace::from_json(
            r#"{"version": 1, "lane": "model", "steps": [
                {"kind": "start", "entered": ["a"], "configuration": ["a"]},
                {"kind": "external", "event": "go", "fired": [{"source": "a", "index": 0}],
                 "exited": ["a"], "entered": ["p", "p1", "p2"], "configuration": ["p", "p1", "p2"]}],
               "inputs": [{"kind": "arrive", "after": 1, "event": "go"}]}"#,
        )
        .expect("a scenario")
    }

    fn lane(scenario: &RunTrace, name: &str) -> RunTrace {
        RunTrace {
            lane: name.to_string(),
            steps: scenario.steps.clone(),
            inputs: Vec::new(),
            cycle: None,
        }
    }

    /// Turns red if a lane's run that takes the model's steps is refused for
    /// the order it lists a configuration in, or one that parts from it is
    /// let through.
    #[test]
    fn a_run_is_the_scenarios_when_it_takes_its_steps() {
        let scenario = scenario();
        let mut run = lane(&scenario, "rust");
        assert_eq!(parting(&scenario, &run), None);
        if let Step::External { microstep, .. } = &mut run.steps[1] {
            microstep.configuration.reverse();
        }
        assert_eq!(parting(&scenario, &run), None, "a configuration is a set");
        if let Step::External { microstep, .. } = &mut run.steps[1] {
            microstep.entered.reverse();
        }
        assert!(parting(&scenario, &run).is_some_and(|why| why.starts_with("step 1:")));
        let mut other = lane(&scenario, "c11");
        if let Step::External { microstep, .. } = &mut other.steps[1] {
            microstep.fired = vec![Fired {
                source: "a".to_string(),
                index: 1,
            }];
        }
        assert!(parting(&scenario, &other).is_some());
        let mut short = lane(&scenario, "c11");
        short.steps.pop();
        assert_eq!(
            parting(&scenario, &short).as_deref(),
            Some("the model took 2 steps, and the lane c11 took 1")
        );
    }

    /// Turns red if a scenario a lane did not replay, a run that parts from
    /// its scenario, or a file of neither passes.
    #[test]
    fn every_lane_replays_every_scenario_of_a_directory() {
        let dir = std::env::temp_dir().join(format!("sce-replay-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a directory");
        let scenario = scenario();
        std::fs::write(dir.join("R1.scenario.json"), scenario.to_json()).expect("written");
        std::fs::write(dir.join("R1.rust.json"), lane(&scenario, "rust").to_json())
            .expect("written");
        let holding = judge_dir(&dir, &["rust"]);
        let missing = judge_dir(&dir, &["rust", "c11"]);
        let mut parted = lane(&scenario, "rust");
        parted.steps.pop();
        std::fs::write(dir.join("R1.rust.json"), parted.to_json()).expect("written");
        std::fs::write(dir.join("R2.rust.json"), lane(&scenario, "rust").to_json())
            .expect("written");
        let refused = judge_dir(&dir, &["rust"]);
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(holding, Vec::<String>::new());
        assert_eq!(
            missing,
            ["R1.c11.json: the lane c11 did not replay the scenario R1"]
        );
        assert_eq!(refused.len(), 2, "{refused:?}");
        assert!(refused[0].starts_with("R1.rust.json is not the run of R1: the model took 2"));
        assert_eq!(
            refused[1],
            "R2.rust.json is neither a scenario nor the run of a lane of one"
        );
    }
}
