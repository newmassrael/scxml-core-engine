// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A test suite of the test profile as its runner reads it
//! (docs/SCE_TEST_PROFILE.md §4.11, §11.7, §11.8, §12.2): the test cases in run
//! order, each with its guard and the selection that runs it in a world of the
//! suite's features, and the suite's module parameters; and the runner, which
//! runs the cases a world selects one at a time, in that order.
//!
//! The tables are generated from the `test-case` and `test-suite` documents (a
//! case's configuration from its `test-configuration`). They are data: what a
//! case does is its components' machines, which the run drives
//! ([`super::TestCaseRun`]). The runner is a library function, with no `main`:
//! a generated or hand-written test calls it with the system adapters that play
//! the SUT.

use std::collections::BTreeSet;
use std::fmt;
use std::sync::Arc;

use super::module_parameters::ModuleParameters;
use super::port::Duration;
use super::run::{
    Configuration, LogKind, LogRecord, MachineFactory, SystemAdapter, TestCaseResult, TestCaseRun,
    MTC_NAME,
};
use super::test_log::{TestLogEvent, TestLogRecord};
use super::verdict::Verdict;

/// The features a world of a suite's feature model enables (ADR 0057 §2): a
/// configuration of the model, closed under its `requires` edges.
pub type Features = BTreeSet<String>;

/// Whether `world` enables `feature`: the one test a selection is made of.
pub fn enabled(world: &Features, feature: &str) -> bool {
    world.contains(feature)
}

/// A `test-case` (§11.7): its name and what it runs in.
///
/// The configuration and the machines of its components are its
/// configuration's; the module parameters are the ids every component of the
/// configuration reads, which the suite must give.
#[derive(Clone)]
pub struct TestCase {
    /// The test case's name in the suite and in the test log.
    pub name: String,
    /// The configuration it runs.
    pub configuration: Configuration,
    /// The machines of the configuration's component types.
    pub machines: MachineFactory,
    /// The ids of the module parameters its components read.
    pub module_parameters: Vec<String>,
}

/// A selection (§11.8): whether a world of the suite's features runs a test
/// case. A condition of ADR 0057's grammar, lowered to the features it names.
pub type Selection = fn(&Features) -> bool;

/// One `<sce:execute>` (§11.8, P11): a test case, its guard, and its selection.
#[derive(Clone)]
pub struct Execute {
    /// The test case.
    pub test_case: TestCase,
    /// The guard: the test case's time limit, if it has one.
    pub guard: Option<Duration>,
    /// The selection, or none for a test case every world runs.
    pub selection: Option<Selection>,
}

/// A `test-suite` (§11.8): its test cases in run order and its module
/// parameters at their defaults.
#[derive(Clone, Default)]
pub struct Suite {
    /// The suite's name.
    pub name: String,
    /// The module parameters at their defaults.
    pub module_parameters: ModuleParameters,
    /// The test cases, in run order.
    pub executes: Vec<Execute>,
}

/// What the runner reports of one test case it ran.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaseResult {
    /// The test case's name.
    pub test_case: String,
    /// Its verdict and log.
    pub result: TestCaseResult,
}

/// An override of a module parameter the suite does not declare: the caller's
/// mistake, not a verdict of any test case, so no case is run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OverrideRefusal(String);

impl OverrideRefusal {
    /// Why.
    pub fn reason(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for OverrideRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for OverrideRefusal {}

impl CaseResult {
    /// The test log of this run of the test case in the suite `suite`, in its
    /// one canonical text ([`TestCaseResult::canonical_log`]).
    pub fn canonical_log(&self, suite: &str) -> String {
        self.result.canonical_log(suite, &self.test_case)
    }
}

/// P13: the test case cannot start, so it ends at once with `error`. No
/// component was created, so the test log holds its start and its end alone.
fn refused(test_case: &str, why: String) -> CaseResult {
    let at = |event: TestLogEvent| TestLogRecord { at: 0, event };
    CaseResult {
        test_case: test_case.to_string(),
        result: TestCaseResult {
            verdict: Verdict::Error,
            ended_at: 0,
            log: vec![LogRecord {
                at: 0,
                component: MTC_NAME.to_string(),
                kind: LogKind::Error,
                detail: why,
            }],
            records: vec![
                at(TestLogEvent::TestCaseStarted),
                at(TestLogEvent::TestCaseEnded {
                    verdict: Verdict::Error,
                }),
            ],
        },
    }
}

/// Runs the test cases of `suite` that `world` selects (P11).
///
/// One at a time, on the calling thread, in the suite's order, each a fresh run
/// from `NOW = 0` with fresh entities against the adapter `adapters` builds
/// for it, given the test case's name, so that no case sees another's trace.
/// Each component reads the suite's module parameters, with the values
/// `overrides` gives in place of their defaults.
///
/// A test case whose components read a module parameter the run has no value
/// for ends with `error` (P13) before any component is created, as does one
/// for which `adapters` builds no adapter. An override of a module parameter
/// the suite does not declare is refused before any case runs.
pub fn run_suite(
    suite: &Suite,
    world: &Features,
    mut adapters: impl FnMut(&str) -> Option<Box<dyn SystemAdapter>>,
    overrides: &ModuleParameters,
) -> Result<Vec<CaseResult>, OverrideRefusal> {
    let mut parameters = suite.module_parameters.clone();
    for (id, value) in overrides.values() {
        if !parameters.has(id) {
            return Err(OverrideRefusal(format!(
                "overrides the module parameter '{id}', which the suite '{}' does not declare",
                suite.name
            )));
        }
        parameters.set(id.clone(), value.clone());
    }
    let mut results = Vec::new();
    for execute in &suite.executes {
        if execute.selection.is_some_and(|selects| !selects(world)) {
            continue;
        }
        let test_case = &execute.test_case;
        if let Some(missing) = test_case
            .module_parameters
            .iter()
            .find(|id| !parameters.has(id))
        {
            results.push(refused(
                &test_case.name,
                format!(
                    "the test case reads the module parameter '{missing}', which is given no value"
                ),
            ));
            continue;
        }
        let Some(adapter) = adapters(&test_case.name) else {
            results.push(refused(
                &test_case.name,
                "no system adapter is given for the test case".to_string(),
            ));
            continue;
        };
        let run = TestCaseRun::new(
            test_case.configuration.clone(),
            Arc::clone(&test_case.machines),
            adapter,
            execute.guard,
            parameters.clone(),
        );
        results.push(CaseResult {
            test_case: test_case.name.clone(),
            result: run.run(),
        });
    }
    Ok(results)
}
