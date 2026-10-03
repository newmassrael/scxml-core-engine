// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What the owner accepts of a design: the requirement list a text is read into, the
//! product's measure of the design against it, and the record that pins what was shown
//! when the owner said yes.
//!
//! What is measured here is what the application owes. The product decides whether a
//! requirement is carried and whether an acceptance still holds (the stand-in decides by
//! the bytes of each pinned file, as the product does); the application owes that the
//! owner accepts what they were SHOWN, that a design for an old text is not accepted as
//! an answer to a new one, and that a refusal or a silence leaves nothing accepted.

mod common;

use std::path::PathBuf;

use sce_app_core::{call, CommandError, FixedClock, Revision, WorkId, WorkStore};
use serde_json::{json, Value};

use common::{scratch, FakeRenderer, RefusingRenderer};

const T1: &str = "2026-10-03T09:00:01Z";
const T2: &str = "2026-10-03T09:00:02Z";

const MODEL: &str = "<scxml xmlns=\"http://www.w3.org/2005/07/scxml\" version=\"1.0\"/>";
const MANIFEST: &str =
    "{\"doc_id\":\"door\",\"rev\":\"1\",\"requirements\":[{\"id\":\"R1\"},{\"id\":\"R2\"}]}\n";
const SIDECAR: &str = "{\"doc_id\":\"door\",\"rev\":\"1\",\"text\":{\"R1\":\"The door opens.\"}}\n";

struct Fixture {
    root: PathBuf,
    id: WorkId,
}

impl Fixture {
    fn new(label: &str) -> Fixture {
        let root = scratch(label);
        let id = WorkStore::with_clock(&root, FixedClock(T1.to_string()))
            .create_work("Door lock")
            .unwrap()
            .id;
        Fixture { root, id }
    }

    fn store(&self, now: &str) -> WorkStore<FixedClock> {
        WorkStore::with_clock(&self.root, FixedClock(now.to_string()))
    }

    fn run(&self, name: &str, args: Value) -> Result<Value, CommandError> {
        self.run_at(T1, name, args)
    }

    fn run_at(&self, now: &str, name: &str, args: Value) -> Result<Value, CommandError> {
        call(&self.store(now), &FakeRenderer, name, args)
    }

    fn ask(&self, name: &str) -> Value {
        self.run(name, json!({"id": self.id.as_str()})).unwrap()
    }

    fn head(&self) -> Revision {
        self.store(T1).head(&self.id).unwrap().expect("a text")
    }

    fn save_source(&self, text: &str, base: Option<&Revision>) -> Revision {
        revision_of(
            &self
                .run(
                    "save_source",
                    json!({"id": self.id.as_str(), "text": text, "base": base}),
                )
                .unwrap(),
        )
    }

    fn save_model(&self, model: Value, base: Option<&Revision>) -> Revision {
        let mut args = json!({
            "id": self.id.as_str(),
            "base": base,
            "written_for": self.head(),
        });
        args.as_object_mut()
            .unwrap()
            .extend(model.as_object().unwrap().clone());
        revision_of(&self.run("save_model", args).unwrap())
    }

    fn save_requirements(&self, base: Option<&Revision>) -> Revision {
        revision_of(
            &self
                .run(
                    "save_requirements",
                    json!({
                        "id": self.id.as_str(),
                        "manifest": MANIFEST,
                        "sidecar": SIDECAR,
                        "base": base,
                        "written_for": self.head(),
                    }),
                )
                .unwrap(),
        )
    }

    /// What the owner is shown: the revisions of everything the page is about.
    fn shown(&self) -> Value {
        self.ask("requirements_report")["basis"].clone()
    }

    fn accept(&self, expect: Value) -> Result<Value, CommandError> {
        self.accept_at(T1, expect)
    }

    fn accept_at(&self, now: &str, expect: Value) -> Result<Value, CommandError> {
        self.run_at(
            now,
            "accept",
            json!({"id": self.id.as_str(), "expect": expect}),
        )
    }

    fn accepted(&self) -> usize {
        self.store(T1).acceptance_history(&self.id).unwrap().len()
    }
}

fn revision_of(answer: &Value) -> Revision {
    serde_json::from_value(answer["revision"].clone()).expect("a revision")
}

/// A work with a text, a design for it and a requirement list for it, all saved
/// against the text as it stands.
fn ready(label: &str, model: &str) -> Fixture {
    let f = Fixture::new(label);
    f.save_source("The door opens for a listed card.", None);
    f.save_model(json!({"text": model}), None);
    f.save_requirements(None);
    f
}

