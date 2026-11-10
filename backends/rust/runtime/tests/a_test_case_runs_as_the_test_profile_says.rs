// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The test profile's runtime (docs/SCE_TEST_PROFILE.md §12.2), held to the
// rules of §2 to §5 with components written here by hand, before any machine
// is generated: each one keeps its context from `start` on, reads its snapshot
// and calls its context as a generated machine does (§12.3). The runner of a
// suite is held to P11 over test cases made of the same components.
//
// Each module and case is the twin of the C++ gtest of the same name in
// tests/test_runtime/TestCaseRunTest.cpp (`TestCaseRunTest.AnAnsweredRequest...`
// is `test_case_run_test::an_answered_request_...`), with the same scenario,
// verdict, `NOW` and log. The cases of `the_shared_handle` hold what only the
// Rust runtime has to: its context is a handle, which is refused re-entrant
// use, and outlives no component it names. The case of `the_doors` reaches the
// run only through the two host processors a generated component registers on
// its engine (§12.3), written here with the bounds the engine registers them
// under.

// The test profile's runtime is the `std` profile's.
#![cfg(feature = "std")]

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use sce_rust_runtime::json::Value;
use sce_rust_runtime::saved_state::SavedValue;
use sce_rust_runtime::test::{
    canonical, enabled, no_inputs, receives, run_suite, worse, ComponentContext, ComponentMachine,
    Configuration, Duration, Endpoint, Execute, Features, Instant, LogKind, MachineFactory,
    Message, ModuleParameters, Port, Suite, SystemAdapter, TestCase, TestCaseResult, TestCaseRun,
    Timer, TimerStatus, Verdict,
};

const SECOND: Duration = 1_000_000;

/// What a hand-written component does when it starts, or on a snapshot.
type Step = Arc<dyn Fn(&ComponentContext, &mut Scripted) + Send + Sync>;

fn step(f: impl Fn(&ComponentContext, &mut Scripted) + Send + Sync + 'static) -> Step {
    Arc::new(f)
}

fn idle() -> Step {
    step(|_, _| {})
}

/// A component written by hand: what it does when it starts, and what it does
/// on each snapshot. Either may end it, as entering a top-level final does.
struct Scripted {
    on_start: Step,
    on_snapshot: Step,
    context: Option<ComponentContext>,
    ended: bool,
}

impl Scripted {
    fn end(&mut self) {
        self.ended = true;
    }
}

impl ComponentMachine for Scripted {
    fn start(&mut self, context: ComponentContext) {
        self.context = Some(context.clone());
        let on_start = Arc::clone(&self.on_start);
        on_start(&context, self);
    }

    fn deliver_snapshot(&mut self) {
        let context = self
            .context
            .clone()
            .expect("a snapshot is delivered only to a started machine");
        let on_snapshot = Arc::clone(&self.on_snapshot);
        on_snapshot(&context, self);
    }

    fn terminated(&self) -> bool {
        self.ended
    }
}

/// The machines of a test, by component type.
#[derive(Default)]
struct Machines {
    types: BTreeMap<String, (Step, Step)>,
}

impl Machines {
    fn with(mut self, component_type: &str, steps: (Step, Step)) -> Self {
        self.types.insert(component_type.to_string(), steps);
        self
    }

    fn factory(&self) -> MachineFactory {
        let types = self.types.clone();
        Arc::new(
            move |component_type: &str| -> Option<Box<dyn ComponentMachine>> {
                let (on_start, on_snapshot) = types.get(component_type)?;
                Some(Box::new(Scripted {
                    on_start: Arc::clone(on_start),
                    on_snapshot: Arc::clone(on_snapshot),
                    context: None,
                    ended: false,
                }))
            },
        )
    }
}

/// One input of a trace, or one send the adapter was handed.
struct Input {
    at: Instant,
    system_port: String,
    event: String,
}

fn input(at: Instant, system_port: &str, event: &str) -> Input {
    Input {
        at,
        system_port: system_port.to_string(),
        event: event.to_string(),
    }
}

/// A system adapter that replays a trace and records what was sent to it.
#[derive(Default)]
struct TraceAdapter {
    trace: Vec<Input>,
    next: usize,
    sent: Arc<Mutex<Vec<Input>>>,
}

impl TraceAdapter {
    fn replaying(trace: Vec<Input>) -> Self {
        TraceAdapter {
            trace,
            ..TraceAdapter::default()
        }
    }
}

impl SystemAdapter for TraceAdapter {
    fn send(&mut self, system_port: &str, message: &Message, now: Instant) {
        self.sent
            .lock()
            .unwrap()
            .push(input(now, system_port, &message.event));
    }

    fn next_input(&self) -> Option<Instant> {
        self.trace.get(self.next).map(|input| input.at)
    }

    fn take_inputs_due(&mut self, now: Instant) -> Vec<(String, Message)> {
        let mut due = Vec::new();
        while let Some(input) = self.trace.get(self.next).filter(|input| input.at <= now) {
            due.push((input.system_port.clone(), message(&input.event)));
            self.next += 1;
        }
        due
    }
}

/// The canonical form of a message whose record holds no field.
fn no_fields(_: &()) -> Value {
    Value::Object(Vec::new())
}

/// A message of the record `event` that holds no field: `{}` in the test log,
/// of the record type `empty`.
fn message(event: &str) -> Message {
    Message::new(event, (), "empty", no_fields)
}

fn logged(result: &TestCaseResult, component: &str, kind: LogKind) -> bool {
    result
        .log
        .iter()
        .any(|record| record.component == component && record.kind == kind)
}

fn ids(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| item.to_string()).collect()
}

/// The ids each component type declares, from `(type, id)` pairs.
fn by_type(entries: &[(&str, &str)]) -> BTreeMap<String, Vec<String>> {
    let mut declared: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (component_type, id) in entries {
        declared
            .entry(component_type.to_string())
            .or_default()
            .push(id.to_string());
    }
    declared
}

