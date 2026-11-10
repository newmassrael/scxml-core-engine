// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A recorded input trace replayed by the test profile's runtime
// (docs/SCE_TTCN3_LANE.md §10; docs/SCE_TEST_PROFILE.md §3.2, §3.3, §12.4):
// the trace is read from its canonical text, each value by the reader of its
// configuration, written here by hand as the unit of a test configuration
// generates it (`input`, over `sce_rust_runtime::test::from_canonical`); the
// player delivers each input at its instant, in the trace's order, and the run
// writes the one test log the trace and the document determine. A trace that
// does not fit the configuration ends the test case with `error` before it
// starts, naming why.

// The test profile's runtime is the `std` profile's.
#![cfg(feature = "std")]

use std::sync::Arc;

use sce_rust_runtime::json::Value;
use sce_rust_runtime::test::{
    canonical, from_canonical, run_suite_with_traces, CaseResult, ComponentContext,
    ComponentMachine, Configuration, Endpoint, Execute, Features, LogKind, MachineFactory, Message,
    ModuleParameters, Suite, TestCase, TestCaseRun, Trace, TraceAdapter, TraceRefusal, Verdict,
};

/// The deadline the asker waits for its last reply within.
const DEADLINE: i64 = 1000;

/// What a component does when it starts, or on a snapshot; true when it ends
/// it, as entering a top-level final does.
type Step = fn(&ComponentContext) -> bool;

/// A component written by hand, as the runtime's own tests write one.
struct Scripted {
    on_start: Step,
    on_snapshot: Step,
    context: Option<ComponentContext>,
    ended: bool,
}

impl ComponentMachine for Scripted {
    fn start(&mut self, context: ComponentContext) {
        self.ended = (self.on_start)(&context);
        self.context = Some(context);
    }

    fn deliver_snapshot(&mut self) {
        let context = self
            .context
            .clone()
            .expect("a snapshot is delivered only to a started machine");
        self.ended = (self.on_snapshot)(&context);
    }

    fn terminated(&self) -> bool {
        self.ended
    }
}

/// The request the asker sends, a record of no field.
fn request() -> Message {
    Message::new("request", (), "request", |_: &()| Value::Object(Vec::new()))
}

/// A reply, a record of one number, `n`, in the canonical form of the test log.
fn reply_canonical(n: &u32) -> Value {
    canonical::record([("n", Some(canonical::integer(*n)))])
}

/// The reader of the inputs of the asker's configuration, as the unit of a
/// test configuration generates it: the system port `sut` takes the record
/// `reply`, read from its canonical form.
fn read_input(system_port: &str, type_name: &str, value: &Value) -> Result<Message, TraceRefusal> {
    match (system_port, type_name) {
        ("sut", "reply") => {
            let record = from_canonical::record(value, &["n"], "reply")?;
            let n: u32 =
                from_canonical::integer(from_canonical::field(record, "n", "reply.n")?, "reply.n")?;
            Ok(Message::new("reply", n, "reply", reply_canonical))
        }
        _ => Err(TraceRefusal::not_an_input(system_port, type_name, &["sut"])),
    }
}

/// The asker sends its request and waits for the reply of 8 within its
/// deadline, taking each reply as it comes: the reply of 8 passes, the
/// deadline fails.
fn asks(context: &ComponentContext) -> bool {
    context.send("p", request());
    context.start_timer("deadline", DEADLINE);
    false
}

fn waits(context: &ComponentContext) -> bool {
    if context.snapshot_present("p") {
        let reply = context.receive("p");
        if reply.value_as::<u32>() != Some(&8) {
            return false;
        }
        context.stop_timer("deadline");
        context.setverdict(Verdict::Pass, "");
        return true;
    }
    if context.timed_out("deadline") {
        context.timeout("deadline");
        context.setverdict(Verdict::Fail, "no reply in time");
        return true;
    }
    false
}

fn machine_of(component_type: &str) -> Option<Box<dyn ComponentMachine>> {
    (component_type == "Asker").then(|| {
        Box::new(Scripted {
            on_start: asks,
            on_snapshot: waits,
            context: None,
            ended: false,
        }) as Box<dyn ComponentMachine>
    })
}

