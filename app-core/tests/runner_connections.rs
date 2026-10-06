// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A runner that runs for the connections people chose, and for none other.
//!
//! A request pinned to a connection is taken by the executor of that connection and by no other,
//! so the runner does not take a pinned request with whatever generator it holds: it asks where
//! it finds the generator for the connection the request names, offers that connection when it
//! takes the request, and writes with that generator. A request it cannot run (the connection is
//! not one it has an adapter for, the way of signing in is not one the build uses) is left
//! queued, and the runner says what it left and why, so that the screen can tell the person
//! what to do instead of waiting for an AI that will not come.

mod common;

use std::sync::{Arc, Mutex};

use sce_app_core::connection::AdapterKind;
use sce_app_core::requests::{ConnectionRef, Inputs, Pin, State};
use sce_app_core::runner::{
    Cancel, Directory, Draft, GenerateError, Generator, Job, Outcome, Runner, RunnerConfig,
    Unrunnable,
};
use sce_app_core::{
    ConnectionId, Limits, ManualClock, ModelFiles, Registration, Requirements, Revision, WorkId,
    WorkStore,
};

use common::FakeRenderer;

const T0: u64 = 1_791_190_800;

/// A generator that writes a draft marked with its own name, and remembers it was asked.
struct Named {
    name: &'static str,
    asked: Mutex<u32>,
}

impl Named {
    fn new(name: &'static str) -> Arc<Self> {
        Arc::new(Named {
            name,
            asked: Mutex::new(0),
        })
    }

    fn times(&self) -> u32 {
        *self.asked.lock().unwrap()
    }
}

impl Generator for Named {
    fn kind(&self) -> &str {
        "claude-code"
    }

    fn instructions(&self) -> Option<String> {
        Some(format!("named/{}", self.name))
    }

    fn generate(&self, _job: &Job, _cancel: &Cancel) -> Result<Draft, GenerateError> {
        *self.asked.lock().unwrap() += 1;
        Ok(Draft {
            model: ModelFiles::single(format!("<scxml><!-- {} --></scxml>", self.name)),
            requirements: Requirements::new(
                format!(
                    "{{\"doc_id\":\"door\",\"rev\":\"{}\",\"requirements\":[{{\"id\":\"R1\"}}]}}\n",
                    self.name
                ),
                None,
            )
            .expect("a list the product's stand-in reads"),
        })
    }
}

/// A directory that has a generator for some connections, and says why it has none for the others.
struct Shelf {
    known: Vec<(ConnectionId, Arc<Named>)>,
    pins: Mutex<Vec<Pin>>,
}

impl Shelf {
    fn holding(known: Vec<(&str, Arc<Named>)>) -> Arc<Self> {
        Arc::new(Shelf {
            known: known
                .into_iter()
                .map(|(id, generator)| (ConnectionId::parse(id).unwrap(), generator))
                .collect(),
            pins: Mutex::new(Vec::new()),
        })
    }

    fn asked_for(&self) -> Vec<Pin> {
        self.pins.lock().unwrap().clone()
    }
}

impl Directory for Shelf {
    fn generator_for(&self, pin: &Pin) -> Result<Arc<dyn Generator>, Unrunnable> {
        self.pins.lock().unwrap().push(pin.clone());
        self.known
            .iter()
            .find(|(id, _)| *id == pin.connection)
            .map(|(_, generator)| Arc::clone(generator) as Arc<dyn Generator>)
            .ok_or_else(|| Unrunnable::new(format!("nothing runs `{}` here", pin.connection)))
    }
}

struct Fixture {
    store: Arc<WorkStore<Arc<ManualClock>>>,
    id: WorkId,
}

fn fixture(label: &str) -> Fixture {
    let clock = Arc::new(ManualClock::at(T0));
    let store = Arc::new(WorkStore::with_clock(common::scratch(label), clock));
    let id = store.create_work("Door lock").unwrap().id;
    store.save_source(&id, "The lock opens.", None).unwrap();
    Fixture { store, id }
}

fn pin(connection: &str, revision: &str) -> Pin {
    Pin {
        connection: ConnectionId::parse(connection).unwrap(),
        revision: Revision::of(revision.as_bytes()),
        adapter: AdapterKind::ClaudeCode,
        model: Some("opus".to_string()),
        limits: Limits {
            turns: Some(7),
            seconds: None,
        },
    }
}

impl Fixture {
    fn ask(&self, key: &str, pin: Option<Pin>) -> String {
        self.store
            .register_request_for(
                &self.id,
                Registration {
                    key,
                    origin: "gui",
                    expect: Inputs {
                        source: self.store.head(&self.id).unwrap().unwrap(),
                        answers: None,
                    },
                    supersede: true,
                },
                pin,
            )
            .unwrap()
            .request
            .id
    }

    fn runner(
        &self,
        fixed: &Arc<Named>,
        shelf: Option<&Arc<Shelf>>,
    ) -> Runner<Arc<ManualClock>, Named> {
        let runner = Runner::new(
            Arc::clone(&self.store),
            Arc::new(FakeRenderer),
            Arc::clone(fixed),
            RunnerConfig::named("desktop"),
        );
        match shelf {
            Some(shelf) => runner.with_connections(Arc::clone(shelf) as Arc<dyn Directory>),
            None => runner,
        }
    }

    fn state_of(&self, request: &str) -> State {
        self.store.read_request(&self.id, request).unwrap().state
    }
}

