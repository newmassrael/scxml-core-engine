// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The owner's answers to the questions a model leaves open: a chain of their own
//! beside the text and the model, saved by the same code and read back as the map
//! from a question's id to the owner's words.

mod common;

use sce_app_core::{call, CommandError, FixedClock, Revision, WorkId, WorkStore};
use serde_json::{json, Value};

use common::{scratch, FakeRenderer};

const T1: &str = "2026-10-03T09:00:01Z";
const T2: &str = "2026-10-03T09:00:02Z";

fn store_at(root: &std::path::Path, now: &str) -> WorkStore<FixedClock> {
    WorkStore::with_clock(root, FixedClock(now.to_string()))
}

fn run(store: &WorkStore<FixedClock>, name: &str, args: Value) -> Result<Value, CommandError> {
    call(store, &FakeRenderer, name, args)
}

fn revision_of(answer: &Value) -> Revision {
    serde_json::from_value(answer["revision"].clone()).expect("a revision")
}

fn work(store: &WorkStore<FixedClock>) -> WorkId {
    store.create_work("Door lock").unwrap().id
}

/// A work the owner has answered nothing of has no answers to read, and the three
/// chains are separate: saving answers moves neither the text nor the model.
#[test]
fn a_work_has_no_answers_until_the_owner_saves_some_and_the_chains_are_separate() {
    let root = scratch("answers-separate");
    let store = store_at(&root, T1);
    let id = work(&store);
    store.save_source(&id, "The door opens.", None).unwrap();
    assert_eq!(
        run(&store, "read_answers", json!({"id": id.as_str()})).unwrap(),
        json!({"answers": null})
    );

    let saved = run(
        &store,
        "save_answers",
        json!({"id": id.as_str(), "answers": {"open-guard": "Any card on the list."}}),
    )
    .unwrap();
    assert_eq!(saved["outcome"], "saved");

    let read = run(&store, "read_answers", json!({"id": id.as_str()})).unwrap();
    assert_eq!(read["answers"]["revision"], saved["revision"]);
    assert_eq!(
        read["answers"]["entries"],
        json!({"open-guard": {"answer": "Any card on the list.", "answered_at": T1}})
    );
    assert_eq!(
        store.history(&id).unwrap().len(),
        1,
        "the text did not move"
    );
    assert_eq!(store.model_head(&id).unwrap(), None, "no model appeared");
}

/// An answer is stamped when its words change; saving the others again carries
/// their stamps, and saving what is already saved is no change at all.
#[test]
fn an_answer_keeps_the_time_it_was_said_when_others_are_saved_around_it() {
    let root = scratch("answers-stamps");
    let first_clock = store_at(&root, T1);
    let id = work(&first_clock);
    let first = run(
        &first_clock,
        "save_answers",
        json!({"id": id.as_str(), "answers": {"a": "yes", "b": "no"}}),
    )
    .unwrap();

    let later = store_at(&root, T2);
    let unchanged = run(
        &later,
        "save_answers",
        json!({"id": id.as_str(), "answers": {"b": "no", "a": "yes"}, "base": first["revision"]}),
    )
    .unwrap();
    assert_eq!(unchanged["outcome"], "unchanged", "same words, same bytes");

    let second = run(
        &later,
        "save_answers",
        json!({"id": id.as_str(), "answers": {"a": "yes", "b": "never"}, "base": first["revision"]}),
    )
    .unwrap();
    assert_eq!(second["outcome"], "saved");
    let entries = run(&later, "read_answers", json!({"id": id.as_str()})).unwrap()["answers"]
        ["entries"]
        .clone();
    assert_eq!(entries["a"]["answered_at"], T1, "carried, not new");
    assert_eq!(entries["b"]["answered_at"], T2, "its words changed");
    assert_eq!(entries["b"]["answer"], "never");
}

/// A question left out is no longer answered, and the earlier answers stay in
/// the history as the revision they were.
#[test]
fn leaving_a_question_out_unanswers_it_and_the_earlier_answers_stay_readable() {
    let root = scratch("answers-unanswer");
    let store = store_at(&root, T1);
    let id = work(&store);
    let first = run(
        &store,
        "save_answers",
        json!({"id": id.as_str(), "answers": {"a": "1", "b": "2"}}),
    )
    .unwrap();
    run(
        &store,
        "save_answers",
        json!({"id": id.as_str(), "answers": {"a": "1"}, "base": first["revision"]}),
    )
    .unwrap();

    let now = run(&store, "read_answers", json!({"id": id.as_str()})).unwrap();
    assert_eq!(now["answers"]["entries"].as_object().unwrap().len(), 1);
    let then = run(
        &store,
        "read_answers",
        json!({"id": id.as_str(), "revision": first["revision"]}),
    )
    .unwrap();
    assert_eq!(then["answers"]["entries"].as_object().unwrap().len(), 2);
}

