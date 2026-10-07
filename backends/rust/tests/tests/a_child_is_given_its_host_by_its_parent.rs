// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Accepted Subset §2.15, "Child sessions" (docs/adr/0005, decision 6): a
// child that declares `<sce:action>`s takes the host that performs them when it
// is built, because its first `<onentry>` can already perform an act — a host
// installed afterwards would arrive one act too late. So the host has to exist
// when the invocation starts, and the parent obtains it from its own host, which
// answers one for the child each time the invocation starts.
//
// `static_child_host.scxml` invokes `worker`, which announces itself on entry
// (`started`) and reports its steps when it ends (`finished`). The parent
// declares no act: its host is there for the child alone, and its trait names
// the type of the child's host (`ActionsForWorker`) because the child's machine
// is generic over it. `static_child_host_hybrid.scxml` invokes whichever of two
// candidates its `srcexpr` names, each with an act of its own, and the host
// answered is the one for THAT candidate. A restore starts the child again, so
// the parent's host is asked again.
//
// The parent's recording host is the generated one: it takes the function that
// answers each child's host and records each question beside the acts.

use std::sync::{Arc, Mutex};

use sce_rust_runtime::saved_state::SavedState;
use sce_rust_runtime::{Engine, SceClock, StatePolicy};
use sce_rust_tests::integration::static_datamodel::static_child_host__sce_synth_invoke__worker_sm::StaticChildHostSceSynthInvokeWorkerActions;
use sce_rust_tests::integration::static_datamodel::static_child_host_hybrid_sm::{
    RecordingStaticChildHostHybridActions, StaticChildHostHybridActionsCall,
    StaticChildHostHybridPolicy,
};
use sce_rust_tests::integration::static_datamodel::static_child_host_sm::{
    RecordingStaticChildHostActions, StaticChildHostActionsCall, StaticChildHostPersist,
    StaticChildHostPolicy,
};
use sce_rust_tests::integration::static_datamodel::static_hosted_first_sm::StaticHostedFirstActions;
use sce_rust_tests::integration::static_datamodel::static_hosted_second_sm::StaticHostedSecondActions;

/// What a child asked of its host, shared with the test that reads it: the
/// machine owns the host it was built with, so a test keeps a handle of its own.
type Log = Arc<Mutex<Vec<String>>>;

/// The host a `worker` child performs its acts through.
struct WorkerHost {
    log: Log,
}

impl StaticChildHostSceSynthInvokeWorkerActions for WorkerHost {
    fn started(&mut self) {
        self.log.lock().unwrap().push("started".to_string());
    }

    fn finished(&mut self, steps: u32) {
        self.log.lock().unwrap().push(format!("finished({steps})"));
    }
}

/// Every child host a parent's host answered, in the order asked, each with the
/// log of what its child asked of it.
type Workers = Arc<Mutex<Vec<Log>>>;

type Parent = StaticChildHostPolicy<RecordingStaticChildHostActions<WorkerHost>>;

fn parent(workers: &Workers) -> Parent {
    let workers = Arc::clone(workers);
    StaticChildHostPolicy::new(RecordingStaticChildHostActions::new(move || {
        let log: Log = Arc::default();
        workers.lock().unwrap().push(Arc::clone(&log));
        WorkerHost { log }
    }))
}

fn started(workers: &Workers) -> Engine<Parent> {
    let mut engine = Engine::new(parent(workers));
    engine.set_clock(SceClock::Manual(0));
    engine.initialize();
    engine
}

/// Send `event` to the parent, which forwards it to its child, and let the
/// child run.
fn send<P: StatePolicy + 'static>(engine: &mut Engine<P>, event: &str) {
    engine.raise_external_by_name(event, "");
    for _ in 0..40 {
        engine.tick();
    }
}

fn calls(workers: &Workers, index: usize) -> Vec<String> {
    workers.lock().unwrap()[index].lock().unwrap().clone()
}

#[test]
fn the_child_performs_its_first_act_through_the_host_its_parent_answered() {
    let workers = Workers::default();
    let engine = started(&workers);
    assert_eq!(
        engine.policy().actions().calls(),
        &[StaticChildHostActionsCall::ActionsForWorker],
        "the parent's host was asked once"
    );
    // The act of its first `<onentry>` is already performed: the host was there
    // when the child was built, not installed after.
    assert_eq!(calls(&workers, 0), ["started"]);
}