#[test]
fn a_pinned_request_is_written_with_the_generator_of_its_connection() {
    let f = fixture("rc-pinned");
    let fixed = Named::new("fixed");
    let main = Named::new("main");
    let shelf = Shelf::holding(vec![("main", Arc::clone(&main))]);
    let pinned = pin("main", "r1");
    let request = f.ask("press-1", Some(pinned.clone()));
    let runner = f.runner(&fixed, Some(&shelf));

    let outcome = runner.run_once().unwrap();

    assert!(matches!(outcome, Outcome::Completed { .. }), "{outcome:?}");
    assert_eq!(f.state_of(&request), State::Completed);
    // The generator the directory gave wrote it, and the one the runner holds did not.
    assert_eq!((main.times(), fixed.times()), (1, 0));
    let model = f.store.read_model(&f.id, None).unwrap().unwrap();
    assert!(model.text.contains("main"));
    // The directory was asked for the connection as the request pinned it, which is where a
    // generator finds the model and the limits to run with.
    assert_eq!(shelf.asked_for(), vec![pinned]);
}

#[test]
fn it_takes_the_request_by_offering_the_connection_the_request_names() {
    let f = fixture("rc-offers");
    let fixed = Named::new("fixed");
    let shelf = Shelf::holding(vec![("main", Named::new("main"))]);
    let pinned = pin("main", "r1");
    let request = f.ask("press-1", Some(pinned.clone()));
    let runner = f.runner(&fixed, Some(&shelf));

    runner.run_once().unwrap();

    // The claim that was made is the claim of that connection's executor: another cannot make
    // it, and this one cannot make another's.
    let view = f.store.read_request(&f.id, &request).unwrap();
    assert_eq!(view.request.lease.as_ref().unwrap().holder, "desktop");
    let other: ConnectionRef = pin("pc2", "r1").reference();
    let refused = f
        .store
        .claim_request_for(&f.id, &request, "other", None, true, Some(&other))
        .unwrap_err();
    assert_eq!(refused.kind(), "wrong-connection");
}

#[test]
fn a_request_the_runner_cannot_run_is_left_queued_and_says_why() {
    let f = fixture("rc-unrunnable");
    let fixed = Named::new("fixed");
    let shelf = Shelf::holding(vec![("main", Named::new("main"))]);
    let request = f.ask("press-1", Some(pin("elsewhere", "r1")));
    let runner = f.runner(&fixed, Some(&shelf));

    let outcome = runner.run_once().unwrap();

    assert_eq!(outcome, Outcome::NothingToDo);
    assert_eq!(f.state_of(&request), State::Queued);
    let waiting = runner.waiting();
    assert_eq!(waiting.len(), 1);
    assert_eq!(waiting[0].work, f.id);
    assert_eq!(waiting[0].request, request);
    assert_eq!(waiting[0].connection.as_str(), "elsewhere");
    assert!(waiting[0].reason.contains("nothing runs `elsewhere`"));
}

#[test]
fn a_runner_with_no_directory_leaves_every_pinned_request_alone() {
    let f = fixture("rc-no-directory");
    let fixed = Named::new("fixed");
    let request = f.ask("press-1", Some(pin("main", "r1")));
    let runner = f.runner(&fixed, None);

    let outcome = runner.run_once().unwrap();

    assert_eq!(outcome, Outcome::NothingToDo);
    assert_eq!(f.state_of(&request), State::Queued);
    assert_eq!(fixed.times(), 0);
    let waiting = runner.waiting();
    assert_eq!(waiting.len(), 1);
    assert!(
        waiting[0].reason.contains("no connection"),
        "{}",
        waiting[0].reason
    );
}

#[test]
fn a_request_with_no_connection_is_still_written_by_the_generator_the_runner_holds() {
    let f = fixture("rc-unpinned");
    let fixed = Named::new("fixed");
    let shelf = Shelf::holding(vec![("main", Named::new("main"))]);
    let request = f.ask("press-1", None);
    let runner = f.runner(&fixed, Some(&shelf));

    let outcome = runner.run_once().unwrap();

    assert!(matches!(outcome, Outcome::Completed { .. }), "{outcome:?}");
    assert_eq!(f.state_of(&request), State::Completed);
    assert_eq!(fixed.times(), 1);
    assert!(shelf.asked_for().is_empty());
    assert!(runner.waiting().is_empty());
}

#[test]
fn what_was_waiting_is_forgotten_once_the_request_is_taken_or_gone() {
    let f = fixture("rc-forgets");
    let fixed = Named::new("fixed");
    let main = Named::new("main");
    let shelf = Shelf::holding(vec![("main", Arc::clone(&main))]);
    let runner = f.runner(&fixed, Some(&shelf));
    let stuck = f.ask("press-1", Some(pin("elsewhere", "r1")));
    runner.run_once().unwrap();
    assert_eq!(runner.waiting().len(), 1);

    // The person called it off and asked again for a connection that can be run.
    f.store.cancel_request(&f.id, &stuck).unwrap();
    let again = f.ask("press-2", Some(pin("main", "r1")));
    let outcome = runner.run_once().unwrap();

    assert!(matches!(outcome, Outcome::Completed { .. }), "{outcome:?}");
    assert_eq!(f.state_of(&again), State::Completed);
    assert!(runner.waiting().is_empty());
}
