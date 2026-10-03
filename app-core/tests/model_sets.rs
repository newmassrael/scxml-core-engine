// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A model of several documents: a statechart and the event schemas it imports.
//!
//! The store still keeps ONE text for a revision. What is new is that the text may be
//! a set (a JSON object holding every document under its file name), that a model
//! saved before sets existed reads back exactly as it was written, and that the
//! product is asked about the whole set: each document staged under its own name,
//! beside the entry, where an import finds it.

mod common;

use sce_app_core::{call, CommandError, FixedClock, WorkId, WorkStore};
use serde_json::{json, Value};

use common::{scratch, FakeRenderer};

const DOOR: &str = "<scxml xmlns=\"http://www.w3.org/2005/07/scxml\" version=\"1.0\"/>";

fn store(label: &str) -> WorkStore<FixedClock> {
    WorkStore::with_clock(
        scratch(label),
        FixedClock("2026-10-03T09:00:00Z".to_string()),
    )
}

fn run(store: &WorkStore<FixedClock>, name: &str, args: Value) -> Result<Value, CommandError> {
    call(store, &FakeRenderer, name, args)
}

fn work(store: &WorkStore<FixedClock>) -> WorkId {
    store.create_work("Door lock").unwrap().id
}

fn set() -> Value {
    json!([
        {"name": "open.scxml", "text": "<event-schema name=\"open\"/>"},
        {"name": "door.scxml", "text": DOOR},
        {"name": "close.scxml", "text": "<event-schema name=\"close\"/>"},
    ])
}

/// A set is read back as the documents it was saved as: the entry first, the others
/// by name, however the caller listed them, with the entry's text beside them for a
/// screen that shows one.
#[test]
fn a_set_reads_back_as_its_documents_with_the_entry_first() {
    let store = store("sets-read");
    let id = work(&store);
    let saved = run(
        &store,
        "save_model",
        json!({"id": id.as_str(), "documents": set(), "entry": "door.scxml"}),
    )
    .unwrap();
    assert_eq!(saved["outcome"], "saved");

    let read = run(&store, "read_model", json!({"id": id.as_str()})).unwrap();
    let model = &read["model"];
    assert_eq!(model["revision"], saved["revision"]);
    assert_eq!(model["entry"], "door.scxml");
    assert_eq!(model["text"], DOOR);
    let names: Vec<&str> = model["documents"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["door.scxml", "close.scxml", "open.scxml"]);
    assert_eq!(read["standing"], "unstated");
}

/// The same documents are the same revision however they were listed, and saving them
/// again is no change.
#[test]
fn the_same_documents_are_the_same_revision_in_any_order() {
    let store = store("sets-order");
    let id = work(&store);
    let first = run(
        &store,
        "save_model",
        json!({"id": id.as_str(), "documents": set(), "entry": "door.scxml"}),
    )
    .unwrap();
    let mut reversed = set().as_array().unwrap().clone();
    reversed.reverse();
    let again = run(
        &store,
        "save_model",
        json!({"id": id.as_str(), "documents": reversed, "entry": "door.scxml",
               "base": first["revision"]}),
    )
    .unwrap();
    assert_eq!(again["outcome"], "unchanged");
}

/// A model of one document is the document itself: saved as `text` it is stored as the
/// text, read back under the name `model.scxml`, and a model saved by an older core
/// (the text, with nothing of a set about it) reads the same way.
#[test]
fn a_model_of_one_document_is_what_it_always_was() {
    let store = store("sets-single");
    let id = work(&store);
    run(
        &store,
        "save_model",
        json!({"id": id.as_str(), "text": DOOR}),
    )
    .unwrap();
    let head = store.model_head(&id).unwrap().unwrap();
    let stored = store.read_model(&id, Some(&head)).unwrap().unwrap();
    assert_eq!(
        stored.text, DOOR,
        "the store holds the document, not a wrapper"
    );

    let read = run(&store, "read_model", json!({"id": id.as_str()})).unwrap();
    assert_eq!(read["model"]["entry"], "model.scxml");
    assert_eq!(read["model"]["text"], DOOR);
    assert_eq!(
        read["model"]["documents"],
        json!([{"name": "model.scxml", "text": DOOR}])
    );

    // What an earlier core wrote: the text, saved straight to the store.
    let older = self::store("sets-older");
    let old_id = work(&older);
    older
        .save_model(&old_id, "<scxml version=\"1.0\"/>", None, None)
        .unwrap();
    let read = run(&older, "read_model", json!({"id": old_id.as_str()})).unwrap();
    assert_eq!(read["model"]["text"], "<scxml version=\"1.0\"/>");
}

