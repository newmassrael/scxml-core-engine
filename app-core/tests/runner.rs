// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A runner that hosts a generator: what it takes, what it keeps, and how it ends.
//!
//! The generator here is a script, so that what it writes and when is the test's to say:
//! what is held is the runner's own behaviour with a folder and a clock, not what an AI
//! would write. A request is taken when it is queued and not when it was let go of; the
//! claim is renewed while the generator works and the generator is stopped when the owner
//! calls the request off; a model the core refuses is written again from the product's own
//! words; and a request that ended under the runner is never reported as its failure.

mod common;

use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use sce_app_core::requests::{Inputs, State};
use sce_app_core::runner::{
    Cancel, Draft, GenerateError, Generator, Job, Outcome, Runner, RunnerConfig,
};
use sce_app_core::{
    call, ManualClock, ModelFiles, Registration, Requirements, Revision, WorkId, WorkStore,
};
use serde_json::json;

use common::FakeRenderer;

const T0: u64 = 1_791_190_800;

type Step = Box<dyn Fn(usize, &Job, &Cancel) -> Result<Draft, GenerateError> + Send + Sync>;

/// A generator that does what the test said, and remembers what it was asked.
struct Scripted {
    jobs: Mutex<Vec<Job>>,
    step: Step,
}

impl Scripted {
    fn new(
        step: impl Fn(usize, &Job, &Cancel) -> Result<Draft, GenerateError> + Send + Sync + 'static,
    ) -> Arc<Self> {
        Arc::new(Scripted {
            jobs: Mutex::new(Vec::new()),
            step: Box::new(step),
        })
    }

    fn asked(&self) -> Vec<Job> {
        self.jobs.lock().unwrap().clone()
    }
}

impl Generator for Scripted {
    fn kind(&self) -> &str {
        "claude-code"
    }

    fn version(&self) -> Option<String> {
        Some("test 1".to_string())
    }

    fn instructions(&self) -> Option<String> {
        Some("scripted/v1".to_string())
    }

    fn generate(&self, job: &Job, cancel: &Cancel) -> Result<Draft, GenerateError> {
        let n = {
            let mut jobs = self.jobs.lock().unwrap();
            jobs.push(job.clone());
            jobs.len() - 1
        };
        (self.step)(n, job, cancel)
    }
}

fn draft(tag: &str) -> Draft {
    Draft {
        model: ModelFiles::single(format!("<scxml><!-- {tag} --></scxml>")),
        requirements: Requirements::new(
            format!(
                "{{\"doc_id\":\"door\",\"rev\":\"{tag}\",\"requirements\":[{{\"id\":\"R1\"}}]}}\n"
            ),
            None,
        )
        .expect("a list the product's stand-in reads"),
    }
}

struct Fixture {
    clock: Arc<ManualClock>,
    store: Arc<WorkStore<Arc<ManualClock>>>,
    id: WorkId,
    source: Revision,
}

fn fixture(label: &str) -> Fixture {
    let clock = Arc::new(ManualClock::at(T0));
    let store = Arc::new(WorkStore::with_clock(
        common::scratch(label),
        Arc::clone(&clock),
    ));
    let id = store.create_work("Door lock").unwrap().id;
    store.save_source(&id, "The lock opens.", None).unwrap();
    let source = store.head(&id).unwrap().expect("a text");
    Fixture {
        clock,
        store,
        id,
        source,
    }
}

impl Fixture {
    fn ask(&self, key: &str) -> String {
        let answers = self
            .store
            .read_answers(&self.id, None)
            .unwrap()
            .map(|a| a.revision);
        self.store
            .register_request(
                &self.id,
                Registration {
                    key,
                    origin: "gui",
                    expect: Inputs {
                        source: self.store.head(&self.id).unwrap().unwrap(),
                        answers,
                    },
                    supersede: false,
                },
            )
            .unwrap()
            .request
            .id
    }

    fn config(&self) -> RunnerConfig {
        let mut config = RunnerConfig::named("desktop");
        // A heartbeat and a poll that a test can wait for.
        config.heartbeat = Duration::from_millis(20);
        config.poll = Duration::from_millis(10);
        config
    }

    fn runner(
        &self,
        generator: &Arc<Scripted>,
        config: RunnerConfig,
    ) -> Runner<Arc<ManualClock>, Scripted> {
        Runner::new(
            Arc::clone(&self.store),
            Arc::new(FakeRenderer),
            Arc::clone(generator),
            config,
        )
    }

    fn state_of(&self, request: &str) -> State {
        self.store.read_request(&self.id, request).unwrap().state
    }
}

