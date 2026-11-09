// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Test components generated in Rust (docs/SCE_TEST_PROFILE.md §12.3, §12.6
// slice 6), run end to end by the profile's runtime, `TestCaseRun`. The
// machines are the statecharts sce-codegen writes from the fixtures under
// `sce-build/tests/fixtures/test_profile/runtime/` into
// `src/integration/static_datamodel/` (regen:
// `scripts/regen_static_datamodel_rust.sh`), driven only through the doors of
// §12.3; the configuration, the factory of machines and the system adapter are
// written here, so that each component is held apart from the generated
// tables, which `a_test_suite_runs_through_its_runner.rs` runs.
//
// Each case is the twin of the C++ gtest of the same name in
// `tests/integration/ATestComponentRunsThroughItsDoorsTest.cpp`
// (`ATestComponent.ThatIsAnsweredWithAMatchingReplyPasses` is
// `a_test_component_that_is_answered_with_a_matching_reply_passes`), with the
// same scenario, verdict, `NOW` and log.

use std::sync::{Arc, Mutex};

use sce_rust_runtime::test::{
    ComponentMachine, Configuration, Duration, Endpoint, Instant, LogKind, MachineFactory, Message,
    ModuleParameters, SystemAdapter, TestCaseResult, TestCaseRun, Verdict,
};
use sce_rust_tests::integration::static_datamodel::{
    answerer_sm, asker_sm, coordinator_sm, idler_sm, job_reply, job_request, ptc_keeper_sm,
    reply_ok,
};

type Request = job_request::JobRequestPayload;
type Reply = job_reply::JobReplyPayload;

const MILLISECOND: Duration = 1000;
const SECOND: Duration = 1000 * MILLISECOND;

/// How long the system under test takes to answer a request.
const ANSWER_DELAY: Duration = 500;

/// The generated machine of each component type a configuration here names,
/// and none for another type, which ends the run with `error`.
fn machine_of(component_type: &str) -> Option<Box<dyn ComponentMachine>> {
    let machine: Box<dyn ComponentMachine> = match component_type {
        "asker" => Box::<asker_sm::AskerComponent>::default(),
        "coordinator" => Box::<coordinator_sm::CoordinatorComponent>::default(),
        "answerer" => Box::<answerer_sm::AnswererComponent>::default(),
        "ptc_keeper" => Box::<ptc_keeper_sm::PtcKeeperComponent>::default(),
        "idler" => Box::<idler_sm::IdlerComponent>::default(),
        _ => return None,
    };
    Some(machine)
}

fn factory() -> MachineFactory {
    Arc::new(machine_of)
}

/// What the system under test was handed: each send, its system port and
/// event, and the job each request named.
#[derive(Default)]
struct Seen {
    sent: Vec<(String, String)>,
    requested: Vec<u32>,
}

/// The system under test on the mapped port `sut`: it answers each request,
/// ANSWER_DELAY after it, with a reply of `code` for the requested job, or for
/// `job` when one is given. It records what it was sent in `seen`, which the
/// test keeps while the run owns the adapter.
struct Responder {
    job: Option<u32>,
    code: u32,
    pending: Vec<(Instant, Message)>,
    seen: Arc<Mutex<Seen>>,
}

impl Responder {
    /// A responder, and the record of what it is sent.
    fn answering(job: Option<u32>, code: u32) -> (Self, Arc<Mutex<Seen>>) {
        let seen = Arc::new(Mutex::new(Seen::default()));
        let responder = Responder {
            job,
            code,
            pending: Vec::new(),
            seen: Arc::clone(&seen),
        };
        (responder, seen)
    }
}

