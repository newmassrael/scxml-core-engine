// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Generation requests, kept in the works folder.
//!
//! The pure state machine (`requests.rs`) is tested where it is. What is held here is
//! what only a folder can break: that two callers who meet at the lock agree on who got
//! there first, that a request survives a new store opened on the same folder, that a
//! save of the text or of the answers ends the request that was asked about the old one,
//! and that the lease runs out by the clock and not by anybody writing it down.

mod common;

use std::sync::{Arc, Barrier};
use std::thread;

use sce_app_core::bundle::{BundleCheck, CheckedBy};
use sce_app_core::requests::{Inputs, State};
use sce_app_core::{
    CandidateWrite, ManualClock, Registration, RequestView, Revision, StoreError, WorkId, WorkStore,
};

const T0: u64 = 1_791_190_800;

/// A store whose clock the test holds, and a work with a text.
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

    fn registration<'a>(&self, key: &'a str) -> Registration<'a> {
        Registration {
            key,
            origin: "gui",
            expect: self.inputs(),
            supersede: false,
        }
    }

    fn register(&self, key: &str) -> String {
        self.store
            .register_request(&self.id, self.registration(key))
            .unwrap()
            .request
            .id
    }

    fn state_of(&self, request: &str) -> State {
        self.store.read_request(&self.id, request).unwrap().state
    }

    /// The executor writes a model and a list and says it is done: the request completes
    /// by publishing, which is the only way a request completes.
    fn complete(
        &self,
        request: &str,
        holder: &str,
        attempt: u32,
    ) -> Result<RequestView, StoreError> {
        self.store.save_candidate(
            &self.id,
            request,
            holder,
            attempt,
            CandidateWrite {
                model: Some(format!("<scxml>{request}</scxml>")),
                requirements: Some(format!("{{\"list\":\"{request}\"}}")),
                instructions: None,
            },
        )?;
        self.publish(request, holder, attempt)
    }

    /// Say it is done, as an executor does, with the check the core made of the model.
    fn publish(
        &self,
        request: &str,
        holder: &str,
        attempt: u32,
    ) -> Result<RequestView, StoreError> {
        let core = BundleCheck {
            by: CheckedBy::Core,
            name: "model".to_string(),
            verdict: "accepted".to_string(),
            generator: None,
            digest: None,
            subject: None,
        };
        self.store
            .publish_candidate(&self.id, request, holder, attempt, vec![core])
            .map(|published| published.request)
    }

    /// Save a text that differs from the current one.
    fn move_the_text(&self, text: &str) {
        self.store
            .save_source(&self.id, text, Some(&self.source()))
            .unwrap();
    }
}

fn refused(error: StoreError) -> (&'static str, String) {
    match error {
        StoreError::Refused { kind, message, .. } => (kind, message),
        other => panic!("expected a refusal of the request, got {other:?}"),
    }
}

// -- registering ------------------------------------------------------------

#[test]
fn a_request_is_registered_against_the_text_and_answers_it_was_asked_about() {
    let f = fixture("requests-register");

    let registered = f
        .store
        .register_request(&f.id, f.registration("press-1"))
        .unwrap();

    assert!(registered.created);
    let request = &registered.request;
    assert_eq!(registered.state, State::Queued);
    assert_eq!(request.inputs, f.inputs());
    assert_eq!(
        (request.seq, request.attempt, request.origin.as_str()),
        (1, 0, "gui")
    );
    assert_eq!(request.created_at, "2026-10-05T09:00:00Z");
    assert!(request.id.starts_with("req-"), "{}", request.id);
}

#[test]
fn a_registration_repeated_is_the_request_it_repeats() {
    let f = fixture("requests-key");
    let first = f.register("press-1");

    let again = f
        .store
        .register_request(&f.id, f.registration("press-1"))
        .unwrap();

    assert!(!again.created);
    assert_eq!(again.request.id, first);
    assert_eq!(f.store.list_requests(&f.id).unwrap().len(), 1);
}

#[test]
fn a_key_used_for_other_inputs_is_refused_and_nothing_is_registered() {
    let f = fixture("requests-key-reused");
    f.register("press-1");
    f.move_the_text("The lock opens twice.");

    let (kind, _) = refused(
        f.store
            .register_request(&f.id, f.registration("press-1"))
            .unwrap_err(),
    );

    assert_eq!(kind, "key-reused");
    assert_eq!(f.store.list_requests(&f.id).unwrap().len(), 1);
}