/// An MTC and one PTC joined by one connection, the MTC's `p` to the PTC's `q`.
fn mtc_and_echo() -> Configuration {
    Configuration {
        mtc_type: "Tester".to_string(),
        ptc_types: BTreeMap::from([("echo".to_string(), "Echo".to_string())]),
        ports_of_type: by_type(&[("Tester", "p"), ("Echo", "q")]),
        timers_of_type: by_type(&[("Tester", "reply")]),
        connections: vec![(Endpoint::new("mtc", "p"), Endpoint::new("echo", "q"))],
        mappings: Vec::new(),
    }
}

/// An MTC alone, of the type `component_type`, whose port `p` is mapped to the
/// system port `sut`.
fn mapped(component_type: &str) -> Configuration {
    Configuration {
        mtc_type: component_type.to_string(),
        ports_of_type: by_type(&[(component_type, "p")]),
        mappings: vec![(Endpoint::new("mtc", "p"), "sut".to_string())],
        ..Configuration::default()
    }
}

/// An MTC alone, of the type `component_type`, with no port and no timer.
fn lone(component_type: &str) -> Configuration {
    Configuration {
        mtc_type: component_type.to_string(),
        ..Configuration::default()
    }
}

/// The MTC creates the echo, sends to it, and waits for the answer within a
/// second: a receive branch passes, a timeout branch fails.
fn tester() -> (Step, Step) {
    (
        step(|context, _| {
            context.create("echo");
            context.send("p", message("ping"));
            context.start_timer("reply", SECOND);
        }),
        replies_in_time(),
    )
}

fn replies_in_time() -> Step {
    step(|context, this| {
        if context.snapshot_present("p") {
            assert_eq!(context.receive("p").event, "pong");
            context.stop_timer("reply");
            context.setverdict(Verdict::Pass, "");
            this.end();
        } else if context.timed_out("reply") {
            context.timeout("reply");
            context.setverdict(Verdict::Fail, "no reply in time");
            this.end();
        }
    })
}

fn run(
    configuration: Configuration,
    machines: &Machines,
    adapter: impl SystemAdapter + 'static,
    guard: Option<Duration>,
) -> TestCaseResult {
    run_with(
        configuration,
        machines,
        adapter,
        guard,
        ModuleParameters::default(),
    )
}

fn run_with(
    configuration: Configuration,
    machines: &Machines,
    adapter: impl SystemAdapter + 'static,
    guard: Option<Duration>,
    parameters: ModuleParameters,
) -> TestCaseResult {
    TestCaseRun::new(
        configuration,
        machines.factory(),
        Box::new(adapter),
        guard,
        parameters,
    )
    .run()
}

mod test_verdict {
    use super::*;

    #[test]
    fn only_ever_becomes_worse() {
        assert_eq!(worse(Verdict::Pass, Verdict::None), Verdict::Pass);
        assert_eq!(
            worse(Verdict::Pass, Verdict::Inconclusive),
            Verdict::Inconclusive
        );
        assert_eq!(worse(Verdict::Fail, Verdict::Pass), Verdict::Fail);
        assert_eq!(worse(Verdict::Fail, Verdict::Error), Verdict::Error);
    }
}

mod test_port {
    use super::*;

    #[test]
    fn a_snapshot_sees_only_what_was_queued_when_it_was_taken() {
        let mut port = Port::default();
        port.take_snapshot();
        port.enqueue(message("late"));
        assert!(
            port.snapshot().is_none(),
            "a message that arrives after the snapshot is not in it"
        );
        port.take_snapshot();
        assert!(port.snapshot().is_some());
        port.enqueue(message("behind"));
        assert_eq!(
            port.snapshot().unwrap().event,
            "late",
            "a later message goes behind the head"
        );
        assert_eq!(port.remove_head().event, "late");
        assert_eq!(port.len(), 1);
    }
}

mod test_timer {
    use super::*;

    #[test]
    fn times_out_at_its_instant_and_restarts_when_started_again() {
        let mut timer = Timer::default();
        timer.start(0, 10);
        timer.update(9);
        assert_eq!(timer.status(), TimerStatus::Running);
        timer.start(5, 10);
        assert_eq!(
            timer.expiry(),
            Some(15),
            "a running timer that is started again restarts"
        );
        timer.update(15);
        assert_eq!(timer.status(), TimerStatus::Timeout);
        timer.take_snapshot();
        assert!(timer.snapshot_timed_out());
        timer.timeout_taken();
        assert_eq!(timer.status(), TimerStatus::Idle);
        timer.stop();
        assert_eq!(
            timer.status(),
            TimerStatus::Idle,
            "stopping a timer that is not running does nothing"
        );
    }
}

mod test_case_run_test {
    use super::*;

    /// P6, P9, §3.2: a message sent in a round is taken in a later round, and
    /// the whole exchange happens at one instant, because a branch fired in each
    /// round.
    #[test]
    fn an_answered_request_passes_without_time_moving() {
        let machines = Machines::default().with("Tester", tester()).with(
            "Echo",
            (
                idle(),
                step(|context, _| {
                    if context.snapshot_present("q") {
                        assert_eq!(context.receive("q").event, "ping");
                        context.send("q", message("pong"));
                    }
                }),
            ),
        );
        let result = run(mtc_and_echo(), &machines, TraceAdapter::default(), None);
        assert_eq!(result.verdict, Verdict::Pass);
        assert_eq!(result.ended_at, 0);
        assert!(
            logged(&result, "echo", LogKind::Kill),
            "the MTC's end removes the PTC (P10)"
        );
    }

    /// P7, §3.3: with nothing to receive, time advances to the timer's instant.
    #[test]
    fn an_unanswered_request_fails_when_its_timer_expires() {
        let machines = Machines::default()
            .with("Tester", tester())
            .with("Echo", (idle(), idle()));
        let result = run(mtc_and_echo(), &machines, TraceAdapter::default(), None);
        assert_eq!(result.verdict, Verdict::Fail);
        assert_eq!(result.ended_at, SECOND);
    }

