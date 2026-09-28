// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE_MESH.md §mesh-19: a `targetexpr` that evaluates to a Mesh peer reaches
// the host's Mesh router — Rust AOT path.
//
// Fixture: integration_resources/a_peer_named_at_run_time_reaches_the_router/
// a_peer_named_at_run_time_reaches_the_router.scxml (canonical, shared with the
// other channels).

use std::sync::{Arc, Mutex};

use sce_rust_runtime::{Engine, HostSendRequest, IScriptEngine};
use sce_rust_tests::integration::a_peer_named_at_run_time_reaches_the_router::{
    APeerNamedAtRunTimeReachesTheRouterPolicy as Policy,
    APeerNamedAtRunTimeReachesTheRouterState as State,
};

fn engine() -> Engine<Policy> {
    let script_engine: Arc<dyn IScriptEngine> = Arc::new(sce_rust_lua::LuaEngine::new());
    Engine::new(Policy::new(script_engine))
}

/// With a router registered, the peer send is the router's — processor
/// `sce:mesh`, the evaluated target, the event — and the non-peer target is
/// still this session's.
#[test]
fn a_peer_named_at_run_time_is_sent_to_the_router() {
    let routed: Arc<Mutex<Vec<HostSendRequest>>> = Arc::default();
    let mut e = engine();
    let sink = Arc::clone(&routed);
    e.register_mesh_router(move |request| {
        sink.lock().unwrap().push(request);
        Vec::new()
    });
    e.initialize();
    e.step();

    let routed = routed.lock().unwrap();
    assert_eq!(
        routed.len(),
        1,
        "exactly the peer send reaches the router: {routed:?}"
    );
    assert_eq!(
        routed[0].processor_type,
        sce_rust_runtime::MESH_PROCESSOR_TYPE
    );
    assert_eq!(routed[0].target, "#hmi");
    assert_eq!(routed[0].event_name, "ping");
    assert_eq!(e.policy().refused(), Some(0));
    assert_eq!(e.policy().looped(), Some(1));
    assert_eq!(e.terminal_state(), Some(State::Done));
}

/// With no router registered, the peer send is a send to a host processor
/// nobody serves: one error.execution, and the non-peer target is unaffected.
#[test]
fn a_peer_with_no_router_registered_is_error_execution() {
    let mut e = engine();
    e.initialize();
    e.step();

    assert_eq!(e.policy().refused(), Some(1));
    assert_eq!(e.policy().looped(), Some(1));
    assert_eq!(e.terminal_state(), Some(State::Done));
}