#[test]
fn a_request_asked_about_a_text_that_has_moved_is_refused_and_says_what_moved() {
    let f = fixture("requests-moved");
    let stale = f.inputs();
    f.move_the_text("The lock opens twice.");
    f.store.save_answers(&f.id, "{}", None).unwrap();

    let error = f
        .store
        .register_request(
            &f.id,
            Registration {
                key: "press-1",
                origin: "gui",
                expect: stale,
                supersede: false,
            },
        )
        .unwrap_err();

    match error {
        StoreError::Refused { kind, detail, .. } => {
            assert_eq!(kind, "moved");
            assert_eq!(detail["moved"], serde_json::json!(["source", "answers"]));
        }
        other => panic!("{other:?}"),
    }
    assert!(f.store.list_requests(&f.id).unwrap().is_empty());
}

#[test]
fn a_work_with_no_text_cannot_be_asked_about() {
    let clock = Arc::new(ManualClock::at(T0));
    let store = WorkStore::with_clock(common::scratch("requests-no-text"), clock);
    let id = store.create_work("Empty").unwrap().id;

    let error = store
        .register_request(
            &id,
            Registration {
                key: "k",
                origin: "gui",
                expect: Inputs {
                    source: Revision::of(b"nothing"),
                    answers: None,
                },
                supersede: false,
            },
        )
        .unwrap_err();

    assert_eq!(error.kind(), "not-found");
}

#[test]
fn only_one_request_of_a_work_is_open_and_a_new_one_replaces_it_only_when_asked_to() {
    let f = fixture("requests-open");
    let first = f.register("press-1");

    let (kind, _) = refused(
        f.store
            .register_request(&f.id, f.registration("press-2"))
            .unwrap_err(),
    );
    assert_eq!(kind, "active-request");
    assert_eq!(f.state_of(&first), State::Queued);

    let replacing = f
        .store
        .register_request(
            &f.id,
            Registration {
                supersede: true,
                ..f.registration("press-2")
            },
        )
        .unwrap();

    assert!(replacing.created);
    assert_eq!(f.state_of(&first), State::Superseded);
    let all = f.store.list_requests(&f.id).unwrap();
    assert_eq!(
        all.iter()
            .map(|v| (v.request.seq, v.state))
            .collect::<Vec<_>>(),
        vec![(2, State::Queued), (1, State::Superseded)],
        "newest first, and one of them open"
    );
}

#[test]
fn a_request_that_ended_leaves_the_work_free_to_be_asked_again() {
    let f = fixture("requests-free");
    let first = f.register("press-1");
    f.store.cancel_request(&f.id, &first).unwrap();

    let second = f
        .store
        .register_request(&f.id, f.registration("press-2"))
        .unwrap();

    assert!(second.created);
    assert_eq!(f.store.list_requests(&f.id).unwrap().len(), 2);
}

// -- an executor -------------------------------------------------------------

#[test]
fn an_executor_takes_a_request_keeps_it_and_finishes_it() {
    let f = fixture("requests-run");
    let id = f.register("press-1");

    let taken = f
        .store
        .claim_request(&f.id, &id, "adapter-a", None, false)
        .unwrap();
    assert_eq!((taken.state, taken.request.attempt), (State::Running, 1));
    let lease = taken.request.lease.clone().unwrap();
    assert_eq!(
        (lease.holder.as_str(), lease.expires_at),
        ("adapter-a", T0 + 60)
    );

    f.clock.advance(30);
    let kept = f
        .store
        .heartbeat_request(&f.id, &id, "adapter-a", 1, None)
        .unwrap();
    assert_eq!(kept.request.lease.unwrap().expires_at, T0 + 90);

    let done = f.complete(&id, "adapter-a", 1).unwrap();
    assert_eq!(done.state, State::Completed);
    // Said twice is said once.
    f.clock.advance(5);
    let again = f.publish(&id, "adapter-a", 1).unwrap();
    assert_eq!(again.request, done.request);
}