    /// P13: no branch can fire and no instant is pending, so the test case can
    /// never end; the runtime ends it with error.
    #[test]
    fn a_test_case_that_can_never_end_is_an_error() {
        let machines = Machines::default().with("Waiter", (idle(), idle()));
        let result = run(mapped("Waiter"), &machines, TraceAdapter::default(), None);
        assert_eq!(result.verdict, Verdict::Error);
        assert!(logged(&result, "mtc", LogKind::Error));
    }

    /// P11: the guard instant ends a test case that has not ended, with error.
    #[test]
    fn the_guard_ends_a_test_case_with_error() {
        let machines = Machines::default().with(
            "Waiter",
            (
                step(|context, _| context.setverdict(Verdict::Pass, "")),
                idle(),
            ),
        );
        let result = run(
            mapped("Waiter"),
            &machines,
            TraceAdapter::default(),
            Some(5 * SECOND),
        );
        assert_eq!(result.verdict, Verdict::Error);
        assert_eq!(result.ended_at, 5 * SECOND);
    }

    /// §2.2, §12.2: a mapped port's sends reach the adapter, and the trace's
    /// inputs reach the port at their instants.
    #[test]
    fn a_mapped_port_talks_to_the_system_adapter_on_the_logical_clock() {
        let configuration = Configuration {
            timers_of_type: by_type(&[("Tester", "reply")]),
            ..mapped("Tester")
        };
        let machines = Machines::default().with(
            "Tester",
            (
                step(|context, _| {
                    context.send("p", message("ping"));
                    context.start_timer("reply", SECOND);
                }),
                replies_in_time(),
            ),
        );
        let adapter = TraceAdapter::replaying(vec![input(500, "sut", "pong")]);
        let sent = Arc::clone(&adapter.sent);
        let result = run(configuration, &machines, adapter, None);
        assert_eq!(result.verdict, Verdict::Pass);
        assert_eq!(result.ended_at, 500);
        let sent = sent.lock().unwrap();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].system_port, "sut");
        assert_eq!(sent[0].event, "ping");
    }

    /// P9, P14: a removed PTC's verdict still counts in the test case verdict.
    #[test]
    fn a_removed_ptcs_verdict_still_counts() {
        let machines = Machines::default()
            .with(
                "Tester",
                (
                    step(|context, _| {
                        context.create("echo");
                        context.start_timer("reply", 10);
                    }),
                    step(|context, this| {
                        if context.timed_out("reply") {
                            context.timeout("reply");
                            context.kill("echo");
                            context.setverdict(Verdict::Pass, "");
                            this.end();
                        }
                    }),
                ),
            )
            .with(
                "Echo",
                (
                    step(|context, _| context.setverdict(Verdict::Fail, "")),
                    idle(),
                ),
            );
        let result = run(mtc_and_echo(), &machines, TraceAdapter::default(), None);
        assert_eq!(result.verdict, Verdict::Fail);
    }

    /// §3.2 step 4: components select one at a time in creation order, so when
    /// the MTC terminates in a round, a PTC that would have selected after it in
    /// that round does not.
    #[test]
    fn nothing_selects_after_the_mtc_terminates_in_the_round() {
        let mut configuration = mtc_and_echo();
        configuration
            .timers_of_type
            .insert("Echo".to_string(), ids(&["late"]));
        let machines = Machines::default()
            .with(
                "Tester",
                (
                    step(|context, _| {
                        context.create("echo");
                        context.start_timer("reply", 10);
                    }),
                    step(|context, this| {
                        if context.timed_out("reply") {
                            context.timeout("reply");
                            context.setverdict(Verdict::Pass, "");
                            this.end();
                        }
                    }),
                ),
            )
            .with(
                "Echo",
                (
                    step(|context, _| context.start_timer("late", 10)),
                    step(|context, _| {
                        if context.timed_out("late") {
                            context.timeout("late");
                            context.setverdict(Verdict::Fail, "");
                        }
                    }),
                ),
            );
        let result = run(configuration, &machines, TraceAdapter::default(), None);
        assert_eq!(result.verdict, Verdict::Pass);
        assert!(!logged(&result, "echo", LogKind::Timeout));
    }

    /// P2, P10: a PTC that terminates is in DONE from the next snapshot on, and
    /// a done branch removes nothing.
    #[test]
    fn a_done_branch_sees_a_ptc_that_terminated() {
        let machines = Machines::default()
            .with(
                "Tester",
                (
                    step(|context, _| context.create("echo")),
                    step(|context, this| {
                        if context.done("echo") {
                            context.done_taken("echo");
                            context.kill("echo");
                            this.end();
                        }
                    }),
                ),
            )
            .with(
                "Echo",
                (
                    step(|context, this| {
                        context.setverdict(Verdict::Pass, "");
                        this.end();
                    }),
                    idle(),
                ),
            );
        let result = run(mtc_and_echo(), &machines, TraceAdapter::default(), None);
        assert_eq!(result.verdict, Verdict::Pass);
        assert!(logged(&result, "echo", LogKind::Done));
    }

    /// The SUT's four messages, all at `NOW = 0`: a record of another type,
    /// another event, another value, and the one the template matches.
    #[derive(Default)]
    struct FourInputs {
        given: bool,
    }

    impl SystemAdapter for FourInputs {
        fn send(&mut self, _system_port: &str, _message: &Message, _now: Instant) {}

        fn next_input(&self) -> Option<Instant> {
            None
        }

        fn take_inputs_due(&mut self, now: Instant) -> Vec<(String, Message)> {
            if now != 0 || self.given {
                return Vec::new();
            }
            self.given = true;
            let written = |written: &String| canonical::text(written);
            let number = |n: &i32| canonical::integer(*n);
            vec![
                (
                    "sut".to_string(),
                    Message::new("number", "7".to_string(), "text", written),
                ),
                (
                    "sut".to_string(),
                    Message::new("other", 7_i32, "number", number),
                ),
                (
                    "sut".to_string(),
                    Message::new("number", 6_i32, "number", number),
                ),
                (
                    "sut".to_string(),
                    Message::new("number", 7_i32, "number", number),
                ),
            ]
        }
    }

    /// P2, §12.3: a receive branch with a template matches only a message of
    /// its record, under its record's event, whose value its `match` holds; the
    /// guard reads the snapshot and removes nothing.
    #[test]
    fn a_receive_with_a_template_matches_only_its_records_value() {
        let seen = Arc::new(Mutex::new(Vec::<String>::new()));
        let saw = Arc::clone(&seen);
        let machines = Machines::default().with(
            "Reader",
            (
                idle(),
                step(move |context, this| {
                    let is_seven = |value: &i32| *value == 7;
                    if receives::<i32>(context, "p", "number", is_seven) {
                        saw.lock().unwrap().push("seven".to_string());
                        assert!(
                            receives::<i32>(context, "p", "number", is_seven),
                            "a guard that matched removed the message"
                        );
                        context.receive("p");
                        this.end();
                    } else if context.snapshot_present("p") {
                        let event = context
                            .with_snapshot("p", |head| head.map(|message| message.event.clone()));
                        saw.lock().unwrap().extend(event);
                        context.receive("p");
                    }
                }),
            ),
        );
        let result = run(mapped("Reader"), &machines, FourInputs::default(), None);
        assert_eq!(
            *seen.lock().unwrap(),
            ids(&["number", "other", "number", "seven"]),
            "a value of another type, another event and another value do not match"
        );
        assert_eq!(result.verdict, Verdict::None);
    }

    /// §11.5, §11.8: a component reads the module parameters of its run into
    /// its variables before it starts, as a generated machine reads each through
    /// the reader of the type it declares.
    pub(super) fn reads_job(job: &Arc<Mutex<Option<u32>>>) -> (Step, Step) {
        let job = Arc::clone(job);
        (
            step(move |context, this| {
                let read = context.read_module_parameters(|given| {
                    let value = u32::from_saved(given.of("job")?, "job")?;
                    *job.lock().unwrap() = Some(value);
                    Ok(())
                });
                if !read {
                    return;
                }
                context.setverdict(Verdict::Pass, "");
                this.end();
            }),
            idle(),
        )
    }

    /// §11.8: the value the run is given is the one the component reads, and
    /// the component starts on it.
    #[test]
    fn a_component_reads_the_module_parameter_its_run_is_given() {
        let job = Arc::new(Mutex::new(None));
        let machines = Machines::default().with("Reader", reads_job(&job));
        let mut parameters = ModuleParameters::default();
        parameters.set("job", 7_u32.to_saved());
        let result = run_with(
            lone("Reader"),
            &machines,
            TraceAdapter::default(),
            None,
            parameters,
        );
        assert_eq!(*job.lock().unwrap(), Some(7));
        assert_eq!(result.verdict, Verdict::Pass);
    }

    /// P13: a module parameter the run has no value for is a dynamic error,
    /// named in the log, and the component goes no further than the read.
    #[test]
    fn a_module_parameter_the_run_is_not_given_is_an_error() {
        let job = Arc::new(Mutex::new(None));
        let machines = Machines::default().with("Reader", reads_job(&job));
        let result = run(lone("Reader"), &machines, TraceAdapter::default(), None);
        assert_eq!(result.verdict, Verdict::Error);
        assert_eq!(*job.lock().unwrap(), None);
        let last = result.log.last().expect("the run logged its error");
        assert_eq!(last.kind, LogKind::Error);
        assert!(last.detail.contains("'job'"), "{}", last.detail);
    }

    /// P13: a value that is not of the type the component declares is refused
    /// as a restored value of another type is, and not converted.
    #[test]
    fn a_module_parameter_of_another_type_is_an_error() {
        let job = Arc::new(Mutex::new(None));
        let machines = Machines::default().with("Reader", reads_job(&job));
        let mut parameters = ModuleParameters::default();
        parameters.set("job", "seven".to_string().to_saved());
        let result = run_with(
            lone("Reader"),
            &machines,
            TraceAdapter::default(),
            None,
            parameters,
        );
        assert_eq!(result.verdict, Verdict::Error);
        assert_eq!(*job.lock().unwrap(), None);
        assert!(logged(&result, "mtc", LogKind::Error));
    }

    /// P13: a component type the factory of machines makes no machine for ends
    /// the run with error where a component of it is created.
    #[test]
    fn a_component_type_with_no_machine_is_an_error() {
        let none: MachineFactory =
            Arc::new(|_: &str| -> Option<Box<dyn ComponentMachine>> { None });
        let result = TestCaseRun::new(
            lone("Ghost"),
            none,
            Box::new(TraceAdapter::default()),
            None,
            ModuleParameters::default(),
        )
        .run();
        assert_eq!(result.verdict, Verdict::Error);
        assert!(logged(&result, "mtc", LogKind::Error));
        assert!(
            !logged(&result, "mtc", LogKind::Start),
            "no component of the type was created"
        );
    }

    /// P6, P13: a send on a port whose peer is not present is a dynamic error.
    #[test]
    fn a_send_to_an_absent_peer_is_an_error() {
        let machines = Machines::default().with(
            "Tester",
            (
                step(|context, _| context.send("p", message("ping"))),
                idle(),
            ),
        );
        let result = run(mtc_and_echo(), &machines, TraceAdapter::default(), None);
        assert_eq!(result.verdict, Verdict::Error);
        assert_eq!(result.ended_at, 0);
    }
}

