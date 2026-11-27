// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// docs/adr/0113, D2 (amended) under `datamodel="sce-static"`: a server with no script engine
// answers an instance of the open machine `phone` at the origin its request carried,
// `mesh://phone/<instance>`, through the Rust host core — the reply reaches the instance that
// sent the request and no other.
//
// The machine under `src/integration/host_processor/` is generated from
// `sce-build/tests/fixtures/host_processor/statechart_static_answers_an_instance.scxml` by the
// crate's build script (build.rs, tree `rust-integration`). Its `<send targetexpr="_event.origin"
// sce:targets="mesh://phone/{instance}">` holds the computed target to the one route over every
// instance of `phone`.

use std::sync::{Arc, Mutex};

use sce_forge_runtime::codec::{Bounded, SceCursor};
use sce_rust_mesh::endpoint::{
    self, Endpoint, Environment, SharedEndpoint, Transport, TransportFailure,
};
use sce_rust_mesh::generated::envelope::Envelope;
use sce_rust_mesh::generated::pattern_kind::PatternKind;
use sce_rust_mesh::generated::payload_codec::PayloadCodec;
use sce_rust_mesh::inbound::Delivery;
use sce_rust_mesh::router::{PeerConfig, Responders, Router};
use sce_rust_runtime::Engine;
use sce_rust_tests::integration::host_processor::StatechartStaticAnswersAnInstancePolicy as Policy;

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

/// The server's endpoint, bound to the open machine `phone` and to no instance of it, and what
/// its transport sends.
fn endpoint() -> (SharedEndpoint<Recorder, Fixed>, Sent) {
    let mut router = Router::new("server", 8, 50).unwrap();
    router
        .open_machine(
            "phone",
            PeerConfig {
                transport: "https",
                buffer: None,
                retry: None,
                stamp_sequence: false,
                delivery: Delivery {
                    dedup: true,
                    ordered: false,
                },
                responders: Responders::Instance,
                deadline_ms: None,
                reply_events: &[],
                causal: None,
            },
            4,
        )
        .unwrap();
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

/// A machine with no script engine, initialised and standing at `serving`, its Mesh router the
/// endpoint's.
fn engine(shared: &SharedEndpoint<Recorder, Fixed>) -> Engine<Policy> {
    let mut e = Engine::new(Policy::new());
    endpoint::register(&mut e, shared);
    let Ok(()) = e.initialize();
    e.step();
    e
}

/// `ping`, as the instance `instance` of `phone` sends it, under the envelope id `[id; 16]`.
fn ping_from(instance: &str, id: u8) -> Vec<u8> {
    Envelope {
        id: (&[id; 16]).into(),
        source: Bounded::new("phone").unwrap(),
        instance: Some(Bounded::new(instance).unwrap()),
        event_type: Bounded::new("ping").unwrap(),
        pattern: PatternKind::FireForget,
        datacontenttype: PayloadCodec::None,
        ..Envelope::new()
    }
    .encode_to_vec()
}

/// Deliver `bytes` on the link of `peer` and let the machine run what they raise.
fn deliver(
    e: &mut Engine<Policy>,
    shared: &SharedEndpoint<Recorder, Fixed>,
    peer: &str,
    bytes: Vec<u8>,
) {
    let calls = {
        let mut endpoint = shared.lock().unwrap();
        endpoint.receive(peer, bytes);
        endpoint.take_calls()
    };
    endpoint::apply_to(e, calls);
    e.step();
}

/// Each instance's `ping` is answered with `pong` on its own link: the reply reaches the
/// instance that sent the request and no other. Red if `_event.origin` is not lowered, the
/// instance route does not hold the origin, or the computed target does not reach the Mesh
/// router.
#[test]
fn a_static_servers_reply_reaches_the_instance_that_sent_the_request() {
    let (shared, sent) = endpoint();
    let mut e = engine(&shared);
    deliver(&mut e, &shared, "phone/a1", ping_from("a1", 1));
    deliver(&mut e, &shared, "phone/b2", ping_from("b2", 2));

    let sent = sent.lock().unwrap();
    let replies: Vec<(&str, String, String)> = sent
        .iter()
        .map(|(peer, bytes)| {
            let reply = Envelope::decode(&mut SceCursor::new(bytes)).unwrap();
            (
                peer.as_str(),
                reply.event_type.to_string(),
                reply.source.to_string(),
            )
        })
        .collect();
    assert_eq!(
        replies,
        [
            ("phone/a1", "pong".to_string(), "server".to_string()),
            ("phone/b2", "pong".to_string(), "server".to_string()),
        ]
    );
    assert_eq!(e.policy().answered(), 2);
    assert_eq!(e.policy().unreachable(), 0);
}

/// A `ping` no instance sent has no origin the route holds: the send is error.communication and
/// nothing is sent. Red if a value the route does not hold is sent anyway.
#[test]
fn a_ping_from_no_instance_is_answered_nowhere() {
    let (shared, sent) = endpoint();
    let mut e = engine(&shared);
    e.raise_external_by_name("ping", "");
    e.step();

    assert!(sent.lock().unwrap().is_empty(), "nothing reached the wire");
    assert_eq!(e.policy().answered(), 0);
    assert_eq!(e.policy().unreachable(), 1);
}
