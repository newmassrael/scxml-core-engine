//! The JSON form of an inbox history and the command that judges it
//! (docs/adr/0147, D3).
//!
//! The checker is one search written once, against the sequential reading of
//! `docs/formal/InboxService.tla`; what each runtime adds is an inbox
//! conformance test that writes its observations in this form. So the form is
//! held from both sides here, as the queue history's is: the real
//! `sce-codegen check-inbox-history` judges every fixture history by the name
//! the fixture carries, the schema that documents the form accepts exactly the
//! histories the reader accepts, and a history written by the reader's own
//! writer reads back the same. The fixtures `not_linearizable_*` hold one
//! history for each part `docs/formal/EngineInbox.tla` takes out.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use sce_build::inbox_history::{
    check, Delivery, History, OwnerCall, OwnerOperation, Post, Source, Verdict,
    HISTORY_FORMAT_VERSION,
};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent directory")
        .to_path_buf()
}

fn histories_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/inbox_histories")
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
        .arg("check-inbox-history")
        .args(files)
        .output()
        .expect("run sce-codegen check-inbox-history")
}

/// What a fixture's name says the judgement is: the prefix, and a phrase the
/// refusal must contain.
fn expectation(name: &str) -> Option<&'static str> {
    if name.starts_with("linearizable_") {
        None
    } else if name.starts_with("not_linearizable_") {
        Some("is not linearizable")
    } else if name == "malformed_an_input_posted_twice.json" {
        Some("the inputs of a run are distinct")
    } else if name.starts_with("malformed_") {
        Some("cannot have come from a run")
    } else if name == "not_a_history_unknown_field.json" {
        Some("not an inbox history")
    } else if name == "not_a_history_an_enqueue_without_a_source.json" {
        Some("owner[0]: an enqueue names where its item came from")
    } else if name == "not_a_history_a_post_by_the_owner.json" {
        // A post is a poster's, and the owner's list has no shape for one.
        Some("not an inbox history")
    } else if name == "not_a_history_without_closes.json" {
        // Whether the runtime closes decides what a refusal means, and the
        // format refuses to guess it.
        Some("not an inbox history")
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
        files.len() >= 29,
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

/// Every file is judged and a line printed for each, then the command fails.
#[test]
fn a_command_over_many_files_says_which_failed() {
    let dir = histories_dir();
    let good = dir.join("linearizable_sequential.json");
    let bad = dir.join("not_linearizable_two_entered_in_reverse.json");
    let run = judge(&[bad.as_path(), good.as_path()]);
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(
        !run.status.success(),
        "one refused history fails the command"
    );
    assert!(
        stdout.contains("FAIL ") && stdout.contains("not_linearizable_two_entered_in_reverse.json"),
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

/// The schema is as strict as the reader: a history the reader takes
/// validates against it, and one the reader refuses for its shape does not.
#[test]
fn the_schema_accepts_what_the_reader_accepts() {
    let schema: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(
            repo_root().join("tests/forge/conformance/inbox_history.schema.json"),
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

fn post(session: u64, input: u64, outcome: Delivery, invoked: u64, returned: u64) -> Post {
    Post {
        session,
        input,
        outcome,
        invoked,
        returned,
    }
}

fn enqueue(session: u64, item: u64, source: Source, invoked: u64, returned: u64) -> OwnerOperation {
    OwnerOperation {
        call: OwnerCall::Enqueue {
            session,
            item,
            source,
        },
        invoked,
        returned,
    }
}

fn end(session: u64, invoked: u64, returned: u64) -> OwnerOperation {
    OwnerOperation {
        call: OwnerCall::End { session },
        invoked,
        returned,
    }
}

fn is_not_linearizable(history: &History) -> bool {
    matches!(check(history), Verdict::NotLinearizable { .. })
}

/// What the writer writes, the reader reads back, including values beyond
/// 2^53 that a writer in a language with doubles would lose.
#[test]
fn a_written_history_reads_back_the_same() {
    let big = (1u64 << 60) + 7;
    let history = History {
        closes: false,
        owner: vec![
            enqueue(big, big, Source::Posted, 3, 4),
            enqueue(big, 9, Source::Own, 5, 6),
            end(big, 7, 8),
        ],
        posters: vec![
            vec![post(big, big, Delivery::Taken, 1, 2)],
            vec![],
            vec![post(big, 2, Delivery::Refused, 9, 10)],
        ],
    };
    let written = history.to_json();
    let read = History::from_json(&written).expect("the writer's output reads");
    assert_eq!(
        read.to_json(),
        written,
        "writing what was read gives the same text"
    );
    assert!(!read.closes);
    assert_eq!(read.owner, history.owner);
    assert_eq!(read.posters, history.posters);
    assert_eq!(check(&read), check(&history));
}

fn refused(text: &str) -> String {
    History::from_json(text)
        .expect_err("the reader refuses this document")
        .to_string()
}

fn with_owner(operation: &str) -> String {
    format!(r#"{{"version":1,"closes":true,"owner":[{operation}],"posters":[]}}"#)
}

#[test]
fn the_reader_refuses_what_is_not_the_form() {
    assert!(refused("not json").contains("not an inbox history"));
    assert!(
        refused(r#"{"version":2,"closes":true,"owner":[],"posters":[]}"#).contains(&format!(
            "is not the version this reader reads ({HISTORY_FORMAT_VERSION})"
        ))
    );
    assert!(refused(r#"{"version":1,"owner":[],"posters":[]}"#).contains("closes"));
    assert!(
        refused(r#"{"version":1,"closes":true,"owner":[],"posters":[null]}"#)
            .contains("not an inbox history"),
        "a poster that made no attempt is an empty list, not null"
    );
    // An operation of the owner that cannot be one, with the place it is at.
    assert!(refused(&with_owner(
        r#"{"call":"enqueue","session":0,"source":"own","invoked":1,"returned":2}"#
    ))
    .contains("owner[0]: an enqueue names the item that entered"));
    assert!(refused(&with_owner(
        r#"{"call":"enqueue","session":0,"item":1,"invoked":1,"returned":2}"#
    ))
    .contains("owner[0]: an enqueue names where its item came from"));
    assert!(refused(&with_owner(
        r#"{"call":"end","session":0,"item":1,"invoked":1,"returned":2}"#
    ))
    .contains("owner[0]: an end names its session only"));
    assert!(refused(&with_owner(
        r#"{"call":"end","session":0,"source":"posted","invoked":1,"returned":2}"#
    ))
    .contains("owner[0]: an end names its session only"));
    assert!(
        refused(&with_owner(
            r#"{"call":"post","session":0,"input":1,"outcome":"taken","invoked":1,"returned":2}"#
        ))
        .contains("not an inbox history"),
        "a post is a poster's"
    );
    assert!(
        refused(
            r#"{"version":1,"closes":true,"owner":[],"posters":[[{"call":"end","session":0,"invoked":1,"returned":2}]]}"#
        )
        .contains("not an inbox history"),
        "an end is the owner's"
    );
    assert!(
        refused(
            r#"{"version":1,"closes":true,"owner":[],"posters":[[{"call":"post","session":0,"input":-1,"outcome":"taken","invoked":1,"returned":2}]]}"#
        )
        .contains("not an inbox history"),
        "a negative input is not an input"
    );
}

/// An own event enters behind every post taken before it, and a post still in
/// flight may linearize after it: the drain before an own event (ADR 0084, D2
/// as amended at step 9) is what the order asks, and no more.
#[test]
fn an_own_event_enters_behind_the_posts_taken_before_it() {
    let observed = |post_returned| History {
        closes: true,
        owner: vec![
            enqueue(0, 100, Source::Own, 3, 4),
            enqueue(0, 1, Source::Posted, 6, 7),
        ],
        posters: vec![vec![post(0, 1, Delivery::Taken, 1, post_returned)]],
    };
    // The post returned before the own event was invoked: it was taken first,
    // and entered after it.
    assert!(is_not_linearizable(&observed(2)));
    // The post overlaps the own event and may have been taken after it.
    assert_eq!(check(&observed(5)), Verdict::Linearizable);
}

/// Each session's queue takes its posts in the order they linearized: posts
/// that do not overlap enter in their real-time order, and posts that overlap
/// in either.
#[test]
fn posts_enter_in_the_order_they_linearized() {
    let observed = |first: (u64, u64), second: (u64, u64)| History {
        closes: true,
        owner: vec![
            enqueue(0, 2, Source::Posted, 5, 6),
            enqueue(0, 1, Source::Posted, 7, 8),
        ],
        posters: vec![
            vec![post(0, 1, Delivery::Taken, first.0, first.1)],
            vec![post(0, 2, Delivery::Taken, second.0, second.1)],
        ],
    };
    assert_eq!(check(&observed((1, 4), (2, 3))), Verdict::Linearizable);
    assert!(is_not_linearizable(&observed((1, 2), (3, 4))));
}

/// Two producers whose posts overlap their neighbours' all along, entered after
/// the last post in one order. A search that places overlapping posts in either
/// order and learns the order only when the enqueues are reached has about 2^30
/// waiting lists to try; the enqueue order fixes the order of the posts, and a
/// search that uses it finishes at once.
#[test]
fn the_enqueue_order_fixes_the_order_of_overlapping_posts() {
    const PER_POSTER: u64 = 30;
    let mut a = Vec::new();
    let mut b = Vec::new();
    for i in 0..PER_POSTER {
        a.push(post(0, 2 * i, Delivery::Taken, 10 * i + 1, 10 * i + 9));
        b.push(post(0, 2 * i + 1, Delivery::Taken, 10 * i + 6, 10 * i + 14));
    }
    let after = 10 * PER_POSTER + 20;
    let owner: Vec<OwnerOperation> = (0..2 * PER_POSTER)
        .map(|input| {
            enqueue(
                0,
                input,
                Source::Posted,
                after + 4 * input,
                after + 4 * input + 2,
            )
        })
        .collect();
    let history = History {
        closes: true,
        owner,
        posters: vec![a, b],
    };
    assert_eq!(check(&history), Verdict::Linearizable);

    // Two inputs swapped across posts that do not overlap (10 returned before
    // 14 was invoked) are still refused.
    let mut swapped = history;
    swapped.owner[10].call = OwnerCall::Enqueue {
        session: 0,
        item: 14,
        source: Source::Posted,
    };
    swapped.owner[14].call = OwnerCall::Enqueue {
        session: 0,
        item: 10,
        source: Source::Posted,
    };
    assert!(is_not_linearizable(&swapped));
}

/// A taken input enters exactly once, into the session it was posted to, and a
/// refused one never; a history ends with nothing taken left out.
#[test]
fn a_taken_input_enters_once_and_a_refused_one_never() {
    let once = History {
        closes: true,
        owner: vec![enqueue(0, 1, Source::Posted, 3, 4)],
        posters: vec![vec![post(0, 1, Delivery::Taken, 1, 2)]],
    };
    assert_eq!(check(&once), Verdict::Linearizable);
    let twice = History {
        owner: vec![
            enqueue(0, 1, Source::Posted, 3, 4),
            enqueue(0, 1, Source::Posted, 5, 6),
        ],
        ..once.clone()
    };
    assert!(is_not_linearizable(&twice));
    let never = History {
        owner: vec![end(0, 3, 4)],
        ..once.clone()
    };
    assert!(is_not_linearizable(&never));
    let elsewhere = History {
        owner: vec![enqueue(1, 1, Source::Posted, 3, 4)],
        ..once
    };
    assert!(is_not_linearizable(&elsewhere));
    let refused_entered = History {
        closes: true,
        owner: vec![end(0, 1, 2), enqueue(0, 1, Source::Posted, 5, 6)],
        posters: vec![vec![post(0, 1, Delivery::Refused, 3, 4)]],
    };
    assert!(is_not_linearizable(&refused_entered));
}

/// A post is refused only once its session has ended, and only by a runtime
/// that closes; a runtime that closes refuses every post after the end.
#[test]
fn a_post_is_refused_only_after_the_end_and_only_where_the_runtime_closes() {
    let after_end = |closes: bool, outcome: Delivery| History {
        closes,
        owner: match outcome {
            Delivery::Taken => vec![end(0, 1, 2), enqueue(0, 1, Source::Posted, 5, 6)],
            Delivery::Refused => vec![end(0, 1, 2)],
        },
        posters: vec![vec![post(0, 1, outcome, 3, 4)]],
    };
    assert_eq!(
        check(&after_end(true, Delivery::Refused)),
        Verdict::Linearizable
    );
    assert!(is_not_linearizable(&after_end(true, Delivery::Taken)));
    assert_eq!(
        check(&after_end(false, Delivery::Taken)),
        Verdict::Linearizable
    );
    assert!(is_not_linearizable(&after_end(false, Delivery::Refused)));
    let before_end = History {
        closes: true,
        owner: vec![end(0, 3, 4)],
        posters: vec![vec![post(0, 1, Delivery::Refused, 1, 2)]],
    };
    assert!(is_not_linearizable(&before_end));
    // The end of another session refuses nothing of this one.
    let other_session_ended = History {
        closes: true,
        owner: vec![end(1, 1, 2)],
        posters: vec![vec![post(0, 1, Delivery::Refused, 3, 4)]],
    };
    assert!(is_not_linearizable(&other_session_ended));
}

/// An own event enters once and before its session ends, and a session ends
/// once.
#[test]
fn an_own_event_enters_once_before_the_end_and_a_session_ends_once() {
    let owner_only = |owner| History {
        closes: true,
        owner,
        posters: vec![],
    };
    assert_eq!(
        check(&owner_only(vec![
            enqueue(0, 100, Source::Own, 1, 2),
            enqueue(1, 101, Source::Own, 3, 4),
            end(0, 5, 6),
            end(1, 7, 8),
        ])),
        Verdict::Linearizable
    );
    assert!(is_not_linearizable(&owner_only(vec![
        enqueue(0, 100, Source::Own, 1, 2),
        enqueue(0, 100, Source::Own, 3, 4),
    ])));
    assert!(is_not_linearizable(&owner_only(vec![
        end(0, 1, 2),
        enqueue(0, 100, Source::Own, 3, 4),
    ])));
    assert!(is_not_linearizable(&owner_only(vec![
        end(0, 1, 2),
        end(0, 3, 4)
    ])));
}

/// The inputs of a run are distinct, as the promise takes them, and a history
/// that posts one twice cannot be judged against it.
#[test]
fn an_input_posted_twice_is_malformed() {
    let history = History {
        closes: true,
        owner: vec![enqueue(0, 1, Source::Posted, 5, 6)],
        posters: vec![
            vec![post(0, 1, Delivery::Taken, 1, 2)],
            vec![post(0, 1, Delivery::Refused, 3, 4)],
        ],
    };
    assert!(matches!(
        check(&history),
        Verdict::Malformed(why) if why.contains("the inputs of a run are distinct")
    ));
}
