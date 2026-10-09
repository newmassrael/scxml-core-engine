// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What a revision of a work did, requirement by requirement (`read_revision_report`).
//!
//! The words come from two states of the work's own chain of requirement lists, the list the
//! owner accepted and the list the work has now, each with its lineage; the evidence is the
//! product's comparison of the design now with what the owner was shown. The judgment is
//! `sce-revision`, held to the Python by the shared cases; what these tests hold is the work
//! laid out for it, with a stand-in product that says what the real one says in the shape it
//! says it (the real product is held by `acceptance_product.rs`):
//!
//! * a work nobody accepted answers `null`, and one accepted and not touched carries every
//!   requirement over;
//! * a revision that drops a sentence retires its id cleanly, and the design moving where the
//!   words did not is outside the reach of the revision;
//! * a list that keeps no lineage and no sidecar is refused in the sentence a person is told,
//!   and the specification's sentences are on the page only when asked for;
//! * a text changed after the model and the list were written for it holds the report back
//!   (`revision-not-current`), because the list the work has is then the one the owner accepted
//!   and comparing it with itself would call a revision nobody made "all carried over".

mod common;

use sce_app_core::{
    call, Acceptor, CheckOutcome, CommandError, FigureRenderer, FigureRequest, FigureSet,
    FixedClock, ModelReviewer, RenderError, RequirementsReport, Review, ReviewRequest, Snapshot,
    Taken, WorkStore,
};
use serde_json::{json, Value};

use common::FakeRenderer;

const CASES: &str = include_str!("../../sce-build/tests/fixtures/revision_judgment/cases.json");

const BARE: &str = "{\"doc_id\":\"door\",\"rev\":\"1\",\
                    \"requirements\":[{\"id\":\"R1\"},{\"id\":\"R2\"}]}\n";

fn command(store: &WorkStore<FixedClock>, name: &str, args: Value) -> Result<Value, CommandError> {
    call(store, &FakeRenderer, name, args)
}

/// The three texts of a list the shared cases hold: `first`, `second` and `third` are three
/// revisions of one specification, each continuing the one before.
fn list(name: &str) -> Value {
    let all: Value = serde_json::from_str(CASES).expect("the cases are JSON");
    let list = all["lists"][name].clone();
    assert!(list.is_object(), "the cases have no list called {name}");
    json!({
        "manifest": list["manifest_text"],
        "sidecar": list["sidecar_text"],
        "lineage": list["lineage_text"],
    })
}

fn saved(store: &WorkStore<FixedClock>, id: &str, mut args: Value) -> Value {
    args["id"] = json!(id);
    command(store, "save_requirements", args).unwrap()
}

/// A work with a text, a model and the list `args` give, accepted by its owner. Returns the
/// store, the work, the text's revision and the revision of the list.
fn accepted(label: &str, args: Value) -> (WorkStore<FixedClock>, String, Value, Value) {
    let store = WorkStore::with_clock(
        common::scratch(label),
        FixedClock("2026-10-09T09:00:00Z".to_string()),
    );
    let id = command(&store, "create_work", json!({"title": "Lamp"})).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();
    let source = command(
        &store,
        "save_source",
        json!({"id": id, "text": "The lamp starts off."}),
    )
    .unwrap()["revision"]
        .clone();
    command(
        &store,
        "save_model",
        json!({"id": id, "text": "<scxml><state id=\"a\"/></scxml>", "written_for": source}),
    )
    .unwrap();
    let mut args = args;
    args["written_for"] = source.clone();
    let list = saved(&store, &id, args);
    let report = command(&store, "requirements_report", json!({"id": id})).unwrap();
    command(
        &store,
        "accept",
        json!({"id": id, "expect": report["basis"]}),
    )
    .unwrap();
    (store, id, source, list["revision"].clone())
}

/// The work revised to the list `name`, saved from the list it holds now.
fn revised_to(store: &WorkStore<FixedClock>, id: &str, source: &Value, held: &Value, name: &str) {
    let mut args = list(name);
    args["base"] = held.clone();
    args["written_for"] = source.clone();
    saved(store, id, args);
}

