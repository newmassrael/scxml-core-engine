//! The JSON form of a broadcast-log history and the command that judges it
//! (ADR 0062 §5).
//!
//! The checker is one search written once; what each backend adds is a stress
//! run that writes its observations in this form. So the form is held from
//! both sides here: the real `sce-codegen check-broadcast-history` judges every
//! fixture history by the name the fixture carries, the schema that documents
//! the form accepts exactly the histories the reader accepts, and a history
//! written by the reader's own writer reads back the same.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use sce_build::broadcast_history::{
    check, Call, History, OpenFrom, Operation, Outcome, HISTORY_FORMAT_VERSION,
};
use sce_build::history_common::Verdict;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent directory")
        .to_path_buf()
}

fn histories_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/broadcast_histories")
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
        .arg("check-broadcast-history")
        .args(files)
        .output()
        .expect("run sce-codegen check-broadcast-history")
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
        Some("not a broadcast-log history")
    } else if name == "not_a_history_a_publish_without_its_value.json" {
        Some("publisher[0]: names no `value`, which this call takes")
    } else if name == "not_a_history_a_read_that_names_a_head.json" {
        Some("readers[0][1]: names `head`, which this call does not take")
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
        files.len() >= 12,
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
    let bad = dir.join("not_linearizable_an_item_outside_the_window.json");
    let run = judge(&[bad.as_path(), good.as_path()]);
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(
        !run.status.success(),
        "one refused history fails the command"
    );
    assert!(
        stdout.contains("FAIL ")
            && stdout.contains("not_linearizable_an_item_outside_the_window.json"),
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
            repo_root().join("tests/forge/conformance/broadcast_history.schema.json"),
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

fn publish(value: u64, seq: u64, invoked: u64, returned: u64) -> Operation {
    op(
        Call::Publish(value),
        Outcome::Published(seq),
        invoked,
        returned,
    )
}

fn open(from: OpenFrom, cursor: u64, invoked: u64, returned: u64) -> Operation {
    op(Call::Open(from), Outcome::Opened(cursor), invoked, returned)
}

fn item(seq: u64, value: u64, invoked: u64, returned: u64) -> Operation {
    op(Call::Read, Outcome::Item { seq, value }, invoked, returned)
}

fn lagged(skipped: u64, invoked: u64, returned: u64) -> Operation {
    op(Call::Read, Outcome::Lagged { skipped }, invoked, returned)
}

/// What the writer writes, the reader reads back, including values beyond
/// 2^53 that a writer in a language with doubles would lose.
#[test]
fn a_written_history_reads_back_the_same() {
    let big = (1u64 << 60) + 7;
    let history = History {
        retention: 3,
        publisher: vec![
            publish(big, 0, 1, 2),
            op(Call::Head, Outcome::Head(1), 3, 4),
        ],
        readers: vec![vec![
            open(OpenFrom::At(0), 0, 5, 6),
            item(0, big, 7, 8),
            op(Call::Read, Outcome::Empty, 9, 10),
            open(OpenFrom::Head, 1, 11, 12),
            lagged(2, 13, 14),
        ]],
    };
    let written = history.to_json();
    let read = History::from_json(&written).expect("the writer's output reads");
    assert_eq!(
        read.to_json(),
        written,
        "writing what was read gives the same text"
    );
    assert_eq!(read.retention, 3);
    assert_eq!(read.publisher[0].call, Call::Publish(big));
    assert_eq!(read.readers[0][0].call, Call::Open(OpenFrom::At(0)));
    assert_eq!(
        read.readers[0][1].outcome,
        Outcome::Item { seq: 0, value: big }
    );
    assert_eq!(check(&read), check(&history));
}

fn refused(text: &str) -> String {
    History::from_json(text)
        .expect_err("the reader refuses this document")
        .to_string()
}

fn publisher_document(operation: &str) -> String {
    format!(r#"{{"version":1,"retention":2,"publisher":[{operation}],"readers":[]}}"#)
}

#[test]
fn the_reader_refuses_what_is_not_the_form() {
    assert!(refused("not json").contains("not a broadcast-log history"));
    assert!(
        refused(r#"{"version":2,"retention":2,"publisher":[],"readers":[]}"#).contains(&format!(
            "is not the version this reader reads ({HISTORY_FORMAT_VERSION})"
        ))
    );
    assert!(refused(r#"{"version":1,"publisher":[],"readers":[]}"#).contains("retention"));
    // An operation that cannot be one, each with the place it is at.
    assert!(refused(&publisher_document(
        r#"{"call":"publish","seq":0,"invoked":1,"returned":2}"#
    ))
    .contains("publisher[0]: names no `value`, which this call takes"));
    assert!(refused(&publisher_document(
        r#"{"call":"publish","value":1,"seq":0,"cursor":0,"invoked":1,"returned":2}"#
    ))
    .contains("names `cursor`, which this call does not take"));
    assert!(refused(&publisher_document(
        r#"{"call":"open","cursor":0,"invoked":1,"returned":2}"#
    ))
    .contains("an open names where the cursor starts"));
    assert!(refused(&publisher_document(
        r#"{"call":"open","from":"oldest","at":3,"cursor":0,"invoked":1,"returned":2}"#
    ))
    .contains("names `at`, which this call does not take"));
    assert!(refused(&publisher_document(
        r#"{"call":"open","from":"at","cursor":0,"invoked":1,"returned":2}"#
    ))
    .contains("names no `at`, which this call takes"));
    assert!(refused(&publisher_document(
        r#"{"call":"read","invoked":1,"returned":2}"#
    ))
    .contains("a read is an item, empty or lagged"));
    assert!(refused(&publisher_document(
        r#"{"call":"read","outcome":"lagged","invoked":1,"returned":2}"#
    ))
    .contains("names no `skipped`, which this call takes"));
    assert!(
        refused(&publisher_document(
            r#"{"call":"publish","value":-1,"seq":0,"invoked":1,"returned":2}"#
        ))
        .contains("not a broadcast-log history"),
        "a negative value is not a value"
    );
}

/// The window is `retention` entries: an entry `retention` behind the head is
/// readable, one more behind is not, and a reader that finds itself one more
/// behind is told so with the distance to the window.
#[test]
fn the_window_is_exactly_the_retention() {
    let history = |read: Operation| History {
        retention: 2,
        publisher: vec![
            publish(10, 0, 1, 2),
            publish(11, 1, 3, 4),
            publish(12, 2, 5, 6),
        ],
        readers: vec![vec![open(OpenFrom::At(0), 0, 7, 8), read]],
    };
    // h = 3, retention 2: the window is [1, 3), so entry 0 is outside it.
    assert!(matches!(
        check(&history(item(0, 10, 9, 10))),
        Verdict::NotLinearizable { .. }
    ));
    assert_eq!(check(&history(lagged(1, 9, 10))), Verdict::Linearizable);
    assert!(matches!(
        check(&history(lagged(2, 9, 10))),
        Verdict::NotLinearizable { .. }
    ));
}

/// A read is decided by a load of the head, so it may take effect before a
/// publish that overlaps it moves the window on (ADR 0062 §5): the entry it
/// returns is then in the window it saw, even though by the time it has
/// returned the publish has pushed that entry out. The same read begun after
/// the publish returned has no such instant.
#[test]
fn a_read_may_take_effect_before_an_overlapping_publish_moves_the_window() {
    let history = |read_invoked, read_returned| History {
        retention: 1,
        publisher: vec![publish(10, 0, 1, 2), publish(11, 1, 5, 9)],
        readers: vec![vec![
            open(OpenFrom::Oldest, 0, 3, 4),
            item(0, 10, read_invoked, read_returned),
        ]],
    };
    assert_eq!(check(&history(6, 7)), Verdict::Linearizable);
    assert!(matches!(
        check(&history(10, 11)),
        Verdict::NotLinearizable { .. }
    ));
}

/// The late `lagged`: a publish that overlaps a read may move the window past
/// the reader's cursor before the read's instant, and the read then reports the
/// distance to the window as it was at that instant.
#[test]
fn a_read_may_find_itself_behind_a_window_an_overlapping_publish_moved() {
    let history = History {
        retention: 1,
        publisher: vec![publish(10, 0, 1, 2), publish(11, 1, 5, 9)],
        readers: vec![vec![open(OpenFrom::Oldest, 0, 3, 4), lagged(1, 6, 7)]],
    };
    assert_eq!(check(&history), Verdict::Linearizable);
}

/// Each reader keeps its own cursor and its own loss: one that fell behind and
/// one that did not are both judged against the one log.
#[test]
fn readers_do_not_share_a_cursor() {
    let history = History {
        retention: 2,
        publisher: vec![
            publish(10, 0, 1, 2),
            publish(11, 1, 3, 4),
            publish(12, 2, 5, 6),
        ],
        readers: vec![
            vec![
                open(OpenFrom::At(0), 0, 7, 8),
                lagged(1, 9, 10),
                item(1, 11, 11, 12),
            ],
            vec![
                open(OpenFrom::Oldest, 1, 13, 14),
                item(1, 11, 15, 16),
                item(2, 12, 17, 18),
            ],
        ],
    };
    assert_eq!(check(&history), Verdict::Linearizable);
}

/// A history may not name a retention of nothing.
#[test]
fn a_retention_of_zero_cannot_have_come_from_a_run() {
    let history = History {
        retention: 0,
        publisher: vec![publish(1, 0, 1, 2)],
        readers: vec![],
    };
    assert!(matches!(check(&history), Verdict::Malformed(_)));
}