#[test]
fn a_lease_that_runs_out_is_seen_without_anybody_writing_it() {
    let f = fixture("requests-expiry");
    let id = f.register("press-1");
    f.store
        .claim_request(&f.id, &id, "adapter-a", Some(30), false)
        .unwrap();

    f.clock.advance(29);
    assert_eq!(f.state_of(&id), State::Running);
    f.clock.advance(1);

    assert_eq!(f.state_of(&id), State::Interrupted);
    let listed = &f.store.list_requests(&f.id).unwrap()[0];
    assert_eq!(listed.state, State::Interrupted);
    assert_eq!(
        listed.request.state,
        State::Running,
        "the file says what it last was"
    );
}

#[test]
fn an_interrupted_request_is_resumed_only_by_asking_and_the_attempt_before_is_fenced() {
    let f = fixture("requests-resume");
    let id = f.register("press-1");
    f.store
        .claim_request(&f.id, &id, "adapter-a", Some(30), false)
        .unwrap();
    f.clock.advance(100);

    let (kind, _) = refused(
        f.store
            .claim_request(&f.id, &id, "adapter-b", None, false)
            .unwrap_err(),
    );
    assert_eq!(kind, "not-resuming");

    let resumed = f
        .store
        .claim_request(&f.id, &id, "adapter-b", None, true)
        .unwrap();
    assert_eq!(
        (resumed.state, resumed.request.attempt),
        (State::Running, 2)
    );
    let (kind, _) = refused(f.complete(&id, "adapter-a", 1).unwrap_err());
    assert_eq!(kind, "not-holder");
    assert_eq!(f.state_of(&id), State::Running);
}

#[test]
fn an_executor_that_slept_past_its_lease_and_woke_with_the_run_done_finishes_it() {
    let f = fixture("requests-slept");
    let id = f.register("press-1");
    f.store
        .claim_request(&f.id, &id, "adapter-a", Some(30), false)
        .unwrap();
    f.clock.advance(3_600);
    assert_eq!(f.state_of(&id), State::Interrupted);

    let done = f.complete(&id, "adapter-a", 1).unwrap();

    assert_eq!(done.state, State::Completed);
}

#[test]
fn a_request_held_by_one_executor_is_not_taken_by_another() {
    let f = fixture("requests-held");
    let id = f.register("press-1");
    f.store
        .claim_request(&f.id, &id, "adapter-a", None, false)
        .unwrap();

    let (kind, _) = refused(
        f.store
            .claim_request(&f.id, &id, "adapter-b", None, true)
            .unwrap_err(),
    );

    assert_eq!(kind, "request-held");
}

#[test]
fn a_cancelled_request_tells_the_executor_at_its_next_word() {
    let f = fixture("requests-cancel");
    let id = f.register("press-1");
    f.store
        .claim_request(&f.id, &id, "adapter-a", None, false)
        .unwrap();

    let cancelled = f.store.cancel_request(&f.id, &id).unwrap();
    assert_eq!(cancelled.state, State::Cancelled);

    let (kind, message) = refused(
        f.store
            .heartbeat_request(&f.id, &id, "adapter-a", 1, None)
            .unwrap_err(),
    );
    assert_eq!(kind, "request-ended");
    assert!(message.contains("cancelled"), "{message}");
    // Cancelling again is the same cancellation; a finished request cannot be cancelled.
    assert_eq!(
        f.store.cancel_request(&f.id, &id).unwrap().state,
        State::Cancelled
    );
    let other = f.register("press-2");
    f.store
        .claim_request(&f.id, &other, "adapter-a", None, false)
        .unwrap();
    f.complete(&other, "adapter-a", 1).unwrap();
    let (kind, _) = refused(f.store.cancel_request(&f.id, &other).unwrap_err());
    assert_eq!(kind, "request-ended");
}

