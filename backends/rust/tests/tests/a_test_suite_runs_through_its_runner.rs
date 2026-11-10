// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A test suite generated in Rust (docs/SCE_TEST_PROFILE.md §11.6 to §11.8,
// §12.2, §12.6 slice 6), run end to end by the profile's runner,
// `sce_rust_runtime::test::run_suite`. The suite, its test cases and their
// configurations are the tables sce-codegen writes from the fixtures under
// `sce-build/tests/fixtures/test_profile/runtime/` into
// `src/integration/static_datamodel/` (regen:
// `scripts/regen_static_datamodel_rust.sh`), and the components are the
// machines it writes from the components there. Only the system adapter, which
// plays the system under test, is written here.
//
// Each case is the twin of the C++ gtest of the same name in
// `tests/integration/ATestSuiteRunsEndToEndTest.cpp`
// (`ATestSuite.RunsEveryTestCaseOfAWorldWithTheVerdictItExpects` is
// `runs_every_test_case_of_a_world_with_the_verdict_it_expects`), with the
// same suite, world, verdicts and `NOW`.

use std::sync::{Arc, Mutex};

use sce_rust_runtime::saved_state::SavedValue;
use sce_rust_runtime::test::{
    run_suite, run_suite_with_traces, CaseResult, Duration, Features, Instant, LogKind, Message,
    ModuleParameters, SystemAdapter, Trace, Verdict,
};
use sce_rust_tests::integration::static_datamodel::{
    answers_the_job, asking_configuration, job_reply, job_request, reply_ok, runtime_suite,
};

type Request = job_request::JobRequestPayload;
type Reply = job_reply::JobReplyPayload;

const MILLISECOND: Duration = 1000;
const SECOND: Duration = 1000 * MILLISECOND;

/// How long the system under test takes to answer a request.
const ANSWER_DELAY: Duration = 500;

/// The jobs the system under test was asked for, across the test cases of a
/// run, which the test keeps while each run owns its adapter.
type Requested = Arc<Mutex<Vec<u32>>>;

/// The system under test on the system port `sut`: it answers each request,
/// ANSWER_DELAY after it, with a reply of `code` for the job the request
/// names, and records each job it was asked for.
struct Responder {
    code: u32,
    requested: Requested,
    pending: Vec<(Instant, Message)>,
}

impl SystemAdapter for Responder {
    fn send(&mut self, system_port: &str, message: &Message, now: Instant) {
        assert_eq!(system_port, "sut");
        let request = message
            .value_as::<Request>()
            .expect("a request travels as its record's value");
        self.requested
            .lock()
            .expect("the record of requests")
            .push(request.job);
        let reply = Reply {
            job: request.job,
            code: self.code,
        };
        // The reply's canonical form in the test log is the one the unit of a
        // template over its record writes.
        self.pending.push((
            now + ANSWER_DELAY,
            Message::new(
                job_reply::EVENT_NAME,
                reply,
                reply_ok::TYPE,
                reply_ok::canonical,
            ),
        ));
    }

    fn next_input(&self) -> Option<Instant> {
        self.pending.first().map(|(at, _)| *at)
    }

    fn take_inputs_due(&mut self, now: Instant) -> Vec<(String, Message)> {
        let due = self.pending.iter().take_while(|(at, _)| *at <= now).count();
        self.pending
            .drain(..due)
            .map(|(_, message)| ("sut".to_string(), message))
            .collect()
    }
}

/// A fresh Responder for each test case, answering with `code`.
fn responders(
    code: u32,
    requested: &Requested,
) -> impl FnMut(&str) -> Option<Box<dyn SystemAdapter>> + '_ {
    move |_: &str| -> Option<Box<dyn SystemAdapter>> {
        Some(Box::new(Responder {
            code,
            requested: Arc::clone(requested),
            pending: Vec::new(),
        }))
    }
}

/// A world that enables `features`.
fn world(features: &[&str]) -> Features {
    features.iter().map(|feature| feature.to_string()).collect()
}

/// The suite's run in `world`, answered with `code`, with `overrides` in
/// place of the defaults of the module parameters they give.
fn run(
    world_features: &[&str],
    code: u32,
    requested: &Requested,
    overrides: &ModuleParameters,
) -> Vec<CaseResult> {
    run_suite(
        &runtime_suite::suite(),
        &world(world_features),
        responders(code, requested),
        overrides,
    )
    .expect("the suite declares every module parameter overridden")
}

fn names(results: &[CaseResult]) -> Vec<&str> {
    results
        .iter()
        .map(|result| result.test_case.as_str())
        .collect()
}

fn logged(result: &CaseResult, kind: LogKind) -> bool {
    result.result.log.iter().any(|record| record.kind == kind)
}

fn requested_jobs(requested: &Requested) -> Vec<u32> {
    requested.lock().expect("the record of requests").clone()
}