/// What is not a model is refused before anything is written, as the caller's own
/// mistake, with the reason.
#[test]
fn what_is_not_a_set_is_refused_and_writes_nothing() {
    let store = store("sets-refused");
    let id = work(&store);
    let doc = |name: &str| json!({"name": name, "text": "x"});
    for (args, kind, wanted) in [
        (
            json!({"documents": [doc("a")], "text": "x"}),
            "bad-request",
            "not both",
        ),
        (json!({}), "bad-request", "not neither"),
        (json!({"text": "x", "entry": "a"}), "bad-request", "entry"),
        (json!({"documents": []}), "invalid-model", "no document"),
        (
            json!({"documents": [doc("a"), doc("a")]}),
            "invalid-model",
            "two documents",
        ),
        (
            json!({"documents": [doc("a")], "entry": "b"}),
            "invalid-model",
            "not one of the documents",
        ),
        (
            json!({"documents": [doc("../escape")]}),
            "invalid-model",
            "file name an import can name",
        ),
        (
            json!({"documents": [doc("a b")]}),
            "invalid-model",
            "file name an import can name",
        ),
        (
            json!({"documents": [{"name": "a", "text": "x", "extra": 1}]}),
            "bad-request",
            "extra",
        ),
    ] {
        let mut full = args.clone();
        full["id"] = json!(id.as_str());
        let refused = run(&store, "save_model", full).unwrap_err();
        assert_eq!(refused.kind, kind, "{args}: {}", refused.message);
        assert!(
            refused.message.contains(wanted),
            "{args}: {}",
            refused.message
        );
    }
    assert_eq!(store.model_head(&id).unwrap(), None, "nothing was written");
}

/// A saved text that begins like a set and is not ours is corrupt, never guessed at.
#[test]
fn a_saved_text_that_claims_to_be_a_set_and_is_not_one_is_corrupt() {
    let store = store("sets-corrupt");
    let id = work(&store);
    store
        .save_model(&id, "{\"format\":\"other\"}", None, None)
        .unwrap();
    let refused = run(&store, "read_model", json!({"id": id.as_str()})).unwrap_err();
    assert_eq!(refused.kind, "corrupt");
}

/// The product is asked about the whole set: the entry under its own file name, every
/// other document beside it (which is where an import finds it), and a model of one
/// document as before, under the work's own name.
#[test]
fn the_renderer_is_given_every_document_of_the_set() {
    let store = store("sets-render");
    let id = work(&store);
    run(
        &store,
        "save_model",
        json!({"id": id.as_str(), "documents": set(), "entry": "door.scxml"}),
    )
    .unwrap();
    let drawn = run(&store, "figures", json!({"id": id.as_str()})).unwrap();
    let text = drawn["sheets"][0]["svg"].as_str().unwrap();
    assert!(text.contains("entry_file=door.scxml"), "{text}");
    assert!(text.contains("siblings=close.scxml,open.scxml"), "{text}");

    let single = self::store("sets-render-single");
    let single_id = work(&single);
    run(
        &single,
        "save_model",
        json!({"id": single_id.as_str(), "text": DOOR}),
    )
    .unwrap();
    let drawn = run(&single, "figures", json!({"id": single_id.as_str()})).unwrap();
    let text = drawn["sheets"][0]["svg"].as_str().unwrap();
    assert!(text.contains("entry_file=- siblings= "), "{text}");
    assert!(text.contains("name=door-lock"), "{text}");
}
