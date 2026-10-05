// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A work read as one state (`read_work_snapshot`, `read_work_heads`).
//!
//! The screen used to read a work as six commands, and a save between two of them
//! handed it a text and a model that were never the work's at one moment. The
//! snapshot is those reads taken so that they belong together, and the heads are the
//! same without the texts, for a screen that asks often whether the work moved. Two
//! things are held here: that they say what the commands that read one chain say (they
//! are not a second definition of what a model or a list is), and that a work being
//! saved while it is read is never shown in a state it was not in.

mod common;

use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

use sce_app_core::{call, FixedClock, Revision, WorkId, WorkStore};
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
        "bundle",
    ] {
        assert_eq!(snapshot[key], Value::Null, "{key}");
    }
}

/// A work with every chain filled, and the text moved on after the design was accepted,
/// so the model and the list are behind it: what the commands that read one chain say
/// is then not the same as the empty work's, in every chain.
fn a_work_with_every_chain_and_a_text_that_moved_on(store: &WorkStore<FixedClock>) -> String {
    let id = run(store, "create_work", json!({"title": "Garage door"}))["id"]
        .as_str()
        .unwrap()
        .to_string();
    let text = run(
        store,
        "save_source",
        json!({"id": id, "text": "The door opens for a listed card."}),
    );
    let head = text["revision"].as_str().unwrap().to_string();
    run(
        store,
        "save_model",
        json!({"id": id, "text": "<scxml><!-- OPEN --></scxml>", "written_for": head}),
    );
    run(
        store,
        "save_answers",
        json!({"id": id, "answers": {"open-guard": "Any card on the list opens it."}}),
    );
    let manifest = "{\"doc_id\":\"door\",\"rev\":\"1\",\
                    \"requirements\":[{\"id\":\"R1\"},{\"id\":\"R2\"}]}\n";
    let sidecar = "{\"doc_id\":\"door\",\"rev\":\"1\",\"text\":{\"R1\":\"The door opens.\"}}\n";
    run(
        store,
        "save_requirements",
        json!({"id": id, "manifest": manifest, "sidecar": sidecar, "written_for": head}),
    );
    let shown = run(store, "requirements_report", json!({"id": id}))["basis"].clone();
    run(store, "accept", json!({"id": id, "expect": shown}));
    run(
        store,
        "save_source",
        json!({"id": id, "text": "The door opens for a listed card, twice.", "base": head}),
    );
    id
}