#[test]
fn the_open_requests_of_every_work_are_listed_oldest_first_for_an_executor_to_find() {
    let f = fixture("requests-open");
    let first = f.register("press-1");
    f.clock.advance(5);
    // A second work, asked later, and a third whose request is over.
    let second_id = f.store.create_work("Garage").unwrap().id;
    let second_text = f.store.save_source(&second_id, "The garage opens.", None);
    assert!(second_text.is_ok());
    let second = f
        .store
        .register_request(
            &second_id,
            Registration {
                key: "press-2",
                origin: "gui",
                expect: Inputs {
                    source: f.store.head(&second_id).unwrap().unwrap(),
                    answers: None,
                },
                supersede: false,
            },
        )
        .unwrap()
        .request
        .id;
    f.clock.advance(5);
    let third_id = f.store.create_work("Gate").unwrap().id;
    f.store
        .save_source(&third_id, "The gate opens.", None)
        .unwrap();
    let over = f
        .store
        .register_request(
            &third_id,
            Registration {
                key: "press-3",
                origin: "gui",
                expect: Inputs {
                    source: f.store.head(&third_id).unwrap().unwrap(),
                    answers: None,
                },
                supersede: false,
            },
        )
        .unwrap()
        .request
        .id;
    f.store.cancel_request(&third_id, &over).unwrap();

    let open = f.store.open_requests().unwrap();

    let listed: Vec<(String, String, State)> = open
        .iter()
        .map(|(work, view)| {
            (
                work.as_str().to_string(),
                view.request.id.clone(),
                view.state,
            )
        })
        .collect();
    assert_eq!(
        listed,
        vec![
            (f.id.as_str().to_string(), first.clone(), State::Queued),
            (second_id.as_str().to_string(), second, State::Queued),
        ],
        "oldest first, and a request that ended is not work"
    );

    // One that was taken is still open, and read by the clock: a lease that ran out is
    // interrupted, which is still a request an executor may look at.
    f.store
        .claim_request(&f.id, &first, "adapter-a", Some(30), false)
        .unwrap();
    f.clock.advance(3_600);
    let states: Vec<State> = f
        .store
        .open_requests()
        .unwrap()
        .iter()
        .map(|(_, view)| view.state)
        .collect();
    assert_eq!(states, vec![State::Interrupted, State::Queued]);
}

#[test]
fn a_failure_is_kept_with_its_reason() {
    let f = fixture("requests-fail");
    let id = f.register("press-1");
    f.store
        .claim_request(&f.id, &id, "adapter-a", None, false)
        .unwrap();

    let failed = f
        .store
        .fail_request(&f.id, &id, "adapter-a", 1, "the model was refused")
        .unwrap();

    assert_eq!(failed.state, State::Failed);
    assert_eq!(
        failed.request.note.as_deref(),
        Some("the model was refused")
    );
}

#[test]
fn a_request_that_is_not_there_is_not_found() {
    let f = fixture("requests-missing");

    for error in [
        f.store.read_request(&f.id, "req-nothing").unwrap_err(),
        f.store.cancel_request(&f.id, "req-nothing").unwrap_err(),
        f.store
            .claim_request(&f.id, "req-nothing", "adapter-a", None, false)
            .unwrap_err(),
    ] {
        assert_eq!(error.kind(), "not-found");
    }
    // A name that is not a request id names no file, whatever it holds.
    assert_eq!(
        f.store.read_request(&f.id, "../work").unwrap_err().kind(),
        "not-found"
    );
}

#[test]
fn a_lease_and_a_holder_are_asked_for_within_the_limits() {
    let f = fixture("requests-limits");
    let id = f.register("press-1");

    for ttl in [0, 9, 901, 100_000] {
        let error = f
            .store
            .claim_request(&f.id, &id, "adapter-a", Some(ttl), false)
            .unwrap_err();
        assert_eq!(error.kind(), "bad-lease", "{ttl}");
    }
    assert_eq!(f.state_of(&id), State::Queued);
    for holder in ["", "has space", "../x", &"a".repeat(65)] {
        let error = f
            .store
            .claim_request(&f.id, &id, holder, None, false)
            .unwrap_err();
        assert_eq!(error.kind(), "bad-holder", "{holder:?}");
    }
}

// -- what moves a request ----------------------------------------------------

#[test]
fn a_text_saved_ends_the_request_asked_about_the_one_before() {
    let f = fixture("requests-supersede-text");
    let id = f.register("press-1");
    f.store
        .claim_request(&f.id, &id, "adapter-a", None, false)
        .unwrap();

    f.move_the_text("The lock opens twice.");

    let view = f.store.read_request(&f.id, &id).unwrap();
    assert_eq!(view.state, State::Superseded);
    assert!(view.request.note.unwrap().contains("text"));
    let (kind, _) = refused(
        f.store
            .heartbeat_request(&f.id, &id, "adapter-a", 1, None)
            .unwrap_err(),
    );
    assert_eq!(kind, "request-ended");
}

