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
//! history for each part `docs/formal/EngineInbox.tla` takes out whose
//! absence a poster can see; a full inbox's refusal is the one part whose
//! absence tells no poster anything false.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use sce_build::inbox_history::{
    check, Delivery, Door, History, OwnerCall, OwnerOperation, Post, Refusal, Source, Verdict,
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
    } else if name == "not_a_history_without_sessions.json" {
        // Which sessions the tree holds decides whether a post naming one may
        // be refused as unknown, and the format refuses to guess it too.
        Some("not an inbox history")
    } else if name == "not_a_history_a_refusal_without_a_reason.json" {
        Some("posters[0][1]: a refused post says why it was refused")
    } else if name == "not_a_history_a_capacity_of_zero.json" {
        Some("`capacity` is 0")
    } else if name == "not_a_history_a_post_without_a_door.json" {
        // Which door a post went through decides whether it may be refused as
        // unknown once its session has ended, and the format refuses to guess.
        Some("missing field `door`")
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
        files.len() >= 50,
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

/// A post through the handle of its session's own inbox.
fn post(session: u64, input: u64, outcome: Delivery, invoked: u64, returned: u64) -> Post {
    Post {
        session,
        input,
        door: Door::Handle,
        outcome,
        invoked,
        returned,
    }
}

/// A post through the tree's directory, by the id of the session it names.
fn addressed(session: u64, input: u64, outcome: Delivery, invoked: u64, returned: u64) -> Post {
    Post {
        door: Door::Addressed,
        ..post(session, input, outcome, invoked, returned)
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
        sessions: vec![big, 1],
        closes: false,
        capacity: Some(big),
        owner: vec![
            enqueue(big, big, Source::Posted, 3, 4),
            enqueue(big, 9, Source::Own, 5, 6),
            end(big, 7, 8),
        ],
        posters: vec![
            vec![post(big, big, Delivery::Taken, 1, 2)],
            vec![],
            vec![
                post(big, 2, Delivery::Refused(Refusal::Ended), 9, 10),
                addressed(big + 1, 3, Delivery::Refused(Refusal::Unknown), 11, 12),
                post(1, 4, Delivery::Refused(Refusal::Full), 13, 14),
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
    assert!(!read.closes);
    assert_eq!(read.sessions, history.sessions);
    assert_eq!(read.capacity, history.capacity);
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
    format!(r#"{{"version":2,"sessions":[0],"closes":true,"owner":[{operation}],"posters":[]}}"#)
}

#[test]
fn the_reader_refuses_what_is_not_the_form() {
    assert!(refused("not json").contains("not an inbox history"));
    assert!(
        refused(r#"{"version":3,"sessions":[0],"closes":true,"owner":[],"posters":[]}"#).contains(
            &format!("is not the version this reader reads ({HISTORY_FORMAT_VERSION})")
        )
    );
    assert!(refused(r#"{"version":2,"sessions":[0],"owner":[],"posters":[]}"#).contains("closes"));
    assert!(
        refused(r#"{"version":2,"sessions":[0],"closes":true,"owner":[],"posters":[null]}"#)
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
            r#"{"call":"post","session":0,"door":"handle","input":1,"outcome":"taken","invoked":1,"returned":2}"#
        ))
        .contains("not an inbox history"),
        "a post is a poster's"
    );
    assert!(
        refused(
            r#"{"version":2,"sessions":[0],"closes":true,"owner":[],"posters":[[{"call":"end","session":0,"invoked":1,"returned":2}]]}"#
        )
        .contains("not an inbox history"),
        "an end is the owner's"
    );
    assert!(
        refused(
            r#"{"version":2,"sessions":[0],"closes":true,"owner":[],"posters":[[{"call":"post","session":0,"door":"handle","input":-1,"outcome":"taken","invoked":1,"returned":2}]]}"#
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
        sessions: vec![0, 1],
        closes: true,
        capacity: None,
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
        sessions: vec![0, 1],
        closes: true,
        capacity: None,
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
        sessions: vec![0, 1],
        closes: true,
        capacity: None,
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
        sessions: vec![0, 1],
        closes: true,
        capacity: None,
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
        sessions: vec![0, 1],
        closes: true,
        capacity: None,
        owner: vec![end(0, 1, 2), enqueue(0, 1, Source::Posted, 5, 6)],
        posters: vec![vec![post(0, 1, Delivery::Refused(Refusal::Ended), 3, 4)]],
    };
    assert!(is_not_linearizable(&refused_entered));
}

/// A post is refused only once its session has ended, and only by a runtime
/// that closes; a runtime that closes refuses every post after the end.
#[test]
fn a_post_is_refused_only_after_the_end_and_only_where_the_runtime_closes() {
    let after_end = |closes: bool, outcome: Delivery| History {
        sessions: vec![0, 1],
        closes,
        capacity: None,
        owner: match outcome {
            Delivery::Taken => vec![end(0, 1, 2), enqueue(0, 1, Source::Posted, 5, 6)],
            Delivery::Refused(_) => vec![end(0, 1, 2)],
        },
        posters: vec![vec![post(0, 1, outcome, 3, 4)]],
    };
    assert_eq!(
        check(&after_end(true, Delivery::Refused(Refusal::Ended))),
        Verdict::Linearizable
    );
    assert!(is_not_linearizable(&after_end(true, Delivery::Taken)));
    assert_eq!(
        check(&after_end(false, Delivery::Taken)),
        Verdict::Linearizable
    );
    assert!(is_not_linearizable(&after_end(
        false,
        Delivery::Refused(Refusal::Ended)
    )));
    let before_end = History {
        sessions: vec![0, 1],
        closes: true,
        capacity: None,
        owner: vec![end(0, 3, 4)],
        posters: vec![vec![post(0, 1, Delivery::Refused(Refusal::Ended), 1, 2)]],
    };
    assert!(is_not_linearizable(&before_end));
    // The end of another session refuses nothing of this one.
    let other_session_ended = History {
        sessions: vec![0, 1],
        closes: true,
        capacity: None,
        owner: vec![end(1, 1, 2)],
        posters: vec![vec![post(0, 1, Delivery::Refused(Refusal::Ended), 3, 4)]],
    };
    assert!(is_not_linearizable(&other_session_ended));
}

/// A post through the directory naming a session the tree does not hold is
/// refused as unknown, and never taken; a session the tree holds is unknown to
/// the directory only once its run has ended, since it leaves the directory
/// then, whether the runtime closes or not. A post through a handle is never
/// unknown.
#[test]
fn a_post_naming_a_session_the_tree_does_not_hold_is_refused_as_unknown() {
    let posted = |session: u64, outcome: Delivery| History {
        sessions: vec![0],
        closes: true,
        capacity: None,
        owner: vec![enqueue(0, 100, Source::Own, 3, 4)],
        posters: vec![vec![addressed(session, 1, outcome, 1, 2)]],
    };
    assert_eq!(
        check(&posted(7, Delivery::Refused(Refusal::Unknown))),
        Verdict::Linearizable
    );
    assert!(is_not_linearizable(&posted(7, Delivery::Taken)));
    assert!(is_not_linearizable(&posted(
        7,
        Delivery::Refused(Refusal::Ended)
    )));
    assert!(is_not_linearizable(&posted(
        7,
        Delivery::Refused(Refusal::Full)
    )));
    assert!(is_not_linearizable(&posted(
        0,
        Delivery::Refused(Refusal::Unknown)
    )));

    let after_end = |closes: bool, door: Door| History {
        sessions: vec![0],
        closes,
        capacity: None,
        owner: vec![end(0, 1, 2)],
        posters: vec![vec![Post {
            door,
            ..post(0, 1, Delivery::Refused(Refusal::Unknown), 3, 4)
        }]],
    };
    assert_eq!(
        check(&after_end(true, Door::Addressed)),
        Verdict::Linearizable
    );
    assert_eq!(
        check(&after_end(false, Door::Addressed)),
        Verdict::Linearizable
    );
    assert!(is_not_linearizable(&after_end(true, Door::Handle)));
    assert!(is_not_linearizable(&after_end(false, Door::Handle)));
    // Before the end the session is in the directory.
    let before_end = History {
        sessions: vec![0],
        closes: false,
        capacity: None,
        owner: vec![end(0, 3, 4)],
        posters: vec![vec![addressed(
            0,
            1,
            Delivery::Refused(Refusal::Unknown),
            1,
            2,
        )]],
    };
    assert!(is_not_linearizable(&before_end));
}

/// A handle is a session's own, so a history whose post went through the
/// handle of a session the tree does not hold cannot have come from a run.
#[test]
fn a_handle_names_a_session_the_tree_holds() {
    let history = History {
        sessions: vec![0],
        closes: true,
        capacity: None,
        owner: vec![],
        posters: vec![vec![post(7, 1, Delivery::Refused(Refusal::Unknown), 1, 2)]],
    };
    assert!(matches!(
        check(&history),
        Verdict::Malformed(why) if why.contains("posts through the handle of session 7")
    ));
}

/// A bounded inbox refuses a post as full when its session holds the capacity
/// of inputs taken and not yet entered (here no other post is under way, which
/// would count too), and never with room left, without a
/// bound, or once the session has ended where the runtime closes. A post it
/// takes past that count is taken: a drain frees room before it enters what it
/// took out.
#[test]
fn a_full_inbox_refuses_and_only_when_full() {
    let full = Delivery::Refused(Refusal::Full);
    let observed = |capacity: Option<u64>, second: Delivery| History {
        sessions: vec![0],
        closes: true,
        capacity,
        owner: match second {
            Delivery::Taken => vec![
                enqueue(0, 1, Source::Posted, 5, 6),
                enqueue(0, 2, Source::Posted, 7, 8),
            ],
            Delivery::Refused(_) => vec![enqueue(0, 1, Source::Posted, 5, 6)],
        },
        posters: vec![vec![
            post(0, 1, Delivery::Taken, 1, 2),
            post(0, 2, second, 3, 4),
        ]],
    };
    assert_eq!(check(&observed(Some(1), full)), Verdict::Linearizable);
    assert!(is_not_linearizable(&observed(Some(2), full)));
    assert!(is_not_linearizable(&observed(None, full)));
    assert_eq!(
        check(&observed(Some(1), Delivery::Taken)),
        Verdict::Linearizable
    );

    // The input entered before the second post was invoked: the inbox had room.
    let after_room = History {
        sessions: vec![0],
        closes: true,
        capacity: Some(1),
        owner: vec![enqueue(0, 1, Source::Posted, 3, 4)],
        posters: vec![vec![
            post(0, 1, Delivery::Taken, 1, 2),
            post(0, 2, full, 5, 6),
        ]],
    };
    assert!(is_not_linearizable(&after_room));

    // Once the session has ended and the runtime closed its inbox, a post is
    // refused for that, whatever its session still holds.
    let after_end = History {
        sessions: vec![0],
        closes: true,
        capacity: Some(1),
        owner: vec![end(0, 3, 4), enqueue(0, 1, Source::Posted, 7, 8)],
        posters: vec![vec![
            post(0, 1, Delivery::Taken, 1, 2),
            post(0, 2, full, 5, 6),
        ]],
    };
    assert!(is_not_linearizable(&after_end));
}

/// docs/adr/0147, D1: a post under way may hold storage before it is taken, so
/// a refusal as full counts the other posts to its session that overlap it and
/// are placed after it with what the session holds, and only posts to its
/// session. Post 1 is under way from 1 to 20 and enters last; post 4 is refused
/// while its session holds input 3 alone.
#[test]
fn a_full_inbox_counts_the_posts_to_it_under_way() {
    let full = Delivery::Refused(Refusal::Full);
    let run = |capacity: u64, under_way_to: u64| History {
        sessions: vec![0, 1],
        closes: true,
        capacity: Some(capacity),
        owner: vec![
            enqueue(0, 2, Source::Posted, 4, 5),
            enqueue(0, 3, Source::Posted, 12, 13),
            enqueue(0, 5, Source::Posted, 16, 17),
            enqueue(under_way_to, 1, Source::Posted, 21, 22),
        ],
        posters: vec![
            vec![post(under_way_to, 1, Delivery::Taken, 1, 20)],
            vec![
                post(0, 2, Delivery::Taken, 2, 3),
                post(0, 3, Delivery::Taken, 6, 7),
            ],
            vec![post(0, 4, full, 8, 9), post(0, 5, Delivery::Taken, 14, 15)],
        ],
    };
    assert_eq!(
        check(&run(2, 0)),
        Verdict::Linearizable,
        "one input held and one post to the session under way did not reach a capacity of two"
    );
    assert!(
        is_not_linearizable(&run(3, 0)),
        "one input held and one post under way reached a capacity of three"
    );
    assert!(
        is_not_linearizable(&run(2, 1)),
        "a post to another session under way was counted toward this session's capacity"
    );
}

/// A refused post never takes effect, whatever its reason: its input enters
/// no queue.
#[test]
fn a_refused_post_never_takes_effect() {
    let full = History {
        sessions: vec![0],
        closes: true,
        capacity: Some(1),
        owner: vec![
            enqueue(0, 1, Source::Posted, 5, 6),
            enqueue(0, 2, Source::Posted, 7, 8),
        ],
        posters: vec![vec![
            post(0, 1, Delivery::Taken, 1, 2),
            post(0, 2, Delivery::Refused(Refusal::Full), 3, 4),
        ]],
    };
    assert!(is_not_linearizable(&full));
    let unknown = History {
        sessions: vec![0],
        closes: true,
        capacity: None,
        owner: vec![enqueue(0, 1, Source::Posted, 3, 4)],
        posters: vec![vec![addressed(
            7,
            1,
            Delivery::Refused(Refusal::Unknown),
            1,
            2,
        )]],
    };
    assert!(is_not_linearizable(&unknown));
}

/// The owner runs only the sessions the tree holds, and a history that says
/// it ran another cannot have come from a run.
#[test]
fn the_owner_runs_only_the_sessions_the_tree_holds() {
    let history = History {
        sessions: vec![0],
        closes: true,
        capacity: None,
        owner: vec![enqueue(0, 100, Source::Own, 1, 2), end(1, 3, 4)],
        posters: vec![],
    };
    assert!(matches!(
        check(&history),
        Verdict::Malformed(why)
            if why.contains("participant 0 operation 1") && why.contains("on session 1")
    ));
}

/// The tree and the bound are part of the form: the reader refuses a history
/// that lists no session or one twice, gives a capacity of 0, or gives a post
/// a reason that does not match its outcome.
#[test]
fn the_reader_refuses_a_tree_or_a_refusal_it_cannot_judge() {
    let with = |head: &str, posted: &str| {
        format!(r#"{{"version":2,{head},"owner":[],"posters":[[{posted}]]}}"#)
    };
    let taken = r#"{"call":"post","session":0,"door":"handle","input":1,"outcome":"taken","invoked":1,"returned":2}"#;
    assert!(refused(&with(r#""sessions":[],"closes":true"#, taken))
        .contains("`sessions` lists no session"));
    assert!(refused(&with(r#""sessions":[0,3,0],"closes":true"#, taken))
        .contains("`sessions` lists session 0 twice"));
    assert!(refused(&with(r#""closes":true"#, taken)).contains("sessions"));
    assert!(
        refused(&with(r#""sessions":[0],"closes":true,"capacity":0"#, taken))
            .contains("`capacity` is 0")
    );
    let tree = r#""sessions":[0],"closes":true"#;
    assert!(refused(&with(
        tree,
        r#"{"call":"post","session":0,"door":"handle","input":1,"outcome":"refused","invoked":1,"returned":2}"#
    ))
    .contains("posters[0][0]: a refused post says why it was refused"));
    assert!(refused(&with(
        tree,
        r#"{"call":"post","session":0,"door":"handle","input":1,"outcome":"taken","reason":"full","invoked":1,"returned":2}"#
    ))
    .contains("posters[0][0]: a taken post gives no `reason`"));
    assert!(refused(&with(
        tree,
        r#"{"call":"post","session":0,"door":"handle","input":1,"outcome":"refused","reason":"busy","invoked":1,"returned":2}"#
    ))
    .contains("not an inbox history"));

    let read = History::from_json(&with(
        r#""sessions":[0],"closes":true,"capacity":2"#,
        r#"{"call":"post","session":0,"door":"handle","input":1,"outcome":"refused","reason":"full","invoked":1,"returned":2}"#,
    ))
    .expect("a bounded tree with a refusal and its reason reads");
    assert_eq!(read.sessions, vec![0]);
    assert_eq!(read.capacity, Some(2));
    assert_eq!(read.posters[0][0].outcome, Delivery::Refused(Refusal::Full));
}

/// An own event enters once and before its session ends, and a session ends
/// once.
#[test]
fn an_own_event_enters_once_before_the_end_and_a_session_ends_once() {
    let owner_only = |owner| History {
        sessions: vec![0, 1],
        closes: true,
        capacity: None,
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
        sessions: vec![0, 1],
        closes: true,
        capacity: None,
        owner: vec![enqueue(0, 1, Source::Posted, 5, 6)],
        posters: vec![
            vec![post(0, 1, Delivery::Taken, 1, 2)],
            vec![post(0, 1, Delivery::Refused(Refusal::Ended), 3, 4)],
        ],
    };
    assert!(matches!(
        check(&history),
        Verdict::Malformed(why) if why.contains("the inputs of a run are distinct")
    ));
}
