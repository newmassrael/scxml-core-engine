// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE_MESH.md §9.5, §mesh-19 under `datamodel="sce-static"` (docs/adr/0005,
// decisions 5 and 7): an `<invoke type="sce:mesh-rpc">` whose peer and `<param>`s
// are typed expressions over the machine's own fields reaches the host's Mesh
// router through the runtime's mesh-rpc door — Rust AOT path. The twin of
// `a_mesh_request_reaches_the_router`, which holds the same request under a
// script data model.
//
// The committed machine under `src/integration/host_processor/` is generated
// from `sce-build/tests/fixtures/host_processor/statechart_static_mesh_request.scxml`
// (regen: `scripts/regen_host_processor.sh`) and constructed with NO script
// engine: its variables are fields, and a peer or a `<param>` that needed an
// engine to be read would not have one to ask.
//
// The run `bump`, `go` changes `load` and `label` before the request reads them,
// so a copy taken at start-up (3, "idle") is told from what the fields hold now
// (4, "busy"): the request carries 8 and "busy".

use std::sync::{Arc, Mutex};

use sce_rust_runtime::{
    Engine, HostInvokeEvent, HostInvokeRequest, HostInvokeResponse, MESH_RPC_INVOKE_TYPE,
};
use sce_rust_tests::integration::host_processor::{
    StatechartStaticMeshRequestPolicy as Policy, StatechartStaticMeshRequestState as State,
};

/// A machine with no script engine, initialised and standing at `idle`.
fn engine() -> Engine<Policy> {
    let mut e = Engine::new(Policy::new());
    e.initialize();
    e.step();
    e
}