mod test_suite_run {
    use super::test_case_run_test::reads_job;
    use super::*;

    /// A test case whose MTC, of type `component_type`, sets `verdict` as it
    /// starts and ends there.
    fn ends_with(name: &str, component_type: &str, verdict: Verdict) -> TestCase {
        let machines = Machines::default().with(
            component_type,
            (
                step(move |context, this| {
                    context.setverdict(verdict, "");
                    this.end();
                }),
                idle(),
            ),
        );
        TestCase {
            name: name.to_string(),
            configuration: lone(component_type),
            machines: machines.factory(),
            module_parameters: Vec::new(),
            inputs: no_inputs,
        }
    }

    /// The adapters of a suite's run, each a fresh TraceAdapter, recording the
    /// test case each was built for.
    fn tracing(built: &mut Vec<String>) -> impl FnMut(&str) -> Option<Box<dyn SystemAdapter>> + '_ {
        move |test_case: &str| -> Option<Box<dyn SystemAdapter>> {
            built.push(test_case.to_string());
            Some(Box::new(TraceAdapter::default()))
        }
    }

    fn selects_x(world: &Features) -> bool {
        enabled(world, "x")
    }

    fn selects_y(world: &Features) -> bool {
        enabled(world, "y")
    }

    fn world(features: &[&str]) -> Features {
        features.iter().map(|feature| feature.to_string()).collect()
    }

    fn execute(test_case: TestCase, guard: Option<Duration>) -> Execute {
        Execute {
            test_case,
            guard,
            selection: None,
        }
    }

    /// P11: the runner runs the test cases a world selects, in the suite's
    /// order, and skips the others; a test case with no selection runs in every
    /// world.
    #[test]
    fn runs_the_test_cases_a_world_selects_in_the_suites_order() {
        let suite = Suite {
            name: "ordered".to_string(),
            module_parameters: ModuleParameters::default(),
            executes: vec![
                execute(ends_with("first", "First", Verdict::Pass), None),
                Execute {
                    selection: Some(selects_x),
                    ..execute(ends_with("second", "Second", Verdict::Fail), None)
                },
                Execute {
                    selection: Some(selects_y),
                    ..execute(ends_with("third", "Third", Verdict::Inconclusive), None)
                },
            ],
        };
        let mut built = Vec::new();
        let results = run_suite(
            &suite,
            &world(&["y"]),
            tracing(&mut built),
            &ModuleParameters::default(),
        )
        .expect("no override is given");
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].test_case, "first");
        assert_eq!(results[0].result.verdict, Verdict::Pass);
        assert_eq!(results[1].test_case, "third");
        assert_eq!(results[1].result.verdict, Verdict::Inconclusive);
        assert_eq!(
            built,
            ids(&["first", "third"]),
            "an adapter is built for each case run"
        );
    }

    /// P11: each test case is a fresh run from NOW = 0 under its own guard.
    #[test]
    fn each_test_case_is_a_fresh_run_under_its_own_guard() {
        let waiting = Machines::default().with("Waiter", (idle(), idle()));
        let stuck = TestCase {
            name: "stuck".to_string(),
            configuration: lone("Waiter"),
            machines: waiting.factory(),
            module_parameters: Vec::new(),
            inputs: no_inputs,
        };
        let suite = Suite {
            name: "guarded".to_string(),
            module_parameters: ModuleParameters::default(),
            executes: vec![
                execute(stuck, Some(5 * SECOND)),
                execute(ends_with("quick", "Quick", Verdict::Pass), None),
            ],
        };
        let mut built = Vec::new();
        let results = run_suite(
            &suite,
            &BTreeSet::new(),
            tracing(&mut built),
            &ModuleParameters::default(),
        )
        .expect("no override is given");
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].result.verdict, Verdict::Error);
        assert_eq!(
            results[0].result.ended_at,
            5 * SECOND,
            "the guard ends the first"
        );
        assert_eq!(results[1].result.verdict, Verdict::Pass);
        assert_eq!(
            results[1].result.ended_at, 0,
            "the second starts again at NOW = 0"
        );
    }

    /// -1 §8.2.1: a value the caller gives a module parameter replaces the
    /// suite's default, and a parameter the suite does not declare cannot be
    /// given one.
    #[test]
    fn an_override_replaces_the_default_of_a_module_parameter_the_suite_declares() {
        let job = Arc::new(Mutex::new(None));
        let machines = Machines::default().with("Reader", reads_job(&job));
        let mut suite = Suite {
            name: "parameters".to_string(),
            ..Suite::default()
        };
        suite.module_parameters.set("job", 7_u32.to_saved());
        suite.executes.push(execute(
            TestCase {
                name: "reads".to_string(),
                configuration: lone("Reader"),
                machines: machines.factory(),
                module_parameters: ids(&["job"]),
                inputs: no_inputs,
            },
            None,
        ));
        let mut built = Vec::new();
        let results = run_suite(
            &suite,
            &BTreeSet::new(),
            tracing(&mut built),
            &ModuleParameters::default(),
        )
        .expect("no override is given");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].result.verdict, Verdict::Pass);
        assert_eq!(*job.lock().unwrap(), Some(7), "the default");

        let mut overrides = ModuleParameters::default();
        overrides.set("job", 9_u32.to_saved());
        let results = run_suite(&suite, &BTreeSet::new(), tracing(&mut built), &overrides)
            .expect("the suite declares the override's parameter");
        assert_eq!(results.len(), 1);
        assert_eq!(*job.lock().unwrap(), Some(9), "the override");

        let mut stray = ModuleParameters::default();
        stray.set("limit", 1_u32.to_saved());
        let refused = run_suite(&suite, &BTreeSet::new(), tracing(&mut built), &stray)
            .expect_err("an override of an undeclared parameter is refused");
        assert!(refused.reason().contains("'limit'"), "{refused}");
    }

    /// P13: a test case whose components read a module parameter the run has no
    /// value for ends with error before anything of it runs, its adapter
    /// included.
    #[test]
    fn a_test_case_that_reads_a_module_parameter_with_no_value_is_an_error() {
        let mut reads = ends_with("reads", "Reader", Verdict::Pass);
        reads.module_parameters = ids(&["job"]);
        let suite = Suite {
            name: "undeclared".to_string(),
            module_parameters: ModuleParameters::default(),
            executes: vec![execute(reads, None)],
        };
        let mut built = Vec::new();
        let results = run_suite(
            &suite,
            &BTreeSet::new(),
            tracing(&mut built),
            &ModuleParameters::default(),
        )
        .expect("no override is given");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].result.verdict, Verdict::Error);
        assert_eq!(results[0].result.log.len(), 1);
        assert!(results[0].result.log[0].detail.contains("'job'"));
        assert!(
            built.is_empty(),
            "no adapter is built for a case that cannot start"
        );
    }

    /// P13: a test case the caller gives no system adapter ends with error.
    #[test]
    fn a_test_case_with_no_system_adapter_is_an_error() {
        let suite = Suite {
            name: "unadapted".to_string(),
            module_parameters: ModuleParameters::default(),
            executes: vec![execute(ends_with("only", "Only", Verdict::Pass), None)],
        };
        let results = run_suite(
            &suite,
            &BTreeSet::new(),
            |_: &str| -> Option<Box<dyn SystemAdapter>> { None },
            &ModuleParameters::default(),
        )
        .expect("no override is given");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].result.verdict, Verdict::Error);
    }
}