/// Answers are saved from the revision the owner saw, as a text is: a stale base
/// is refused with both revisions, and nothing is written.
#[test]
fn answers_saved_from_a_stale_base_are_refused() {
    let root = scratch("answers-conflict");
    let store = store_at(&root, T1);
    let id = work(&store);
    let first = run(
        &store,
        "save_answers",
        json!({"id": id.as_str(), "answers": {"a": "1"}}),
    )
    .unwrap();
    // Another entrance saves on top.
    let second = run(
        &store,
        "save_answers",
        json!({"id": id.as_str(), "answers": {"a": "2"}, "base": first["revision"]}),
    )
    .unwrap();

    let stale = run(
        &store,
        "save_answers",
        json!({"id": id.as_str(), "answers": {"a": "mine"}, "base": first["revision"]}),
    )
    .unwrap_err();
    assert_eq!(stale.kind, "conflict");
    assert_eq!(stale.detail["current"], second["revision"]);

    // A writer that never read has not seen what it would replace.
    let blind = run(
        &store,
        "save_answers",
        json!({"id": id.as_str(), "answers": {"a": "mine"}}),
    )
    .unwrap_err();
    assert_eq!(blind.kind, "conflict");

    let still = run(&store, "read_answers", json!({"id": id.as_str()})).unwrap();
    assert_eq!(still["answers"]["entries"]["a"]["answer"], "2");
    assert_eq!(revision_of(&still["answers"]), revision_of(&second));
}

/// What is not an answer is refused as such, with the reason, and writes nothing.
#[test]
fn what_is_not_an_answer_is_refused_and_writes_nothing() {
    let root = scratch("answers-invalid");
    let store = store_at(&root, T1);
    let id = work(&store);
    for (answers, wanted) in [
        (json!({"a": ""}), "is empty"),
        (json!({"a": "   "}), "is empty"),
        (json!({"has space": "yes"}), "space"),
        (json!({"": "yes"}), "1 to"),
    ] {
        let refused = run(
            &store,
            "save_answers",
            json!({"id": id.as_str(), "answers": answers}),
        )
        .unwrap_err();
        assert_eq!(refused.kind, "invalid-answers", "{answers}");
        assert!(refused.message.contains(wanted), "{}", refused.message);
    }
    // Not a map of words at all is a malformed request, as for any command.
    for answers in [json!(["a"]), json!({"a": 3}), json!("a")] {
        let refused = run(
            &store,
            "save_answers",
            json!({"id": id.as_str(), "answers": answers}),
        )
        .unwrap_err();
        assert_eq!(refused.kind, "bad-request", "{answers}");
    }
    assert_eq!(
        run(&store, "read_answers", json!({"id": id.as_str()})).unwrap(),
        json!({"answers": null})
    );
}

/// An argument nothing reads is refused, a work that was removed has no answers,
/// and answers that are not the store's own document are corrupt, not guessed at.
#[test]
fn the_commands_refuse_what_they_do_not_read_and_a_damaged_document() {
    let root = scratch("answers-refusals");
    let store = store_at(&root, T1);
    let id = work(&store);
    let unknown = run(
        &store,
        "save_answers",
        json!({"id": id.as_str(), "answers": {}, "colour": "blue"}),
    )
    .unwrap_err();
    assert_eq!(unknown.kind, "bad-request");

    let saved = run(
        &store,
        "save_answers",
        json!({"id": id.as_str(), "answers": {"a": "1"}}),
    )
    .unwrap();
    // Damage the saved file the way a hand edit would: its bytes no longer hash to
    // its name, which the store refuses before the document is even read.
    let file = root
        .join(id.as_str())
        .join("answers")
        .join(format!("{}.json", saved["revision"].as_str().unwrap()));
    std::fs::write(&file, "{\"format\":\"sce-answers\"}").unwrap();
    let damaged = run(&store, "read_answers", json!({"id": id.as_str()})).unwrap_err();
    assert_eq!(damaged.kind, "corrupt");

    store.remove_work(&id).unwrap();
    let gone = run(&store, "read_answers", json!({"id": id.as_str()})).unwrap_err();
    assert_eq!(gone.kind, "not-found");
    assert!(gone.message.contains("removed"), "{}", gone.message);
}