/// Nothing is accepted of a work that has no design or no list, and a work nobody
/// accepted anything of says so instead of failing.
#[test]
fn nothing_is_accepted_before_the_text_the_design_and_the_list_exist() {
    let f = Fixture::new("acceptance-nothing-yet");
    let none = f.ask("read_acceptance");
    assert_eq!(none["acceptance"], Value::Null);
    assert_eq!(none["standing"], "none");
    assert_eq!(none["lapse"], Value::Null);

    f.save_source("The door opens.", None);
    let bare = f
        .accept(json!({"source": f.head(), "model": f.head(), "requirements": f.head()}))
        .expect_err("a work with no design cannot be accepted");
    assert_eq!(bare.kind, "not-found");
    assert!(bare.message.contains("a model"), "{}", bare.message);

    f.save_model(json!({"text": MODEL}), None);
    let listless = f.run("requirements_report", json!({"id": f.id.as_str()}));
    assert_eq!(listless.unwrap_err().kind, "not-found");
    assert_eq!(f.accepted(), 0);
}

/// The owner reads the page, accepts what it showed, and the acceptance holds: the
/// record is the product's, the channel is the application's own button, and the
/// design's questions are kept as they stood.
#[test]
fn the_owner_accepts_what_was_shown_and_it_holds() {
    let f = ready("acceptance-holds", MODEL);
    let report = f.ask("requirements_report");
    assert_eq!(report["model_standing"], "current");
    assert_eq!(report["requirements_standing"], "current");
    assert_eq!(report["denominator"], "synthesized");
    assert_eq!(report["outcomes"].as_array().unwrap().len(), 2);
    assert_eq!(report["outcomes"][0]["outcome"], "implemented");
    assert!(report["page"]
        .as_str()
        .unwrap()
        .contains("2 requirement(s)"));

    let saved = f.accept(report["basis"].clone()).unwrap();
    assert_eq!(saved["outcome"], "saved");
    let read = f.ask("read_acceptance");
    assert_eq!(read["standing"], "holds");
    assert_eq!(read["lapse"], Value::Null);
    assert_eq!(read["acceptance"]["revision"], saved["revision"]);
    assert_eq!(read["acceptance"]["accepted_at"], T1);
    assert_eq!(
        read["acceptance"]["channel"], "direct",
        "the app's button states the direct channel"
    );
    assert_eq!(read["acceptance"]["basis"], report["basis"]);
    assert_eq!(read["acceptance"]["open"], json!([]));
    assert_eq!(read["now"], report["basis"], "nothing moved since");
}

/// A design that misses a requirement or leaves a question open is still the owner's
/// to accept: the application tells them and keeps what was open when they decided.
#[test]
fn a_design_with_a_gap_is_told_to_the_owner_and_stays_theirs_to_accept() {
    let f = ready("acceptance-gap", "<scxml><!-- MISSING OPEN --></scxml>");
    let report = f.ask("requirements_report");
    assert_eq!(report["outcomes"][0]["outcome"], "missing");
    assert_eq!(report["outcomes"][0]["node_paths"], json!([]));
    assert_eq!(report["outcomes"][1]["outcome"], "implemented");

    f.accept(report["basis"].clone()).unwrap();
    let read = f.ask("read_acceptance");
    assert_eq!(read["standing"], "holds");
    assert_eq!(
        read["acceptance"]["open"],
        json!(["1 question(s) the specification leaves open (open-guard)"]),
        "what was open when they said yes is part of the record"
    );
}

/// The owner accepts what they were shown. If anything moved since (a text saved from
/// another window, the client's next design), nothing is accepted and they are told
/// which, with what is current.
#[test]
fn what_moved_after_it_was_shown_is_not_accepted() {
    let f = ready("acceptance-moved", MODEL);
    let shown = f.shown();

    let model = revision_of(&f.ask("read_model")["model"]);
    f.save_model(
        json!({"text": "<scxml><!-- another --></scxml>"}),
        Some(&model),
    );

    let refusal = f.accept(shown.clone()).expect_err("the design moved");
    assert_eq!(refusal.kind, "moved");
    assert_eq!(refusal.detail["moved"], json!(["model"]));
    assert_eq!(refusal.detail["current"], f.shown());
    assert_ne!(refusal.detail["current"]["model"], shown["model"]);
    assert_eq!(f.accepted(), 0, "nothing was written");
}

