// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! `sce-codegen check-trace <suite> <trace>` holds a recorded input trace to
//! the suite it is of without running it (docs/SCE_TTCN3_LANE.md §4, §10.2).
//!
//! What it holds:
//!   * every committed trace of the runtime suite fits it, and a
//!     trace of each test case of the every-form fixtures, the union form
//!     among them, fits a suite of them: exit 0 and nothing written;
//!   * each refusal of §10.2 is its own `test-profile/trace-*` code, exit 1:
//!     a text that is no trace, a trace of another suite or test case, an
//!     input on a port that is no system port, of a type its port does not
//!     take, or whose value does not belong to its type, each value refused
//!     in the words of the Rust runtime's reader at the JSON pointer of the
//!     place it is about;
//!   * every refusal of one trace is reported, not only the first;
//!   * an input that is not read leaves with the status its record has
//!     (SCE_ERROR_CONTRACT.md §6): a trace file not read, a suite missing.
//!
//! Every trace that is refused is written here, into a directory of the test's
//! own, and never committed: a committed fixture holding what the check
//! refuses would be a file every other sweep of the fixtures reads too.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn runtime_dir() -> PathBuf {
    manifest_dir().join("tests/fixtures/test_profile/runtime")
}

fn runtime_suite() -> PathBuf {
    runtime_dir().join("runtime_suite.scxml")
}

fn trace_forms_dir() -> PathBuf {
    manifest_dir().join("tests/fixtures/test_profile/trace_forms")
}

/// What one run of the command did.
struct Checked {
    status: i32,
    stdout: String,
    stderr: String,
}

impl Checked {
    /// The NDJSON records the run wrote to the standard error, each a JSON
    /// object.
    fn records(&self) -> Vec<Value> {
        self.stderr
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| {
                serde_json::from_str(line)
                    .unwrap_or_else(|e| panic!("`{line}` is no JSON record: {e}"))
            })
            .collect()
    }

    /// The `code` of each record, in the order written.
    fn codes(&self) -> Vec<String> {
        self.records()
            .iter()
            .map(|record| {
                record["code"]
                    .as_str()
                    .expect("a record has a code")
                    .to_string()
            })
            .collect()
    }

    /// The one record the run wrote.
    fn only_record(&self) -> Value {
        let records = self.records();
        assert_eq!(records.len(), 1, "one record:\n{}", self.stderr);
        records.into_iter().next().expect("one record")
    }
}