fn kinds(report: &Value) -> Vec<(String, u64)> {
    report["summary"]["kinds"]
        .as_object()
        .expect("kinds")
        .iter()
        .map(|(kind, count)| (kind.clone(), count.as_u64().unwrap()))
        .collect()
}

#[test]
fn a_work_nobody_accepted_has_no_revision_to_report() {
    let store = WorkStore::with_clock(
        common::scratch("report-none"),
        FixedClock("2026-10-09T09:00:00Z".to_string()),
    );
    let id = command(&store, "create_work", json!({"title": "Lamp"})).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();
    let answer = command(&store, "read_revision_report", json!({"id": id})).unwrap();
    assert_eq!(answer, json!({"acceptance": null, "report": null}));
}

#[test]
fn an_accepted_work_that_was_not_touched_carries_every_requirement_over() {
    let (store, id, _, _) = accepted("report-untouched", list("first"));
    let answer = command(&store, "read_revision_report", json!({"id": id})).unwrap();
    let report = &answer["report"];
    assert_eq!(report["verdict"], "within-reach", "{answer}");
    assert_eq!(
        kinds(report),
        vec![("carries-over".to_string(), 5)],
        "{answer}"
    );
    assert_eq!(report["of"]["accepted"], report["of"]["now"], "{answer}");
    let page = report["page"].as_str().unwrap();
    assert!(page.contains("Verdict: within-reach"), "{page}");
    assert!(page.contains("## Carries over (5)"), "{page}");
}

/// The text of the work changed to `text`, with nothing written again for it.
fn text_changed(store: &WorkStore<FixedClock>, id: &str, source: &Value, text: &str) -> Value {
    command(
        store,
        "save_source",
        json!({"id": id, "text": text, "base": source}),
    )
    .unwrap()["revision"]
        .clone()
}

#[test]
fn a_text_changed_after_the_list_was_written_holds_the_report_back() {
    let (store, id, source, _) = accepted("report-behind", list("first"));
    text_changed(&store, &id, &source, "The lamp starts on.");

    let refused = command(&store, "read_revision_report", json!({"id": id})).unwrap_err();

    assert_eq!(refused.kind, "revision-not-current", "{refused:?}");
    // It says what is behind and what to do, in the words a person is told.
    for part in ["requirement list", "model", "generate"] {
        assert!(refused.message.contains(part), "{refused:?}");
    }
    assert_eq!(refused.detail["requirements"], "behind", "{refused:?}");
    assert_eq!(refused.detail["model"], "behind", "{refused:?}");
}

#[test]
fn a_list_written_for_the_new_text_is_not_held_back_by_a_model_that_was_not() {
    let (store, id, source, held) = accepted("report-model-behind", list("first"));
    let changed = text_changed(&store, &id, &source, "The lamp starts on.");
    revised_to(&store, &id, &changed, &held, "second");

    let refused = command(&store, "read_revision_report", json!({"id": id})).unwrap_err();

    // The list is current and the model is not: what is held back is named, and what is not
    // is not blamed.
    assert_eq!(refused.kind, "revision-not-current", "{refused:?}");
    assert_eq!(refused.detail["requirements"], "current", "{refused:?}");
    assert_eq!(refused.detail["model"], "behind", "{refused:?}");
}

#[test]
fn a_model_and_a_list_written_again_for_the_new_text_are_judged_again() {
    let (store, id, source, held) = accepted("report-written-again", list("first"));
    let changed = text_changed(&store, &id, &source, "The lamp starts on.");
    let snapshot = command(&store, "read_work_snapshot", json!({"id": id})).unwrap();
    command(
        &store,
        "save_model",
        json!({"id": id, "text": "<scxml><state id=\"b\"/></scxml>",
               "base": snapshot["model"]["revision"], "written_for": changed}),
    )
    .unwrap();
    revised_to(&store, &id, &changed, &held, "second");

    let answer = command(&store, "read_revision_report", json!({"id": id})).unwrap();

    assert!(answer["report"]["verdict"].is_string(), "{answer}");
}