/// The asker alone, its port `p` mapped to the system port `sut`.
fn configuration() -> Configuration {
    Configuration {
        mtc_type: "Asker".to_string(),
        ports_of_type: [("Asker".to_string(), vec!["p".to_string()])].into(),
        timers_of_type: [("Asker".to_string(), vec!["deadline".to_string()])].into(),
        mappings: vec![(Endpoint::new("mtc", "p"), "sut".to_string())],
        ..Configuration::default()
    }
}

fn factory() -> MachineFactory {
    Arc::new(machine_of)
}

/// The suite of the one test case `asks_twice`.
fn suite() -> Suite {
    Suite {
        name: "traced_suite".to_string(),
        module_parameters: ModuleParameters::default(),
        executes: vec![Execute {
            test_case: TestCase {
                name: "asks_twice".to_string(),
                configuration: configuration(),
                machines: factory(),
                module_parameters: Vec::new(),
                inputs: read_input,
            },
            guard: None,
            selection: None,
        }],
    }
}

/// The trace of `asks_twice` whose inputs are `inputs`, each written as the
/// trace writes an input.
fn trace_of(inputs: &[&str]) -> String {
    format!(
        "{{\"inputs\":[{}],\"schema\":\"sce-test-trace.v1\",\"suite\":\"traced_suite\",\
         \"test_case\":\"asks_twice\"}}\n",
        inputs.join(",")
    )
}

/// An input of a reply of `n`, at `at`, on `port`.
fn reply_at(at: &str, port: &str, n: &str) -> String {
    format!(
        "{{\"at\":\"{at}\",\"port\":\"{port}\",\"type\":\"reply\",\"value\":{{\"n\":\"{n}\"}}}}"
    )
}

/// The suite's run against `trace`.
fn replayed(trace: &str) -> Vec<CaseResult> {
    run_suite_with_traces(
        &suite(),
        &Features::default(),
        |_: &str| Trace::parse(trace),
        &ModuleParameters::default(),
    )
    .expect("no override is given")
}

/// §3.2 step 2, §3.3, §12.4, §12.5: the whole log of a replay. Time advances
/// to the trace's instant, 300, before the deadline; both replies of that
/// instant are queued in the trace's order and taken in two rounds at 300, the
/// first of 7, which the asker takes and goes on waiting, then of 8, which
/// passes.
#[test]
fn a_replayed_trace_writes_the_log_its_inputs_determine() {
    let trace = trace_of(&[&reply_at("300", "sut", "7"), &reply_at("300", "sut", "8")]);
    let results = replayed(&trace);
    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0].canonical_log("traced_suite"),
        concat!(
            "{\"records\":[",
            "{\"at\":\"0\",\"kind\":\"test-case-started\"},",
            "{\"at\":\"0\",\"component\":\"mtc\",\"kind\":\"component-created\",\"type\":\"Asker\"},",
            "{\"at\":\"0\",\"component\":\"mtc\",\"kind\":\"message-sent\",\"port\":\"p\",",
            "\"type\":\"request\",\"value\":{}},",
            "{\"at\":\"0\",\"component\":\"mtc\",\"duration_us\":\"1000\",\"kind\":\"timer-started\",",
            "\"timer\":\"deadline\"},",
            "{\"at\":\"300\",\"component\":\"mtc\",\"kind\":\"message-received\",\"port\":\"p\",",
            "\"type\":\"reply\",\"value\":{\"n\":\"7\"}},",
            "{\"at\":\"300\",\"component\":\"mtc\",\"kind\":\"message-received\",\"port\":\"p\",",
            "\"type\":\"reply\",\"value\":{\"n\":\"8\"}},",
            "{\"at\":\"300\",\"component\":\"mtc\",\"kind\":\"timer-stopped\",\"timer\":\"deadline\"},",
            "{\"at\":\"300\",\"component\":\"mtc\",\"kind\":\"verdict-set\",\"verdict\":\"pass\"},",
            "{\"at\":\"300\",\"component\":\"mtc\",\"kind\":\"component-done\",\"verdict\":\"pass\"},",
            "{\"at\":\"300\",\"kind\":\"test-case-ended\",\"verdict\":\"pass\"}",
            "],\"schema\":\"sce-test-log.v1\",\"suite\":\"traced_suite\",",
            "\"test_case\":\"asks_twice\",\"verdict\":\"pass\"}",
        )
    );
}