#[test]
fn answers_saved_end_the_request_too_and_the_same_text_saved_again_does_not() {
    let f = fixture("requests-supersede-answers");
    let id = f.register("press-1");

    // The same text again is no change: nothing was saved, so nothing moved.
    f.store
        .save_source(&f.id, "The lock opens.", Some(&f.source()))
        .unwrap();
    assert_eq!(f.state_of(&id), State::Queued);

    f.store.save_answers(&f.id, "{\"a\":1}", None).unwrap();

    let view = f.store.read_request(&f.id, &id).unwrap();
    assert_eq!(view.state, State::Superseded);
    assert!(view.request.note.unwrap().contains("answers"));
}

#[test]
fn what_the_executor_saves_does_not_end_the_request_it_is_working_for() {
    let f = fixture("requests-executor-saves");
    let id = f.register("press-1");
    f.store
        .claim_request(&f.id, &id, "adapter-a", None, false)
        .unwrap();

    f.store
        .save_model(&f.id, "<scxml/>", None, Some(&f.source()))
        .unwrap();
    f.store
        .save_requirements(&f.id, "{}", None, Some(&f.source()))
        .unwrap();

    assert_eq!(f.state_of(&id), State::Running);
}

#[test]
fn a_request_that_ended_is_not_ended_again_by_a_save() {
    let f = fixture("requests-ended-save");
    let id = f.register("press-1");
    f.store.cancel_request(&f.id, &id).unwrap();
    let before = f.store.read_request(&f.id, &id).unwrap().request;

    f.move_the_text("The lock opens twice.");

    assert_eq!(f.store.read_request(&f.id, &id).unwrap().request, before);
}

// -- what is kept ------------------------------------------------------------

#[test]
fn requests_are_read_back_by_a_store_opened_on_the_same_folder() {
    let f = fixture("requests-reopen");
    let id = f.register("press-1");
    f.store
        .claim_request(&f.id, &id, "adapter-a", None, false)
        .unwrap();
    f.store
        .heartbeat_request(&f.id, &id, "adapter-a", 1, Some(120))
        .unwrap();

    let other = WorkStore::with_clock(f.store.root(), Arc::clone(&f.clock));
    let read = other.read_request(&f.id, &id).unwrap();

    assert_eq!(read.state, State::Running);
    assert_eq!(read.request.lease.unwrap().expires_at, T0 + 120);
}

#[test]
fn every_change_of_state_is_a_line_of_the_history_and_a_heartbeat_is_not() {
    let f = fixture("requests-history");
    let id = f.register("press-1");
    f.store
        .claim_request(&f.id, &id, "adapter-a", None, false)
        .unwrap();
    for _ in 0..5 {
        f.clock.advance(10);
        f.store
            .heartbeat_request(&f.id, &id, "adapter-a", 1, None)
            .unwrap();
    }
    f.complete(&id, "adapter-a", 1).unwrap();

    let history = f.store.request_history(&f.id).unwrap();

    assert_eq!(
        history.iter().map(|h| (h.n, h.to)).collect::<Vec<_>>(),
        vec![
            (1, State::Queued),
            (2, State::Running),
            (3, State::Completed)
        ]
    );
    assert!(history.iter().all(|h| h.request == id));
}

#[test]
fn a_request_file_that_is_not_a_request_is_a_damaged_folder_and_not_a_panic() {
    let f = fixture("requests-corrupt");
    let id = f.register("press-1");
    let path = f
        .store
        .root()
        .join(f.id.as_str())
        .join("requests")
        .join(format!("{id}.json"));
    std::fs::write(&path, "{ not json").unwrap();

    let error = f.store.read_request(&f.id, &id).unwrap_err();

    assert_eq!(error.kind(), "corrupt");
    assert_eq!(f.store.list_requests(&f.id).unwrap_err().kind(), "corrupt");
}

#[test]
fn a_removed_work_takes_no_request() {
    let f = fixture("requests-removed");
    let id = f.register("press-1");
    // Built while the work is there: what it expects is read from it.
    let another = f.registration("press-2");
    f.store.remove_work(&f.id).unwrap();

    assert_eq!(
        f.store.read_request(&f.id, &id).unwrap_err().kind(),
        "not-found"
    );
    assert_eq!(
        f.store.register_request(&f.id, another).unwrap_err().kind(),
        "not-found"
    );
    assert_eq!(
        f.store.cancel_request(&f.id, &id).unwrap_err().kind(),
        "not-found"
    );
}