#[test]
fn a_model_that_says_nothing_of_its_text_is_not_known_to_be_behind() {
    // Only a state known to be behind is held back; one that cannot say what it was written
    // for is judged as it was before the report asked. (An acceptance is never taken of one,
    // so it comes about by a model saved again after it.)
    let (store, id, _, _) = accepted("report-unstated", list("first"));
    let snapshot = command(&store, "read_work_snapshot", json!({"id": id})).unwrap();
    command(
        &store,
        "save_model",
        json!({"id": id, "text": "<scxml><state id=\"b\"/></scxml>",
               "base": snapshot["model"]["revision"]}),
    )
    .unwrap();

    let answer = command(&store, "read_revision_report", json!({"id": id})).unwrap();

    assert!(answer["report"]["verdict"].is_string(), "{answer}");
}

#[test]
fn a_revision_that_drops_a_sentence_retires_its_id_cleanly() {
    let (store, id, source, held) = accepted("report-retired", list("first"));
    revised_to(&store, &id, &source, &held, "second");
    let answer = command(&store, "read_revision_report", json!({"id": id})).unwrap();
    let report = &answer["report"];
    assert_eq!(report["verdict"], "within-reach", "{answer}");
    let by_kind = kinds(report);
    assert!(
        by_kind.contains(&("carries-over".to_string(), 4)),
        "{by_kind:?}"
    );
    assert!(
        by_kind.contains(&("retired-cleanly".to_string(), 1)),
        "{by_kind:?}"
    );
    // The two states are the work's own: the list accepted and the list the work has now.
    assert_ne!(report["of"]["accepted"], report["of"]["now"], "{answer}");
}

#[test]
fn a_design_that_moved_where_the_words_did_not_is_outside_the_reach_of_the_revision() {
    let (store, id, source, held) = accepted("report-outside", list("first"));
    revised_to(&store, &id, &source, &held, "second");
    // The design is edited after the revision: every requirement the words carried over now
    // reads as changed, and nothing in the revision asked it to.
    let model = command(&store, "read_work_snapshot", json!({"id": id})).unwrap()["model"].clone();
    command(
        &store,
        "save_model",
        json!({"id": id, "text": "<scxml><state id=\"b\"/></scxml>",
               "base": model["revision"], "written_for": source}),
    )
    .unwrap();
    let answer = command(&store, "read_revision_report", json!({"id": id})).unwrap();
    let report = &answer["report"];
    assert_eq!(report["verdict"], "outside-reach", "{answer}");
    assert_eq!(report["summary"]["violations"], 4, "{answer}");
    let page = report["page"].as_str().unwrap();
    assert!(
        page.contains("## Outside the revision's reach (4)"),
        "{page}"
    );
}

#[test]
fn a_list_that_keeps_no_lineage_and_no_sidecar_is_refused_in_the_words_a_person_is_told() {
    let (store, id, _, _) = accepted("report-unjudged", json!({"manifest": BARE}));
    let refused = command(&store, "read_revision_report", json!({"id": id})).unwrap_err();
    assert_eq!(refused.kind, "revision-not-judged", "{refused:?}");
    assert!(
        refused
            .message
            .contains("cannot say what the revision did to each requirement's words"),
        "{refused:?}"
    );
    assert!(
        refused.message.contains("keeps no lineage and no sidecar"),
        "{refused:?}"
    );
}

#[test]
fn the_sentences_of_the_specification_are_on_the_page_only_when_asked_for() {
    let (store, id, source, held) = accepted("report-sentences", list("first"));
    revised_to(&store, &id, &source, &held, "third");
    let plain = command(&store, "read_revision_report", json!({"id": id})).unwrap();
    let with = command(
        &store,
        "read_revision_report",
        json!({"id": id, "sentences": true}),
    )
    .unwrap();
    let plain = plain["report"]["page"].as_str().unwrap();
    let with = with["report"]["page"].as_str().unwrap();
    assert!(!plain.contains("local artefact"), "{plain}");
    assert!(with.contains("local artefact"), "{with}");
    // The new requirement's sentence is the third revision's own.
    assert!(with.contains("A reset key clears it."), "{with}");
    assert!(!plain.contains("A reset key clears it."), "{plain}");
}