/// P11, P12, P13, P14: in a world with every feature the suite runs every test
/// case in its order, and each ends with the verdict it expects: a job answered
/// in time passes, a PTC served passes, a removed PTC's inconclusive outweighs
/// its MTC's pass, and a guard shorter than the answer ends in error.
#[test]
fn runs_every_test_case_of_a_world_with_the_verdict_it_expects() {
    let requested = Requested::default();
    let results = run(
        &["ptcs", "removal", "guard"],
        0,
        &requested,
        &ModuleParameters::default(),
    );
    assert_eq!(
        names(&results),
        [
            "answers_the_job",
            "serves_a_ptc",
            "keeps_a_ptc",
            "answers_before_the_guard"
        ]
    );
    let outcome = |i: usize| &results[i].result;
    assert_eq!(outcome(0).verdict, Verdict::Pass, "{:?}", outcome(0).log);
    assert_eq!(
        outcome(0).ended_at,
        ANSWER_DELAY,
        "the reply is taken in the round it arrives in"
    );
    assert_eq!(outcome(1).verdict, Verdict::Pass, "{:?}", outcome(1).log);
    assert_eq!(
        outcome(1).ended_at,
        0,
        "a branch fired in every round, so time never moved"
    );
    assert_eq!(
        outcome(2).verdict,
        Verdict::Inconclusive,
        "{:?}",
        outcome(2).log
    );
    assert_eq!(outcome(2).ended_at, 15 * MILLISECOND);
    assert_eq!(outcome(3).verdict, Verdict::Error, "{:?}", outcome(3).log);
    assert_eq!(
        outcome(3).ended_at,
        100,
        "the guard, 0.1ms, ends the case before the answer"
    );
    assert!(logged(&results[3], LogKind::Error));
    assert_eq!(
        requested_jobs(&requested),
        [7, 7],
        "each asking case asks for the job the suite's module parameter gives"
    );
}

/// P2, P7: a system under test that answers with a failure code fails the
/// test case when its timer runs out. A world with no feature runs only the
/// test case no selection guards.
#[test]
fn fails_a_job_the_system_under_test_answers_with_a_failure() {
    let requested = Requested::default();
    let results = run(&[], 3, &requested, &ModuleParameters::default());
    assert_eq!(names(&results), ["answers_the_job"]);
    assert_eq!(results[0].result.verdict, Verdict::Fail);
    assert_eq!(
        results[0].result.ended_at, SECOND,
        "time advances to the timer's instant"
    );
    assert!(logged(&results[0], LogKind::Timeout));
}

/// The test log every runtime writes of `answers_the_job` in a world with no
/// feature, answered with success: one file the Rust and the C++ runs are both
/// held to (`ATestSuite.AGeneratedTestCaseWritesItsTestLog` in
/// `tests/integration/ATestSuiteRunsEndToEndTest.cpp`), so the agreement of
/// the backends is one set of bytes. The file ends in a newline, which no
/// canonical text holds.
const ANSWERS_THE_JOB_LOG: &str = include_str!(
    "../../../../sce-build/tests/fixtures/test_profile/runtime/expected_logs/answers_the_job.json"
);

/// docs/SCE_TEST_PROFILE.md §12.5: the test log of a generated test case, in
/// its one canonical text. The request the component sends carries the value
/// its template's unit writes, the job of the suite's module parameter; the
/// reply arrives at 500 and is taken in that round, which stops the timer,
/// passes and ends the MTC.
#[test]
fn a_generated_test_case_writes_its_test_log() {
    let requested = Requested::default();
    let results = run(&[], 0, &requested, &ModuleParameters::default());
    assert_eq!(names(&results), ["answers_the_job"]);
    assert_eq!(
        results[0].canonical_log("runtime_suite"),
        ANSWERS_THE_JOB_LOG
            .strip_suffix('\n')
            .expect("the shared log ends in one newline")
    );
}

/// The recorded input trace of `answers_the_job` (docs/SCE_TTCN3_LANE.md §10):
/// the input the Responder above gives the case when it answers with success,
/// the reply of 0 to job 7 at 500, as a file every backend's player reads.
const ANSWERS_THE_JOB_TRACE: &str = include_str!(
    "../../../../sce-build/tests/fixtures/test_profile/runtime/traces/answers_the_job.json"
);

/// The directory of the suite's committed traces.
fn traces_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../sce-build/tests/fixtures/test_profile/runtime/traces")
}

/// docs/SCE_TTCN3_LANE.md §10.5: the generated suite replayed from the
/// committed trace of `answers_the_job` writes the test log every runtime
/// writes of it, byte for byte: the configuration's unit reads the reply from
/// its canonical form and writes it back the same, and the clock advances to
/// the trace's instant as it did to the Responder's.
#[test]
fn a_recorded_trace_replays_to_the_test_log_every_runtime_writes() {
    let results = run_suite_with_traces(
        &runtime_suite::suite(),
        &world(&[]),
        |_: &str| Trace::parse(ANSWERS_THE_JOB_TRACE),
        &ModuleParameters::default(),
    )
    .expect("the suite declares every module parameter overridden");
    assert_eq!(names(&results), ["answers_the_job"]);
    assert_eq!(results[0].result.verdict, Verdict::Pass);
    assert_eq!(
        results[0].canonical_log("runtime_suite"),
        ANSWERS_THE_JOB_LOG
            .strip_suffix('\n')
            .expect("the shared log ends in one newline")
    );
}