/// The answers are part of what is shown and accepted: an owner who saw none does not
/// accept the work with some, and one who answered since is told.
#[test]
fn the_owners_answers_are_part_of_what_is_accepted() {
    let f = ready("acceptance-answers", MODEL);
    let before = f.shown();
    assert!(
        before["answers"].is_null(),
        "the owner had answered nothing"
    );

    f.run(
        "save_answers",
        json!({"id": f.id.as_str(), "answers": {"open-guard": "Any listed card."}}),
    )
    .unwrap();
    let refusal = f
        .accept(before)
        .expect_err("answered after the page was shown");
    assert_eq!(refusal.kind, "moved");
    assert_eq!(refusal.detail["moved"], json!(["answers"]));

    f.accept(f.shown()).unwrap();
    assert_eq!(f.ask("read_acceptance")["standing"], "holds");

    let held = revision_of(&f.ask("read_answers")["answers"]);
    f.run(
        "save_answers",
        json!({
            "id": f.id.as_str(),
            "answers": {"open-guard": "Only cards on today's list."},
            "base": held,
        }),
    )
    .unwrap();
    let read = f.ask("read_acceptance");
    assert_eq!(read["standing"], "lapsed");
    assert_eq!(read["lapse"], "spec/answers.json moved");
}

/// A design written for an earlier text is not an answer to this one: when the text
/// moves on, neither the design nor the list can be accepted until each is written for it.
#[test]
fn a_design_for_an_earlier_text_cannot_be_accepted_as_an_answer_to_a_newer_one() {
    let f = ready("acceptance-behind", MODEL);
    let first = f.head();
    f.save_source(
        "The door opens for a listed card and closes after ten seconds.",
        Some(&first),
    );

    let report = f.ask("requirements_report");
    assert_eq!(report["model_standing"], "behind");
    assert_eq!(report["requirements_standing"], "behind");

    let refusal = f.accept(report["basis"].clone()).expect_err("behind");
    assert_eq!(refusal.kind, "not-current");
    assert_eq!(refusal.detail["behind"], json!(["model", "requirements"]));
    assert_eq!(refusal.detail["source_head"], json!(f.head()));
    assert_eq!(f.accepted(), 0);

    // Each is brought up to the text in its own save; accepting then goes through.
    let model = revision_of(&f.ask("read_model")["model"]);
    f.save_model(json!({"text": MODEL}), Some(&model));
    let still = f.accept(f.shown()).expect_err("the list is behind yet");
    assert_eq!(still.detail["behind"], json!(["requirements"]));
    let list = revision_of(&f.ask("read_requirements")["requirements"]);
    f.save_requirements(Some(&list));
    f.accept(f.shown()).unwrap();
    assert_eq!(f.accepted(), 1);
}

/// Whether an acceptance still holds is the product's to say about the work as it is
/// now: a design that moved lapses it, naming the file, and a design put back to the
/// bytes that were accepted is the design that was accepted.
#[test]
fn an_acceptance_lapses_when_a_pinned_file_moves_and_says_which() {
    let f = ready("acceptance-lapses", MODEL);
    f.accept(f.shown()).unwrap();
    assert_eq!(f.ask("read_acceptance")["standing"], "holds");

    let model = revision_of(&f.ask("read_model")["model"]);
    let moved = f.save_model(
        json!({"text": "<scxml><!-- edited --></scxml>"}),
        Some(&model),
    );
    let read = f.ask("read_acceptance");
    assert_eq!(read["standing"], "lapsed");
    assert_eq!(read["lapse"], "design/model.scxml moved");
    assert_eq!(read["acceptance"]["basis"]["model"], json!(model));
    assert_eq!(
        read["now"]["model"],
        json!(moved),
        "the acceptance says what it was of"
    );

    f.save_model(json!({"text": MODEL}), Some(&moved));
    assert_eq!(
        f.ask("read_acceptance")["standing"],
        "holds",
        "the product compares the files, not the revisions the application gave them"
    );

    let head = f.head();
    f.save_source("The door opens for a card on the list.", Some(&head));
    assert_eq!(f.ask("read_acceptance")["lapse"], "spec/source.txt moved");

    // More than one thing moved is still the product's one sentence, passed on whole.
    f.save_model(
        json!({"text": "<scxml><!-- again --></scxml>"}),
        Some(&current_model(&f)),
    );
    assert_eq!(
        f.ask("read_acceptance")["lapse"],
        "design/model.scxml moved; spec/source.txt moved"
    );
}

/// The revision of the design as it is kept now.
fn current_model(f: &Fixture) -> Revision {
    revision_of(&f.ask("read_model")["model"])
}

