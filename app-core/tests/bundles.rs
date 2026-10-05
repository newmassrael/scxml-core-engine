// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Candidates and bundles: what a request makes, and how it becomes the work's model.
//!
//! The point of a bundle is that the work's model and its requirement list move together:
//! one pointer, so no reader is handed a model of one generation and a list of another. What
//! is held here is that, against a writer that publishes while readers read, and the rest of
//! what has to be true for it: that only the core's own check lets a bundle through, that the
//! old per-chain saves are refused once a work keeps bundles (so that nothing can move one
//! half), that a process that stops between publishing and writing it down is put right, and
//! that works that never asked for a generation read as they always did.

mod common;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

use sce_app_core::bundle::{BundleCheck, CheckedBy};
use sce_app_core::requests::{Inputs, State};
use sce_app_core::{
    CandidateWrite, ManualClock, Published, Registration, Revision, StoreError, WorkId, WorkStore,
};

const T0: u64 = 1_791_190_800;

struct Fixture {
    clock: Arc<ManualClock>,
    store: WorkStore<Arc<ManualClock>>,
    id: WorkId,
}

fn fixture(label: &str) -> Fixture {
    let clock = Arc::new(ManualClock::at(T0));
    let store = WorkStore::with_clock(common::scratch(label), Arc::clone(&clock));
    let id = store.create_work("Door lock").unwrap().id;
    store.save_source(&id, "The lock opens.", None).unwrap();
    Fixture { clock, store, id }
}

fn core_check() -> BundleCheck {
    BundleCheck {
        by: CheckedBy::Core,
        name: "model".to_string(),
        verdict: "accepted".to_string(),
        generator: Some("sce-codegen 0".to_string()),
        digest: Some(Revision::of(b"record").to_string()),
        subject: None,
    }
}

impl Fixture {
    fn source(&self) -> Revision {
        self.store.head(&self.id).unwrap().expect("a text")
    }

    fn inputs(&self) -> Inputs {
        Inputs {
            source: self.source(),
            answers: self
                .store
                .read_answers(&self.id, None)
                .unwrap()
                .map(|a| a.revision),
        }
    }

    /// A request, taken by `adapter-a`.
    fn running(&self, key: &str) -> String {
        let made = self
            .store
            .register_request(
                &self.id,
                Registration {
                    key,
                    origin: "gui",
                    expect: self.inputs(),
                    supersede: false,
                },
            )
            .unwrap();
        self.store
            .claim_request(&self.id, &made.request.id, "adapter-a", None, false)
            .unwrap();
        made.request.id
    }

    fn write_both(&self, request: &str, tag: &str) {
        self.store
            .save_candidate(
                &self.id,
                request,
                "adapter-a",
                1,
                CandidateWrite {
                    model: Some(format!("<scxml>{tag}</scxml>")),
                    requirements: Some(format!("{{\"list\":\"{tag}\"}}")),
                    instructions: None,
                },
            )
            .unwrap();
    }

    fn publish(&self, request: &str) -> Published {
        self.store
            .publish_candidate(&self.id, request, "adapter-a", 1, vec![core_check()])
            .unwrap()
    }

    /// A request that is written for and published, start to finish.
    fn generate(&self, key: &str, tag: &str) -> Published {
        let request = self.running(key);
        self.write_both(&request, tag);
        self.publish(&request)
    }
}

