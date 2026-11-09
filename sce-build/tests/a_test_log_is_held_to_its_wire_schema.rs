// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The test log's wire schema (`schemas/sce-test-log.v1.schema.json`;
//! docs/SCE_TEST_PROFILE.md §12.5; docs/adr/0091, C3 and answer 8), held to
//! the logs its producer writes: a run of the Rust runtime, written here by
//! hand, that reaches every kind of record the schema names. The negative half
//! changes one member of a log the runtime wrote at a time, and the schema
//! refuses each.
//!
//! What each record holds, and the whole canonical text of a run, are held by
//! the runtime's own tests (`backends/rust/runtime/tests/
//! a_test_case_runs_as_the_test_profile_says.rs`, module `the_test_log`).

use std::collections::BTreeMap;
use std::sync::Arc;

use sce_rust_runtime::test::{
    canonical, ComponentContext, ComponentMachine, Configuration, Endpoint, Instant,
    MachineFactory, Message, ModuleParameters, SystemAdapter, TestCaseRun, Verdict,
    TEST_LOG_SCHEMA,
};

fn schema() -> serde_json::Value {
    serde_json::from_str(include_str!("../../schemas/sce-test-log.v1.schema.json"))
        .expect("the test log's schema is JSON")
}

fn violations(schema: &serde_json::Value, instance: &serde_json::Value) -> Vec<String> {
    let validator = jsonschema::JSONSchema::options()
        .with_draft(jsonschema::Draft::Draft7)
        .compile(schema)
        .expect("the schema compiles");
    let outcome = validator.validate(instance);
    match outcome {
        Ok(()) => Vec::new(),
        Err(errors) => errors.map(|e| e.to_string()).collect(),
    }
}

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

/// A ping, a record of one number and one text, as the unit of a template over
/// its record writes it.
struct Ping {
    n: u64,
    note: String,
}

fn ping_canonical(ping: &Ping) -> sce_rust_runtime::json::Value {
    canonical::record([
        ("n", Some(canonical::integer(ping.n))),
        ("note", Some(canonical::text(&ping.note))),
    ])
}

/// The MTC creates the echo, pings it, starts two timers and logs; at the
/// first timer's timeout it stops the second and passes.
fn tester_starts(context: &ComponentContext) -> bool {
    context.create("echo");
    context.send(
        "p",
        Message::new(
            "ping",
            Ping {
                n: u64::MAX,
                note: "a \"quoted\" note".to_string(),
            },
            "ping",
            ping_canonical,
        ),
    );
    context.start_timer("t", 10);
    context.start_timer("s", 20);
    context.log("asked");
    false
}

fn tester_waits(context: &ComponentContext) -> bool {
    if !context.timed_out("t") {
        return false;
    }
    context.timeout("t");
    context.stop_timer("s");
    context.setverdict(Verdict::Pass, "");
    true
}

/// The echo takes the ping and passes with a reason.
fn echo_waits(context: &ComponentContext) -> bool {
    if !context.snapshot_present("q") {
        return false;
    }
    context.receive("q");
    context.setverdict(Verdict::Inconclusive, "pinged");
    true
}

fn starts_idle(_: &ComponentContext) -> bool {
    false
}

fn machine_of(component_type: &str) -> Option<Box<dyn ComponentMachine>> {
    let (on_start, on_snapshot): (Step, Step) = match component_type {
        "Tester" => (tester_starts, tester_waits),
        "Echo" => (starts_idle, echo_waits),
        _ => return None,
    };
    Some(Box::new(Scripted {
        on_start,
        on_snapshot,
        context: None,
        ended: false,
    }))
}

/// A system under test nothing is mapped to.
struct Silent;

impl SystemAdapter for Silent {
    fn send(&mut self, _system_port: &str, _message: &Message, _now: Instant) {}

    fn next_input(&self) -> Option<Instant> {
        None
    }

    fn take_inputs_due(&mut self, _now: Instant) -> Vec<(String, Message)> {
        Vec::new()
    }
}