fn drive(e: &mut Engine<Policy>, events: &[&str]) {
    for event in events {
        e.raise_external_by_name(event, "");
        e.step();
    }
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
/// document wrote and the fields held when it started.
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
    assert_eq!(param("force"), ["8"]);
    assert_eq!(param("speed"), ["busy"]);
    // The author's pairs alone, typed: `force` was computed, `speed` is a
    // string, and the envelope fields are not payload.
    assert_eq!(request.event_data, r#"{"force":8,"speed":"busy"}"#);
    request
}

/// The router answers: done.invoke.ask ends the run.
#[test]
fn a_static_mesh_request_is_answered_through_the_router() {
    let requests: Arc<Mutex<Vec<HostInvokeRequest>>> = Arc::default();
    let mut e = engine();
    e.register_mesh_rpc_invoker(recording_router(&requests));
    drive(&mut e, &["bump", "go"]);

    let request = the_request(&requests);
    assert!(e.complete_host_invoke(MESH_RPC_INVOKE_TYPE, "ask", request.token, "\"ok\""));
    e.step();

    assert_eq!(e.policy().answered(), 1);
    assert_eq!(e.policy().failed(), 0);
    assert_eq!(e.policy().refused(), 0);
    assert_eq!(e.terminal_state(), Some(State::Done));
}

/// The router fails the request: error.invoke.ask, carrying the router's data.
#[test]
fn a_static_mesh_request_the_router_fails_is_error_invoke() {
    let requests: Arc<Mutex<Vec<HostInvokeRequest>>> = Arc::default();
    let mut e = engine();
    e.register_mesh_rpc_invoker(recording_router(&requests));
    drive(&mut e, &["bump", "go"]);

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

    assert_eq!(e.policy().answered(), 0);
    assert_eq!(e.policy().failed(), 1);
    assert_eq!(e.policy().refused(), 0);
    assert_eq!(e.terminal_state(), Some(State::Done));
}

/// The control: nothing has written a field, so the request carries what
/// `<data expr>` gave them and not the values `bump` would have set.
#[test]
fn the_request_carries_the_fields_as_they_stand_and_not_a_copy_from_start_up() {
    let requests: Arc<Mutex<Vec<HostInvokeRequest>>> = Arc::default();
    let mut e = engine();
    e.register_mesh_rpc_invoker(recording_router(&requests));
    drive(&mut e, &["go"]);

    let requests = requests.lock().unwrap();
    assert_eq!(requests.len(), 1, "one request: {requests:?}");
    assert_eq!(requests[0].event_data, r#"{"force":6,"speed":"idle"}"#);
}

/// With no router registered the invoke names a type nobody runs:
/// error.execution (W3C SCXML 6.4.1).
#[test]
fn a_static_mesh_request_with_no_router_registered_is_error_execution() {
    let mut e = engine();
    drive(&mut e, &["bump", "go"]);

    assert_eq!(e.policy().answered(), 0);
    assert_eq!(e.policy().failed(), 0);
    assert_eq!(e.policy().refused(), 1);
    assert_eq!(e.terminal_state(), Some(State::Done));
}

// ── The same document through the Rust host core itself (sce-rust-mesh) ──

mod through_the_host_core {
    use super::*;
    use sce_forge_runtime::codec::SceCursor;
    use sce_rust_mesh::endpoint::{
        self, Endpoint, Environment, SharedEndpoint, Transport, TransportFailure,
    };
    use sce_rust_mesh::generated::envelope::Envelope;
    use sce_rust_mesh::generated::pattern_kind::PatternKind;
    use sce_rust_mesh::generated::payload_codec::PayloadCodec;
    use sce_rust_mesh::generated::rpc_status::RpcStatus;
    use sce_rust_mesh::inbound::Delivery;
    use sce_rust_mesh::router::{PeerConfig, Router};

    /// What a transport was handed: each peer it named and the bytes.
    type Sent = Arc<Mutex<Vec<(String, Vec<u8>)>>>;

    /// A transport that keeps what it was handed.
    #[derive(Default)]
    struct Recorder {
        sent: Sent,
    }

    impl Transport for Recorder {
        fn transmit(&mut self, peer: &str, bytes: &[u8]) -> Result<(), TransportFailure> {
            self.sent
                .lock()
                .unwrap()
                .push((peer.to_string(), bytes.to_vec()));
            Ok(())
        }
    }

    /// A clock that stands still and ids that count.
    #[derive(Default)]
    struct Fixed {
        ids: u8,
    }

    impl Environment for Fixed {
        fn now_ms(&mut self) -> i64 {
            0
        }
        fn now_unix_ms(&mut self) -> u64 {
            1_000_000
        }
        fn envelope_id(&mut self) -> [u8; 16] {
            self.ids += 1;
            [self.ids; 16]
        }
        fn jitter_draw(&mut self) -> i64 {
            0
        }
    }

    /// The requester's endpoint, bound to `motor`, and what its transport sends.
    fn endpoint() -> (SharedEndpoint<Recorder, Fixed>, Sent) {
        let mut router = Router::new("brake", 8, 50).unwrap();
        router.add_peer(
            "motor",
            PeerConfig {
                transport: "wss",
                buffer: None,
                retry: None,
                stamp_sequence: false,
                delivery: Delivery {
                    dedup: true,
                    ordered: false,
                },
                responders: &["motor"],
                deadline_ms: None,
                reply_events: &[],
            },
        );
        let recorder = Recorder::default();
        let sent = Arc::clone(&recorder.sent);
        (
            Arc::new(Mutex::new(Endpoint::new(
                router,
                recorder,
                Fixed::default(),
            ))),
            sent,
        )
    }

    /// The request the document sent reaches `motor` as an `RpcRequest`, and
    /// motor's `Ok` reply ends the invoke with `done.invoke.ask`.
    #[test]
    fn a_reply_through_the_host_core_answers_the_document() {
        let (shared, sent) = endpoint();
        let mut e = engine();
        endpoint::register(&mut e, &shared);
        drive(&mut e, &["bump", "go"]);

        let (peer, bytes) = sent.lock().unwrap().pop().expect("the request was sent");
        assert_eq!(peer, "motor");
        let request = Envelope::decode(&mut SceCursor::new(&bytes)).unwrap();
        assert_eq!(request.pattern, PatternKind::RpcRequest);
        assert_eq!(request.event_type, "service.request.force");
        assert_eq!(request.data, br#"{"force":8,"speed":"busy"}"#);
        // The document's own deadline, on the wall clock.
        assert_eq!(request.deadline_unix_ms, Some(1_000_250));

        let reply = Envelope {
            id: &[0xAA; 16],
            source: "motor",
            event_type: "service.response.force",
            pattern: PatternKind::RpcReply,
            datacontenttype: PayloadCodec::Json,
            data: b"\"ok\"",
            invoke_id: request.invoke_id,
            rpc_status: Some(RpcStatus::Ok),
            ..Envelope::new()
        }
        .encode_to_vec()
        .unwrap();
        let calls = {
            let mut endpoint = shared.lock().unwrap();
            endpoint.receive("motor", reply);
            endpoint.take_calls()
        };
        endpoint::apply_to(&mut e, calls);
        e.step();

        assert_eq!(e.policy().answered(), 1);
        assert_eq!(e.policy().failed(), 0);
        assert_eq!(e.policy().refused(), 0);
        assert_eq!(e.terminal_state(), Some(State::Done));
    }

    /// The peer the field names is looked up among the bindings the host core
    /// has: `ghost` makes it a name it has none for, which cannot reach the
    /// wire, so the invocation is refused and the document sees error.execution
    /// (SCE_MESH.md §mesh-9.5's pre-envelope tier, as the generated C++ router
    /// answers it).
    #[test]
    fn a_peer_with_no_binding_through_the_host_core_is_error_execution() {
        let (shared, sent) = endpoint();
        let mut e = engine();
        endpoint::register(&mut e, &shared);
        drive(&mut e, &["ghost", "go"]);

        assert!(sent.lock().unwrap().is_empty(), "nothing reached the wire");
        assert_eq!(e.policy().answered(), 0);
        assert_eq!(e.policy().failed(), 0);
        assert_eq!(e.policy().refused(), 1);
        assert_eq!(e.terminal_state(), Some(State::Done));
    }
}
