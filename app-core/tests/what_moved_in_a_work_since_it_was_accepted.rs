// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What moved in a work since the owner accepted it, asked of the product requirement by
//! requirement (`read_acceptance_delta`).
//!
//! The comparison is the product's (`sce-codegen acceptance-delta`); what this command adds is
//! the work: the acceptance and the design and list it is compared with are read as ONE state,
//! the design is laid out the way the product is always shown a work, and the manifest the
//! record pinned is passed on so that a caller holding a words-side comparison can say it is
//! about this list and not another of the same name and number. These tests drive the command
//! with a stand-in product that says what the real one says in the shape it says it (the real
//! product is held by `acceptance_product.rs`):
//!
//! * a work nobody accepted answers `null`, and one that was accepted and not touched says every
//!   requirement `unchanged`;
//! * a design that moved says `changed` for what depends on it, and the answer carries the
//!   acceptance, the manifest it pinned and the revisions the comparison was made at;
//! * the product not answering is the command's refusal, and nothing is invented. A record the
//!   real product cannot compare (made before it kept the rows, or under another rule) is
//!   refused in its words, which only the real product can be asked for: that is held in
//!   `acceptance_product.rs` and in the product's own tests.

mod common;

use sce_app_core::{call, CommandError, FixedClock, Revision, WorkStore};
use serde_json::{json, Value};

use common::{FakeRenderer, RefusingRenderer};

const MANIFEST: &str = "{\"doc_id\":\"door\",\"rev\":\"1\",\
                        \"requirements\":[{\"id\":\"R1\"},{\"id\":\"R2\"}]}\n";

fn command(store: &WorkStore<FixedClock>, name: &str, args: Value) -> Result<Value, CommandError> {
    call(store, &FakeRenderer, name, args)
}

/// A work with a text, a model and a list that the owner has accepted.
fn accepted(label: &str) -> (WorkStore<FixedClock>, String, Value) {
    let store = WorkStore::with_clock(
        common::scratch(label),
        FixedClock("2026-10-09T09:00:00Z".to_string()),
    );
    let id = command(&store, "create_work", json!({"title": "Door"})).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();
    let source = command(
        &store,
        "save_source",
        json!({"id": id, "text": "The door opens."}),
    )
    .unwrap()["revision"]
        .clone();
    command(
        &store,
        "save_model",
        json!({"id": id, "text": "<scxml><state id=\"a\"/></scxml>", "written_for": source}),
    )
    .unwrap();
    command(
        &store,
        "save_requirements",
        json!({"id": id, "manifest": MANIFEST, "written_for": source}),
    )
    .unwrap();
    let report = command(&store, "requirements_report", json!({"id": id})).unwrap();
    command(
        &store,
        "accept",
        json!({"id": id, "expect": report["basis"]}),
    )
    .unwrap();
    (store, id, source)
}

fn lines_of(answer: &Value) -> Vec<&Value> {
    answer["lines"].as_array().expect("lines").iter().collect()
}

fn evidence_of<'a>(answer: &'a Value, requirement: &str) -> &'a str {
    lines_of(answer)
        .into_iter()
        .find(|l| l["kind"] == "acceptance-delta" && l["requirement"] == requirement)
        .unwrap_or_else(|| panic!("no line for {requirement}: {answer}"))["evidence"]
        .as_str()
        .unwrap()
}

#[test]
fn a_work_nobody_accepted_has_nothing_to_compare() {
    let store = WorkStore::with_clock(
        common::scratch("delta-none"),
        FixedClock("2026-10-09T09:00:00Z".to_string()),
    );
    let id = command(&store, "create_work", json!({"title": "Door"})).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();
    let answer = command(&store, "read_acceptance_delta", json!({"id": id})).unwrap();
    assert_eq!(
        answer,
        json!({"acceptance": null, "manifest": null, "lines": null, "now": null})
    );
}