mod the_shared_handle {
    use super::*;

    /// A snapshot reader runs while the run's state is held, so a call it makes
    /// into its context is re-entrant: it panics, naming the component, and
    /// does not deadlock.
    #[test]
    #[should_panic(expected = "re-entrant access to the test run from mtc")]
    fn a_call_into_the_context_from_a_snapshot_reader_panics() {
        let machines = Machines::default().with(
            "Reader",
            (
                step(|context, _| {
                    context.with_snapshot("p", |_| context.log("inside the reader"));
                }),
                idle(),
            ),
        );
        run(mapped("Reader"), &machines, TraceAdapter::default(), None);
    }

    /// P9, P13: a component whose machine is running is not removed under it.
    /// The PTC's start kills the MTC, whose own start is still running; that is
    /// a dynamic error, and the MTC stays.
    #[test]
    fn a_kill_of_a_component_whose_machine_runs_is_an_error() {
        let machines = Machines::default()
            .with(
                "Tester",
                (
                    step(|context, _| {
                        context.create("echo");
                        context.setverdict(Verdict::Pass, "");
                    }),
                    idle(),
                ),
            )
            .with("Echo", (step(|context, _| context.kill("mtc")), idle()));
        let result = run(mtc_and_echo(), &machines, TraceAdapter::default(), None);
        assert_eq!(result.verdict, Verdict::Error);
        assert!(logged(&result, "echo", LogKind::Error));
        assert!(
            !logged(&result, "mtc", LogKind::Kill),
            "the MTC was not removed"
        );
        assert!(
            logged(&result, "mtc", LogKind::Verdict),
            "the MTC's start ran on after the refused kill"
        );
    }

