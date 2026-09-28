// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE_MESH.md §9.5, §mesh-19: an `<invoke type="sce:mesh-rpc">` reaches the
// host's Mesh router through the runtime's mesh-rpc door — Rust AOT path.
//
// Fixture: integration_resources/a_mesh_request_reaches_the_router/
// a_mesh_request_reaches_the_router.scxml (canonical, shared with the other
// channels).

use std::sync::{Arc, Mutex};

use sce_rust_runtime::{
    Engine, HostInvokeEvent, HostInvokeRequest, HostInvokeResponse, IScriptEngine,
    MESH_RPC_INVOKE_TYPE,
};
use sce_rust_tests::integration::a_mesh_request_reaches_the_router::{
    AMeshRequestReachesTheRouterPolicy as Policy, AMeshRequestReachesTheRouterState as State,
};

fn engine() -> Engine<Policy> {
    let script_engine: Arc<dyn IScriptEngine> = Arc::new(sce_rust_lua::LuaEngine::new());
    Engine::new(Policy::new(script_engine))
}

/// A router that keeps every request it is started with and answers none,
/// so the test decides how each one ends.
fn recording_router(
    requests: &Arc<Mutex<Vec<HostInvokeRequest>>>,
) -> impl FnMut(HostInvokeEvent) -> Option<HostInvokeResponse> + Send + 'static {
    let requests = Arc::clone(requests);
    move |event| {
        if let HostInvokeEvent::Start(request) = event {
            requests.lock().unwrap().push(request);
        }
        None
    }
}

/// The one request the router was started with, checked against what the
/// document wrote.
fn the_request(requests: &Arc<Mutex<Vec<HostInvokeRequest>>>) -> HostInvokeRequest {
    let requests = requests.lock().unwrap();
    assert_eq!(
        requests.len(),
        1,
        "exactly one request reaches the router: {requests:?}"
    );
    let request = requests[0].clone();
    assert_eq!(request.processor_type, MESH_RPC_INVOKE_TYPE);
    assert_eq!(request.invoke_id, "ask");
    assert_eq!(request.src, "#motor");
    let param = |name: &str| request.params.get(name).cloned().unwrap_or_default();
    assert_eq!(param("_mesh_event"), ["service.request.force"]);
    assert_eq!(param("_mesh_deadline_ms"), ["250"]);
    assert_eq!(param("force"), ["3"]);
    assert_eq!(param("speed"), ["3"]);
    // The author's pairs alone, typed: `force` was computed, `speed` was
    // written as a string, and the envelope fields are not payload.
    assert_eq!(request.event_data, r#"{"force":3,"speed":"3"}"#);
    request
}

/// The router answers: done.invoke.ask ends the run.
#[test]
fn a_mesh_request_is_answered_through_the_router() {
    let requests: Arc<Mutex<Vec<HostInvokeRequest>>> = Arc::default();
    let mut e = engine();
    e.register_mesh_rpc_invoker(recording_router(&requests));
    e.initialize();
    e.step();

    let request = the_request(&requests);
    assert!(e.complete_host_invoke(MESH_RPC_INVOKE_TYPE, "ask", request.token, "\"ok\""));
    e.step();

    assert_eq!(e.policy().answered(), Some(1));
    assert_eq!(e.policy().failed(), Some(0));
    assert_eq!(e.policy().refused(), Some(0));
    assert_eq!(e.terminal_state(), Some(State::Done));
}

/// The router fails the request: error.invoke.ask, carrying the router's data.
#[test]
fn a_mesh_request_the_router_fails_is_error_invoke() {
    let requests: Arc<Mutex<Vec<HostInvokeRequest>>> = Arc::default();
    let mut e = engine();
    e.register_mesh_rpc_invoker(recording_router(&requests));
    e.initialize();
    e.step();

    let request = the_request(&requests);
    assert!(e.fail_host_invoke(
        MESH_RPC_INVOKE_TYPE,
        "ask",
        request.token,
        "\"unreachable\"",
        "mesh://motor",
        ""
    ));
    e.step();

    assert_eq!(e.policy().answered(), Some(0));
    assert_eq!(e.policy().failed(), Some(1));
    assert_eq!(e.policy().refused(), Some(0));
    assert_eq!(e.terminal_state(), Some(State::Done));
}

/// With no router registered the invoke names a type nobody runs:
/// error.execution (W3C SCXML 6.4.1).
#[test]
fn a_mesh_request_with_no_router_registered_is_error_execution() {
    let mut e = engine();
    e.initialize();
    e.step();

    assert_eq!(e.policy().answered(), Some(0));
    assert_eq!(e.policy().failed(), Some(0));
    assert_eq!(e.policy().refused(), Some(1));
    assert_eq!(e.terminal_state(), Some(State::Done));
}