/// A design of several documents is pinned document by document, so the one that
/// moved is the one named.
#[test]
fn a_design_of_several_documents_names_the_document_that_moved() {
    let f = Fixture::new("acceptance-set");
    f.save_source("The door opens for a listed card.", None);
    let documents = |lib: &str| {
        json!({
            "documents": [
                {"name": "door.scxml", "text": "<scxml><!-- door --></scxml>"},
                {"name": "cards.scxml", "text": lib},
            ],
            "entry": "door.scxml",
        })
    };
    let first = f.save_model(documents("<scxml><!-- cards 1 --></scxml>"), None);
    f.save_requirements(None);
    f.accept(f.shown()).unwrap();
    assert_eq!(f.ask("read_acceptance")["standing"], "holds");

    f.save_model(documents("<scxml><!-- cards 2 --></scxml>"), Some(&first));
    assert_eq!(
        f.ask("read_acceptance")["lapse"],
        "design/cards.scxml moved"
    );
}

/// The product refusing the design, or not answering at all, leaves nothing accepted
/// and says which of the two it was; the list the owner kept is untouched.
#[test]
fn a_product_that_refuses_or_is_silent_accepts_nothing() {
    let f = ready(
        "acceptance-no-product",
        "<scxml><!-- UNACCEPTABLE --></scxml>",
    );
    let shown = f.shown();
    let refused = f.accept(shown.clone()).expect_err("the product refuses");
    assert_eq!(refused.kind, "sce-refused");
    assert_eq!(refused.detail["code"], "xml/parse-error");
    assert_eq!(f.accepted(), 0);

    let silent = call(
        &f.store(T1),
        &RefusingRenderer,
        "accept",
        json!({"id": f.id.as_str(), "expect": shown}),
    )
    .expect_err("the product is silent");
    assert_eq!(silent.kind, "sce-timeout");
    assert_eq!(f.accepted(), 0);
    assert!(f
        .store(T1)
        .read_requirements(&f.id, None)
        .unwrap()
        .is_some());
}

/// Accepting again is a new entry of the history when the time differs and no change
/// when nothing does; what is read is the latest.
#[test]
fn accepting_again_adds_to_the_history_and_the_latest_is_what_is_read() {
    let f = ready("acceptance-history", MODEL);
    let shown = f.shown();
    let first = f.accept_at(T1, shown.clone()).unwrap();
    assert_eq!(first["outcome"], "saved");
    let same = f.accept_at(T1, shown.clone()).unwrap();
    assert_eq!(same["outcome"], "unchanged");
    assert_eq!(f.accepted(), 1);

    let second = f.accept_at(T2, shown).unwrap();
    assert_eq!(second["outcome"], "saved");
    assert_eq!(f.accepted(), 2);
    let read = f.ask("read_acceptance");
    assert_eq!(read["acceptance"]["revision"], second["revision"]);
    assert_eq!(read["acceptance"]["accepted_at"], T2);
}

/// A list is two JSON objects the product reads, kept as bytes against the text it
/// was read from; what is not a list is refused before anything is written.
#[test]
fn a_requirement_list_is_kept_as_bytes_and_a_wrong_one_is_refused() {
    let f = Fixture::new("acceptance-list");
    f.save_source("The door opens.", None);
    let empty = f.ask("read_requirements");
    assert_eq!(empty["requirements"], Value::Null);
    assert_eq!(empty["standing"], Value::Null);

    for (manifest, sidecar) in [("not json", None), ("[1]", None), (MANIFEST, Some("[]"))] {
        let refused = f
            .run(
                "save_requirements",
                json!({"id": f.id.as_str(), "manifest": manifest, "sidecar": sidecar}),
            )
            .expect_err("not a list");
        assert_eq!(
            refused.kind, "invalid-requirements",
            "{manifest:?} {sidecar:?}"
        );
    }
    let nobody = Revision::of(b"a text this work never held");
    let unknown = f
        .run(
            "save_requirements",
            json!({"id": f.id.as_str(), "manifest": MANIFEST, "written_for": nobody}),
        )
        .expect_err("written for a text nobody saved");
    assert_eq!(unknown.kind, "not-found");
    assert!(f
        .store(T1)
        .read_requirements(&f.id, None)
        .unwrap()
        .is_none());

    let first = f.save_requirements(None);
    let read = f.ask("read_requirements");
    assert_eq!(read["requirements"]["manifest"], MANIFEST, "byte for byte");
    assert_eq!(read["requirements"]["sidecar"], SIDECAR);
    assert_eq!(read["standing"], "current");
    assert_eq!(read["requirements"]["revision"], json!(first));

    let stale = f
        .run(
            "save_requirements",
            json!({"id": f.id.as_str(), "manifest": MANIFEST, "base": null}),
        )
        .expect_err("a writer that never read the list");
    assert_eq!(stale.kind, "conflict");

    let head = f.head();
    f.save_source("The door opens for a listed card.", Some(&head));
    assert_eq!(f.ask("read_requirements")["standing"], "behind");
}
