//! The JSON form of a queue history and the command that judges it (SCE
//! Protocol-Synthesis RFC §synth-5-P, verification layer 2).
//!
//! The checker is one search written once; what each backend adds is a stress
//! run that writes its observations in this form. So the form is held from
//! both sides here: the real `sce-codegen check-queue-history` judges every
//! fixture history by the name the fixture carries, the schema that documents
//! the form accepts exactly the histories the reader accepts, and a history
//! written by the reader's own writer reads back the same.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use sce_build::queue_history::{
    check, Call, History, Operation, Outcome, Refusal, Verdict, HISTORY_FORMAT_VERSION,
};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent directory")
        .to_path_buf()
}

fn histories_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/queue_histories")
}

fn fixtures() -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(histories_dir())
        .expect("the history fixtures exist")
        .map(|entry| entry.expect("a directory entry").path())
        .filter(|path| path.extension().is_some_and(|e| e == "json"))
        .collect();
    files.sort();
    files
}

fn name_of(path: &Path) -> String {
    path.file_name()
        .and_then(|n| n.to_str())
        .expect("a UTF-8 file name")
        .to_string()
}

fn judge(files: &[&Path]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_sce-codegen"))
        .arg("check-queue-history")
        .args(files)
        .output()
        .expect("run sce-codegen check-queue-history")
}

/// What a fixture's name says the judgement is: the prefix, and a phrase the
/// refusal must contain.
fn expectation(name: &str) -> Option<&'static str> {
    if name.starts_with("linearizable_") {
        None
    } else if name.starts_with("not_linearizable_") {
        Some("is not linearizable")
    } else if name.starts_with("malformed_") {
        Some("cannot have come from a run")
    } else if name == "not_a_history_unknown_field.json" {
        Some("not a queue history")
    } else if name == "not_a_history_push_without_a_value.json" {
        Some("participants[0][0]: a push names the value it pushed")
    } else if name == "not_a_history_a_participant_that_is_null.json" {
        // A participant that made no attempt is an empty list. `null` is what a
        // nil slice becomes in a language that has one, and the format refuses
        // it rather than guessing it meant the empty list.
        Some("not a queue history")
    } else if name.starts_with("not_a_history_") {
        panic!("{name} states no phrase its refusal must carry")
    } else if name == "records_no_operation.json" {
        Some("records no operation")
    } else {
        panic!("{name} carries no prefix that says how it is judged")
    }
}

#[test]
fn every_fixture_is_judged_as_its_name_says() {
    let files = fixtures();
    assert!(
        files.len() >= 9,
        "the fixtures are the checker's own tests; only {} are here",
        files.len()
    );
    for file in &files {
        let name = name_of(file);
        let run = judge(&[file.as_path()]);
        let stdout = String::from_utf8_lossy(&run.stdout);
        match expectation(&name) {
            None => {
                assert!(
                    run.status.success(),
                    "{name} should be linearizable: {stdout}"
                );
                assert!(stdout.starts_with("ok   "), "{name}: {stdout}");
            }
            Some(phrase) => {
                assert!(!run.status.success(), "{name} should be refused: {stdout}");
                assert!(
                    stdout.starts_with("FAIL ") && stdout.contains(phrase),
                    "{name} should say {phrase:?}: {stdout}"
                );
            }
        }
    }
}

/// Every file is judged and a line printed for each, then the command fails:
/// a run that wrote six histories and broke two says which two.
#[test]
fn a_command_over_many_files_says_which_failed() {
    let dir = histories_dir();
    let good = dir.join("linearizable_sequential.json");
    let bad = dir.join("not_linearizable_order.json");
    let run = judge(&[bad.as_path(), good.as_path()]);
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(
        !run.status.success(),
        "one refused history fails the command"
    );
    assert!(
        stdout.contains("FAIL ") && stdout.contains("not_linearizable_order.json"),
        "{stdout}"
    );
    assert!(
        stdout.contains("ok   ") && stdout.contains("linearizable_sequential.json"),
        "the good history after a bad one is still judged: {stdout}"
    );
}

#[test]
fn a_file_that_cannot_be_read_fails_the_command() {
    let missing = histories_dir().join("there_is_no_such_history.json");
    let run = judge(&[missing.as_path()]);
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(!run.status.success());
    assert!(
        stdout.contains("FAIL ") && stdout.contains("cannot be read"),
        "{stdout}"
    );
}

/// The schema is as strict as the reader. A history the reader takes
/// validates against it, and one the reader refuses for its shape does not;
/// were it laxer, a backend could write what the schema allows and the
/// checker would refuse, and were it stricter, the schema would turn away
/// histories that are judged.
#[test]
fn the_schema_accepts_what_the_reader_accepts() {
    let schema: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(
            repo_root().join("tests/forge/conformance/queue_history.schema.json"),
        )
        .expect("read the schema"),
    )
    .expect("the schema is JSON");
    let validator = jsonschema::JSONSchema::options()
        .with_draft(jsonschema::Draft::Draft7)
        .compile(&schema)
        .expect("the schema compiles");

    for file in fixtures() {
        let name = name_of(&file);
        let text = std::fs::read_to_string(&file).expect("read the fixture");
        let document: serde_json::Value = serde_json::from_str(&text).expect("the fixture is JSON");
        let valid = validator.validate(&document).is_ok();
        let reader_takes_it = History::from_json(&text).is_ok();
        assert_eq!(
            valid, reader_takes_it,
            "{name}: the schema says {valid}, the reader says {reader_takes_it}"
        );
    }
}

