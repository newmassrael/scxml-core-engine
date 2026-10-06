// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What SCE says of a work, asked of the revisions that were read (`read_judgment`).
//!
//! A screen reads a work (the snapshot, which needs no SCE) and then asks what SCE says of it:
//! whether the owner's acceptance holds, and how the design measures against the list. The
//! work can move between the two, and a verdict of the design after the move beside the design
//! before it is a false statement about what the owner is looking at. The question therefore
//! names the revisions, which are never rewritten, and the answer is of those and of no other.
//! Held here: that the answer says what the commands that judge the work as it stands say
//! (it is not a second definition of what holds), that it is of the revisions named whatever
//! was saved since, that a revision the work does not keep is refused, and that SCE not
//! answering is an answer, not a failure of the read.

mod common;

use sce_app_core::{call, CommandError, FixedClock, WorkStore};
use serde_json::{json, Value};

use common::{FakeRenderer, RefusingRenderer};

fn store(label: &str) -> WorkStore<FixedClock> {
    WorkStore::with_clock(
        common::scratch(label),
        FixedClock("2026-10-06T09:00:00Z".to_string()),
    )
}

fn run(store: &WorkStore<FixedClock>, name: &str, args: Value) -> Value {
    call(store, &FakeRenderer, name, args).unwrap_or_else(|e| panic!("{name} was refused: {e:?}"))
}

fn refused(store: &WorkStore<FixedClock>, name: &str, args: Value) -> CommandError {
    call(store, &FakeRenderer, name, args).expect_err("the command should be refused")
}

/// A work that was accepted and has not moved since: a text, a design that leaves a question
/// open, the list the text was read into, and the owner's acceptance of them.
struct Accepted {
    id: String,
    head: String,
    model: String,
    /// The revisions the owner was shown and accepted.
    basis: Value,
    /// The revision of the acceptance.
    acceptance: String,
}

fn an_accepted_work(store: &WorkStore<FixedClock>) -> Accepted {
    let id = run(store, "create_work", json!({"title": "Garage door"}))["id"]
        .as_str()
        .unwrap()
        .to_string();
    let head = run(
        store,
        "save_source",
        json!({"id": id, "text": "The door opens for a listed card."}),
    )["revision"]
        .as_str()
        .unwrap()
        .to_string();
    let model = run(
        store,
        "save_model",
        json!({"id": id, "text": "<scxml><!-- OPEN --></scxml>", "written_for": head}),
    )["revision"]
        .as_str()
        .unwrap()
        .to_string();
    let manifest = "{\"doc_id\":\"door\",\"rev\":\"1\",\
                    \"requirements\":[{\"id\":\"R1\"},{\"id\":\"R2\"}]}\n";
    let sidecar = "{\"doc_id\":\"door\",\"rev\":\"1\",\"text\":{\"R1\":\"The door opens.\"}}\n";
    run(
        store,
        "save_requirements",
        json!({"id": id, "manifest": manifest, "sidecar": sidecar, "written_for": head}),
    );
    let basis = run(store, "requirements_report", json!({"id": id}))["basis"].clone();
    run(store, "accept", json!({"id": id, "expect": basis}));
    let acceptance = run(store, "read_acceptance", json!({"id": id}))["acceptance"]["revision"]
        .as_str()
        .unwrap()
        .to_string();
    Accepted {
        id,
        head,
        model,
        basis,
        acceptance,
    }
}

/// The design is saved again: the work now holds another model, and the acceptance was of the
/// one before it. Answers the basis of the work as it stands.
fn the_design_moves(store: &WorkStore<FixedClock>, work: &Accepted) -> Value {
    run(
        store,
        "save_model",
        json!({"id": work.id, "text": "<scxml><!-- OPEN, edited --></scxml>",
               "base": work.model, "written_for": work.head}),
    );
    run(store, "read_acceptance", json!({"id": work.id}))["now"].clone()
}

#[test]
fn a_judgment_says_what_the_commands_that_judge_the_work_as_it_stands_say() {
    // Not a second definition of what holds or what is measured: the measure is the report's
    // and the standing is the acceptance's, in the same words, when the work has not moved.
    let store = store("judgment-same-words");
    let work = an_accepted_work(&store);
    let now = the_design_moves(&store, &work);

    let judged = run(
        &store,
        "read_judgment",
        json!({"id": work.id, "basis": now, "acceptance": work.acceptance}),
    );
    let report = run(&store, "requirements_report", json!({"id": work.id}));
    let acceptance = run(&store, "read_acceptance", json!({"id": work.id}));

    assert_eq!(judged["basis"], now);
    assert_eq!(judged["report"]["said"], report);
    assert_eq!(
        judged["acceptance"]["said"]["standing"],
        acceptance["standing"]
    );
    assert_eq!(judged["acceptance"]["said"]["lapse"], acceptance["lapse"]);
    assert_eq!(judged["acceptance"]["said"]["standing"], "lapsed");
}