fn kind_of(error: StoreError) -> (&'static str, serde_json::Value) {
    match error {
        StoreError::Refused { kind, detail, .. } => (kind, detail),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

// -- candidates --------------------------------------------------------------

#[test]
fn what_the_executor_writes_is_kept_for_the_request_and_is_not_the_works_model() {
    let f = fixture("bundles-candidate");
    let request = f.running("press-1");

    let view = f
        .store
        .save_candidate(
            &f.id,
            &request,
            "adapter-a",
            1,
            CandidateWrite {
                model: Some("<scxml>draft</scxml>".to_string()),
                requirements: None,
                instructions: None,
            },
        )
        .unwrap();

    let candidate = view.request.candidate.expect("a candidate");
    assert_eq!(candidate.model, Some(Revision::of(b"<scxml>draft</scxml>")));
    assert_eq!(candidate.requirements, None);
    // It is nobody's model yet.
    assert!(f.store.read_model(&f.id, None).unwrap().is_none());
    assert!(f.store.read_work_heads(&f.id).unwrap().model.is_none());
    let texts = f.store.read_candidate(&f.id, &request).unwrap();
    assert_eq!(texts.model.unwrap().1, "<scxml>draft</scxml>");
    assert!(texts.requirements.is_none());
}

#[test]
fn a_later_word_names_the_other_half_and_a_later_model_replaces_the_earlier() {
    let f = fixture("bundles-candidate-add");
    let request = f.running("press-1");
    let write = |model: Option<&str>, requirements: Option<&str>| {
        f.store
            .save_candidate(
                &f.id,
                &request,
                "adapter-a",
                1,
                CandidateWrite {
                    model: model.map(str::to_string),
                    requirements: requirements.map(str::to_string),
                    instructions: None,
                },
            )
            .unwrap()
    };

    write(Some("<scxml>one</scxml>"), None);
    write(None, Some("{\"list\":1}"));
    let view = write(Some("<scxml>two</scxml>"), None);

    let candidate = view.request.candidate.unwrap();
    assert_eq!(candidate.model, Some(Revision::of(b"<scxml>two</scxml>")));
    assert_eq!(candidate.requirements, Some(Revision::of(b"{\"list\":1}")));
}

#[test]
fn only_the_holder_writes_and_only_something_is_written() {
    let f = fixture("bundles-candidate-refused");
    let request = f.running("press-1");
    let draft = || CandidateWrite {
        model: Some("<scxml/>".to_string()),
        requirements: None,
        instructions: None,
    };

    let stranger = f
        .store
        .save_candidate(&f.id, &request, "adapter-b", 1, draft())
        .unwrap_err();
    assert_eq!(stranger.kind(), "not-holder");
    let wrong_attempt = f
        .store
        .save_candidate(&f.id, &request, "adapter-a", 2, draft())
        .unwrap_err();
    assert_eq!(wrong_attempt.kind(), "not-holder");
    let nothing = f
        .store
        .save_candidate(
            &f.id,
            &request,
            "adapter-a",
            1,
            CandidateWrite {
                model: None,
                requirements: None,
                instructions: None,
            },
        )
        .unwrap_err();
    assert_eq!(nothing.kind(), "bad-candidate");
    f.store.cancel_request(&f.id, &request).unwrap();
    let ended = f
        .store
        .save_candidate(&f.id, &request, "adapter-a", 1, draft())
        .unwrap_err();
    assert_eq!(ended.kind(), "request-ended");
}

#[test]
fn an_executor_that_slept_past_its_lease_still_writes_for_the_request() {
    let f = fixture("bundles-candidate-slept");
    let request = f.running("press-1");
    f.clock.advance(3_600);
    assert_eq!(
        f.store.read_request(&f.id, &request).unwrap().state,
        State::Interrupted
    );

    f.write_both(&request, "late");

    assert!(f
        .store
        .read_request(&f.id, &request)
        .unwrap()
        .request
        .candidate
        .unwrap()
        .missing()
        .is_empty());
}

// -- publishing --------------------------------------------------------------

#[test]
fn a_bundle_is_not_published_without_both_halves() {
    let f = fixture("bundles-incomplete");
    let request = f.running("press-1");

    let (kind, detail) = kind_of(
        f.store
            .publish_candidate(&f.id, &request, "adapter-a", 1, vec![core_check()])
            .unwrap_err(),
    );
    assert_eq!(kind, "no-candidate");
    assert_eq!(
        detail["missing"],
        serde_json::json!(["model", "requirements"])
    );

    f.store
        .save_candidate(
            &f.id,
            &request,
            "adapter-a",
            1,
            CandidateWrite {
                model: Some("<scxml/>".to_string()),
                requirements: None,
                instructions: None,
            },
        )
        .unwrap();
    let (_, detail) = kind_of(
        f.store
            .publish_candidate(&f.id, &request, "adapter-a", 1, vec![core_check()])
            .unwrap_err(),
    );
    assert_eq!(detail["missing"], serde_json::json!(["requirements"]));
    assert_eq!(
        f.store.read_request(&f.id, &request).unwrap().state,
        State::Running
    );
}

#[test]
fn a_published_bundle_is_the_works_model_and_list_as_one() {
    let f = fixture("bundles-publish");
    let source = f.source();

    let published = f.generate("press-1", "one");

    assert_eq!(published.request.state, State::Completed);
    let outcome = published
        .request
        .request
        .outcome
        .clone()
        .expect("an outcome");
    assert_eq!(outcome.bundle, published.bundle);
    let model = f.store.read_model(&f.id, None).unwrap().expect("a model");
    assert_eq!(model.text, "<scxml>one</scxml>");
    assert_eq!(
        model.written_for,
        Some(source.clone()),
        "the core's own word, not the client's"
    );
    let list = f
        .store
        .read_requirements(&f.id, None)
        .unwrap()
        .expect("a list");
    assert_eq!(list.text, "{\"list\":\"one\"}");
    assert_eq!(list.written_for, Some(source.clone()));

    let heads = f.store.read_work_heads(&f.id).unwrap();
    assert_eq!(heads.model.as_ref().unwrap().revision, model.revision);
    assert_eq!(heads.requirements.as_ref().unwrap().revision, list.revision);
    assert_eq!(heads.bundle, Some(published.bundle.clone()));

    let read = f.store.read_bundle(&f.id, None).unwrap().expect("a bundle");
    assert_eq!(read.revision, published.bundle);
    assert_eq!(read.bundle.request, published.request.request.id);
    assert_eq!(
        (read.bundle.attempt, read.bundle.executor.as_str()),
        (1, "adapter-a")
    );
    assert_eq!(read.bundle.source, source);
    assert_eq!(read.bundle.answers, None);
    assert_eq!(read.bundle.model, model.revision);
    assert_eq!(read.bundle.requirements, list.revision);
    assert_eq!(read.bundle.checks, vec![core_check()]);
    assert_eq!(read.bundle.published_at, "2026-10-05T09:00:00Z");
    assert!(read.bundle.previous.is_none());
}

#[test]
fn a_bundle_without_the_cores_own_check_is_refused_and_so_is_one_that_was_refused() {
    let f = fixture("bundles-checks");
    let request = f.running("press-1");
    f.write_both(&request, "one");
    let client_only = BundleCheck {
        by: CheckedBy::Client,
        name: "decisions".to_string(),
        verdict: "accepted".to_string(),
        generator: None,
        digest: None,
        subject: None,
    };

    let (kind, _) = kind_of(
        f.store
            .publish_candidate(&f.id, &request, "adapter-a", 1, vec![client_only.clone()])
            .unwrap_err(),
    );
    assert_eq!(kind, "no-core-check", "a report is not a measurement");
    let refused = BundleCheck {
        verdict: "refused".to_string(),
        ..core_check()
    };
    let (kind, detail) = kind_of(
        f.store
            .publish_candidate(&f.id, &request, "adapter-a", 1, vec![refused, client_only])
            .unwrap_err(),
    );
    assert_eq!(kind, "check-refused");
    assert_eq!(detail["checks"], serde_json::json!(["model"]));

    // Nothing moved: the request is still running and the work has no model.
    assert_eq!(
        f.store.read_request(&f.id, &request).unwrap().state,
        State::Running
    );
    assert!(f.store.read_bundle(&f.id, None).unwrap().is_none());
    assert!(f.store.read_model(&f.id, None).unwrap().is_none());
}

#[test]
fn a_check_of_a_model_that_was_written_again_afterwards_is_not_a_check_of_this_one() {
    let f = fixture("bundles-subject");
    let request = f.running("press-1");
    f.write_both(&request, "one");
    let checked = f
        .store
        .read_candidate(&f.id, &request)
        .unwrap()
        .model
        .expect("a model")
        .0;
    // The executor wrote another model after the core ran its check on the first.
    f.write_both(&request, "two");
    let stale = BundleCheck {
        subject: Some(checked.clone()),
        ..core_check()
    };

    let (kind, detail) = kind_of(
        f.store
            .publish_candidate(&f.id, &request, "adapter-a", 1, vec![stale])
            .unwrap_err(),
    );

    assert_eq!(kind, "candidate-moved");
    assert_eq!(detail["checked"], serde_json::json!(checked));
    assert!(f.store.read_bundle(&f.id, None).unwrap().is_none());

    // A check of the model the candidate names now is the check of it.
    let current = f
        .store
        .read_candidate(&f.id, &request)
        .unwrap()
        .model
        .expect("a model")
        .0;
    let fresh = BundleCheck {
        subject: Some(current),
        ..core_check()
    };
    f.store
        .publish_candidate(&f.id, &request, "adapter-a", 1, vec![fresh])
        .unwrap();
}

#[test]
fn a_client_check_that_was_refused_stops_the_bundle_too() {
    let f = fixture("bundles-client-refused");
    let request = f.running("press-1");
    f.write_both(&request, "one");
    let refused = BundleCheck {
        by: CheckedBy::Client,
        name: "decisions".to_string(),
        verdict: "refused".to_string(),
        generator: None,
        digest: None,
        subject: None,
    };

    let (kind, detail) = kind_of(
        f.store
            .publish_candidate(&f.id, &request, "adapter-a", 1, vec![core_check(), refused])
            .unwrap_err(),
    );

    assert_eq!(kind, "check-refused");
    assert_eq!(detail["checks"], serde_json::json!(["decisions"]));
}

#[test]
fn a_publication_said_twice_by_the_same_attempt_is_one_bundle() {
    let f = fixture("bundles-twice");
    let request = f.running("press-1");
    f.write_both(&request, "one");
    let first = f.publish(&request);
    f.clock.advance(30);

    let again = f.publish(&request);

    assert_eq!(again.bundle, first.bundle);
    assert_eq!(again.request.request, first.request.request);
    assert_eq!(f.store.bundle_history(&f.id).unwrap().len(), 1);
}

#[test]
fn only_the_holder_of_the_current_attempt_publishes() {
    let f = fixture("bundles-holder");
    let request = f.running("press-1");
    f.write_both(&request, "one");

    for (holder, attempt) in [("adapter-b", 1), ("adapter-a", 2)] {
        let error = f
            .store
            .publish_candidate(&f.id, &request, holder, attempt, vec![core_check()])
            .unwrap_err();
        assert_eq!(error.kind(), "not-holder", "{holder} {attempt}");
    }
    assert!(f.store.read_bundle(&f.id, None).unwrap().is_none());
}

#[test]
fn a_request_whose_text_moved_under_it_publishes_nothing() {
    let f = fixture("bundles-moved");
    let request = f.running("press-1");
    f.write_both(&request, "one");
    f.store
        .save_source(&f.id, "The lock opens twice.", Some(&f.source()))
        .unwrap();

    let error = f
        .store
        .publish_candidate(&f.id, &request, "adapter-a", 1, vec![core_check()])
        .unwrap_err();

    assert_eq!(error.kind(), "request-ended");
    assert!(f.store.read_bundle(&f.id, None).unwrap().is_none());
    assert!(f.store.read_model(&f.id, None).unwrap().is_none());
}

#[test]
fn a_cancelled_request_publishes_nothing_though_it_had_written_everything() {
    let f = fixture("bundles-cancelled");
    let request = f.running("press-1");
    f.write_both(&request, "one");
    f.store.cancel_request(&f.id, &request).unwrap();

    let error = f
        .store
        .publish_candidate(&f.id, &request, "adapter-a", 1, vec![core_check()])
        .unwrap_err();

    assert_eq!(error.kind(), "request-ended");
    assert!(f.store.read_bundle(&f.id, None).unwrap().is_none());
}

#[test]
fn the_answers_a_request_was_asked_about_are_the_bundles() {
    let f = fixture("bundles-answers");
    f.store.save_answers(&f.id, "{\"a\":1}", None).unwrap();
    let answers = f.store.read_answers(&f.id, None).unwrap().unwrap().revision;

    let published = f.generate("press-1", "one");

    let read = f.store.read_bundle(&f.id, None).unwrap().unwrap();
    assert_eq!(read.bundle.answers, Some(answers));
    assert_eq!(
        published.request.request.inputs.answers,
        read.bundle.answers
    );
}

// -- the work after a bundle -------------------------------------------------

#[test]
fn a_work_that_keeps_bundles_refuses_the_old_saves_of_one_half() {
    let f = fixture("bundles-legacy-refused");
    f.generate("press-1", "one");
    let model_before = f.store.read_model(&f.id, None).unwrap().unwrap().revision;

    let model = f
        .store
        .save_model(
            &f.id,
            "<scxml>legacy</scxml>",
            Some(&model_before),
            Some(&f.source()),
        )
        .unwrap_err();
    let list = f
        .store
        .save_requirements(&f.id, "{}", None, Some(&f.source()))
        .unwrap_err();

    assert_eq!(model.kind(), "bundled-work");
    assert_eq!(list.kind(), "bundled-work");
    assert_eq!(
        f.store.read_model(&f.id, None).unwrap().unwrap().revision,
        model_before
    );
    // The text and the answers are the owner's, and are saved as ever.
    f.store
        .save_source(&f.id, "The lock opens twice.", Some(&f.source()))
        .unwrap();
}

#[test]
fn a_work_that_never_asked_for_a_generation_is_read_and_written_as_it_always_was() {
    let f = fixture("bundles-legacy-works");
    let source = f.source();

    f.store
        .save_model(&f.id, "<scxml>old</scxml>", None, Some(&source))
        .unwrap();
    f.store
        .save_requirements(&f.id, "{}", None, Some(&source))
        .unwrap();

    let model = f.store.read_model(&f.id, None).unwrap().unwrap();
    assert_eq!(model.text, "<scxml>old</scxml>");
    assert_eq!(model.written_for, Some(source));
    assert!(f.store.read_bundle(&f.id, None).unwrap().is_none());
    assert!(f.store.read_work_heads(&f.id).unwrap().bundle.is_none());
}

#[test]
fn a_bundle_says_which_instructions_its_executor_worked_to_and_which_bundle_it_replaced() {
    let f = fixture("bundles-instructions");
    let first = f.generate("press-1", "one");
    let request = f.running("press-2");
    let write = |model: Option<&str>, requirements: Option<&str>, instructions: Option<&str>| {
        f.store
            .save_candidate(
                &f.id,
                &request,
                "adapter-a",
                1,
                CandidateWrite {
                    model: model.map(str::to_string),
                    requirements: requirements.map(str::to_string),
                    instructions: instructions.map(str::to_string),
                },
            )
            .unwrap()
    };

    // Said with the first half, and kept when the other half is written without saying it again.
    let said = write(
        Some("<scxml>two</scxml>"),
        None,
        Some("claude-code/0123456789ab"),
    );
    let kept = write(None, Some("{\"list\":\"two\"}"), None);
    let second = f.publish(&request);

    assert_eq!(
        said.request.candidate.unwrap().instructions.as_deref(),
        Some("claude-code/0123456789ab")
    );
    assert_eq!(
        kept.request.candidate.unwrap().instructions.as_deref(),
        Some("claude-code/0123456789ab")
    );
    let first_read = f
        .store
        .read_bundle(&f.id, Some(&first.bundle))
        .unwrap()
        .unwrap();
    let second_read = f
        .store
        .read_bundle(&f.id, Some(&second.bundle))
        .unwrap()
        .unwrap();
    assert_eq!(
        first_read.bundle.instructions, None,
        "an executor that said nothing of its instructions"
    );
    assert_eq!(
        first_read.bundle.replaces, None,
        "the first bundle replaced none"
    );
    assert_eq!(
        second_read.bundle.instructions.as_deref(),
        Some("claude-code/0123456789ab")
    );
    assert_eq!(second_read.bundle.replaces, Some(first.bundle));
}

#[test]
fn instructions_are_named_in_plain_visible_characters_and_nothing_else_is_kept() {
    let f = fixture("bundles-bad-instructions");
    let request = f.running("press-1");
    for bad in [
        "",
        "two words",
        "tab\tinside",
        &"x".repeat(129),
        "caf\u{e9}",
    ] {
        let refused = f
            .store
            .save_candidate(
                &f.id,
                &request,
                "adapter-a",
                1,
                CandidateWrite {
                    model: Some("<scxml/>".to_string()),
                    requirements: None,
                    instructions: Some(bad.to_string()),
                },
            )
            .unwrap_err();
        assert_eq!(refused.kind(), "bad-instructions", "{bad:?}");
    }
    // Nothing was written for any of them.
    assert!(f
        .store
        .read_candidate(&f.id, &request)
        .unwrap()
        .model
        .is_none());
}

#[test]
fn the_first_bundle_of_a_work_that_had_a_model_says_what_it_took_over_from() {
    let f = fixture("bundles-lineage");
    let source = f.source();
    f.store
        .save_model(&f.id, "<scxml>old</scxml>", None, Some(&source))
        .unwrap();
    f.store
        .save_requirements(&f.id, "{\"old\":1}", None, Some(&source))
        .unwrap();

    f.generate("press-1", "one");
    f.store
        .save_source(&f.id, "The lock opens twice.", Some(&source))
        .unwrap();
    f.generate("press-2", "two");

    let history = f.store.bundle_history(&f.id).unwrap();
    assert_eq!(history.len(), 2);
    let first = f
        .store
        .read_bundle(&f.id, Some(&history[0].revision))
        .unwrap()
        .unwrap();
    let previous = first
        .bundle
        .previous
        .expect("the first bundle says what it took over from");
    assert_eq!(previous.model, Some(Revision::of(b"<scxml>old</scxml>")));
    assert_eq!(previous.requirements, Some(Revision::of(b"{\"old\":1}")));
    let second = f.store.read_bundle(&f.id, None).unwrap().unwrap();
    assert!(second.bundle.previous.is_none());
    assert_eq!(history[1].parent, Some(history[0].revision.clone()));
}

#[test]
fn the_models_history_of_a_bundled_work_continues_where_the_chain_stopped() {
    let f = fixture("bundles-model-history");
    let source = f.source();
    f.store
        .save_model(&f.id, "<scxml>old</scxml>", None, Some(&source))
        .unwrap();
    f.store
        .save_requirements(&f.id, "{}", None, Some(&source))
        .unwrap();
    f.generate("press-1", "one");

    let history = f.store.model_history(&f.id).unwrap();

    let revisions: Vec<Revision> = history.iter().map(|e| e.revision.clone()).collect();
    assert_eq!(
        revisions,
        vec![
            Revision::of(b"<scxml>old</scxml>"),
            Revision::of(b"<scxml>one</scxml>")
        ]
    );
    assert_eq!(history[1].written_for, Some(source));
}

#[test]
fn a_revision_of_the_model_named_by_its_digest_is_read_with_what_a_bundle_says_it_was_written_for()
{
    let f = fixture("bundles-named");
    let source = f.source();
    f.generate("press-1", "one");
    f.store
        .save_source(&f.id, "The lock opens twice.", Some(&source))
        .unwrap();
    f.generate("press-2", "two");

    let first = f
        .store
        .read_model(&f.id, Some(&Revision::of(b"<scxml>one</scxml>")))
        .unwrap()
        .unwrap();

    assert_eq!(first.text, "<scxml>one</scxml>");
    assert_eq!(first.written_for, Some(source));
}

// -- a process that stops, and readers that read ------------------------------

#[test]
fn a_bundle_published_and_not_written_down_completes_its_request_when_the_work_is_next_read() {
    let f = fixture("bundles-recovery");
    let request = f.running("press-1");
    f.write_both(&request, "one");
    let published = f.publish(&request);
    // The process stopped after the pointer moved and before the request was written down:
    // the request is as it was when it was running.
    let path = f
        .store
        .root()
        .join(f.id.as_str())
        .join("requests")
        .join(format!("{request}.json"));
    let mut running = published.request.request.clone();
    running.state = State::Running;
    running.outcome = None;
    running.ended_at = None;
    std::fs::write(&path, serde_json::to_string_pretty(&running).unwrap()).unwrap();

    let view = f.store.read_request(&f.id, &request).unwrap();
    let heads = f.store.read_work_heads(&f.id).unwrap();

    assert_eq!(
        view.state,
        State::Completed,
        "the bundle says it was made by this request"
    );
    assert_eq!(view.request.outcome.unwrap().bundle, published.bundle);
    assert_eq!(heads.request.unwrap().state, State::Completed);
    // Said again, it is the same completion and the work is not published a second time.
    let again = f.publish(&request);
    assert_eq!(again.bundle, published.bundle);
    assert_eq!(f.store.bundle_history(&f.id).unwrap().len(), 1);
    assert_eq!(
        f.store.read_request(&f.id, &request).unwrap().request.state,
        State::Completed,
        "and written down now"
    );
}

/// The writer publishes bundle after bundle, each a model and a list that name the same
/// number. At every moment the work was in, the model and the list it shows name the same
/// one: a reader that took the model's pointer and then the list's, with a publish between,
/// would show a model of one generation beside a list of another.
fn assert_one_generation(model: &str, list: &str) {
    let m = model
        .trim_start_matches("<scxml>")
        .trim_end_matches("</scxml>");
    let l = list
        .trim_start_matches("{\"list\":\"")
        .trim_end_matches("\"}");
    assert_eq!(m, l, "a model of {m} beside a list of {l}");
}

#[test]
fn readers_of_a_work_that_publishes_never_see_a_model_and_a_list_of_two_generations() {
    const ROUNDS: usize = 60;
    let f = fixture("bundles-race");
    let done = AtomicBool::new(false);

    thread::scope(|scope| {
        scope.spawn(|| {
            for round in 0..ROUNDS {
                // A text for the round, so that the request is asked about something new.
                f.store
                    .save_source(
                        &f.id,
                        &format!("The lock opens {round} times."),
                        Some(&f.source()),
                    )
                    .unwrap();
                f.generate(&format!("press-{round}"), &round.to_string());
            }
            done.store(true, Ordering::SeqCst);
        });

        let mut looked = 0;
        while !done.load(Ordering::SeqCst) {
            let snapshot = f.store.read_work_snapshot(&f.id).unwrap();
            if let (Some(model), Some(list)) = (&snapshot.model, &snapshot.requirements) {
                assert_one_generation(&model.text, &list.text);
            }
            let model = f.store.read_model(&f.id, None).unwrap();
            let list = f.store.read_requirements(&f.id, None).unwrap();
            // Two reads are two moments; between them a publish may land, and the pair
            // is then of two moments. The snapshot above is the one that is one state.
            let _ = (model, list);
            let heads = f.store.read_work_heads(&f.id).unwrap();
            if let (Some(model), Some(list)) = (&heads.model, &heads.requirements) {
                // The same bundle named both: both written for the same text.
                assert_eq!(model.written_for, list.written_for);
            }
            looked += 1;
        }
        assert!(looked > 0);
    });
}