impl SystemAdapter for Responder {
    fn send(&mut self, system_port: &str, message: &Message, now: Instant) {
        let mut seen = self.seen.lock().expect("the record of sends");
        seen.sent
            .push((system_port.to_string(), message.event.clone()));
        let Some(request) = message.value_as::<Request>() else {
            return;
        };
        seen.requested.push(request.job);
        let reply = Reply {
            job: self.job.unwrap_or(request.job),
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

/// A system under test nothing is mapped to.
struct Silent;

impl SystemAdapter for Silent {
    fn send(&mut self, system_port: &str, message: &Message, _now: Instant) {
        panic!(
            "no port is mapped, so nothing reaches the system adapter, yet {} was sent on \
             {system_port}",
            message.event
        );
    }

    fn next_input(&self) -> Option<Instant> {
        None
    }

    fn take_inputs_due(&mut self, _now: Instant) -> Vec<(String, Message)> {
        Vec::new()
    }
}

/// The index of the first record of `kind` by `component`, or none.
fn first_record(result: &TestCaseResult, component: &str, kind: LogKind) -> Option<usize> {
    result
        .log
        .iter()
        .position(|record| record.component == component && record.kind == kind)
}

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| item.to_string()).collect()
}

fn run(configuration: Configuration, adapter: Box<dyn SystemAdapter>) -> TestCaseResult {
    TestCaseRun::new(
        configuration,
        factory(),
        adapter,
        None,
        ModuleParameters::default(),
    )
    .run()
}

/// The asker as the MTC, its port `sut` mapped to the system port `sut`.
fn asker_configuration() -> Configuration {
    Configuration {
        mtc_type: "asker".to_string(),
        ports_of_type: [("asker".to_string(), strings(&["sut"]))].into(),
        timers_of_type: [("asker".to_string(), strings(&["reply"]))].into(),
        mappings: vec![(Endpoint::new("mtc", "sut"), "sut".to_string())],
        ..Configuration::default()
    }
}

fn run_asker(job: Option<u32>, code: u32) -> (TestCaseResult, Arc<Mutex<Seen>>) {
    let (responder, seen) = Responder::answering(job, code);
    (run(asker_configuration(), Box::new(responder)), seen)
}

/// P6, P2, P4: the send carries the template's value bound to the component's
/// variable, a reply its receive branch's template matches is taken when it
/// arrives, and the content reads the reply as `_event.data`.
#[test]
fn a_test_component_that_is_answered_with_a_matching_reply_passes() {
    let (result, seen) = run_asker(None, 0);
    assert_eq!(result.verdict, Verdict::Pass, "{:?}", result.log);
    assert_eq!(
        result.ended_at, ANSWER_DELAY,
        "the reply is taken in the round it arrives in"
    );
    let seen = seen.lock().expect("the record of sends");
    assert_eq!(
        seen.sent,
        [("sut".to_string(), job_request::EVENT_NAME.to_string())],
        "a message travels under its template's record's event"
    );
    assert_eq!(seen.requested, [7], "the template's value, given `job`");
    assert!(first_record(&result, "mtc", LogKind::Receive).is_some());
    // P8: the `<log>` of the branch's content is in the test log, with the
    // instant and the component, its label and the job it read.
    let logged = first_record(&result, "mtc", LogKind::Log)
        .expect("the component's <log> is recorded in the test log");
    assert_eq!(result.log[logged].detail, "answered: 7");
    assert_eq!(result.log[logged].at, ANSWER_DELAY);
}

/// P2, P7, §7: a reply the template does not match enables no branch and is
/// not removed, and the timer's timeout branch fails the test case.
#[test]
fn a_test_component_that_is_answered_with_a_reply_its_template_does_not_match_fails_when_its_timer_fires(
) {
    let (result, _) = run_asker(None, 3);
    assert_eq!(result.verdict, Verdict::Fail, "{:?}", result.log);
    assert_eq!(
        result.ended_at, SECOND,
        "time advances to the timer's instant"
    );
    assert!(
        first_record(&result, "mtc", LogKind::Receive).is_none(),
        "a reply that does not match is never taken"
    );
    assert!(first_record(&result, "mtc", LogKind::Timeout).is_some());
}

/// P4: the content reads the message the branch took: a reply of success for
/// another job matches the template, which takes any job, and the content's
/// comparison of the job it read fails the test case.
#[test]
fn a_test_component_reads_the_reply_it_took_as_the_events_data() {
    let (result, _) = run_asker(Some(8), 0);
    assert_eq!(result.verdict, Verdict::Fail, "{:?}", result.log);
    assert_eq!(result.ended_at, ANSWER_DELAY);
    assert!(first_record(&result, "mtc", LogKind::Receive).is_some());
}

/// P9, P6, P2: the MTC creates a PTC connected to it, which asks it for job 42
/// as it starts; the MTC answers with the job it read, the PTC passes on the
/// reply and terminates, and the MTC's done branch passes on the next
/// snapshot. Leaving the invoking state kills the PTC, before the done
/// branch's content runs.
#[test]
fn a_test_component_creates_a_ptc_exchanges_a_message_with_it_and_reads_its_done() {
    let configuration = Configuration {
        mtc_type: "coordinator".to_string(),
        ptc_types: [("answerer".to_string(), "answerer".to_string())].into(),
        ports_of_type: [
            ("coordinator".to_string(), strings(&["peer"])),
            ("answerer".to_string(), strings(&["peer"])),
        ]
        .into(),
        connections: vec![(
            Endpoint::new("mtc", "peer"),
            Endpoint::new("answerer", "peer"),
        )],
        ..Configuration::default()
    };
    let result = run(configuration, Box::new(Silent));

    assert_eq!(result.verdict, Verdict::Pass, "{:?}", result.log);
    assert_eq!(
        result.ended_at, 0,
        "a branch fired in every round, so time never moved"
    );
    let record = |component: &str, kind: LogKind| {
        first_record(&result, component, kind)
            .unwrap_or_else(|| panic!("{component} logs {kind:?}: {:?}", result.log))
    };
    let created = record("answerer", LogKind::Create);
    let asked = record("answerer", LogKind::Send);
    let answered = record("mtc", LogKind::Send);
    let done = record("answerer", LogKind::Done);
    let killed = record("answerer", LogKind::Kill);
    let passed = record("mtc", LogKind::Verdict);
    assert!(
        created < asked,
        "the PTC is created when the macrostep that entered its state ends"
    );
    // P8: a `<log>` that writes only a label is recorded, by the PTC.
    let noted = record("answerer", LogKind::Log);
    assert_eq!(result.log[noted].detail, "asked");
    assert!(
        asked < noted,
        "the log runs in its place, after the send before it"
    );
    assert_eq!(result.log[asked].detail, "peer job.request");
    assert_eq!(result.log[answered].detail, "peer job.reply");
    assert!(answered < done);
    assert!(
        done < killed,
        "a done branch fires on a snapshot taken after the PTC terminated"
    );
    assert!(
        killed < passed,
        "the invoking state is exited before the done branch's content runs"
    );
}

/// P9, P14: leaving the state that invoked a PTC removes it while it still
/// runs, at that instant, and its local verdict still counts.
#[test]
fn a_test_component_kills_a_ptc_on_leaving_the_state_that_invoked_it() {
    let configuration = Configuration {
        mtc_type: "ptc_keeper".to_string(),
        ptc_types: [("idler".to_string(), "idler".to_string())].into(),
        ports_of_type: [("idler".to_string(), strings(&["peer"]))].into(),
        timers_of_type: [("ptc_keeper".to_string(), strings(&["patience", "linger"]))].into(),
        ..Configuration::default()
    };
    let result = run(configuration, Box::new(Silent));

    let killed = first_record(&result, "idler", LogKind::Kill)
        .unwrap_or_else(|| panic!("the idler is killed: {:?}", result.log));
    assert_eq!(
        result.log[killed].at,
        10 * MILLISECOND,
        "the PTC is removed when `patience` runs out"
    );
    assert!(first_record(&result, "idler", LogKind::Done).is_none());
    assert_eq!(result.ended_at, 15 * MILLISECOND);
    assert_eq!(
        result.verdict,
        Verdict::Inconclusive,
        "the removed PTC's inconclusive outweighs the MTC's pass"
    );
}