fn configuration() -> Configuration {
    let ids =
        |items: &[&str]| -> Vec<String> { items.iter().map(|item| item.to_string()).collect() };
    Configuration {
        mtc_type: "Tester".to_string(),
        ptc_types: BTreeMap::from([("echo".to_string(), "Echo".to_string())]),
        ports_of_type: BTreeMap::from([
            ("Tester".to_string(), ids(&["p"])),
            ("Echo".to_string(), ids(&["q"])),
        ]),
        timers_of_type: BTreeMap::from([("Tester".to_string(), ids(&["t", "s"]))]),
        connections: vec![(Endpoint::new("mtc", "p"), Endpoint::new("echo", "q"))],
        mappings: Vec::new(),
    }
}

/// The log of the run, as the runtime writes it.
fn written(guard: Option<i64>) -> String {
    let factory: MachineFactory = Arc::new(machine_of);
    TestCaseRun::new(
        configuration(),
        factory,
        Box::new(Silent),
        guard,
        ModuleParameters::default(),
    )
    .run()
    .canonical_log("held_suite", "pings_the_echo")
}

fn parsed(log: &str) -> serde_json::Value {
    serde_json::from_str(log).expect("a test log is JSON")
}

/// The kind of every record of the schema, read from the schema itself.
fn schema_kinds(schema: &serde_json::Value) -> Vec<String> {
    schema["definitions"]["record"]["oneOf"]
        .as_array()
        .expect("a record is one of its kinds")
        .iter()
        .map(|kind| {
            let name = kind["$ref"]
                .as_str()
                .and_then(|reference| reference.strip_prefix("#/definitions/"))
                .expect("each kind is a definition");
            schema["definitions"][name]["properties"]["kind"]["const"]
                .as_str()
                .expect("each kind names itself")
                .to_string()
        })
        .collect()
}

/// Every log the runtime writes is one the schema states: a run that reaches
/// every kind of record the schema names, so the sweep cannot narrow unseen,
/// and a run the guard ends with the runtime's `error`.
#[test]
fn every_log_the_runtime_writes_validates_against_the_wire_schema() {
    let schema = schema();
    let log = parsed(&written(None));
    assert_eq!(violations(&schema, &log), Vec::<String>::new(), "{log}");
    assert_eq!(log["schema"], TEST_LOG_SCHEMA);
    assert_eq!(
        log["verdict"], "inconc",
        "the echo's inconclusive outweighs the pass"
    );
    let kinds: Vec<&str> = log["records"]
        .as_array()
        .expect("records")
        .iter()
        .map(|record| record["kind"].as_str().expect("a kind"))
        .collect();
    for kind in schema_kinds(&schema) {
        assert!(kinds.contains(&kind.as_str()), "no {kind} in {kinds:?}");
    }
    for record in log["records"].as_array().expect("records") {
        if record["kind"] == "message-sent" || record["kind"] == "message-received" {
            assert_eq!(
                record["type"], "ping",
                "a message carries its type: {record}"
            );
        }
    }

    let ended = parsed(&written(Some(5)));
    assert_eq!(violations(&schema, &ended), Vec::<String>::new(), "{ended}");
    assert_eq!(ended["verdict"], "error");
}

/// An absent element of a list of `optional<T>` is `null` in its place, which
/// the schema admits as an element of an array, at any depth of the value: the
/// array names an element by its position, so it cannot be left out as an
/// absent field of a record is.
#[test]
fn the_test_log_schema_admits_an_absent_element_of_a_list_as_null() {
    let schema = schema();
    let mut log = parsed(&written(None));
    let sent = log["records"]
        .as_array()
        .expect("records")
        .iter()
        .position(|record| record["kind"] == "message-sent")
        .expect("a message-sent record");
    log["records"][sent]["value"]["note"] = serde_json::json!(["1", null]);
    assert_eq!(violations(&schema, &log), Vec::<String>::new(), "{log}");
    log["records"][sent]["value"]["note"] = serde_json::json!([{ "tags": ["1", null] }]);
    assert_eq!(violations(&schema, &log), Vec::<String>::new(), "{log}");
}