/// Wait for `until` to hold, for as long as a test can be asked to.
fn within_ten_seconds(what: &str, until: impl Fn() -> bool) {
    let limit = Instant::now() + Duration::from_secs(10);
    while !until() {
        assert!(Instant::now() < limit, "{what}");
        thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn a_queued_request_is_taken_written_for_and_completed() {
    let f = fixture("runner-complete");
    let request = f.ask("press-1");
    let generator = Scripted::new(|_, _, _| Ok(draft("one")));
    let runner = f.runner(&generator, f.config());

    let outcome = runner.run_once().unwrap();

    let Outcome::Completed {
        work,
        request: done,
        bundle,
    } = outcome
    else {
        panic!("expected a completion, got {outcome:?}");
    };
    assert_eq!((work, done), (f.id.clone(), request.clone()));
    assert_eq!(f.state_of(&request), State::Completed);
    let model = f.store.read_model(&f.id, None).unwrap().expect("a model");
    assert!(model.text.contains("one"));
    assert_eq!(model.written_for, Some(f.source.clone()));
    let published = f.store.read_bundle(&f.id, None).unwrap().expect("a bundle");
    assert_eq!(published.revision, bundle);
    assert_eq!(published.bundle.executor, "desktop");
    // What the generator was given is on the bundle, said by the generator and not by the host.
    assert_eq!(
        published.bundle.instructions.as_deref(),
        Some("scripted/v1")
    );
    assert_eq!(published.bundle.replaces, None);
    assert_eq!(published.bundle.checks.len(), 1);
    assert_eq!(published.bundle.checks[0].name, "model");
}

#[test]
fn the_generator_is_asked_about_the_text_and_the_answers_the_request_was_made_about() {
    let f = fixture("runner-job");
    // A model the work had, and an answer the owner gave to what it left open.
    f.store
        .save_model(
            &f.id,
            "<scxml><!-- earlier --></scxml>",
            None,
            Some(&f.source),
        )
        .unwrap();
    call(
        &*f.store,
        &FakeRenderer,
        "save_answers",
        json!({"id": f.id.as_str(), "answers": {"open-guard": "Any card on the list opens it."}}),
    )
    .unwrap();
    let request = f.ask("press-1");
    let generator = Scripted::new(|_, _, _| Ok(draft("one")));
    let runner = f.runner(&generator, f.config());

    runner.run_once().unwrap();

    let jobs = generator.asked();
    assert_eq!(jobs.len(), 1);
    let job = &jobs[0];
    assert_eq!(job.work, f.id);
    assert_eq!(job.title, "Door lock");
    assert_eq!(job.request, request);
    assert_eq!(job.attempt, 1);
    assert_eq!(job.source, "The lock opens.");
    assert_eq!(job.source_revision, f.source);
    assert_eq!(
        job.answers.get("open-guard").map(String::as_str),
        Some("Any card on the list opens it.")
    );
    let previous = job.previous.as_ref().expect("the work had a model");
    assert!(previous.model.entry_text().contains("earlier"));
    assert!(job.refusal.is_none(), "nothing was refused yet");
}

#[test]
fn with_no_request_waiting_there_is_nothing_to_do_and_the_adapter_is_still_there() {
    let f = fixture("runner-idle");
    let generator = Scripted::new(|_, _, _| Ok(draft("one")));
    let runner = f.runner(&generator, f.config());

    assert_eq!(runner.run_once().unwrap(), Outcome::NothingToDo);

    assert!(generator.asked().is_empty());
    let status = f.store.adapter_status().unwrap();
    assert_eq!(status.adapters.len(), 1);
    let adapter = &status.adapters[0];
    assert_eq!(adapter.adapter.name, "desktop");
    assert_eq!(adapter.adapter.kind, "claude-code");
    assert_eq!(adapter.adapter.capabilities, vec!["generate", "cancel"]);
    assert_eq!(adapter.adapter.version.as_deref(), Some("test 1"));
    assert!(adapter.live);
}

#[test]
fn a_request_a_lease_let_go_of_is_not_taken_unasked() {
    let f = fixture("runner-interrupted");
    let request = f.ask("press-1");
    f.store
        .claim_request(&f.id, &request, "adapter-a", Some(30), false)
        .unwrap();
    f.clock.advance(3_600);
    assert_eq!(f.state_of(&request), State::Interrupted);
    let generator = Scripted::new(|_, _, _| Ok(draft("one")));
    let runner = f.runner(&generator, f.config());

    assert_eq!(runner.run_once().unwrap(), Outcome::NothingToDo);

    assert!(
        generator.asked().is_empty(),
        "a run is the owner's to ask for"
    );
    assert_eq!(f.state_of(&request), State::Interrupted);
}

#[test]
fn a_generator_that_cannot_says_why_and_the_owner_is_told() {
    let f = fixture("runner-fails");
    let request = f.ask("press-1");
    let generator = Scripted::new(|_, _, _| {
        Err(GenerateError::Failed(
            "The text never says which cards open the door.".to_string(),
        ))
    });
    let runner = f.runner(&generator, f.config());

    let outcome = runner.run_once().unwrap();

    assert_eq!(
        outcome,
        Outcome::Failed {
            work: f.id.clone(),
            request: request.clone(),
            reason: "The text never says which cards open the door.".to_string(),
        }
    );
    let seen = f.store.read_request(&f.id, &request).unwrap();
    assert_eq!(seen.state, State::Failed);
    assert_eq!(
        seen.request.note.as_deref(),
        Some("The text never says which cards open the door.")
    );
    assert!(f.store.read_bundle(&f.id, None).unwrap().is_none());
}

#[test]
fn a_model_the_core_refuses_is_written_again_from_the_products_own_words() {
    let f = fixture("runner-repair");
    let request = f.ask("press-1");
    let generator = Scripted::new(|n, _, _| {
        Ok(match n {
            0 => Draft {
                model: ModelFiles::single("<scxml><!-- REFUSE --></scxml>"),
                ..draft("one")
            },
            _ => draft("two"),
        })
    });
    let runner = f.runner(&generator, f.config());

    let outcome = runner.run_once().unwrap();

    assert!(matches!(outcome, Outcome::Completed { .. }), "{outcome:?}");
    let jobs = generator.asked();
    assert_eq!(jobs.len(), 2);
    assert!(jobs[0].refusal.is_none());
    let said = jobs[1].refusal.as_deref().expect("what the core refused");
    assert!(said.contains("validation/invalid-reference"), "{said}");
    assert!(said.contains("nowhere"), "{said}");
    assert!(said.contains("line 3"), "{said}");
    assert_eq!(f.state_of(&request), State::Completed);
    let model = f.store.read_model(&f.id, None).unwrap().unwrap();
    assert!(
        model.text.contains("two"),
        "the repaired draft is the work's"
    );
}

#[test]
fn a_model_the_core_keeps_refusing_fails_the_request_after_the_repairs_with_what_it_last_said() {
    let f = fixture("runner-refused");
    let request = f.ask("press-1");
    let generator = Scripted::new(|_, _, _| {
        Ok(Draft {
            model: ModelFiles::single("<scxml><!-- REFUSE --></scxml>"),
            ..draft("one")
        })
    });
    let mut config = f.config();
    config.repairs = 1;
    let runner = f.runner(&generator, config);

    let outcome = runner.run_once().unwrap();

    let Outcome::Failed { reason, .. } = outcome else {
        panic!("expected a failure, got {outcome:?}");
    };
    assert_eq!(generator.asked().len(), 2, "one try and one repair");
    assert!(reason.contains("2 try(ies)"), "{reason}");
    assert!(reason.contains("validation/invalid-reference"), "{reason}");
    assert_eq!(f.state_of(&request), State::Failed);
    assert!(f.store.read_model(&f.id, None).unwrap().is_none());
}

#[test]
fn something_that_is_not_a_draft_is_told_what_was_wrong_and_written_again() {
    let f = fixture("runner-unusable");
    let request = f.ask("press-1");
    let generator = Scripted::new(|n, _, _| match n {
        0 => Err(GenerateError::Unusable(
            "the manifest is not JSON: expected value at line 1".to_string(),
        )),
        _ => Ok(draft("two")),
    });
    let runner = f.runner(&generator, f.config());

    let outcome = runner.run_once().unwrap();

    assert!(matches!(outcome, Outcome::Completed { .. }), "{outcome:?}");
    let jobs = generator.asked();
    assert_eq!(jobs.len(), 2);
    assert_eq!(
        jobs[1].refusal.as_deref(),
        Some("the manifest is not JSON: expected value at line 1")
    );
    assert_eq!(f.state_of(&request), State::Completed);
}

#[test]
fn the_claim_is_renewed_while_the_generator_works() {
    let f = fixture("runner-renewed");
    let request = f.ask("press-1");
    let store = Arc::clone(&f.store);
    let clock = Arc::clone(&f.clock);
    let (id, watched) = (f.id.clone(), request.clone());
    // The generator works for half a lease, and waits to be seen renewed before it is done.
    let generator = Scripted::new(move |_, _, _| {
        clock.advance(30);
        within_ten_seconds(
            "the lease was not renewed while the generator worked",
            || {
                let lease = store.read_request(&id, &watched).unwrap().request.lease;
                lease.is_some_and(|l| l.expires_at == T0 + 30 + 60)
            },
        );
        Ok(draft("one"))
    });
    let runner = f.runner(&generator, f.config());

    let outcome = runner.run_once().unwrap();

    assert!(matches!(outcome, Outcome::Completed { .. }), "{outcome:?}");
}

#[test]
fn a_request_the_owner_calls_off_stops_the_generator_and_is_not_reported_as_its_failure() {
    let f = fixture("runner-cancelled");
    let request = f.ask("press-1");
    let (started, hears) = mpsc::channel::<()>();
    let started = Mutex::new(started);
    let generator = Scripted::new(move |_, _, cancel| {
        started.lock().unwrap().send(()).unwrap();
        let limit = Instant::now() + Duration::from_secs(10);
        while !cancel.is_cancelled() {
            if Instant::now() > limit {
                return Err(GenerateError::Failed("never told to stop".to_string()));
            }
            thread::sleep(Duration::from_millis(5));
        }
        Err(GenerateError::Cancelled)
    });
    let runner = f.runner(&generator, f.config());

    let outcome = thread::scope(|scope| {
        let running = scope.spawn(|| runner.run_once());
        hears
            .recv_timeout(Duration::from_secs(10))
            .expect("the generator started");
        f.store.cancel_request(&f.id, &request).unwrap();
        running.join().unwrap().unwrap()
    });

    assert_eq!(
        outcome,
        Outcome::Lost {
            work: f.id.clone(),
            request: request.clone(),
            kind: "request-ended".to_string(),
        }
    );
    assert_eq!(
        f.state_of(&request),
        State::Cancelled,
        "the owner's word stands"
    );
}

#[test]
fn a_text_saved_while_the_generator_works_ends_the_request_and_nothing_is_published() {
    let f = fixture("runner-superseded");
    let request = f.ask("press-1");
    let store = Arc::clone(&f.store);
    let (id, base) = (f.id.clone(), f.source.clone());
    let generator = Scripted::new(move |_, _, _| {
        store
            .save_source(&id, "The lock opens for a listed card.", Some(&base))
            .unwrap();
        Ok(draft("one"))
    });
    let runner = f.runner(&generator, f.config());

    let outcome = runner.run_once().unwrap();

    assert!(
        matches!(&outcome, Outcome::Lost { kind, .. } if kind == "request-ended"),
        "{outcome:?}"
    );
    assert_eq!(f.state_of(&request), State::Superseded);
    assert!(f.store.read_bundle(&f.id, None).unwrap().is_none());
    assert!(f.store.read_model(&f.id, None).unwrap().is_none());
}

#[test]
fn a_runner_told_to_stop_leaves_the_request_to_run_out_its_lease() {
    let f = fixture("runner-stopped");
    let request = f.ask("press-1");
    let (started, hears) = mpsc::channel::<()>();
    let started = Mutex::new(started);
    let generator = Scripted::new(move |_, _, cancel| {
        started.lock().unwrap().send(()).unwrap();
        let limit = Instant::now() + Duration::from_secs(10);
        while !cancel.is_cancelled() && Instant::now() < limit {
            thread::sleep(Duration::from_millis(5));
        }
        Err(GenerateError::Cancelled)
    });
    let runner = f.runner(&generator, f.config());
    let shutdown = runner.shutdown();

    let outcome = thread::scope(|scope| {
        let running = scope.spawn(|| runner.run_once());
        hears.recv_timeout(Duration::from_secs(10)).unwrap();
        shutdown.cancel();
        running.join().unwrap().unwrap()
    });

    assert_eq!(
        outcome,
        Outcome::Stopped {
            work: f.id.clone(),
            request: request.clone()
        }
    );
    // Not failed and not cancelled: the executor went away, and the clock says so once the
    // lease runs out.
    assert_eq!(f.state_of(&request), State::Running);
    f.clock.advance(3_600);
    assert_eq!(f.state_of(&request), State::Interrupted);
}

#[test]
fn run_takes_requests_as_they_are_made_until_it_is_told_to_stop() {
    let f = fixture("runner-run");
    let generator = Scripted::new(|_, _, _| Ok(draft("one")));
    let runner = f.runner(&generator, f.config());
    let shutdown = runner.shutdown();

    thread::scope(|scope| {
        let running = scope.spawn(|| runner.run());
        // Nothing waits for a moment, then the owner presses the button.
        thread::sleep(Duration::from_millis(50));
        let request = f.ask("press-1");
        within_ten_seconds("the request was not taken", || {
            f.state_of(&request) == State::Completed
        });
        shutdown.cancel();
        running.join().unwrap();
    });

    assert_eq!(generator.asked().len(), 1);
}