    /// A context names one creation of its component: once the PTC is removed,
    /// a call through a context it handed out panics instead of reaching
    /// another component.
    #[test]
    #[should_panic(expected = "the component echo was removed, and its context outlived it")]
    fn a_context_does_not_outlive_its_component() {
        let kept: Arc<Mutex<Option<ComponentContext>>> = Arc::new(Mutex::new(None));
        let keeps = Arc::clone(&kept);
        let machines = Machines::default()
            .with(
                "Tester",
                (
                    step(move |context, _| {
                        context.create("echo");
                        context.kill("echo");
                        let echo = kept
                            .lock()
                            .unwrap()
                            .clone()
                            .expect("the echo kept its context");
                        echo.setverdict(Verdict::Fail, "after its removal");
                    }),
                    idle(),
                ),
            )
            .with(
                "Echo",
                (
                    step(move |context, _| *keeps.lock().unwrap() = Some(context.clone())),
                    idle(),
                ),
            );
        run(mtc_and_echo(), &machines, TraceAdapter::default(), None);
    }
}

mod the_doors {
    use super::*;
    use sce_rust_runtime::test::message_value;
    use sce_rust_runtime::{
        HostInvokeCancel, HostInvokeEvent, HostInvokeRequest, HostInvokeResponse, HostSendRequest,
        HostSendResponse,
    };

    /// The test-port processor, under the bounds `Engine::register_event_processor`
    /// takes a handler with.
    type SendProcessor = Box<dyn FnMut(HostSendRequest) -> Vec<HostSendResponse> + Send + 'static>;

    /// The test-component invoker, under the bounds `Engine::register_invoker`
    /// takes a handler with.
    type Invoker = Box<dyn FnMut(HostInvokeEvent) -> Option<HostInvokeResponse> + Send + 'static>;

    /// A send (P6): the request carries the template's value, which the run
    /// appends to the connected port, or hands the system adapter.
    fn test_port_processor(context: ComponentContext) -> SendProcessor {
        Box::new(move |request: HostSendRequest| {
            let value = request
                .value
                .expect("a test-port send carries its template's value");
            context.send(
                &request.target,
                Message {
                    event: request.event_name,
                    value,
                },
            );
            Vec::new()
        })
    }

    /// A PTC (P9): the start creates it, the cancel removes it.
    fn test_component_invoker(context: ComponentContext) -> Invoker {
        Box::new(move |event: HostInvokeEvent| {
            match event {
                HostInvokeEvent::Start(request) => context.create(&request.invoke_id),
                HostInvokeEvent::Cancel(cancel) => context.kill(&cancel.invoke_id),
            }
            None
        })
    }

    /// The record a template of the port type reads.
    struct Ping {
        n: u32,
    }

    /// A `Ping` in the canonical form of the test log, as the unit of a
    /// template over its record writes it.
    fn ping_canonical(ping: &Ping) -> Value {
        canonical::record([("n", Some(canonical::integer(ping.n)))])
    }

    /// An MTC wired as a generated one is: its start registers the two
    /// processors, each holding a clone of its context, and it reaches the doors
    /// of the run only through them. Entering its state invokes `echo` and
    /// sends it a `Ping`; a timeout leaves the state, which cancels the invoke.
    #[derive(Default)]
    struct Tester {
        invoker: Option<Invoker>,
        context: Option<ComponentContext>,
        ended: bool,
    }

