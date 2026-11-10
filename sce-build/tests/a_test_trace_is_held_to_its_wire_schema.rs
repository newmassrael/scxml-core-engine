// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The recorded input trace's wire schema (`schemas/sce-test-trace.v1.schema.json`;
//! docs/SCE_TTCN3_LANE.md §10), held to the traces its reader reads: every
//! committed trace of the runtime's suite, and a trace written here by hand
//! whose values take every form the canonical form has. The negative half
//! changes one member of a trace the reader first reads at a time, and both
//! the schema and the reader refuse each.
//!
//! How a trace replays, and the refusal of a value that does not belong to its
//! type, are held by the runtime's own tests
//! (`backends/rust/runtime/tests/a_trace_replays_to_its_test_log.rs`) and by
//! the generated suite's (`backends/rust/tests/tests/a_test_suite_runs_through_its_runner.rs`).

use std::path::{Path, PathBuf};

use sce_rust_runtime::json::Value;
use sce_rust_runtime::test::{canonical, Trace, TraceInput, TEST_TRACE_SCHEMA};

fn schema() -> serde_json::Value {
    serde_json::from_str(include_str!("../../schemas/sce-test-trace.v1.schema.json"))
        .expect("the trace's schema is JSON")
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

/// The directory of the committed traces of the runtime's suite.
fn traces_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/test_profile/runtime/traces")
}