/// Run `check-trace` on `suite` and `trace` with `--error-format=json`.
fn check_trace(suite: &Path, trace: &Path) -> Checked {
    let output = Command::new(env!("CARGO_BIN_EXE_sce-codegen"))
        .arg("--error-format=json")
        .arg("check-trace")
        .arg(suite)
        .arg(trace)
        .output()
        .expect("spawn sce-codegen");
    Checked {
        status: output.status.code().expect("sce-codegen ended by a signal"),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

/// Write `text` to `<name>.json` in `dir`, and answer its path.
fn written(dir: &Path, name: &str, text: &str) -> PathBuf {
    let path = dir.join(format!("{name}.json"));
    std::fs::write(&path, text).unwrap_or_else(|e| panic!("write {}: {e}", path.display()));
    path
}

/// The canonical text of a trace of the test case `test_case` of the suite
/// `suite`, each input its instant, port, type and value, the value in its
/// canonical text already.
fn trace_text(suite: &str, test_case: &str, inputs: &[(&str, &str, &str, &str)]) -> String {
    let inputs: Vec<String> = inputs
        .iter()
        .map(|(at, port, message_type, value)| {
            format!(
                "{{\"at\":\"{at}\",\"port\":\"{port}\",\"type\":\"{message_type}\",\
                 \"value\":{value}}}"
            )
        })
        .collect();
    format!(
        "{{\"inputs\":[{}],\"schema\":\"sce-test-trace.v1\",\"suite\":\"{suite}\",\
         \"test_case\":\"{test_case}\"}}",
        inputs.join(",")
    )
}

/// A trace of `answers_the_job` of the runtime suite whose one input, at 500
/// on `sut`, is of `message_type` and holds `value`, and whose suite and test
/// case are `suite` and `test_case`.
fn answer(suite: &str, test_case: &str, port: &str, message_type: &str, value: &str) -> String {
    trace_text(suite, test_case, &[("500", port, message_type, value)])
}

/// The answer the committed trace of `answers_the_job` holds.
const REPLY: &str = r#"{"code":"0","job":"7"}"#;

/// A refusal's record, as the test reads it.
fn assert_record(record: &Value, trace: &Path, code: &str, message: &str, actual: Option<&str>) {
    assert_eq!(record["code"], code, "{record}");
    assert_eq!(record["stage"], "validation", "{record}");
    assert_eq!(record["spec"], "SCE TTCN-3 Lane §10.2", "{record}");
    assert_eq!(record["message"], message, "{record}");
    assert_eq!(
        record["location"]["file"],
        trace.display().to_string(),
        "{record}"
    );
    assert_eq!(record["location"]["line"], 1, "{record}");
    assert_eq!(record["actual"].as_str(), actual, "{record}");
}

/// docs/SCE_TTCN3_LANE.md §10.4, §12: every committed trace of the runtime
/// suite, which every backend's runner replays, fits it: exit 0, and nothing
/// written to either stream. The trace of each test case the suite runs is
/// among them.
#[test]
fn every_committed_trace_fits_the_runtime_suite() {
    let mut traces: Vec<PathBuf> = std::fs::read_dir(runtime_dir().join("traces"))
        .expect("the traces are a directory")
        .map(|entry| entry.expect("an entry of the traces").path())
        .collect();
    traces.sort();
    let names: Vec<String> = traces
        .iter()
        .map(|trace| {
            trace
                .file_name()
                .expect("a trace is a file")
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    assert_eq!(
        names,
        [
            "answers_before_the_guard.json",
            "answers_the_job.json",
            "keeps_a_ptc.json",
            "serves_a_ptc.json"
        ],
        "a trace of each test case of the runtime suite, and nothing else"
    );
    for trace in &traces {
        let checked = check_trace(&runtime_suite(), trace);
        assert_eq!(checked.status, 0, "{}: {}", trace.display(), checked.stderr);
        assert!(checked.stdout.is_empty(), "{}", checked.stdout);
        assert!(checked.stderr.is_empty(), "{}", checked.stderr);
    }
}

// ── The every-form fixtures ──────────────────────────────────────

/// An order paid by card: two lines, a note and no priority.
const ORDER_CARD: &str = concat!(
    r#"{"customer":{"full_name":"Ada","vip":true},"lines":[{"color":"green","quantity":"2","#,
    r#""sku":"A-1"},{"color":"blue","quantity":"65535","sku":"B-2"}],"note":"ring twice","#,
    r#""order_id":"4294967295","payment":{"card":{"amount":"1","last4":"4242"}}}"#
);

/// An order paid in cash, the case that holds no record: no line, no note, a
/// priority.
const ORDER_CASH: &str = concat!(
    r#"{"customer":{"full_name":"Bo","vip":false},"lines":[],"order_id":"0","#,
    r#""payment":{"cash":{}},"priority":"3"}"#
);

/// An order paid by voucher, the case of a byte string: an empty text.
const ORDER_VOUCHER: &str = concat!(
    r#"{"customer":{"full_name":"Cy","vip":true},"lines":[{"color":"red","quantity":"0","#,
    r#""sku":""}],"note":"","order_id":"7","payment":{"voucher":{"voucher_code":"00ABFF"}}}"#
);

/// The order of the configuration with no union: a byte string in its
/// payment's place.
const ORDER_TAKEN: &str = concat!(
    r#"{"customer":{"full_name":"Ada","vip":true},"lines":[{"color":"green","quantity":"2","#,
    r#""sku":"A-1"}],"note":"ring twice","order_id":"4294967295","voucher_code":"00ABFF"}"#
);

/// A status with an empty list and one optional integer present.
const STATUS_EMPTY: &str = r#"{"codes":[],"count":"7","elapsed_ms":"0","tag":"255"}"#;

/// A status with a full list and the other optionals present.
const STATUS_FULL: &str =
    r#"{"codes":["3","9","5","0"],"count":"0","elapsed_ms":"12","note":"late","retry":"0"}"#;

/// The extras: a declared variant of an open enum, a list with an absent
/// element, the least 64-bit integer, a list at its lower count, an optional
/// list present, and a list of lists with an empty one.
const EXTRAS_DECLARED: &str = concat!(
    r#"{"flags":[true],"grid":[["1","2"],[]],"least":"-9223372036854775808","#,
    r#""shape":"circle","slots":["1",null,"65535"],"spare":["0","255"]}"#
);

/// The extras: a number of the open enum no variant declares, the greatest
/// 64-bit integer, a list at its bound, no optional list, empty lists.
const EXTRAS_UNDECLARED: &str = concat!(
    r#"{"flags":[false,true],"grid":[],"least":"9223372036854775807","shape":"9","#,
    r#""slots":[]}"#
);

/// The extras of the configuration whose enums are all closed.
const CLOSED_EXTRAS: &str = concat!(
    r#"{"flags":[true],"grid":[["1","2"],[]],"least":"-9223372036854775808","#,
    r#""slots":["1",null,"65535"],"spare":["0","255"]}"#
);

/// Write `forms_suite.scxml` to `dir`: a suite that runs the three test cases
/// of the every-form fixtures, each over its own configuration of every form
/// (docs/SCE_TTCN3_LANE.md §10.5), imported where they are committed.
fn write_forms_suite(dir: &Path) -> PathBuf {
    let forms = trace_forms_dir();
    let import = |stem: &str, alias: &str| {
        format!(
            "  <sce:import kind=\"test-case\" src=\"{}\" as=\"{alias}\"/>\n",
            forms.join(format!("{stem}.scxml")).display()
        )
    };
    let text = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <scxml xmlns=\"http://www.w3.org/2005/07/scxml\" xmlns:sce=\"http://sce.dev/ext\" \
         version=\"1.0\" sce:kind=\"test-suite\" name=\"forms_suite\">\n\
         {}{}{}  <sce:execute test-case=\"EveryForm\"/>\n  \
         <sce:execute test-case=\"EveryUnionlessForm\"/>\n  \
         <sce:execute test-case=\"EveryClosedEnumForm\"/>\n</scxml>\n",
        import("replays_every_form", "EveryForm"),
        import("replays_every_unionless_form", "EveryUnionlessForm"),
        import("replays_every_closed_enum_form", "EveryClosedEnumForm"),
    );
    let path = dir.join("forms_suite.scxml");
    std::fs::write(&path, text).unwrap_or_else(|e| panic!("write {}: {e}", path.display()));
    path
}

/// docs/SCE_TTCN3_LANE.md §10.3, §10.5: a trace of every form of the
/// canonical value fits the suite of the every-form fixtures, on each of their
/// three configurations: the union's three cases, a record in a list, an
/// absent optional field and a present one, an open enum's declared name and
/// a number no variant declares, a list's absent element, both ends of a
/// 64-bit integer, a list at either bound, a list of lists and a byte string.
#[test]
fn a_trace_of_every_form_fits_the_suite_of_the_every_form_fixtures() {
    let dir = tempfile::tempdir().expect("a directory of the test's own");
    let suite = write_forms_suite(dir.path());
    for (test_case, inputs) in [
        (
            "replays_every_form",
            vec![
                ("0", "sut", "order_placed", ORDER_CARD),
                ("0", "sut", "order_placed", ORDER_CASH),
                ("0", "sut", "job_status", STATUS_EMPTY),
                ("250", "sut", "job_status", STATUS_FULL),
                ("250", "sut", "order_placed", ORDER_VOUCHER),
                ("250", "sut", "form_extras", EXTRAS_DECLARED),
                ("500", "sut", "form_extras", EXTRAS_UNDECLARED),
            ],
        ),
        (
            "replays_every_unionless_form",
            vec![
                ("0", "sut", "order_taken", ORDER_TAKEN),
                ("0", "sut", "job_status", STATUS_FULL),
                ("250", "sut", "form_extras", EXTRAS_DECLARED),
            ],
        ),
        (
            "replays_every_closed_enum_form",
            vec![
                ("0", "sut", "order_placed", ORDER_CARD),
                ("0", "sut", "job_status", STATUS_EMPTY),
                ("250", "sut", "closed_enum_form_extras", CLOSED_EXTRAS),
            ],
        ),
    ] {
        let trace = written(
            dir.path(),
            test_case,
            &format!("{}\n", trace_text("forms_suite", test_case, &inputs)),
        );
        let checked = check_trace(&suite, &trace);
        assert_eq!(checked.status, 0, "{test_case}: {}", checked.stderr);
        assert!(checked.stdout.is_empty(), "{test_case}: {}", checked.stdout);
        assert!(checked.stderr.is_empty(), "{test_case}: {}", checked.stderr);
    }
}

/// docs/SCE_TTCN3_LANE.md §10.2, §10.3: a value of each form that does not
/// belong to its type is refused in the words of the Rust runtime's reader,
/// at the JSON pointer of the value it refuses: a union's case read before its
/// record, a case's record named as the union is, an element by its index.
#[test]
fn a_value_of_each_form_is_refused_where_it_stands() {
    let dir = tempfile::tempdir().expect("a directory of the test's own");
    let suite = write_forms_suite(dir.path());
    let changed = |base: &str, from: &str, to: &str| {
        assert!(base.contains(from), "`{from}` in {base}");
        base.replacen(from, to, 1)
    };
    for (name, message_type, value, pointer, reason, actual) in [
        (
            "unknown_case",
            "order_placed",
            changed(ORDER_CARD, r#"{"card":"#, r#"{"cheque":"#),
            "/inputs/0/value/payment",
            "order_placed.payment: 'cheque' names no case of its union",
            None,
        ),
        (
            "empty_case_with_a_record",
            "order_placed",
            changed(ORDER_CASH, r#"{"cash":{}}"#, r#"{"cash":{"x":"1"}}"#),
            "/inputs/0/value/payment/cash",
            "order_placed.payment.cash: a case that holds no record is the empty object",
            None,
        ),
        (
            "case_field",
            "order_placed",
            changed(ORDER_CARD, r#""amount":"1""#, r#""amount":"x""#),
            "/inputs/0/value/payment/card/amount",
            "order_placed.payment.amount: 'x' is not the text of an integer's decimal digits",
            Some("x"),
        ),
        (
            "case_bytes",
            "order_placed",
            changed(ORDER_VOUCHER, "00ABFF", "00abff"),
            "/inputs/0/value/payment/voucher/voucher_code",
            "order_placed.payment.voucher_code: '00abff' is not octets in upper-case hex, two \
             digits each",
            Some("00abff"),
        ),
        (
            "element_field",
            "order_placed",
            changed(ORDER_CARD, r#""quantity":"65535""#, r#""quantity":"65536""#),
            "/inputs/0/value/lines/1/quantity",
            "order_placed.lines[].quantity: 65536 is past the range of its type",
            Some("65536"),
        ),
        (
            "declared_number",
            "form_extras",
            changed(EXTRAS_DECLARED, r#""shape":"circle""#, r#""shape":"1""#),
            "/inputs/0/value/shape",
            "form_extras.shape: 1 is the number of a declared variant, which is written by its \
             name",
            Some("1"),
        ),
        (
            "optional_element",
            "form_extras",
            changed(
                EXTRAS_DECLARED,
                r#"["1",null,"65535"]"#,
                r#"["1","x","65535"]"#,
            ),
            "/inputs/0/value/slots/1",
            "form_extras.slots[]: 'x' is not the text of an integer's decimal digits",
            Some("x"),
        ),
        (
            "optional_field_null",
            "form_extras",
            changed(EXTRAS_DECLARED, r#""spare":["0","255"]"#, r#""spare":null"#),
            "/inputs/0/value/spare",
            "form_extras.spare: null is not a list, which is an array",
            Some("null"),
        ),
        (
            "list_count",
            "form_extras",
            changed(EXTRAS_DECLARED, r#""flags":[true]"#, r#""flags":[]"#),
            "/inputs/0/value/flags",
            "form_extras.flags: a list of 0 elements is outside its bounds, 1 to 2",
            None,
        ),
    ] {
        let trace = written(
            dir.path(),
            name,
            &trace_text(
                "forms_suite",
                "replays_every_form",
                &[("0", "sut", message_type, value.as_str())],
            ),
        );
        let checked = check_trace(&suite, &trace);
        assert_eq!(checked.status, 1, "{name}: {}", checked.stderr);
        assert!(checked.stdout.is_empty(), "{name}: {}", checked.stdout);
        assert_record(
            &checked.only_record(),
            &trace,
            "test-profile/trace-value",
            &format!("{pointer}: input 0, at 0: {reason}"),
            actual,
        );
    }
}

// ── Each refusal of §10.2 ───────────────────────────────────────

/// docs/SCE_TTCN3_LANE.md §10.2, refusal 1: a text that is not JSON, not of
/// the shape, out of the order of its instants or not the one canonical text
/// is no trace, and nothing else of it is read: one record, which names no
/// place in it.
#[test]
fn a_text_that_is_no_trace_is_refused_as_one() {
    let dir = tempfile::tempdir().expect("a directory of the test's own");
    let fitting = answer(
        "runtime_suite",
        "answers_the_job",
        "sut",
        "job_reply",
        REPLY,
    );
    let out_of_order = trace_text(
        "runtime_suite",
        "answers_the_job",
        &[
            ("5", "sut", "job_reply", REPLY),
            ("4", "sut", "job_reply", REPLY),
        ],
    );
    for (name, text, message) in [
        ("not_json", "not a trace\n".to_string(), None),
        (
            "out_of_order",
            out_of_order,
            Some(
                "input 1, at 4, comes after an input at 5: the inputs stand in order of their \
                 instants",
            ),
        ),
        (
            "not_canonical",
            fitting.replacen("{\"inputs\"", "{ \"inputs\"", 1),
            Some(
                "the trace is not written in its one canonical text, the JSON Canonicalization \
                 Scheme",
            ),
        ),
        (
            "another_schema",
            fitting.replacen("sce-test-trace.v1", "sce-test-trace.v2", 1),
            Some("the trace's schema is not sce-test-trace.v1"),
        ),
    ] {
        let trace = written(dir.path(), name, &text);
        let checked = check_trace(&runtime_suite(), &trace);
        assert_eq!(checked.status, 1, "{name}: {}", checked.stderr);
        assert!(checked.stdout.is_empty(), "{name}: {}", checked.stdout);
        let record = checked.only_record();
        assert_eq!(
            record["code"], "test-profile/trace-text",
            "{name}: {record}"
        );
        assert_eq!(
            record["location"]["file"],
            trace.display().to_string(),
            "{name}: {record}"
        );
        assert!(record["location"].get("line").is_none(), "{name}: {record}");
        assert!(record.get("actual").is_none(), "{name}: {record}");
        let said = record["message"].as_str().expect("a message");
        match message {
            Some(message) => assert_eq!(said, message, "{name}"),
            None => assert!(said.starts_with("the trace is "), "{name}: {said}"),
        }
    }
}

/// docs/SCE_TTCN3_LANE.md §10.2: a trace of another suite, or of a test case
/// the suite does not run, is refused, naming which.
#[test]
fn a_trace_of_another_suite_or_test_case_is_refused() {
    let dir = tempfile::tempdir().expect("a directory of the test's own");
    let trace = written(
        dir.path(),
        "other_suite",
        &answer("other_suite", "answers_the_job", "sut", "job_reply", REPLY),
    );
    let checked = check_trace(&runtime_suite(), &trace);
    assert_eq!(checked.status, 1, "{}", checked.stderr);
    assert_record(
        &checked.only_record(),
        &trace,
        "test-profile/trace-test-case",
        "/suite: the trace is of the test case 'answers_the_job' of the suite 'other_suite', \
         and the suite it is checked against is 'runtime_suite'",
        Some("other_suite"),
    );

    let trace = written(
        dir.path(),
        "no_such_case",
        &answer("runtime_suite", "no_such_case", "sut", "job_reply", REPLY),
    );
    let checked = check_trace(&runtime_suite(), &trace);
    assert_eq!(checked.status, 1, "{}", checked.stderr);
    assert_record(
        &checked.only_record(),
        &trace,
        "test-profile/trace-test-case",
        "/test_case: the trace is of the test case 'no_such_case', which the suite \
         'runtime_suite' does not run",
        Some("no_such_case"),
    );
}

/// docs/SCE_TTCN3_LANE.md §10.2: an input on a port that is no system port of
/// the test case's configuration is refused, in the words of the runtime's
/// player.
#[test]
fn an_input_on_a_port_that_is_no_system_port_is_refused() {
    let dir = tempfile::tempdir().expect("a directory of the test's own");
    let trace = written(
        dir.path(),
        "answers_the_job",
        &answer(
            "runtime_suite",
            "answers_the_job",
            "oracle",
            "job_reply",
            REPLY,
        ),
    );
    let checked = check_trace(&runtime_suite(), &trace);
    assert_eq!(checked.status, 1, "{}", checked.stderr);
    assert!(checked.stdout.is_empty(), "{}", checked.stdout);
    assert_record(
        &checked.only_record(),
        &trace,
        "test-profile/trace-port",
        "/inputs/0/port: input 0, at 500: 'oracle' is not a system port of the test case's \
         configuration",
        Some("oracle"),
    );
}

/// docs/SCE_TTCN3_LANE.md §10.2: an input of a type the `in` list of its
/// port's type does not name is refused: `job_request` is on the `out` list of
/// the type of `sut`, not on its `in` list.
#[test]
fn an_input_of_a_type_its_port_does_not_take_is_refused() {
    let dir = tempfile::tempdir().expect("a directory of the test's own");
    let trace = written(
        dir.path(),
        "answers_the_job",
        &answer(
            "runtime_suite",
            "answers_the_job",
            "sut",
            "job_request",
            r#"{"job":"7"}"#,
        ),
    );
    let checked = check_trace(&runtime_suite(), &trace);
    assert_eq!(checked.status, 1, "{}", checked.stderr);
    assert_record(
        &checked.only_record(),
        &trace,
        "test-profile/trace-type",
        "/inputs/0/type: input 0, at 500: 'job_request' is not on the `in` list of the type of \
         the system port 'sut'",
        Some("job_request"),
    );
}

/// docs/SCE_TTCN3_LANE.md §10.2: a value that does not belong to its type is
/// refused at the place it stands, and a field that is absent at its record.
#[test]
fn a_value_that_does_not_belong_to_its_type_is_refused() {
    let dir = tempfile::tempdir().expect("a directory of the test's own");
    let trace = written(
        dir.path(),
        "leading_zero",
        &answer(
            "runtime_suite",
            "answers_the_job",
            "sut",
            "job_reply",
            r#"{"code":"007","job":"7"}"#,
        ),
    );
    let checked = check_trace(&runtime_suite(), &trace);
    assert_eq!(checked.status, 1, "{}", checked.stderr);
    assert_record(
        &checked.only_record(),
        &trace,
        "test-profile/trace-value",
        "/inputs/0/value/code: input 0, at 500: job_reply.code: '007' is not the text of an \
         integer's decimal digits",
        Some("007"),
    );

    let trace = written(
        dir.path(),
        "absent_field",
        &answer(
            "runtime_suite",
            "answers_the_job",
            "sut",
            "job_reply",
            r#"{"job":"7"}"#,
        ),
    );
    let checked = check_trace(&runtime_suite(), &trace);
    assert_eq!(checked.status, 1, "{}", checked.stderr);
    assert_record(
        &checked.only_record(),
        &trace,
        "test-profile/trace-value",
        "/inputs/0/value: input 0, at 500: job_reply.code: the field is absent, and its type is \
         not optional",
        None,
    );
}

/// docs/SCE_TTCN3_LANE.md §10.2: every refusal of one trace is reported, in
/// the order of its inputs, not only the first: a port that is no system port
/// in one input and a value that does not belong to its type in the next.
#[test]
fn every_refusal_of_one_trace_is_reported() {
    let dir = tempfile::tempdir().expect("a directory of the test's own");
    let trace = written(
        dir.path(),
        "answers_the_job",
        &trace_text(
            "runtime_suite",
            "answers_the_job",
            &[
                ("500", "oracle", "job_reply", REPLY),
                ("600", "sut", "job_reply", r#"{"code":"x","job":"7"}"#),
            ],
        ),
    );
    let checked = check_trace(&runtime_suite(), &trace);
    assert_eq!(checked.status, 1, "{}", checked.stderr);
    assert!(checked.stdout.is_empty(), "{}", checked.stdout);
    assert_eq!(
        checked.codes(),
        ["test-profile/trace-port", "test-profile/trace-value"],
        "{}",
        checked.stderr
    );
    let records = checked.records();
    assert_record(
        &records[0],
        &trace,
        "test-profile/trace-port",
        "/inputs/0/port: input 0, at 500: 'oracle' is not a system port of the test case's \
         configuration",
        Some("oracle"),
    );
    assert_record(
        &records[1],
        &trace,
        "test-profile/trace-value",
        "/inputs/1/value/code: input 1, at 600: job_reply.code: 'x' is not the text of an \
         integer's decimal digits",
        Some("x"),
    );
}

// ── An input that is not read ────────────────────────────────────

/// SCE_ERROR_CONTRACT.md §6: a trace file that is not read is
/// `cli/read-input`, status 20, and a suite that does not exist is the
/// refusal of its document, `xml/file-not-found`, status 2; neither is a
/// verdict on the trace, so neither leaves with the status of one.
#[test]
fn an_input_that_is_not_read_leaves_with_the_status_of_its_record() {
    let dir = tempfile::tempdir().expect("a directory of the test's own");
    let missing_trace = dir.path().join("answers_the_job.json");
    let checked = check_trace(&runtime_suite(), &missing_trace);
    assert_eq!(checked.status, 20, "{}", checked.stderr);
    assert!(checked.stdout.is_empty(), "{}", checked.stdout);
    assert_eq!(checked.codes(), ["cli/read-input"], "{}", checked.stderr);

    let trace = runtime_dir().join("traces/answers_the_job.json");
    let missing_suite = dir.path().join("no_such_suite.scxml");
    let checked = check_trace(&missing_suite, &trace);
    assert_eq!(checked.status, 2, "{}", checked.stderr);
    assert!(checked.stdout.is_empty(), "{}", checked.stdout);
    assert_eq!(
        checked.codes(),
        ["xml/file-not-found"],
        "{}",
        checked.stderr
    );
}

/// The human form of a refusal says what the JSON record's message says,
/// and the run leaves with the same status.
#[test]
fn a_refusal_reads_alike_in_the_human_form() {
    let dir = tempfile::tempdir().expect("a directory of the test's own");
    let trace = written(
        dir.path(),
        "answers_the_job",
        &answer(
            "runtime_suite",
            "answers_the_job",
            "oracle",
            "job_reply",
            REPLY,
        ),
    );
    let output = Command::new(env!("CARGO_BIN_EXE_sce-codegen"))
        .arg("check-trace")
        .arg(runtime_suite())
        .arg(trace)
        .output()
        .expect("spawn sce-codegen");
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(
            "/inputs/0/port: input 0, at 500: 'oracle' is not a system port of the test case's \
             configuration"
        ),
        "{stderr}"
    );
}
