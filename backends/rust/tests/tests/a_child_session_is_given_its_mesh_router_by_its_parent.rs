// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE-VERIFIES: mesh-21.3 mesh-21.4 mesh-21.5
//
// SCE_MESH.md, Child Sessions in Mesh (docs/adr/0043): a child session that
// talks to a Mesh peer is given a router of its own by the machine that starts
// it, asked of the factory the host registered, and never the machine's own.
//
// `static_child_mesh.scxml` starts `list`, which greets `#server` as it is
// entered and ends when the greeting is refused; the machine counts that end in
// `refusals`. Its hybrid `peer` starts `static_mesh_relay`, whose child `inner`
// answers `inner.ping`.

use sce_rust_runtime::{
    mesh_event_metadata, ChildMeshRouter, Engine, HostInvokeEvent, HostInvokeResponse,
    HostSendRequest, HostSendResponse, MeshCall, SceClock,
};
use sce_rust_tests::integration::static_datamodel::static_child_mesh_sm::{
    StaticChildMeshPersist, StaticChildMeshPolicy,
};
use std::sync::{Arc, Mutex};

type Log = Arc<Mutex<Vec<String>>>;

fn engine() -> Engine<StaticChildMeshPolicy> {
    let mut engine = Engine::new(StaticChildMeshPolicy::new());
    engine.set_clock(SceClock::Manual(0));
    engine
}

/// The rounds a child needs to take an event and its end to reach the parent.
fn settle(engine: &mut Engine<StaticChildMeshPolicy>) {
    for _ in 0..5 {
        engine.tick();
    }
}

/// The machine's own router, which no child's send may reach.
fn parent_router(engine: &mut Engine<StaticChildMeshPolicy>) -> Log {
    let log = Log::default();
    let sink = Arc::clone(&log);
    engine.register_mesh_router(move |request| {
        sink.lock().unwrap().push(request.event_name);
        Vec::new()
    });
    log
}

/// A child's router that writes down what it is told and holds what a test
/// hands it for the child.
struct Recorder {
    name: String,
    log: Log,
    inbox: Arc<Mutex<Vec<MeshCall>>>,
}

impl ChildMeshRouter for Recorder {
    fn send(&mut self, request: HostSendRequest) -> Vec<HostSendResponse> {
        self.log
            .lock()
            .unwrap()
            .push(format!("{} sent {}", self.name, request.event_name));
        Vec::new()
    }
    fn invoke(&mut self, _event: HostInvokeEvent) -> Option<HostInvokeResponse> {
        None
    }
    fn take_calls(&mut self) -> Vec<MeshCall> {
        std::mem::take(&mut *self.inbox.lock().unwrap())
    }
    fn detach(&mut self) {
        self.log
            .lock()
            .unwrap()
            .push(format!("{} detached", self.name));
    }
    fn child_router(
        &mut self,
        invoke_id: &str,
        document: &str,
    ) -> Option<Box<dyn ChildMeshRouter>> {
        self.log.lock().unwrap().push(format!(
            "{} asked for {invoke_id} of '{document}'",
            self.name
        ));
        Some(Box::new(Recorder {
            name: format!("{}/{invoke_id}", self.name),
            log: Arc::clone(&self.log),
            inbox: Arc::clone(&self.inbox),
        }))
    }
}

#[test]
fn a_child_given_no_router_has_its_peer_send_refused_and_the_machines_router_is_not_asked() {
    let mut engine = engine();
    let parent = parent_router(&mut engine);
    engine.initialize();
    settle(&mut engine);
    assert_eq!(
        engine.policy().refusals(),
        1,
        "the child's greeting found no router, raised error.execution in the child, and ended it"
    );
    assert!(
        parent.lock().unwrap().is_empty(),
        "the child's send fell back to the machine's router: {:?}",
        parent.lock().unwrap()
    );
}

#[test]
fn a_factory_that_answers_none_leaves_the_child_with_no_router() {
    let mut engine = engine();
    let parent = parent_router(&mut engine);
    engine.register_child_mesh_routers(|_invoke_id: &str, _document: &str| None);
    engine.initialize();
    settle(&mut engine);
    assert_eq!(engine.policy().refusals(), 1);
    assert!(parent.lock().unwrap().is_empty());
}