#[test]
fn an_accepted_work_that_was_not_touched_says_every_requirement_unchanged() {
    let (store, id, _) = accepted("delta-untouched");
    let answer = command(&store, "read_acceptance_delta", json!({"id": id})).unwrap();
    assert_eq!(evidence_of(&answer, "R1"), "unchanged", "{answer}");
    assert_eq!(evidence_of(&answer, "R2"), "unchanged", "{answer}");
    assert!(answer["acceptance"]["accepted_at"].is_string(), "{answer}");
}

#[test]
fn a_design_that_moved_says_what_it_moved_and_the_answer_names_the_revisions_it_was_read_at() {
    let (store, id, source) = accepted("delta-moved");
    let before = command(&store, "read_acceptance_delta", json!({"id": id})).unwrap();
    let model = command(&store, "read_work_snapshot", json!({"id": id})).unwrap()["model"]
        ["revision"]
        .clone();
    command(
        &store,
        "save_model",
        json!({"id": id, "text": "<scxml><state id=\"b\"/></scxml>", "base": model,
               "written_for": source}),
    )
    .unwrap();
    let after = command(&store, "read_acceptance_delta", json!({"id": id})).unwrap();

    assert_eq!(evidence_of(&after, "R1"), "changed", "{after}");
    let moved = lines_of(&after)
        .into_iter()
        .find(|l| l["requirement"] == "R1")
        .unwrap()["moved"]
        .clone();
    assert_eq!(moved, json!(["design/model.scxml"]));
    // The acceptance is the same one, and the comparison was made at the model that is now.
    assert_eq!(
        before["acceptance"]["revision"],
        after["acceptance"]["revision"]
    );
    assert_ne!(before["now"]["model"], after["now"]["model"]);
    assert_eq!(after["now"]["requirements"], before["now"]["requirements"]);
}

#[test]
fn the_answer_carries_the_manifest_the_acceptance_pinned() {
    let (store, id, _) = accepted("delta-manifest");
    let answer = command(&store, "read_acceptance_delta", json!({"id": id})).unwrap();
    let manifest = &answer["manifest"];
    assert_eq!(manifest["doc_id"], "door");
    assert_eq!(manifest["rev"], "1");
    // The digest the record pins is that of the manifest's bytes: the revision the work keeps
    // them under is the SHA-256 of the same bytes.
    assert_eq!(
        manifest["sha256"].as_str().unwrap(),
        Revision::of(MANIFEST.as_bytes()).to_string()
    );
}

#[test]
fn the_acceptance_and_the_design_are_read_as_one_state() {
    // A requirement list saved after the acceptance is the list the comparison is made with, and
    // `now` says so: the acceptance's own basis names the list it was taken of.
    let (store, id, source) = accepted("delta-one-state");
    let held = command(&store, "read_requirements", json!({"id": id})).unwrap();
    let next = "{\"doc_id\":\"door\",\"rev\":\"2\",\
                \"requirements\":[{\"id\":\"R1\"},{\"id\":\"R2\"},{\"id\":\"R3\"}]}\n";
    command(
        &store,
        "save_requirements",
        json!({"id": id, "manifest": next, "base": held["requirements"]["revision"],
               "written_for": source}),
    )
    .unwrap();
    let answer = command(&store, "read_acceptance_delta", json!({"id": id})).unwrap();
    assert_eq!(
        answer["acceptance"]["basis"]["requirements"],
        held["requirements"]["revision"]
    );
    assert_ne!(
        answer["now"]["requirements"],
        answer["acceptance"]["basis"]["requirements"]
    );
    // The comparison was made with the list the work has NOW (the stand-in names a line for each
    // requirement of the list it is given): R3 is only in that one.
    assert_eq!(evidence_of(&answer, "R3"), "unchanged", "{answer}");
}

#[test]
fn the_product_not_answering_is_the_commands_refusal_and_nothing_is_invented() {
    let (store, id, _) = accepted("delta-refused");
    let refused = call(
        &store,
        &RefusingRenderer,
        "read_acceptance_delta",
        json!({"id": id}),
    )
    .expect_err("the product did not answer");
    assert_eq!(refused.kind, "sce-timeout", "{refused:?}");
}