/// The schema refuses what the runtime does not write: starting from a log it
/// first shows valid, each change of one member is refused.
#[test]
fn the_test_log_schema_rejects_what_the_runtime_does_not_write() {
    let schema = schema();
    let valid = parsed(&written(None));
    assert_eq!(violations(&schema, &valid), Vec::<String>::new());
    let records = valid["records"].as_array().expect("records");
    let position = |kind: &str| {
        records
            .iter()
            .position(|record| record["kind"] == kind)
            .unwrap_or_else(|| panic!("no {kind} record"))
    };
    let sent = position("message-sent");
    let received = position("message-received");
    let started = position("timer-started");
    let verdict = position("verdict-set");
    let ended = position("test-case-ended");
    let changed = |change: &dyn Fn(&mut serde_json::Value)| {
        let mut instance = valid.clone();
        change(&mut instance);
        instance
    };
    let changes: Vec<(&str, serde_json::Value)> = vec![
        (
            "another shape",
            changed(&|l| l["schema"] = serde_json::json!("sce-test-log.v2")),
        ),
        (
            "no suite",
            changed(&|l| {
                l.as_object_mut().expect("a log").remove("suite");
            }),
        ),
        (
            "a verdict spelled as the profile's document spells it",
            changed(&|l| l["verdict"] = serde_json::json!("inconclusive")),
        ),
        (
            "an instant as a number",
            changed(&|l| l["records"][0]["at"] = serde_json::json!(0)),
        ),
        (
            "an instant before the test case",
            changed(&|l| l["records"][0]["at"] = serde_json::json!("-1")),
        ),
        (
            "a kind TCI's subset here does not have",
            changed(&|l| l["records"][0]["kind"] = serde_json::json!("component-killed")),
        ),
        (
            "a sent message without its value",
            changed(&|l| {
                l["records"][sent]
                    .as_object_mut()
                    .expect("a record")
                    .remove("value");
            }),
        ),
        (
            "a sent message without its type",
            changed(&|l| {
                l["records"][sent]
                    .as_object_mut()
                    .expect("a record")
                    .remove("type");
            }),
        ),
        (
            "a received message without its type",
            changed(&|l| {
                l["records"][received]
                    .as_object_mut()
                    .expect("a record")
                    .remove("type");
            }),
        ),
        (
            "a message of an unnamed type",
            changed(&|l| l["records"][sent]["type"] = serde_json::json!("")),
        ),
        (
            "an integer of a value as a number",
            changed(&|l| l["records"][sent]["value"]["n"] = serde_json::json!(3)),
        ),
        (
            "an absent field of a value as null",
            changed(&|l| l["records"][sent]["value"]["note"] = serde_json::Value::Null),
        ),
        (
            "a message's whole value as null",
            changed(&|l| l["records"][sent]["value"] = serde_json::Value::Null),
        ),
        (
            "a field of an element of a list as null",
            changed(&|l| l["records"][sent]["value"]["note"] = serde_json::json!([{ "n": null }])),
        ),
        (
            "a duration as a number",
            changed(&|l| l["records"][started]["duration_us"] = serde_json::json!(10)),
        ),
        (
            "an empty reason",
            changed(&|l| l["records"][verdict]["reason"] = serde_json::json!("")),
        ),
        (
            "a member another kind has",
            changed(&|l| l["records"][verdict]["port"] = serde_json::json!("p")),
        ),
        (
            "the end without its verdict",
            changed(&|l| {
                l["records"][ended]
                    .as_object_mut()
                    .expect("a record")
                    .remove("verdict");
            }),
        ),
        (
            "no record",
            changed(&|l| l["records"] = serde_json::json!([])),
        ),
    ];
    for (what, instance) in changes {
        assert!(
            !violations(&schema, &instance).is_empty(),
            "the schema admits {what}: {instance}"
        );
    }
}