/// Each committed trace, its file's name and its text.
fn committed() -> Vec<(String, String)> {
    let mut traces: Vec<(String, String)> = std::fs::read_dir(traces_dir())
        .expect("the traces are a directory")
        .map(|entry| entry.expect("an entry").path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .map(|path| {
            let name = path.display().to_string();
            let text = std::fs::read_to_string(&path).expect("a trace is text");
            (name, text)
        })
        .collect();
    traces.sort();
    traces
}

/// A trace written by hand whose values take every form of the canonical form:
/// an integer at both ends of 64 bits, a truth value, a text, a byte string, a
/// value of an enum, a list with an absent element, a record within a record
/// and the two shapes of a case of a union; two inputs share an instant.
fn every_form() -> Trace {
    let value = canonical::record([
        ("widest", Some(canonical::integer(u64::MAX))),
        ("least", Some(canonical::integer(i64::MIN))),
        ("rush", Some(canonical::boolean(true))),
        ("note", Some(canonical::text("a \"quoted\" note"))),
        ("key", Some(canonical::bytes(&[0x00, 0xff]))),
        ("color", Some(canonical::enumerated("red"))),
        (
            "tags",
            Some(canonical::list([
                canonical::optional_element(Some(canonical::integer(1_u8))),
                canonical::optional_element(None),
            ])),
        ),
        (
            "line",
            Some(canonical::record([(
                "quantity",
                Some(canonical::integer(2_u16)),
            )])),
        ),
        (
            "payment",
            Some(canonical::union_case(
                "card",
                Some(canonical::record([(
                    "amount",
                    Some(canonical::integer(1_u32)),
                )])),
            )),
        ),
        ("refund", Some(canonical::union_case("cash", None))),
        ("absent", None),
    ]);
    let input = |at: i64, port: &str| TraceInput {
        at,
        port: port.to_string(),
        message_type: "order_placed".to_string(),
        value: value.clone(),
    };
    Trace {
        suite: "held_suite".to_string(),
        test_case: "orders".to_string(),
        inputs: vec![input(0, "shop"), input(250, "shop"), input(250, "audit")],
    }
}

fn parsed(text: &str) -> serde_json::Value {
    serde_json::from_str(text).expect("a trace is JSON")
}

/// Every trace the reader reads is one the schema states: each committed
/// trace, read by the reader and written back as its own bytes, and the trace
/// of every form, which the reader reads back from the text it writes.
#[test]
fn every_trace_the_reader_reads_validates_against_the_wire_schema() {
    let schema = schema();
    let traces = committed();
    assert!(
        !traces.is_empty(),
        "no trace is committed under {}",
        traces_dir().display()
    );
    for (name, text) in traces {
        let trace = Trace::parse(&text).unwrap_or_else(|refused| panic!("{name}: {refused}"));
        assert_eq!(
            format!("{}\n", trace.canonical_text()),
            text,
            "{name} is its canonical text and one line feed"
        );
        assert_eq!(
            violations(&schema, &parsed(&text)),
            Vec::<String>::new(),
            "{name}"
        );
    }

    let written = every_form().canonical_text();
    assert_eq!(
        violations(&schema, &parsed(&written)),
        Vec::<String>::new(),
        "{written}"
    );
    assert_eq!(Trace::parse(&written), Ok(every_form_as_read(&written)));
    assert_eq!(parsed(&written)["schema"], TEST_TRACE_SCHEMA);
}

/// The trace of every form as the reader reads it back: its values keep the
/// order of the members the text writes, which the canonical text sorts.
fn every_form_as_read(written: &str) -> Trace {
    let read = sce_rust_runtime::json::parse(written).expect("JSON");
    let Some(Value::Array(inputs)) = read.member("inputs") else {
        panic!("the trace has inputs");
    };
    let mut trace = every_form();
    for (input, read) in trace.inputs.iter_mut().zip(inputs) {
        input.value = read.member("value").expect("a value").clone();
    }
    trace
}

/// The schema refuses what the reader does not accept: starting from a trace
/// both first accept, each change of one member is refused by both.
#[test]
fn the_trace_schema_rejects_what_the_reader_does_not_accept() {
    let schema = schema();
    let text = every_form().canonical_text();
    let valid = parsed(&text);
    assert_eq!(violations(&schema, &valid), Vec::<String>::new());
    assert!(Trace::parse(&text).is_ok());
    let changed = |change: &dyn Fn(&mut serde_json::Value)| {
        let mut instance = valid.clone();
        change(&mut instance);
        instance
    };
    let changes: Vec<(&str, serde_json::Value)> = vec![
        (
            "another shape",
            changed(&|t| t["schema"] = serde_json::json!("sce-test-trace.v2")),
        ),
        (
            "no suite",
            changed(&|t| {
                t.as_object_mut().expect("a trace").remove("suite");
            }),
        ),
        (
            "an unnamed test case",
            changed(&|t| t["test_case"] = serde_json::json!("")),
        ),
        (
            "a member the shape does not have",
            changed(&|t| t["verdict"] = serde_json::json!("pass")),
        ),
        (
            "inputs that are no array",
            changed(&|t| t["inputs"] = serde_json::json!({})),
        ),
        (
            "an instant as a number",
            changed(&|t| t["inputs"][0]["at"] = serde_json::json!(0)),
        ),
        (
            "an instant with a leading zero",
            changed(&|t| t["inputs"][0]["at"] = serde_json::json!("00")),
        ),
        (
            "an instant before the test case",
            changed(&|t| t["inputs"][0]["at"] = serde_json::json!("-1")),
        ),
        (
            "an input of no port",
            changed(&|t| {
                t["inputs"][0]
                    .as_object_mut()
                    .expect("an input")
                    .remove("port");
            }),
        ),
        (
            "an input of an unnamed type",
            changed(&|t| t["inputs"][0]["type"] = serde_json::json!("")),
        ),
        (
            "an input with a member another record has",
            changed(&|t| t["inputs"][0]["verdict"] = serde_json::json!("pass")),
        ),
    ];
    for (what, instance) in changes {
        assert!(
            !violations(&schema, &instance).is_empty(),
            "the schema admits {what}: {instance}"
        );
        let text = serde_json::to_string(&instance).expect("JSON");
        assert!(
            Trace::parse(&text).is_err(),
            "the reader admits {what}: {text}"
        );
    }
    // What the reader reads by the type of each value, the schema refuses by
    // the form alone: a number, a value that is no record, and a null field.
    let by_form: Vec<(&str, serde_json::Value)> = vec![
        (
            "an integer of a value as a number",
            changed(&|t| t["inputs"][0]["value"]["widest"] = serde_json::json!(3)),
        ),
        (
            "a value that is no record",
            changed(&|t| t["inputs"][0]["value"] = serde_json::json!("order")),
        ),
        (
            "an absent field as null",
            changed(&|t| t["inputs"][0]["value"]["absent"] = serde_json::Value::Null),
        ),
    ];
    for (what, instance) in by_form {
        assert!(
            !violations(&schema, &instance).is_empty(),
            "the schema admits {what}: {instance}"
        );
    }
}
