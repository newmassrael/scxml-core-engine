// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The inductive invariants of a models manifest (docs/adr/0115, D3): what a
//! bounded exploration cannot show, that a property holds after executions of
//! every length, shown at the fixed parameters of a configuration.
//!
//! An invariant `Inv` is inductive when every initial state satisfies it and
//! every step from a state that satisfies it ends in one that does; then every
//! reachable state satisfies it, however long the execution. A theorem `P`
//! follows when every state that satisfies `Inv` satisfies `P` (Lamport,
//! Specifying Systems, 8.9; the three queries of Apalache's manual, "Checking
//! an inductive invariant"). Each is an obligation Apalache checks
//! symbolically, over every state the predicate it starts from allows, not
//! only the reachable ones:
//!
//! - **base**: from `Init`, `Inv` holds (`--length=0`);
//! - **step**: from `Inv`, `Inv` holds after one step of `Next` (`--length=1`);
//! - **implies**: from `Inv`, each theorem holds (`--length=0`).
//!
//! Apalache starts the last two from `Inv` itself when its first conjuncts give
//! every variable a value (`x \in S`), or from the TYPES the module names for it,
//! conjoined with `Inv` by the judge, when a variable can only be given one by
//! Apalache's `Gen` (a sequence). The types give each variable a value of its
//! type and narrow nothing the invariant does not; a person reads that, as a
//! promise is read.
//!
//! A theorem is a property the model's holding runs check, so the invariant
//! proves what the model states and TLC explores. And an invariant that cannot
//! fail shows nothing (ADR 0115, D5): each has a run that takes a part out, or
//! goes against an assumption, and names the obligation that fails without it;
//! the obligations before it must hold, so the failure is the one it names.

use std::collections::BTreeSet;
use std::path::Path;

use serde::Deserialize;
use serde_json::Value;

use super::apalache;
use super::models::{self, Model};
use super::FormalError;

const INDUCTIVE_KEYS: &[&str] = &["invariant", "types", "implies", "runs"];
const RUN_KEYS: &[&str] = &["config", "expect", "obligation", "without", "against"];

/// One inductive invariant of a model, as `schemas/sce-formal-models.v1.schema.json`
/// describes it.
#[derive(Debug, Clone, Deserialize)]
pub struct Inductive {
    /// The operator of the module that is the invariant.
    pub invariant: String,
    /// The operator that gives every variable a value of its type, for Apalache
    /// to start the step and the implication from, when the invariant alone does
    /// not.
    #[serde(default)]
    pub types: Option<String>,
    /// The properties of the model it proves, each one a holding run checks.
    #[serde(default)]
    pub implies: Vec<String>,
    #[serde(default)]
    pub runs: Vec<InductiveRun>,
    #[serde(flatten)]
    pub other: std::collections::BTreeMap<String, Value>,
}

/// One configuration whose parameters an inductive invariant is checked at,
/// and what Apalache must answer.
#[derive(Debug, Clone, Deserialize)]
pub struct InductiveRun {
    pub config: String,
    pub expect: String,
    /// The obligation a violating run fails: `base`, `step` or `implies`.
    #[serde(default)]
    pub obligation: Option<String>,
    #[serde(default)]
    pub without: Option<String>,
    #[serde(default)]
    pub against: Option<String>,
    #[serde(flatten)]
    pub other: std::collections::BTreeMap<String, Value>,
}

impl InductiveRun {
    fn holds(&self) -> bool {
        self.expect == "holds"
    }

    fn violates(&self) -> bool {
        self.expect == "violates"
    }
}

/// What an inductive invariant is checked for, in the order it is checked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Obligation {
    /// Every initial state satisfies the invariant.
    Base,
    /// Every step from a state that satisfies it ends in one that does.
    Step,
    /// Every state that satisfies it satisfies each theorem.
    Implies,
}

impl Obligation {
    /// Every obligation, in the order they are checked.
    pub const ALL: [Obligation; 3] = [Obligation::Base, Obligation::Step, Obligation::Implies];