#[test]
fn each_start_asks_the_factory_and_each_end_detaches_what_it_answered() {
    let mut engine = engine();
    let parent = parent_router(&mut engine);
    let log = Log::default();
    let inbox: Arc<Mutex<Vec<MeshCall>>> = Arc::default();
    let (asked, held) = (Arc::clone(&log), Arc::clone(&inbox));
    engine.register_child_mesh_routers(move |invoke_id: &str, document: &str| {
        asked
            .lock()
            .unwrap()
            .push(format!("asked for {invoke_id} of '{document}'"));
        let router: Box<dyn ChildMeshRouter> = Box::new(Recorder {
            name: invoke_id.to_string(),
            log: Arc::clone(&asked),
            inbox: Arc::clone(&held),
        });
        Some(router)
    });
    engine.initialize();
    settle(&mut engine);
    // Asked before the child was entered, since the greeting went through it.
    assert_eq!(
        *log.lock().unwrap(),
        vec!["asked for list of ''", "list sent list.hello"]
    );
    assert_eq!(engine.policy().refusals(), 0);

    // A peer's event reaches the child when the machine next drives it.
    inbox.lock().unwrap().push(MeshCall::Raise {
        name: "list.ping".to_string(),
        metadata: mesh_event_metadata(String::new(), "server", String::new()),
    });
    settle(&mut engine);
    assert_eq!(log.lock().unwrap().last().unwrap(), "list sent list.pong");

    // The hybrid candidate is asked for by its stem, and its child of its router.
    log.lock().unwrap().clear();
    engine.raise_external_by_name("open", "");
    settle(&mut engine);
    assert_eq!(
        *log.lock().unwrap(),
        vec![
            "asked for peer of 'static_mesh_relay'",
            "peer asked for inner of ''"
        ]
    );

    // Leaving the invoking states ends every session, each detached once: the
    // inner state's child first, and a grandchild before its parent, since
    // stopping a child cancels its children.
    log.lock().unwrap().clear();
    engine.raise_external_by_name("again", "");
    settle(&mut engine);
    assert_eq!(
        *log.lock().unwrap(),
        vec!["peer/inner detached", "peer detached", "list detached"]
    );

    // Entered anew, the invoke asks again.
    log.lock().unwrap().clear();
    engine.raise_external_by_name("back", "");
    settle(&mut engine);
    assert_eq!(
        *log.lock().unwrap(),
        vec!["asked for list of ''", "list sent list.hello"]
    );
    assert!(parent.lock().unwrap().is_empty());
}

/// The factory a host registers: each router it answers writes down what it is
/// told into `log` and holds what a test hands its child in `inbox`.
fn recording_factory(
    log: &Log,
    inbox: &Arc<Mutex<Vec<MeshCall>>>,
) -> impl FnMut(&str, &str) -> Option<Box<dyn ChildMeshRouter>> + Send + 'static {
    let (asked, held) = (Arc::clone(log), Arc::clone(inbox));
    move |invoke_id: &str, document: &str| {
        asked
            .lock()
            .unwrap()
            .push(format!("asked for {invoke_id} of '{document}'"));
        let router: Box<dyn ChildMeshRouter> = Box::new(Recorder {
            name: invoke_id.to_string(),
            log: Arc::clone(&asked),
            inbox: Arc::clone(&held),
        });
        Some(router)
    }
}

#[test]
fn a_restored_machine_asks_for_its_resumed_childrens_routers_once_its_host_registers_the_factory() {
    // docs/adr/0045 + docs/adr/0043: a restore builds the machine and resumes
    // its children before its host can register anything on it. The factory
    // is asked for each resumed child when the host registers it, before the
    // host drives the machine, and each grandchild of the router its parent was
    // given; nothing a resumed child did as it started is done again.
    let mut engine = engine();
    let first_log = Log::default();
    let first_inbox: Arc<Mutex<Vec<MeshCall>>> = Arc::default();
    engine.register_child_mesh_routers(recording_factory(&first_log, &first_inbox));
    engine.initialize();
    settle(&mut engine);
    engine.raise_external_by_name("open", "");
    settle(&mut engine);
    let saved = engine.save_at(0).expect("saves with both children running");

    let mut restored = Engine::<StaticChildMeshPolicy>::restore_with(
        StaticChildMeshPolicy::new(),
        &saved,
        SceClock::Manual(0),
        0,
    )
    .expect("restores");
    let parent = parent_router(&mut restored);
    let log = Log::default();
    let inbox: Arc<Mutex<Vec<MeshCall>>> = Arc::default();
    restored.register_child_mesh_routers(recording_factory(&log, &inbox));
    assert_eq!(
        *log.lock().unwrap(),
        vec![
            "asked for list of ''",
            "asked for peer of 'static_mesh_relay'",
            "peer asked for inner of ''"
        ],
        "each resumed session's router is asked for as the factory is registered, \
         and no greeting is sent again"
    );

    // A peer's event reaches the resumed child through the router it was given.
    inbox.lock().unwrap().push(MeshCall::Raise {
        name: "list.ping".to_string(),
        metadata: mesh_event_metadata(String::new(), "server", String::new()),
    });
    settle(&mut restored);
    assert_eq!(log.lock().unwrap().last().unwrap(), "list sent list.pong");
    assert_eq!(restored.policy().refusals(), 0);
    assert!(parent.lock().unwrap().is_empty());

    // Asked once: a factory registered again is not asked for a child that
    // already has its router.
    log.lock().unwrap().clear();
    restored.register_child_mesh_routers(recording_factory(&log, &inbox));
    let asked_again = log.lock().unwrap().clone();
    assert!(asked_again.is_empty(), "{asked_again:?}");
}