// -- where the work shows it -------------------------------------------------

#[test]
fn the_heads_of_a_work_say_where_its_latest_request_stands_as_the_clock_says() {
    let f = fixture("requests-heads");
    assert!(f.store.read_work_heads(&f.id).unwrap().request.is_none());
    let id = f.register("press-1");
    f.store
        .claim_request(&f.id, &id, "adapter-a", Some(30), false)
        .unwrap();

    let running = f.store.read_work_heads(&f.id).unwrap().request.unwrap();
    assert_eq!(
        (running.id.as_str(), running.state, running.attempt),
        (id.as_str(), State::Running, 1)
    );

    // Nobody wrote anything, and the work says the request was let go of.
    f.clock.advance(30);
    let interrupted = f.store.read_work_heads(&f.id).unwrap().request.unwrap();
    assert_eq!(interrupted.state, State::Interrupted);

    f.store.cancel_request(&f.id, &id).unwrap();
    assert_eq!(
        f.store
            .read_work_heads(&f.id)
            .unwrap()
            .request
            .unwrap()
            .state,
        State::Cancelled
    );
    // A newer request is the latest.
    let next = f.register("press-2");
    assert_eq!(
        f.store.read_work_heads(&f.id).unwrap().request.unwrap().id,
        next
    );
}

// -- callers that meet at the lock -------------------------------------------

#[test]
fn of_several_executors_that_claim_one_request_at_once_one_gets_it() {
    const EXECUTORS: usize = 8;
    let f = fixture("requests-claim-race");
    let id = f.register("press-1");
    let gate = Barrier::new(EXECUTORS);

    let outcomes: Vec<Result<State, &'static str>> = thread::scope(|scope| {
        let handles: Vec<_> = (0..EXECUTORS)
            .map(|i| {
                let (f, id, gate) = (&f, &id, &gate);
                scope.spawn(move || {
                    gate.wait();
                    f.store
                        .claim_request(&f.id, id, &format!("adapter-{i}"), None, false)
                        .map(|v| v.state)
                        .map_err(|e| e.kind())
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });

    assert_eq!(
        outcomes.iter().filter(|o| o.is_ok()).count(),
        1,
        "{outcomes:?}"
    );
    assert!(outcomes
        .iter()
        .filter_map(|o| o.as_ref().err())
        .all(|kind| *kind == "request-held"));
    assert_eq!(f.store.read_request(&f.id, &id).unwrap().request.attempt, 1);
}

#[test]
fn of_several_presses_with_one_key_one_request_is_made() {
    const PRESSES: usize = 8;
    let f = fixture("requests-register-race");
    let gate = Barrier::new(PRESSES);

    let ids: Vec<String> = thread::scope(|scope| {
        let handles: Vec<_> = (0..PRESSES)
            .map(|_| {
                let (f, gate) = (&f, &gate);
                scope.spawn(move || {
                    gate.wait();
                    f.store
                        .register_request(&f.id, f.registration("press-1"))
                        .unwrap()
                        .request
                        .id
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });

    assert!(ids.iter().all(|id| *id == ids[0]), "{ids:?}");
    assert_eq!(f.store.list_requests(&f.id).unwrap().len(), 1);
}

#[test]
fn a_cancel_and_a_heartbeat_that_race_leave_a_cancelled_request_the_executor_is_told_of() {
    let f = fixture("requests-cancel-race");
    let id = f.register("press-1");
    f.store
        .claim_request(&f.id, &id, "adapter-a", None, false)
        .unwrap();
    let gate = Barrier::new(2);

    thread::scope(|scope| {
        let (f, id, gate) = (&f, &id, &gate);
        scope.spawn(move || {
            gate.wait();
            f.store.cancel_request(&f.id, id).unwrap();
        });
        scope.spawn(move || {
            gate.wait();
            // Accepted if it came first, refused if it came after: never anything else.
            match f.store.heartbeat_request(&f.id, id, "adapter-a", 1, None) {
                Ok(view) => assert_eq!(view.state, State::Running),
                Err(error) => assert_eq!(error.kind(), "request-ended"),
            }
        });
    });

    assert_eq!(f.state_of(&id), State::Cancelled);
    assert_eq!(
        f.store
            .heartbeat_request(&f.id, &id, "adapter-a", 1, None)
            .unwrap_err()
            .kind(),
        "request-ended"
    );
}