    /// The name a manifest gives it.
    pub fn name(self) -> &'static str {
        match self {
            Obligation::Base => "base",
            Obligation::Step => "step",
            Obligation::Implies => "implies",
        }
    }

    /// The obligation a manifest names.
    pub fn named(text: &str) -> Option<Self> {
        Obligation::ALL.into_iter().find(|ob| ob.name() == text)
    }
}

/// The name a report and a shard plan give the run of `invariant` at `config`,
/// which no configuration of TLC's runs is named.
pub fn unit_name(invariant: &str, config: &str) -> String {
    format!("{config} [{invariant}]")
}

/// The operator of the module the judge writes that Apalache starts the step
/// and the implication from.
const START: &str = "InductiveStart";

/// The module the judge writes beside a model for the obligations of one
/// invariant, as its name and its text: the model extended with the predicate
/// the step and the implication start from, the types (when there are) conjoined
/// with the invariant, so every state they start from satisfies it.
pub fn start_module(module: &str, inductive: &Inductive) -> (String, String) {
    let name = format!("{module}Inductive");
    let start = match &inductive.types {
        Some(types) => format!("{types} /\\ {}", inductive.invariant),
        None => inductive.invariant.clone(),
    };
    let text = format!("---- MODULE {name} ----\nEXTENDS {module}\n{START} == {start}\n====\n");
    (name, text)
}

/// What Apalache is asked for one obligation: the predicate it starts from, the
/// invariants it checks and the steps it takes.
pub fn asked(obligation: Obligation, inductive: &Inductive) -> (String, Vec<String>, u32) {
    match obligation {
        Obligation::Base => ("Init".to_string(), vec![inductive.invariant.clone()], 0),
        Obligation::Step => (START.to_string(), vec![inductive.invariant.clone()], 1),
        Obligation::Implies => (START.to_string(), inductive.implies.clone(), 0),
    }
}

/// The obligations `run` checks, in order: every one for a run that holds; for
/// one that violates, those up to the one it names.
pub fn obligations(run: &InductiveRun) -> Vec<Obligation> {
    if !run.violates() {
        return Obligation::ALL.to_vec();
    }
    let Some(named) = run.obligation.as_deref().and_then(Obligation::named) else {
        return Obligation::ALL.to_vec();
    };
    Obligation::ALL
        .into_iter()
        .take_while(|ob| *ob != named)
        .chain([named])
        .collect()
}

/// What is wrong with Apalache's answer `status` to `obligation` of `run`, or
/// None where it is as declared: the obligation a violating run names must be
/// violated, and every other it checks must hold.
pub fn answer_judged(run: &InductiveRun, obligation: Obligation, status: i32) -> Option<String> {
    let named = run.violates() && run.obligation.as_deref() == Some(obligation.name());
    let what = obligation.name();
    match (named, status) {
        (false, apalache::HOLDS) | (true, apalache::VIOLATED) => None,
        (false, apalache::VIOLATED) if run.violates() => Some(format!(
            "the {what} is violated, before the obligation the run names fails"
        )),
        (false, apalache::VIOLATED) => Some(format!("the {what} is violated")),
        (true, apalache::HOLDS) => Some(format!(
            "the {what} holds, and the run says it fails without what it takes out"
        )),
        (_, other) => Some(format!(
            "Apalache answered {other} for the {what}, which judged nothing"
        )),
    }
}