/// §3.3: when no branch can fire the clock advances to the earlier of the
/// trace's next instant and the next timer expiry, so a reply due after the
/// deadline is never delivered; the player records what the asker sent to the
/// system port, at its instant.
#[test]
fn the_clock_advances_to_the_earlier_of_the_next_input_and_the_next_expiry() {
    let trace = Trace::parse(&trace_of(&[&reply_at("2000", "sut", "8")])).expect("a trace");
    let adapter = TraceAdapter::replaying(&trace, read_input).expect("the trace fits");
    let sent = adapter.sent();
    let result = TestCaseRun::new(
        configuration(),
        factory(),
        Box::new(adapter),
        None,
        ModuleParameters::default(),
    )
    .run();
    assert_eq!(result.verdict, Verdict::Fail, "{:?}", result.log);
    assert_eq!(result.ended_at, DEADLINE);
    assert!(result.log.iter().all(|line| line.kind != LogKind::Receive));
    let sent = sent.lock().expect("the record of what was sent");
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].at, 0);
    assert_eq!(sent[0].system_port, "sut");
    assert_eq!(sent[0].message.type_name(), "request");
}

/// P13, docs/SCE_TTCN3_LANE.md §10.2: a trace whose value does not belong to
/// its type is refused before the run, naming the input and the field, and the
/// test case ends with `error` with no component created.
#[test]
fn a_value_that_does_not_belong_to_its_type_is_refused_by_name() {
    let results = replayed(&trace_of(&[&reply_at("300", "sut", "seven")]));
    assert_eq!(results.len(), 1);
    let result = &results[0].result;
    assert_eq!(result.verdict, Verdict::Error);
    assert_eq!(result.log.len(), 1);
    assert_eq!(
        result.log[0].detail,
        "the trace is refused: input 0, at 300: reply.n: 'seven' is not the text of an \
         integer's decimal digits"
    );
    assert_eq!(
        results[0].canonical_log("traced_suite"),
        concat!(
            "{\"records\":[",
            "{\"at\":\"0\",\"kind\":\"test-case-started\"},",
            "{\"at\":\"0\",\"kind\":\"test-case-ended\",\"verdict\":\"error\"}",
            "],\"schema\":\"sce-test-log.v1\",\"suite\":\"traced_suite\",",
            "\"test_case\":\"asks_twice\",\"verdict\":\"error\"}",
        )
    );
}

/// docs/SCE_TTCN3_LANE.md §10.2: an input on a port the configuration does not
/// have, or of a type its port does not take, a field the record does not
/// declare, a trace of another test case and a trace not given each end the
/// test case with `error`, naming why.
#[test]
fn a_trace_that_does_not_fit_the_test_case_is_refused_by_name() {
    let refused = |trace: String| {
        let results = replayed(&trace);
        assert_eq!(results[0].result.verdict, Verdict::Error, "{trace}");
        results[0].result.log[0].detail.clone()
    };
    assert!(refused(trace_of(&[&reply_at("0", "oracle", "8")]))
        .ends_with("'oracle' is not a system port of the test case's configuration"));
    assert!(refused(trace_of(&[
        &reply_at("0", "sut", "8").replace("reply", "request")
    ]))
    .ends_with("'request' is not on the `in` list of the type of the system port 'sut'"));
    assert!(refused(trace_of(&[
        &reply_at("0", "sut", "8").replace("\"n\"", "\"m\"")
    ]))
    .ends_with("reply: 'm' is no field of its record"));
    assert_eq!(
        refused(trace_of(&[]).replace("asks_twice", "asks_once")),
        "the trace is of the test case 'asks_once' of the suite 'traced_suite'"
    );

    let unread = run_suite_with_traces(
        &suite(),
        &Features::default(),
        |test_case: &str| Err(TraceRefusal::new(format!("no trace of {test_case}"))),
        &ModuleParameters::default(),
    )
    .expect("no override is given");
    assert_eq!(unread[0].result.verdict, Verdict::Error);
    assert_eq!(
        unread[0].result.log[0].detail,
        "the trace is refused: no trace of asks_twice"
    );
}
