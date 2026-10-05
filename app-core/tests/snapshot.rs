// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A work read as one state (`read_work_snapshot`).
//!
//! The screen used to read a work as six commands, and a save between two of them
//! handed it a text and a model that were never the work's at one moment. The
//! snapshot is those reads taken so that they belong together. Two things are held
//! here: that it says what the commands that read one chain say (it is not a second
//! definition of what a model or a list is), and that a work being saved while it is
//! read is never shown in a state it was not in.

mod common;

use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

use sce_app_core::{call, FixedClock, Revision, WorkStore};
use serde_json::{json, Value};

use common::FakeRenderer;

fn store(label: &str) -> WorkStore<FixedClock> {
    WorkStore::with_clock(
        common::scratch(label),
        FixedClock("2026-10-05T09:00:00Z".to_string()),
    )
}

fn run(store: &WorkStore<FixedClock>, name: &str, args: Value) -> Value {
    call(store, &FakeRenderer, name, args).unwrap_or_else(|e| panic!("{name} was refused: {e:?}"))
}

#[test]
fn a_snapshot_of_a_work_nothing_was_saved_to_says_null_for_every_chain() {
    let store = store("snapshot-empty");
    let work = run(&store, "create_work", json!({"title": "Door lock"}));
    let id = work["id"].as_str().unwrap();

    let snapshot = run(&store, "read_work_snapshot", json!({"id": id}));

    assert_eq!(snapshot["work"], work);
    for key in [
        "source",
        "model",
        "model_standing",
        "answers",
        "requirements",
        "requirements_standing",
        "acceptance",
    ] {
        assert_eq!(snapshot[key], Value::Null, "{key}");
    }
}

#[test]
fn a_snapshot_says_what_each_command_that_reads_one_chain_says() {
    let store = store("snapshot-says");
    let id = run(&store, "create_work", json!({"title": "Garage door"}))["id"]
        .as_str()
        .unwrap()
        .to_string();
    let text = run(
        &store,
        "save_source",
        json!({"id": id, "text": "The door opens for a listed card."}),
    );
    let head = text["revision"].as_str().unwrap().to_string();
    run(
        &store,
        "save_model",
        json!({"id": id, "text": "<scxml><!-- OPEN --></scxml>", "written_for": head}),
    );
    run(
        &store,
        "save_answers",
        json!({"id": id, "answers": {"open-guard": "Any card on the list opens it."}}),
    );
    let manifest = "{\"doc_id\":\"door\",\"rev\":\"1\",\
                    \"requirements\":[{\"id\":\"R1\"},{\"id\":\"R2\"}]}\n";
    let sidecar = "{\"doc_id\":\"door\",\"rev\":\"1\",\"text\":{\"R1\":\"The door opens.\"}}\n";
    run(
        &store,
        "save_requirements",
        json!({"id": id, "manifest": manifest, "sidecar": sidecar, "written_for": head}),
    );
    let shown = run(&store, "requirements_report", json!({"id": id}))["basis"].clone();
    run(&store, "accept", json!({"id": id, "expect": shown}));
    // The text moves on after the design was accepted, so the model and the list are
    // behind it: the snapshot says so in the word the single reads use.
    run(
        &store,
        "save_source",
        json!({"id": id, "text": "The door opens for a listed card, twice.", "base": head}),
    );

    let snapshot = run(&store, "read_work_snapshot", json!({"id": id}));

    assert_eq!(
        snapshot["work"],
        run(&store, "read_work", json!({"id": id}))["work"]
    );
    assert_eq!(
        snapshot["source"],
        run(&store, "read_source", json!({"id": id}))["source"]
    );
    let model = run(&store, "read_model", json!({"id": id}));
    assert_eq!(snapshot["model"], model["model"]);
    assert_eq!(snapshot["model_standing"], model["standing"]);
    assert_eq!(snapshot["model_standing"], json!("behind"));
    assert_eq!(
        snapshot["answers"],
        run(&store, "read_answers", json!({"id": id}))["answers"]
    );
    let list = run(&store, "read_requirements", json!({"id": id}));
    assert_eq!(snapshot["requirements"], list["requirements"]);
    assert_eq!(snapshot["requirements_standing"], list["standing"]);
    assert_eq!(
        snapshot["acceptance"],
        run(&store, "read_acceptance", json!({"id": id}))["acceptance"]
    );
}

#[test]
fn a_snapshot_refuses_what_every_read_refuses() {
    let store = store("snapshot-refuses");

    let missing = call(
        &store,
        &FakeRenderer,
        "read_work_snapshot",
        json!({"id": "absent"}),
    );
    let unknown = call(
        &store,
        &FakeRenderer,
        "read_work_snapshot",
        json!({"id": "absent", "extra": 1}),
    );

    assert_eq!(missing.unwrap_err().kind, "not-found");
    assert_eq!(unknown.unwrap_err().kind, "bad-request");
}

/// The writer saves a text and then the model written for it, over and over. At every
/// moment the folder was in, the model is written for the head text or for the one before
/// it (it is saved right after), and never for a text that is not the head yet. A read
/// that looked at the text first and the model later, with a writer in between, would
/// answer a model of a text newer than the one it holds.
#[test]
fn a_snapshot_taken_while_a_work_is_saved_is_never_a_state_it_was_not_in() {
    const ROUNDS: usize = 120;
    let store = store("snapshot-concurrent");
    let id = run(&store, "create_work", json!({"title": "Busy"}))["id"]
        .as_str()
        .unwrap()
        .to_string();
    let texts: Vec<String> = (0..ROUNDS).map(|i| format!("text {i}")).collect();
    let revisions: Vec<Revision> = texts.iter().map(|t| Revision::of(t.as_bytes())).collect();
    let id = sce_app_core::WorkId::parse(&id).unwrap();
    let done = AtomicBool::new(false);

    thread::scope(|scope| {
        scope.spawn(|| {
            let mut source_base: Option<Revision> = None;
            let mut model_base: Option<Revision> = None;
            for (text, revision) in texts.iter().zip(&revisions) {
                store
                    .save_source(&id, text, source_base.as_ref())
                    .expect("the writer's text");
                source_base = Some(revision.clone());
                let model = format!("model of {text}");
                store
                    .save_model(&id, &model, model_base.as_ref(), Some(revision))
                    .expect("the writer's model");
                model_base = Some(Revision::of(model.as_bytes()));
            }
            done.store(true, Ordering::SeqCst);
        });

        let index = |revision: &Revision| revisions.iter().position(|r| r == revision).unwrap();
        let mut looked = 0;
        while !done.load(Ordering::SeqCst) {
            let snapshot = store.read_work_snapshot(&id).expect("a snapshot");
            looked += 1;
            if let (Some(source), Some(model)) = (&snapshot.source, &snapshot.model) {
                let head = index(&source.revision);
                let written_for = index(model.written_for.as_ref().expect("the writer says"));
                assert!(
                    written_for <= head,
                    "a model of text {written_for} beside head text {head}"
                );
                assert!(
                    head - written_for <= 1,
                    "a model of text {written_for} beside head text {head}"
                );
            }
        }
        assert!(looked > 0);
    });
}