    impl ComponentMachine for Tester {
        fn start(&mut self, context: ComponentContext) {
            let mut send = test_port_processor(context.clone());
            let mut invoker = test_component_invoker(context.clone());
            invoker(HostInvokeEvent::Start(HostInvokeRequest {
                invoke_id: "echo".to_string(),
                ..HostInvokeRequest::default()
            }));
            send(HostSendRequest {
                event_name: "ping".to_string(),
                target: "p".to_string(),
                value: Some(message_value(Ping { n: 3 }, "ping", ping_canonical)),
                ..HostSendRequest::default()
            });
            context.start_timer("reply", 10);
            self.invoker = Some(invoker);
            self.context = Some(context);
        }

        fn deliver_snapshot(&mut self) {
            let context = self
                .context
                .clone()
                .expect("a snapshot is delivered only to a started machine");
            if !context.timed_out("reply") {
                return;
            }
            context.timeout("reply");
            let invoker = self
                .invoker
                .as_mut()
                .expect("the start registered the invoker");
            invoker(HostInvokeEvent::Cancel(HostInvokeCancel {
                invoke_id: "echo".to_string(),
                ..HostInvokeCancel::default()
            }));
            context.setverdict(Verdict::Pass, "");
            self.ended = true;
        }

        fn terminated(&self) -> bool {
            self.ended
        }
    }

    fn position(result: &TestCaseResult, component: &str, kind: LogKind) -> usize {
        result
            .log
            .iter()
            .position(|record| record.component == component && record.kind == kind)
            .unwrap_or_else(|| panic!("no {kind:?} of {component} in {:?}", result.log))
    }

    /// §12.3: the PTC the invoker's start creates receives the value the
    /// processor's send carried, by its type, and the invoker's cancel removes
    /// it at the timeout, before the MTC's verdict.
    #[test]
    fn a_send_and_a_ptc_go_through_the_host_processors() {
        let echo = step(|context, _| {
            if receives::<Ping>(context, "q", "ping", |ping| ping.n == 3) {
                context.receive("q");
                context.setverdict(Verdict::Pass, "the value arrived");
            } else if context.snapshot_present("q") {
                context.receive("q");
                context.setverdict(Verdict::Fail, "another value arrived");
            }
        });
        let factory: MachineFactory = Arc::new(
            move |component_type: &str| -> Option<Box<dyn ComponentMachine>> {
                match component_type {
                    "Tester" => Some(Box::new(Tester::default())),
                    "Echo" => Some(Box::new(Scripted {
                        on_start: idle(),
                        on_snapshot: Arc::clone(&echo),
                        context: None,
                        ended: false,
                    })),
                    _ => None,
                }
            },
        );
        let result = TestCaseRun::new(
            mtc_and_echo(),
            factory,
            Box::new(TraceAdapter::default()),
            None,
            ModuleParameters::default(),
        )
        .run();
        assert_eq!(result.verdict, Verdict::Pass, "{:?}", result.log);
        let created = position(&result, "echo", LogKind::Create);
        let sent = position(&result, "mtc", LogKind::Send);
        let received = position(&result, "echo", LogKind::Receive);
        let killed = position(&result, "echo", LogKind::Kill);
        let verdict = position(&result, "mtc", LogKind::Verdict);
        assert!(created < sent && sent < received, "{:?}", result.log);
        assert_eq!(result.log[sent].detail, "p ping");
        assert_eq!(result.log[received].detail, "q ping");
        assert_eq!(result.log[killed].at, 10, "the cancel came at the timeout");
        assert!(killed < verdict, "the cancel removed the PTC, not the end");
        assert!(logged(&result, "echo", LogKind::Verdict));
    }
}

/// The test log (docs/SCE_TEST_PROFILE.md §12.5; docs/adr/0091, C3): each
/// record written where the semantics places its event, in the order the
/// rounds produce them, and the whole log in its one canonical text. The C++
/// runtime writes the same bytes of the same runs (`TheTestLog` in
/// `tests/test_runtime/TestCaseRunTest.cpp`).
mod the_test_log {
    use super::*;

    /// An MTC and its echo, joined `p` to `q`; the MTC has two timers.
    fn pinging() -> Configuration {
        Configuration {
            mtc_type: "Tester".to_string(),
            ptc_types: BTreeMap::from([("echo".to_string(), "Echo".to_string())]),
            ports_of_type: by_type(&[("Tester", "p"), ("Echo", "q")]),
            timers_of_type: by_type(&[("Tester", "t"), ("Tester", "s")]),
            connections: vec![(Endpoint::new("mtc", "p"), Endpoint::new("echo", "q"))],
            mappings: Vec::new(),
        }
    }

    /// The ping's value, a record of one number of the record type `ping`, as
    /// the unit of a template over it writes it.
    fn ping(n: u32) -> Message {
        Message::new("ping", n, "ping", |n| {
            canonical::record([("n", Some(canonical::integer(*n)))])
        })
    }

    /// The MTC creates the echo, pings it, starts two timers and logs; at the
    /// first timer's timeout it stops the second, and the first again, which
    /// no longer runs, and passes. The echo takes the ping, passes with a
    /// reason, and terminates.
    fn pinging_machines() -> Machines {
        Machines::default()
            .with(
                "Tester",
                (
                    step(|context, _| {
                        context.create("echo");
                        context.send("p", ping(3));
                        context.start_timer("t", 10);
                        context.start_timer("s", 20);
                        context.log("asked \"echo\"");
                    }),
                    step(|context, this| {
                        if context.timed_out("t") {
                            context.timeout("t");
                            context.stop_timer("s");
                            context.stop_timer("t");
                            context.setverdict(Verdict::Pass, "");
                            this.end();
                        }
                    }),
                ),
            )
            .with(
                "Echo",
                (
                    idle(),
                    step(|context, this| {
                        if context.snapshot_present("q") {
                            context.receive("q");
                            context.setverdict(Verdict::Pass, "pinged");
                            this.end();
                        }
                    }),
                ),
            )
    }