#[test]
fn the_child_reports_what_it_did_through_the_same_host() {
    let workers = Workers::default();
    let mut engine = started(&workers);
    send(&mut engine, "a");
    send(&mut engine, "b");
    assert_eq!(
        engine.policy().completed(),
        1,
        "the child ended and the parent counted it"
    );
    assert_eq!(calls(&workers, 0), ["started", "finished(2)"]);
    assert_eq!(
        engine.policy().actions().calls().len(),
        1,
        "nothing asked the parent's host again"
    );
}

#[test]
fn a_state_invoked_again_is_given_a_host_of_its_own() {
    let workers = Workers::default();
    let mut engine = started(&workers);
    for event in ["a", "b", "again", "back"] {
        send(&mut engine, event);
    }
    assert_eq!(
        engine.policy().actions().calls(),
        &[
            StaticChildHostActionsCall::ActionsForWorker,
            StaticChildHostActionsCall::ActionsForWorker
        ],
        "asked once per start"
    );
    // The first child's run is its own, and the second starts from nothing.
    assert_eq!(calls(&workers, 0), ["started", "finished(2)"]);
    assert_eq!(calls(&workers, 1), ["started"]);
}

#[test]
fn a_restored_machine_starts_its_child_again_with_a_host_from_its_own_parent() {
    let before = Workers::default();
    let mut first = started(&before);
    send(&mut first, "a");
    let saved = first.save_at(1_700_000_000_000).expect("saves");
    assert_eq!(saved.invokes, ["worker"], "the child was running");

    // A restore is another machine with another host: the child is started over,
    // and what is asked of the host is asked of that one.
    let after = Workers::default();
    let mut engine = Engine::<Parent>::restore_with(
        parent(&after),
        &SavedState::from_json(&saved.to_json()).expect("parses"),
        SceClock::Manual(0),
        1_700_000_000_000,
    )
    .expect("restores");
    for _ in 0..40 {
        engine.tick();
    }
    assert_eq!(
        engine.policy().actions().calls(),
        &[StaticChildHostActionsCall::ActionsForWorker]
    );
    assert_eq!(calls(&after, 0), ["started"]);

    send(&mut engine, "a");
    send(&mut engine, "b");
    assert_eq!(engine.policy().completed(), 1);
    assert_eq!(calls(&after, 0), ["started", "finished(2)"]);
    // The machine that was saved is not asked for anything more.
    assert_eq!(calls(&before, 0), ["started"]);
    assert_eq!(first.policy().actions().calls().len(), 1);
}

/// The hosts of the two candidates of `static_child_host_hybrid`, each with an
/// act of its own.
struct FirstHost {
    log: Log,
}

impl StaticHostedFirstActions for FirstHost {
    fn first_ran(&mut self) {
        self.log.lock().unwrap().push("first_ran".to_string());
    }
}

struct SecondHost {
    log: Log,
}

impl StaticHostedSecondActions for SecondHost {
    fn second_ran(&mut self) {
        self.log.lock().unwrap().push("second_ran".to_string());
    }
}

#[test]
fn a_candidate_is_given_the_host_answered_for_it_and_no_other() {
    let firsts = Workers::default();
    let seconds = Workers::default();
    let (for_first, for_second) = (Arc::clone(&firsts), Arc::clone(&seconds));
    let host = RecordingStaticChildHostHybridActions::new(
        move || {
            let log: Log = Arc::default();
            for_first.lock().unwrap().push(Arc::clone(&log));
            FirstHost { log }
        },
        move || {
            let log: Log = Arc::default();
            for_second.lock().unwrap().push(Arc::clone(&log));
            SecondHost { log }
        },
    );
    let mut engine = Engine::new(StaticChildHostHybridPolicy::new(host));
    engine.set_clock(SceClock::Manual(0));
    engine.initialize();
    for _ in 0..40 {
        engine.tick();
    }
    assert_eq!(
        engine.policy().actions().calls(),
        &[StaticChildHostHybridActionsCall::ActionsForWorkStaticHostedFirst]
    );
    assert_eq!(calls(&firsts, 0), ["first_ran"]);
    assert!(
        seconds.lock().unwrap().is_empty(),
        "the other was not built"
    );
    assert_eq!(engine.policy().completed(), 1, "it ran and ended");

    send(&mut engine, "again");
    send(&mut engine, "back");
    assert_eq!(
        engine.policy().actions().calls(),
        &[
            StaticChildHostHybridActionsCall::ActionsForWorkStaticHostedFirst,
            StaticChildHostHybridActionsCall::ActionsForWorkStaticHostedSecond
        ]
    );
    assert_eq!(calls(&seconds, 0), ["second_ran"]);
    // The first candidate's run is its own and was not repeated.
    assert_eq!(calls(&firsts, 0), ["first_ran"]);
    assert_eq!(engine.policy().completed(), 2);
}
