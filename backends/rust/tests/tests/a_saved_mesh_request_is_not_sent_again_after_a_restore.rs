// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Accepted Subset §2.15, "Saving and restoring": a Mesh request
// (`<invoke type="sce:mesh-rpc">`) has one request in flight, and the peer it
// reached may already have acted on it. A saved state holds the call as it was
// sent; a restore does NOT send it again, because the peer would act on it twice,
// and the router that carried it is gone with the process that saved it. The
// document is told on the first macrostep that the call was interrupted —
// `error.invoke.<id>` carrying `"interrupted"` — and sends a new request if it
// means to ask again.
//
// `statechart_static_mesh_request.scxml` asks from `asking`; `done.invoke.ask` and
// `error.invoke.ask` each leave it, counting in `answered` and `failed`.
//
// Driven entirely on `SceClock::Manual`: no case sleeps. The shared instance
// `saved/statechart_static_mesh_request_asking.json` is the text the Kotlin suite
// writes and restores too.

use std::sync::{Arc, Mutex};

use sce_rust_runtime::saved_state::SavedState;
use sce_rust_runtime::{
    Engine, HostInvokeEvent, HostInvokeResponse, SceClock, MESH_RPC_INVOKE_TYPE,
};
use sce_rust_tests::integration::host_processor::{
    StatechartStaticMeshRequestPersist as Persist, StatechartStaticMeshRequestPolicy as Policy,
    StatechartStaticMeshRequestState as State,
};

const SHARED_ASKING: &str = include_str!(
    "../../../../sce-build/tests/fixtures/host_processor/saved/statechart_static_mesh_request_asking.json"
);

/// The wall-clock moment the shared instance was saved at.
const SAVED_AT_MS: u64 = 1_700_000_000_000;

type Log = Arc<Mutex<Vec<HostInvokeEvent>>>;

fn register(engine: &mut Engine<Policy>, log: &Log) {
    let log = Arc::clone(log);
    engine.register_mesh_rpc_invoker(move |event| -> Option<HostInvokeResponse> {
        log.lock().unwrap().push(event);
        None
    });
}

/// A machine whose request to `#motor` is in flight, the router having been handed it.
fn asking() -> (Engine<Policy>, Log) {
    let log = Log::default();
    let mut engine = Engine::new(Policy::new());
    engine.set_clock(SceClock::Manual(0));
    register(&mut engine, &log);
    engine.initialize();
    engine.raise_external_by_name("go", "");
    engine.tick();
    (engine, log)
}

/// `text` restored `elapsed_ms` after it was saved, the router registered on the
/// engine the restore returned, the way a Rust host does it.
fn restored(text: &str, elapsed_ms: u64) -> (Engine<Policy>, Log) {
    let mut engine = Engine::<Policy>::restore_with(
        Policy::new(),
        &SavedState::from_json(text).expect("reads"),
        SceClock::Manual(0),
        SAVED_AT_MS + elapsed_ms,
    )
    .expect("restores");
    let log = Log::default();
    register(&mut engine, &log);
    (engine, log)
}

fn standing_in(engine: &Engine<Policy>) -> Vec<State> {
    engine.get_active_states().into_iter().collect()
}

#[test]
fn a_request_in_flight_is_saved_as_the_call_it_was() {
    let (engine, log) = asking();
    assert_eq!(
        log.lock().unwrap().len(),
        1,
        "the router was handed the request"
    );

    let saved = engine.save_at(SAVED_AT_MS).expect("saves");
    let [entry] = saved.host_invokes.as_slice() else {
        panic!("one call in flight: {:?}", saved.host_invokes)
    };
    assert_eq!(entry.processor_type, MESH_RPC_INVOKE_TYPE);
    assert_eq!(entry.invoke_id, "ask");
    assert_eq!(entry.src, "#motor");
    // The router owns the deadline of a Mesh request, so the engine holds none.
    assert_eq!(entry.due, None);
    assert!(saved.pending.is_empty(), "{:?}", saved.pending);

    // The text the Kotlin suite writes for the same machine, byte for byte.
    assert_eq!(saved.to_json(), SHARED_ASKING);
}

#[test]
fn a_restore_asks_nothing_of_the_router_until_the_machine_is_driven() {
    let (engine, log) = restored(SHARED_ASKING, 1000);
    assert!(
        log.lock().unwrap().is_empty(),
        "the restore itself calls nobody"
    );
    assert_eq!(standing_in(&engine), [State::Asking]);
}

#[test]
fn a_restored_request_is_not_sent_again_and_the_document_is_told_it_was_interrupted() {
    let (mut engine, log) = restored(SHARED_ASKING, 1000);
    engine.tick();

    assert!(
        log.lock().unwrap().is_empty(),
        "the peer may have acted on the first request already, so no second one leaves: {:?}",
        log.lock().unwrap()
    );
    assert_eq!(engine.policy().failed(), 1, "error.invoke.ask arrived");
    assert_eq!(engine.policy().answered(), 0);
    assert_eq!(engine.policy().refused(), 0);
    assert_eq!(engine.terminal_state(), Some(State::Done));
}

#[test]
fn a_call_is_interrupted_whenever_the_machine_comes_back() {
    // There is no moment after which the call is still worth sending again.
    for elapsed_ms in [0, 250, 60_000] {
        let (mut engine, log) = restored(SHARED_ASKING, elapsed_ms);
        engine.tick();
        assert!(
            log.lock().unwrap().is_empty(),
            "{elapsed_ms} ms: {:?}",
            log.lock().unwrap()
        );
        assert_eq!(
            engine.policy().failed(),
            1,
            "{elapsed_ms} ms after the save"
        );
    }
}

#[test]
fn a_machine_owing_the_interruption_says_it_needs_a_tick_now() {
    // The interruption is told in the first macrostep, so a host driving the machine by
    // its wake-up query must not be told there is nothing to wait for.
    let (engine, _log) = restored(SHARED_ASKING, 1000);
    assert_eq!(engine.time_until_next_scheduled_ms(), Some(0));
}

#[test]
fn a_restored_machine_saved_again_before_it_was_driven_keeps_the_call() {
    let (engine, _log) = restored(SHARED_ASKING, 1000);
    let again = engine.save_at(SAVED_AT_MS + 1000).expect("saves");
    let [entry] = again.host_invokes.as_slice() else {
        panic!("the call is still held: {:?}", again.host_invokes)
    };
    assert_eq!(entry.processor_type, MESH_RPC_INVOKE_TYPE);
    assert_eq!(entry.invoke_id, "ask");
}