    fn suite_of(test_case: TestCase) -> Suite {
        Suite {
            name: "logged_suite".to_string(),
            module_parameters: ModuleParameters::default(),
            executes: vec![Execute {
                test_case,
                guard: None,
                selection: None,
            }],
        }
    }

    /// P4, P6, P7, P8, P9, P10, P12, P14: the whole log of a run through the
    /// suite's runner. The echo's work comes in the round after the MTC's
    /// start, at NOW = 0, the timeout at 10; the stop of a timer that no
    /// longer runs, and the removal of the terminated echo when the MTC ends,
    /// write no record.
    #[test]
    fn a_run_writes_each_record_where_its_event_happens() {
        let suite = suite_of(TestCase {
            name: "pings_the_echo".to_string(),
            configuration: pinging(),
            machines: pinging_machines().factory(),
            module_parameters: Vec::new(),
            inputs: no_inputs,
        });
        let results = run_suite(
            &suite,
            &Features::default(),
            |_: &str| -> Option<Box<dyn SystemAdapter>> { Some(Box::new(TraceAdapter::default())) },
            &ModuleParameters::default(),
        )
        .expect("no override is given");
        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0].canonical_log(&suite.name),
            concat!(
                "{\"records\":[",
                "{\"at\":\"0\",\"kind\":\"test-case-started\"},",
                "{\"at\":\"0\",\"component\":\"mtc\",\"kind\":\"component-created\",\"type\":\"Tester\"},",
                "{\"at\":\"0\",\"component\":\"echo\",\"kind\":\"component-created\",\"type\":\"Echo\"},",
                "{\"at\":\"0\",\"component\":\"mtc\",\"kind\":\"message-sent\",\"port\":\"p\",",
                "\"type\":\"ping\",\"value\":{\"n\":\"3\"}},",
                "{\"at\":\"0\",\"component\":\"mtc\",\"duration_us\":\"10\",\"kind\":\"timer-started\",",
                "\"timer\":\"t\"},",
                "{\"at\":\"0\",\"component\":\"mtc\",\"duration_us\":\"20\",\"kind\":\"timer-started\",",
                "\"timer\":\"s\"},",
                "{\"at\":\"0\",\"component\":\"mtc\",\"kind\":\"log\",\"text\":\"asked \\\"echo\\\"\"},",
                "{\"at\":\"0\",\"component\":\"echo\",\"kind\":\"message-received\",\"port\":\"q\",",
                "\"type\":\"ping\",\"value\":{\"n\":\"3\"}},",
                "{\"at\":\"0\",\"component\":\"echo\",\"kind\":\"verdict-set\",\"reason\":\"pinged\",",
                "\"verdict\":\"pass\"},",
                "{\"at\":\"0\",\"component\":\"echo\",\"kind\":\"component-done\",\"verdict\":\"pass\"},",
                "{\"at\":\"10\",\"component\":\"mtc\",\"kind\":\"timer-timed-out\",\"timer\":\"t\"},",
                "{\"at\":\"10\",\"component\":\"mtc\",\"kind\":\"timer-stopped\",\"timer\":\"s\"},",
                "{\"at\":\"10\",\"component\":\"mtc\",\"kind\":\"verdict-set\",\"verdict\":\"pass\"},",
                "{\"at\":\"10\",\"component\":\"mtc\",\"kind\":\"component-done\",\"verdict\":\"pass\"},",
                "{\"at\":\"10\",\"kind\":\"test-case-ended\",\"verdict\":\"pass\"}",
                "],\"schema\":\"sce-test-log.v1\",\"suite\":\"logged_suite\",",
                "\"test_case\":\"pings_the_echo\",\"verdict\":\"pass\"}",
            )
        );
    }

    /// P13: an error the runtime sets is a verdict without the runtime's own
    /// reason, which another runtime would not write alike.
    #[test]
    fn an_error_the_runtime_sets_is_logged_without_its_cause() {
        let machines = Machines::default().with("Waiter", (idle(), idle()));
        let result = run(lone("Waiter"), &machines, TraceAdapter::default(), Some(7));
        assert_eq!(result.verdict, Verdict::Error);
        assert_eq!(
            result.canonical_log("logged_suite", "waits"),
            concat!(
                "{\"records\":[",
                "{\"at\":\"0\",\"kind\":\"test-case-started\"},",
                "{\"at\":\"0\",\"component\":\"mtc\",\"kind\":\"component-created\",\"type\":\"Waiter\"},",
                "{\"at\":\"7\",\"component\":\"mtc\",\"kind\":\"verdict-set\",\"verdict\":\"error\"},",
                "{\"at\":\"7\",\"kind\":\"test-case-ended\",\"verdict\":\"error\"}",
                "],\"schema\":\"sce-test-log.v1\",\"suite\":\"logged_suite\",",
                "\"test_case\":\"waits\",\"verdict\":\"error\"}",
            )
        );
    }

    /// P13: a test case that cannot start, for want of an adapter, logs its
    /// start and its end alone, since no component was created.
    #[test]
    fn a_test_case_that_cannot_start_logs_its_start_and_end() {
        let suite = suite_of(TestCase {
            name: "unplayed".to_string(),
            configuration: pinging(),
            machines: pinging_machines().factory(),
            module_parameters: Vec::new(),
            inputs: no_inputs,
        });
        let results = run_suite(
            &suite,
            &Features::default(),
            |_: &str| -> Option<Box<dyn SystemAdapter>> { None },
            &ModuleParameters::default(),
        )
        .expect("no override is given");
        assert_eq!(
            results[0].canonical_log(&suite.name),
            concat!(
                "{\"records\":[",
                "{\"at\":\"0\",\"kind\":\"test-case-started\"},",
                "{\"at\":\"0\",\"kind\":\"test-case-ended\",\"verdict\":\"error\"}",
                "],\"schema\":\"sce-test-log.v1\",\"suite\":\"logged_suite\",",
                "\"test_case\":\"unplayed\",\"verdict\":\"error\"}",
            )
        );
    }
}