fn op(call: Call, outcome: Outcome, invoked: u64, returned: u64) -> Operation {
    Operation {
        call,
        outcome,
        invoked,
        returned,
    }
}

/// What the writer writes, the reader reads back, including values beyond
/// 2^53 that a writer in a language with doubles would lose.
#[test]
fn a_written_history_reads_back_the_same() {
    let big = (1u64 << 60) + 7;
    let history = History {
        capacity: 3,
        refusal: Refusal::WhileSlotsAreHeld,
        participants: vec![
            vec![
                op(Call::Push(big), Outcome::Pushed, 1, 2),
                op(Call::Push(2), Outcome::Full, 3, 4),
            ],
            vec![
                op(Call::Pop, Outcome::Popped(big), 5, 6),
                op(Call::Pop, Outcome::Empty, 7, 8),
            ],
        ],
    };
    let written = history.to_json();
    let read = History::from_json(&written).expect("the writer's output reads");
    assert_eq!(
        read.to_json(),
        written,
        "writing what was read gives the same text"
    );
    assert_eq!(read.capacity, 3);
    assert_eq!(read.refusal, Refusal::WhileSlotsAreHeld);
    assert_eq!(read.participants[0][0].call, Call::Push(big));
    assert_eq!(read.participants[1][0].outcome, Outcome::Popped(big));
    assert_eq!(check(&read), check(&history));
}

fn refused(text: &str) -> String {
    History::from_json(text)
        .expect_err("the reader refuses this document")
        .to_string()
}

fn document(operation: &str) -> String {
    format!(
        r#"{{"version":1,"capacity":2,"refusal":"at-capacity","participants":[[{operation}]]}}"#
    )
}

#[test]
fn the_reader_refuses_what_is_not_the_form() {
    assert!(refused("not json").contains("not a queue history"));
    assert!(
        refused(r#"{"version":2,"capacity":2,"refusal":"at-capacity","participants":[]}"#)
            .contains(&format!(
                "is not the version this reader reads ({HISTORY_FORMAT_VERSION})"
            ))
    );
    assert!(refused(r#"{"version":1,"capacity":2,"participants":[]}"#).contains("refusal"));
    assert!(
        refused(r#"{"version":1,"capacity":2,"refusal":"sometimes","participants":[]}"#)
            .contains("not a queue history")
    );
    // An operation that cannot be one, each with the place it is at.
    assert!(refused(&document(
        r#"{"call":"push","outcome":"pushed","invoked":1,"returned":2}"#
    ))
    .contains("participants[0][0]: a push names the value it pushed"));
    assert!(refused(&document(
        r#"{"call":"push","value":1,"outcome":"popped","invoked":1,"returned":2}"#
    ))
    .contains("a push is pushed or full"));
    assert!(refused(&document(
        r#"{"call":"pop","outcome":"popped","invoked":1,"returned":2}"#
    ))
    .contains("a pop that popped names the value it returned"));
    assert!(refused(&document(
        r#"{"call":"pop","value":1,"outcome":"empty","invoked":1,"returned":2}"#
    ))
    .contains("a pop that found the queue empty names no value"));
    assert!(refused(&document(
        r#"{"call":"pop","value":1,"outcome":"pushed","invoked":1,"returned":2}"#
    ))
    .contains("a pop is popped or empty"));
    assert!(
        refused(&document(
            r#"{"call":"push","value":-1,"outcome":"pushed","invoked":1,"returned":2}"#
        ))
        .contains("not a queue history"),
        "a negative value is not a value"
    );
}

/// The checker still says what it said in the crate it lived in: the
/// histories its own layer-2 tests built by hand are judged the same way.
#[test]
fn the_checker_judges_a_hand_built_history_as_before() {
    let ok = History {
        capacity: 1,
        refusal: Refusal::AtCapacity,
        participants: vec![vec![
            op(Call::Push(1), Outcome::Pushed, 1, 2),
            op(Call::Pop, Outcome::Popped(1), 3, 4),
        ]],
    };
    assert_eq!(check(&ok), Verdict::Linearizable);
    let lost = History {
        capacity: 1,
        refusal: Refusal::AtCapacity,
        participants: vec![vec![
            op(Call::Push(1), Outcome::Pushed, 1, 2),
            op(Call::Pop, Outcome::Empty, 3, 4),
        ]],
    };
    assert!(matches!(check(&lost), Verdict::NotLinearizable { .. }));
}