#[test]
fn a_snapshot_says_what_each_command_that_reads_one_chain_says() {
    let store = store("snapshot-says");
    let id = a_work_with_every_chain_and_a_text_that_moved_on(&store);

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
fn the_heads_of_a_work_nothing_was_saved_to_say_null_for_every_chain() {
    let store = store("heads-empty");
    let id = run(&store, "create_work", json!({"title": "Door lock"}))["id"]
        .as_str()
        .unwrap()
        .to_string();

    let heads = run(&store, "read_work_heads", json!({"id": id}));

    assert_eq!(
        heads,
        json!({
            "source": null,
            "model": null,
            "answers": null,
            "requirements": null,
            "acceptance": null,
            "bundle": null,
            "request": null,
        })
    );
}

/// A work that has a bundle has the model and the list the bundle names, and says so: the
/// snapshot and the heads agree with the bundle and with each other, and a text saved after
/// the bundle makes the model behind it without making it another model.
#[test]
fn a_bundled_work_is_read_by_its_bundle_in_the_snapshot_and_in_the_heads() {
    let store = store("snapshot-bundled");
    let id = run(&store, "create_work", json!({"title": "Door lock"}))["id"]
        .as_str()
        .unwrap()
        .to_string();
    let source = run(
        &store,
        "save_source",
        json!({"id": id, "text": "The door opens for a listed card."}),
    )["revision"]
        .clone();
    let request = run(
        &store,
        "request_generation",
        json!({"id": id, "key": "press-1", "origin": "gui", "expect": {"source": source}}),
    )["request"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    run(
        &store,
        "claim_request",
        json!({"id": id, "request": request, "holder": "adapter-a"}),
    );
    run(
        &store,
        "save_request_candidate",
        json!({"id": id, "request": request, "holder": "adapter-a", "attempt": 1,
               "text": "<scxml>door</scxml>", "manifest": "{\"list\":[]}"}),
    );
    let done = run(
        &store,
        "complete_request",
        json!({"id": id, "request": request, "holder": "adapter-a", "attempt": 1}),
    );

    let snapshot = run(&store, "read_work_snapshot", json!({"id": id}));
    let heads = run(&store, "read_work_heads", json!({"id": id}));

    assert_eq!(snapshot["bundle"], done["bundle"]);
    assert_eq!(heads["bundle"], done["bundle"]);
    assert_eq!(snapshot["model_standing"], json!("current"));
    assert_eq!(snapshot["requirements_standing"], json!("current"));
    assert_eq!(heads["model"]["revision"], snapshot["model"]["revision"]);
    assert_eq!(heads["model"]["written_for"], source);
    assert_eq!(heads["requirements"]["written_for"], source);
    assert_eq!(heads["request"]["state"], json!("completed"));

    // The text moves on: the model is the same and is behind it now.
    run(
        &store,
        "save_source",
        json!({"id": id, "text": "The door opens for a listed card, twice.", "base": source}),
    );
    let later = run(&store, "read_work_snapshot", json!({"id": id}));
    assert_eq!(later["bundle"], done["bundle"]);
    assert_eq!(later["model"]["revision"], snapshot["model"]["revision"]);
    assert_eq!(later["model_standing"], json!("behind"));
}

/// The heads are the revisions the snapshot holds, chain by chain, and the source each of
/// the model and the list was written for: a screen that compares them with what it shows
/// is comparing with what it would read.
#[test]
fn the_heads_are_the_revisions_the_snapshot_holds() {
    let store = store("heads-say");
    let id = a_work_with_every_chain_and_a_text_that_moved_on(&store);

    let heads = run(&store, "read_work_heads", json!({"id": id}));
    let snapshot = run(&store, "read_work_snapshot", json!({"id": id}));

    assert_eq!(heads["source"], snapshot["source"]["revision"]);
    assert_eq!(
        heads["model"],
        json!({
            "revision": snapshot["model"]["revision"],
            "written_for": snapshot["model"]["written_for"],
        })
    );
    assert_eq!(heads["answers"], snapshot["answers"]["revision"]);
    assert_eq!(
        heads["requirements"],
        json!({
            "revision": snapshot["requirements"]["revision"],
            "written_for": snapshot["requirements"]["written_for"],
        })
    );
    assert_eq!(heads["acceptance"], snapshot["acceptance"]["revision"]);
    // The text moved on after the design: what the model was written for is not the head.
    assert_ne!(heads["model"]["written_for"], heads["source"]);
}

#[test]
fn the_heads_refuse_what_every_read_refuses() {
    let store = store("heads-refuses");

    let missing = call(
        &store,
        &FakeRenderer,
        "read_work_heads",
        json!({"id": "absent"}),
    );
    let unknown = call(
        &store,
        &FakeRenderer,
        "read_work_heads",
        json!({"id": "absent", "extra": 1}),
    );

    assert_eq!(missing.unwrap_err().kind, "not-found");
    assert_eq!(unknown.unwrap_err().kind, "bad-request");
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

/// Run a writer that saves a text and then the model written for it, over and over, and
/// call `look` on the work until the writer is done. `look` is handed the number of a
/// revision among the texts the writer saves.
///
/// At every moment the folder was in, the model is written for the head text or for the
/// one before it (it is saved right after), and never for a text that is not the head yet.
/// A read that looked at the text first and the model later, with a writer in between,
/// would answer a model of a text newer than the one it holds.
fn while_a_writer_saves(
    label: &str,
    mut look: impl FnMut(&WorkStore<FixedClock>, &WorkId, &dyn Fn(&Revision) -> usize),
) {
    const ROUNDS: usize = 120;
    let store = store(label);
    let id = run(&store, "create_work", json!({"title": "Busy"}))["id"]
        .as_str()
        .unwrap()
        .to_string();
    let texts: Vec<String> = (0..ROUNDS).map(|i| format!("text {i}")).collect();
    let revisions: Vec<Revision> = texts.iter().map(|t| Revision::of(t.as_bytes())).collect();
    let id = WorkId::parse(&id).unwrap();
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
            look(&store, &id, &index);
            looked += 1;
        }
        assert!(looked > 0);
    });
}

/// The model of a text that is not the head, or of one more than a text behind it, is a
/// state the work was never in.
fn assert_a_model_is_of_the_head_or_the_one_before(head: usize, written_for: usize) {
    assert!(
        written_for <= head,
        "a model of text {written_for} beside head text {head}"
    );
    assert!(
        head - written_for <= 1,
        "a model of text {written_for} beside head text {head}"
    );
}

#[test]
fn a_snapshot_taken_while_a_work_is_saved_is_never_a_state_it_was_not_in() {
    while_a_writer_saves("snapshot-concurrent", |store, id, index| {
        let snapshot = store.read_work_snapshot(id).expect("a snapshot");
        if let (Some(source), Some(model)) = (&snapshot.source, &snapshot.model) {
            assert_a_model_is_of_the_head_or_the_one_before(
                index(&source.revision),
                index(model.written_for.as_ref().expect("the writer says")),
            );
        }
    });
}

#[test]
fn heads_read_while_a_work_is_saved_are_never_a_state_it_was_not_in() {
    while_a_writer_saves("heads-concurrent", |store, id, index| {
        let heads = store.read_work_heads(id).expect("the heads");
        if let (Some(source), Some(model)) = (&heads.source, &heads.model) {
            assert_a_model_is_of_the_head_or_the_one_before(
                index(source),
                index(model.written_for.as_ref().expect("the writer says")),
            );
        }
    });
}