#[test]
fn the_command_takes_only_what_it_names() {
    let (store, id, _, _) = accepted("report-arguments", list("first"));
    let refused = command(
        &store,
        "read_revision_report",
        json!({"id": id, "delta": {}}),
    )
    .unwrap_err();
    assert_eq!(refused.kind, "bad-request", "{refused:?}");
}

/// A product whose acceptance pins another manifest than the list it was taken of. By
/// construction the manifest an acceptance pins IS the list it was taken of, so the two agree on
/// every real path; this holds the check for the day they do not (a staging that laid out
/// another manifest): the words would then be about a list the owner never accepted, and the
/// page would say so with confidence.
struct PinsAnotherManifest;

impl Acceptor for PinsAnotherManifest {
    fn report_requirements(&self, snapshot: &Snapshot) -> Result<RequirementsReport, RenderError> {
        FakeRenderer.report_requirements(snapshot)
    }

    fn take_acceptance(&self, snapshot: &Snapshot) -> Result<Taken, RenderError> {
        let mut taken = FakeRenderer.take_acceptance(snapshot)?;
        let mut record: Value = serde_json::from_str(&taken.record).expect("a fake record");
        record["manifest"]["sha256"] = json!("b".repeat(64));
        taken.record = record.to_string();
        Ok(taken)
    }

    fn check_acceptance(
        &self,
        snapshot: &Snapshot,
        record: &str,
    ) -> Result<CheckOutcome, RenderError> {
        FakeRenderer.check_acceptance(snapshot, record)
    }

    fn delta_acceptance(
        &self,
        snapshot: &Snapshot,
        record: &str,
    ) -> Result<Vec<Value>, RenderError> {
        FakeRenderer.delta_acceptance(snapshot, record)
    }
}

impl ModelReviewer for PinsAnotherManifest {
    fn review(&self, request: &ReviewRequest<'_>) -> Result<Review, RenderError> {
        FakeRenderer.review(request)
    }
}

impl FigureRenderer for PinsAnotherManifest {
    fn render(&self, request: &FigureRequest<'_>) -> Result<FigureSet, RenderError> {
        FakeRenderer.render(request)
    }
}

#[test]
fn an_acceptance_that_pinned_another_manifest_than_the_list_is_refused() {
    let store = WorkStore::with_clock(
        common::scratch("report-other-pin"),
        FixedClock("2026-10-09T09:00:00Z".to_string()),
    );
    let ask = |name: &str, args: Value| call(&store, &PinsAnotherManifest, name, args);
    let id = ask("create_work", json!({"title": "Lamp"})).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();
    let source = ask(
        "save_source",
        json!({"id": id, "text": "The lamp starts off."}),
    )
    .unwrap()["revision"]
        .clone();
    ask(
        "save_model",
        json!({"id": id, "text": "<scxml><state id=\"a\"/></scxml>", "written_for": source}),
    )
    .unwrap();
    let mut first = list("first");
    first["id"] = json!(id);
    first["written_for"] = source.clone();
    let held = ask("save_requirements", first).unwrap()["revision"].clone();
    let shown = ask("requirements_report", json!({"id": id})).unwrap();
    ask("accept", json!({"id": id, "expect": shown["basis"]})).unwrap();
    let mut second = list("second");
    second["id"] = json!(id);
    second["base"] = held;
    second["written_for"] = source;
    ask("save_requirements", second).unwrap();
    let refused = ask("read_revision_report", json!({"id": id})).unwrap_err();
    assert_eq!(refused.kind, "revision-not-judged", "{refused:?}");
    assert!(refused.message.contains("not the same list"), "{refused:?}");
}