fn is_name(text: &str) -> bool {
    let mut chars = text.chars();
    chars
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// The configurations the inductive runs of `model` name: declared by being
/// named, as a TLC run's are.
pub fn configs(model: &Model) -> impl Iterator<Item = &str> {
    model
        .inductive
        .iter()
        .flat_map(|stated| stated.runs.iter().map(|run| run.config.as_str()))
}

/// What is wrong with the inductive invariants `model` declares, each as one
/// line.
pub fn declared(model: &Model, formal: &Path) -> Vec<String> {
    let mut problems = Vec::new();
    let name = &model.module;
    let checked: BTreeSet<String> = model
        .runs
        .iter()
        .filter(|run| run.expect == "holds")
        .filter(|run| formal.join(&run.config).is_file())
        .flat_map(|run| models::invariants_of(&models::read(&formal.join(&run.config))))
        .collect();
    let mut seen = BTreeSet::new();
    for stated in &model.inductive {
        let invariant = &stated.invariant;
        let what = format!("{name}: inductive {invariant}");
        let unknown = models::unknown_keys(&stated.other, INDUCTIVE_KEYS);
        if !unknown.is_empty() {
            problems.push(format!(
                "{what}: unknown keys {}",
                models::listed(&unknown.into_iter().collect())
            ));
        }
        if !seen.insert(invariant.clone()) {
            problems.push(format!("{what}: declared twice"));
        }
        let named = std::iter::once(invariant)
            .chain(&stated.types)
            .chain(&stated.implies);
        for operator in named {
            if !is_name(operator) {
                problems.push(format!("{what}: {operator:?} is no name of an operator"));
            }
        }
        if stated.implies.is_empty() {
            problems.push(format!(
                "{what}: implies nothing, and an inductive invariant is stated for the theorem it proves"
            ));
        }
        for theorem in &stated.implies {
            if !checked.contains(theorem) {
                problems.push(format!(
                    "{what}: implies {theorem}, which no holding run of {name} checks"
                ));
            }
        }
        if !stated.runs.iter().any(InductiveRun::holds) {
            problems.push(format!("{what}: no run says it holds"));
        }
        if !stated.runs.iter().any(InductiveRun::violates) {
            problems.push(format!("{what}: no run shows that it can fail"));
        }
        let mut configs = BTreeSet::new();
        for run in &stated.runs {
            let unit = unit_name(invariant, &run.config);
            if !configs.insert(run.config.as_str()) {
                problems.push(format!("{unit}: declared twice"));
            }
            let stray = models::unknown_keys(&run.other, RUN_KEYS);
            if !stray.is_empty() {
                problems.push(format!(
                    "{unit}: unknown keys {}",
                    models::listed(&stray.into_iter().collect())
                ));
            }
            let path = formal.join(&run.config);
            if !path.is_file() {
                problems.push(format!("{unit}: declared, and no such file"));
                continue;
            }
            let off = models::parts_taken_out(model, &models::read(&path));
            if run.holds() {
                if !off.is_empty() {
                    problems.push(format!(
                        "{unit}: holds, with {} taken out",
                        models::listed(&off)
                    ));
                }
                if run.obligation.is_some() || run.without.is_some() || run.against.is_some() {
                    problems.push(format!(
                        "{unit}: a run that holds checks every obligation, and names no obligation, part or assumption"
                    ));
                }
            } else if run.violates() {
                if run
                    .obligation
                    .as_deref()
                    .and_then(Obligation::named)
                    .is_none()
                {
                    problems.push(format!(
                        "{unit}: a violation names the obligation that fails: base, step or implies"
                    ));
                }
                if run.without.is_some() == run.against.is_some() {
                    problems.push(format!(
                        "{unit}: says either what part it takes out or what it goes against"
                    ));
                }
                if let Some(without) = &run.without {
                    if off != BTreeSet::from([without.clone()]) {
                        let taken = if off.is_empty() {
                            "nothing".to_string()
                        } else {
                            models::listed(&off)
                        };
                        problems.push(format!(
                            "{unit}: says it takes out {without}, and takes out {taken}"
                        ));
                    }
                } else if !off.is_empty() {
                    problems.push(format!(
                        "{unit}: takes out {} without saying so",
                        models::listed(&off)
                    ));
                }
            } else {
                problems.push(format!(
                    "{unit}: expect is 'holds' or 'violates', not '{}'",
                    run.expect
                ));
            }
        }
    }
    problems
}

/// Check `run` of `inductive`, a declared invariant of `model`, with Apalache
/// at `tool` on the Java runtime at `java_home`: each obligation it checks in
/// turn, stopping at the first answer that is not as declared. None where every
/// answer is; otherwise what is wrong and what Apalache printed for it.
pub fn check_run(
    tool: &Path,
    java_home: &Path,
    formal: &Path,
    model: &Model,
    inductive: &Inductive,
    run: &InductiveRun,
) -> Result<Option<(String, String)>, FormalError> {
    let constants = models::constants_config(&models::read(&formal.join(&run.config)));
    let (module, text) = start_module(&model.module, inductive);
    let extra = [(format!("{module}.tla"), text)];
    for obligation in obligations(run) {
        let (init, invariants, length) = asked(obligation, inductive);
        let answer = apalache::run(&apalache::Run {
            tool,
            java_home,
            sources: formal,
            extra: &extra,
            module: &module,
            constants: &constants,
            init: &init,
            invariants: &invariants,
            length,
        })?;
        if let Some(why) = answer_judged(run, obligation, answer.status) {
            return Ok(Some((why, answer.output)));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn run(value: Value) -> InductiveRun {
        serde_json::from_value(value).expect("a run")
    }

    #[test]
    fn a_holding_run_checks_every_obligation_and_a_violating_one_up_to_its_own() {
        let holding = run(json!({"config": "M.cfg", "expect": "holds"}));
        assert_eq!(obligations(&holding), Obligation::ALL.to_vec());
        let step = run(json!({"config": "c", "expect": "violates", "obligation": "step"}));
        assert_eq!(obligations(&step), vec![Obligation::Base, Obligation::Step]);
        let base = run(json!({"config": "c", "expect": "violates", "obligation": "base"}));
        assert_eq!(obligations(&base), vec![Obligation::Base]);
    }

    #[test]
    fn an_answer_is_judged_against_the_obligation_it_answers() {
        let holding = run(json!({"config": "M.cfg", "expect": "holds"}));
        for ob in Obligation::ALL {
            assert_eq!(answer_judged(&holding, ob, apalache::HOLDS), None);
            assert!(answer_judged(&holding, ob, apalache::VIOLATED).is_some());
            assert!(answer_judged(&holding, ob, 120).is_some(), "a type error");
        }
        let step = run(json!({"config": "c", "expect": "violates", "obligation": "step"}));
        assert_eq!(
            answer_judged(&step, Obligation::Base, apalache::HOLDS),
            None
        );
        assert!(answer_judged(&step, Obligation::Base, apalache::VIOLATED).is_some());
        assert_eq!(
            answer_judged(&step, Obligation::Step, apalache::VIOLATED),
            None
        );
        assert!(answer_judged(&step, Obligation::Step, apalache::HOLDS).is_some());
        assert!(answer_judged(&step, Obligation::Step, 255).is_some());
    }

    #[test]
    fn the_step_starts_from_the_invariant_and_its_types() {
        let plain: Inductive = serde_json::from_value(
            json!({"invariant": "Ind", "implies": ["Safe", "Other"], "runs": []}),
        )
        .expect("an invariant");
        let (name, text) = start_module("M", &plain);
        assert_eq!(name, "MInductive");
        assert_eq!(
            text,
            "---- MODULE MInductive ----\nEXTENDS M\nInductiveStart == Ind\n====\n"
        );
        let generated: Inductive = serde_json::from_value(
            json!({"invariant": "Ind", "types": "Typed", "implies": ["Safe"], "runs": []}),
        )
        .expect("an invariant");
        assert!(start_module("M", &generated)
            .1
            .contains("InductiveStart == Typed /\\ Ind\n"));
        assert_eq!(
            asked(Obligation::Base, &plain),
            ("Init".to_string(), vec!["Ind".to_string()], 0)
        );
        assert_eq!(
            asked(Obligation::Step, &plain),
            ("InductiveStart".to_string(), vec!["Ind".to_string()], 1)
        );
        assert_eq!(
            asked(Obligation::Implies, &plain),
            (
                "InductiveStart".to_string(),
                vec!["Safe".to_string(), "Other".to_string()],
                0
            )
        );
    }

    #[test]
    fn an_obligation_is_named_as_the_manifest_names_it() {
        for ob in Obligation::ALL {
            assert_eq!(Obligation::named(ob.name()), Some(ob));
        }
        assert_eq!(Obligation::named("induction"), None);
    }

    #[test]
    fn a_name_of_an_operator_is_one_word() {
        assert!(is_name("IndInv") && is_name("_x1"));
        assert!(!is_name("") && !is_name("1x") && !is_name("A,B") && !is_name("A B"));
    }
}