/// A directory of the runner's logs, made empty, which `name` keeps apart
/// from every other test's.
fn log_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sce-runtime-suite-runner-{}-{name}",
        std::process::id()
    ));
    if dir.exists() {
        std::fs::remove_dir_all(&dir).expect("an earlier run's logs are removed");
    }
    dir
}

/// The suite's runner program, given `args` after the directories of the
/// traces and of the logs.
fn run_program(logs: &std::path::Path, args: &[&str]) -> std::process::Output {
    std::process::Command::new(env!("CARGO_BIN_EXE_runtime_suite_runner"))
        .arg("--trace-dir")
        .arg(traces_dir())
        .arg("--log-dir")
        .arg(logs)
        .args(args)
        .output()
        .expect("the runner program runs")
}

/// docs/SCE_TTCN3_LANE.md §4: the suite's runner program runs each test case
/// its world selects against its trace and writes `<case>.log.json`, the
/// canonical log with nothing after it, and prints each case's verdict; a
/// module parameter's value on its command line replaces its default, and one
/// the suite does not declare is refused with exit status 2 before any case
/// runs.
#[test]
fn the_suites_runner_program_writes_the_log_of_each_trace() {
    let logs = log_dir("default");
    let output = run_program(&logs, &[]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "answers_the_job pass\n"
    );
    let log = std::fs::read_to_string(logs.join("answers_the_job.log.json"))
        .expect("the log of the case is written");
    assert_eq!(
        log,
        ANSWERS_THE_JOB_LOG
            .strip_suffix('\n')
            .expect("the shared log ends in one newline")
    );
    std::fs::remove_dir_all(&logs).expect("the logs are removed");

    let logs = log_dir("overridden");
    let output = run_program(&logs, &["--world", "", "--module-parameter", "job=9"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let log = std::fs::read_to_string(logs.join("answers_the_job.log.json"))
        .expect("the log of the case is written");
    assert!(
        log.contains("\"kind\":\"message-sent\",\"port\":\"sut\",\"type\":\"job_request\",\"value\":{\"job\":\"9\"}"),
        "the request names the job of the value given: {log}"
    );
    std::fs::remove_dir_all(&logs).expect("the logs are removed");

    let logs = log_dir("undeclared");
    let output = run_program(&logs, &["--module-parameter", "limit=1"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("the suite declares no module parameter 'limit'"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!logs.exists(), "no case ran, so no log was written");
}

/// §11.8: a world runs the test cases whose selection it satisfies, in the
/// suite's order: `ptcs` alone selects the served PTC and not the removed one,
/// and `guard` selects the guarded case.
#[test]
fn runs_the_test_cases_a_world_selects() {
    let requested = Requested::default();
    let none = ModuleParameters::default();
    assert_eq!(
        names(&run(&["ptcs"], 0, &requested, &none)),
        ["answers_the_job", "serves_a_ptc"]
    );
    assert_eq!(
        names(&run(&["guard"], 0, &requested, &none)),
        ["answers_the_job", "answers_before_the_guard"]
    );
}

/// -1 §8.2.1, §11.5: a value the caller gives the module parameter replaces the
/// suite's default, and the component asks for the job it names.
#[test]
fn asks_for_the_job_an_override_of_its_module_parameter_names() {
    let requested = Requested::default();
    let mut overrides = ModuleParameters::default();
    overrides.set("job", 9_u32.to_saved());
    let results = run(&[], 0, &requested, &overrides);
    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0].result.verdict,
        Verdict::Pass,
        "{:?}",
        results[0].result.log
    );
    assert_eq!(requested_jobs(&requested), [9]);
}

/// P13: a value of the module parameter that is not of the type the component
/// declares ends the test case in error before the component starts.
#[test]
fn ends_in_error_when_a_module_parameter_is_not_of_its_type() {
    let requested = Requested::default();
    let mut overrides = ModuleParameters::default();
    overrides.set("job", "seven".to_string().to_saved());
    let results = run(&[], 0, &requested, &overrides);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].result.verdict, Verdict::Error);
    assert!(logged(&results[0], LogKind::Error));
    assert!(
        requested_jobs(&requested).is_empty(),
        "the component did not start, so it sent nothing"
    );
}

/// §11.6, §11.7: the configuration's tables name the module parameters its
/// components read, which a suite that runs it must give, and the test case
/// names itself and its configuration.
#[test]
fn reads_the_module_parameters_its_configuration_names() {
    assert_eq!(asking_configuration::module_parameters(), ["job"]);
    assert_eq!(
        asking_configuration::configuration().mtc_type,
        "param_asker"
    );
    assert!(asking_configuration::machine("nobody").is_none());
    assert!(asking_configuration::machine("param_asker").is_some());
    let case = answers_the_job::test_case();
    assert_eq!(case.name, answers_the_job::NAME);
    assert_eq!(case.configuration, asking_configuration::configuration());
    assert_eq!(case.module_parameters, ["job"]);
}