#[test]
fn a_judgment_is_of_the_revisions_it_was_asked_about_whatever_was_saved_since() {
    let store = store("judgment-of-the-named-revisions");
    let work = an_accepted_work(&store);
    let moved = the_design_moves(&store, &work);
    assert_ne!(moved["model"], work.basis["model"]);

    // Asked of the design the owner was shown, the acceptance holds, though the work has moved
    // on: the answer is of that design and of no other.
    let before = run(
        &store,
        "read_judgment",
        json!({"id": work.id, "basis": work.basis, "acceptance": work.acceptance}),
    );
    assert_eq!(before["acceptance"]["said"]["standing"], "holds");
    assert_eq!(before["basis"], work.basis);
    assert_eq!(before["report"]["said"]["basis"], work.basis);

    // Asked of the design as it stands, it has lapsed, and the measure names that design.
    let after = run(
        &store,
        "read_judgment",
        json!({"id": work.id, "basis": moved, "acceptance": work.acceptance}),
    );
    assert_eq!(after["acceptance"]["said"]["standing"], "lapsed");
    assert_eq!(after["report"]["said"]["basis"], moved);

    // The work is put back as it was accepted: asked of the design as it stands, it holds.
    run(
        &store,
        "save_model",
        json!({"id": work.id, "text": "<scxml><!-- OPEN --></scxml>",
               "base": moved["model"], "written_for": work.head}),
    );
    let restored = run(
        &store,
        "read_judgment",
        json!({"id": work.id, "basis": work.basis, "acceptance": work.acceptance}),
    );
    assert_eq!(restored["acceptance"]["said"]["standing"], "holds");
}

#[test]
fn nothing_is_said_of_an_acceptance_that_was_not_named() {
    let store = store("judgment-no-acceptance");
    let work = an_accepted_work(&store);

    let judged = run(
        &store,
        "read_judgment",
        json!({"id": work.id, "basis": work.basis}),
    );

    assert_eq!(judged["acceptance"], Value::Null);
    assert_eq!(judged["report"]["said"]["basis"], work.basis);
}

#[test]
fn a_revision_the_work_does_not_keep_is_refused_as_not_found() {
    let store = store("judgment-unkept");
    let work = an_accepted_work(&store);
    let nowhere = "0".repeat(64);

    for part in ["source", "model", "requirements"] {
        let mut basis = work.basis.clone();
        basis[part] = json!(nowhere);
        let error = refused(
            &store,
            "read_judgment",
            json!({"id": work.id, "basis": basis}),
        );
        assert_eq!(error.kind, "not-found", "{part}");
    }
    let mut basis = work.basis.clone();
    basis["answers"] = json!(nowhere);
    let error = refused(
        &store,
        "read_judgment",
        json!({"id": work.id, "basis": basis}),
    );
    assert_eq!(error.kind, "not-found", "answers");
    let error = refused(
        &store,
        "read_judgment",
        json!({"id": work.id, "basis": work.basis, "acceptance": nowhere}),
    );
    assert_eq!(error.kind, "not-found", "acceptance");
    // A work that is not there is not found either, and arguments nothing reads are refused.
    let error = refused(
        &store,
        "read_judgment",
        json!({"id": "absent", "basis": work.basis}),
    );
    assert_eq!(error.kind, "not-found", "work");
    let error = refused(
        &store,
        "read_judgment",
        json!({"id": work.id, "basis": work.basis, "extra": 1}),
    );
    assert_eq!(error.kind, "bad-request", "extra");
}

#[test]
fn sce_not_answering_is_an_answer_and_says_which_it_did_not_answer() {
    let store = store("judgment-unanswered");
    let work = an_accepted_work(&store);

    let judged = call(
        &store,
        &RefusingRenderer,
        "read_judgment",
        json!({"id": work.id, "basis": work.basis, "acceptance": work.acceptance}),
    )
    .unwrap();

    assert_eq!(judged["basis"], work.basis);
    for part in ["acceptance", "report"] {
        let refusal = &judged[part]["refused"];
        assert_eq!(refusal["kind"], "sce-timeout", "{part}");
        assert!(
            refusal["message"].as_str().unwrap().contains("30 s"),
            "{part}"
        );
        assert_eq!(refusal["code"], Value::Null, "{part}");
    }
    // The refusal says no more than it was asked: with no acceptance named, only the measure.
    let unaccepted = call(
        &store,
        &RefusingRenderer,
        "read_judgment",
        json!({"id": work.id, "basis": work.basis}),
    )
    .unwrap();
    assert_eq!(unaccepted["acceptance"], Value::Null);
    assert_eq!(unaccepted["report"]["refused"]["kind"], "sce-timeout");
}
