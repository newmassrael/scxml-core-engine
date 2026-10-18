// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Zenoh binding across a real watching-zenoh router, with upstream's `zenoh` crate as the
//! client of both machines.
//!
//! Opt-in: it needs a built `wz-ap-demo` with `router-hat-router` and `routing-token-tables` (the
//! liveliness plane the binding's readiness rides on), named by `WZ_ROUTER_BIN`:
//!
//! ```text
//! WZ_ROUTER_BIN=/path/to/wz-ap-demo cargo test -p sce-rust-mesh --features zenoh \
//!     --test zenoh_through_a_wz_router -- --ignored --nocapture
//! ```
//!
//! What it holds: two machines that only know each other's names learn each is up through the
//! router (§mesh-20.3), an envelope one sends reaches the other's engine under the sender's name
//! (§mesh-20.1, §mesh-20.2), and a machine that goes away is told to the other as a lost link
//! (§mesh-20.3), all through a real watching-zenoh router with upstream's client (§mesh-20.4).
//! Nothing here is a timing claim; every wait has a generous bound and the assertions are on what
//! arrived.

#![cfg(feature = "zenoh")]

use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use sce_rust_mesh::endpoint::{
    Endpoint, EngineCall, EngineEvent, Environment, MESH_PROCESSOR_TYPE,
};
use sce_rust_mesh::inbound::Delivery;
use sce_rust_mesh::link::{deliver, LinkEvent};
use sce_rust_mesh::outbound::OutboundBuffer;
use sce_rust_mesh::router::{PeerConfig, Router};
use sce_rust_mesh::zenoh::{client_config, open, ZenohBinding, ZenohEvents, ZenohTransport};
use sce_rust_runtime::HostSendRequest;

/// A clock that counts, and ids that count.
#[derive(Default)]
struct Counting {
    now: i64,
    ids: u8,
}

impl Environment for Counting {
    fn now_ms(&mut self) -> i64 {
        self.now += 1;
        self.now
    }
    fn now_unix_ms(&mut self) -> u64 {
        self.now as u64
    }
    fn envelope_id(&mut self) -> [u8; 16] {
        self.ids += 1;
        [self.ids; 16]
    }
    fn jitter_draw(&mut self) -> i64 {
        0
    }
}

/// The router process, killed when the test ends however it ends.
struct RouterProcess {
    child: Child,
    port: u16,
}

impl Drop for RouterProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Start a router, federated with the router listening on `connect_to` when one is named.
fn start_router(connect_to: Option<u16>) -> RouterProcess {
    let binary = std::env::var("WZ_ROUTER_BIN").expect(
        "WZ_ROUTER_BIN names a wz-ap-demo built with `router-hat-router,routing-token-tables`",
    );
    let mut args = vec!["--router-hat".to_string(), "127.0.0.1:0".to_string()];
    if let Some(port) = connect_to {
        args.push("--connect".to_string());
        args.push(format!("127.0.0.1:{port}"));
    }
    let mut child = Command::new(binary)
        .args(&args)
        .env("RUST_LOG", "info")
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the router starts");
    let stderr = child.stderr.take().expect("stderr is piped");
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            if let Some(rest) = line.split("listening on 127.0.0.1:").nth(1) {
                let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
                if let Ok(port) = digits.parse::<u16>() {
                    let _ = tx.send(port);
                }
            }
        }
    });
    let port = rx
        .recv_timeout(Duration::from_secs(30))
        .expect("the router said which port it listens on");
    RouterProcess { child, port }
}

fn peer_config(peer: &'static str) -> PeerConfig {
    PeerConfig {
        transport: "zenoh",
        // The readiness edge the binding supplies is what gates this buffer: nothing leaves for a
        // peer until its presence is seen.
        buffer: Some(OutboundBuffer {
            max_pending: 8,
            max_age_ms: 0,
        }),
        retry: None,
        stamp_sequence: false,
        delivery: Delivery {
            dedup: true,
            ordered: false,
        },
        responders: Box::leak(Box::new([peer])),
        deadline_ms: None,
        reply_events: &[],
    }
}

fn endpoint(
    machine: &str,
    peer: &'static str,
    transport: ZenohTransport,
) -> Endpoint<ZenohTransport, Counting> {
    let mut router = Router::new(machine, 8, 50).expect("a router");
    router.add_peer(peer, peer_config(peer));
    Endpoint::new(router, transport, Counting::default())
}

fn send(event: &str, target: &str) -> HostSendRequest {
    HostSendRequest {
        processor_type: MESH_PROCESSOR_TYPE.to_string(),
        event_name: event.to_string(),
        target: format!("#{target}"),
        send_id: "s-1".to_string(),
        event_data: r#"{"v":1}"#.to_string(),
        ..HostSendRequest::default()
    }
}

fn raised(endpoint: &mut Endpoint<ZenohTransport, Counting>) -> Vec<EngineEvent> {
    endpoint
        .take_calls()
        .into_iter()
        .filter_map(|call| match call {
            EngineCall::Raise(event) => Some(event),
            _ => None,
        })
        .collect()
}

/// Feed `events` to `endpoint` until what it raised satisfies `done`, or `within` passes. Returns
/// whether it did.
fn pump_until(
    events: &ZenohEvents,
    endpoint: &mut Endpoint<ZenohTransport, Counting>,
    within: Duration,
    mut done: impl FnMut(&[EngineEvent]) -> bool,
) -> bool {
    let deadline = Instant::now() + within;
    let mut seen: Vec<EngineEvent> = Vec::new();
    while Instant::now() < deadline {
        if let Some(event) = events.next_within(Duration::from_millis(100)) {
            deliver(endpoint, event);
            events.pump(endpoint);
        }
        endpoint.tick();
        seen.extend(raised(endpoint));
        if done(&seen) {
            return true;
        }
    }
    false
}

#[test]
#[ignore = "needs WZ_ROUTER_BIN: a built wz-ap-demo with router-hat-router and routing-token-tables"]
fn two_machines_meet_through_a_wz_router_and_one_reaches_the_others_engine() {
    let router = start_router(None);
    two_machines_meet(router.port, router.port);
}

/// The scaling claim, as far as one host can hold it: the machines are on DIFFERENT routers, joined
/// by a federation link, and still see each other up, reach each other's engine, and see each other
/// go. A device connects to whichever router it is assigned and the server to its own; nothing in
/// the binding says which.
#[test]
#[ignore = "needs WZ_ROUTER_BIN: a built wz-ap-demo with router-hat-router and routing-token-tables"]
fn two_machines_on_two_federated_wz_routers_meet_as_if_on_one() {
    let first = start_router(None);
    let second = start_router(Some(first.port));
    two_machines_meet(first.port, second.port);
}

/// A machine "server" on the router at `server_port` and a machine "phone" on the one at
/// `phone_port` (the same router, or two federated).
fn two_machines_meet(server_port: u16, phone_port: u16) {
    let started = Instant::now();
    let config_on =
        |port: u16| client_config(&[&format!("tcp/127.0.0.1:{port}")]).expect("a config");

    let ZenohBinding {
        transport: server_transport,
        events: server_events,
    } = open(config_on(server_port), "server").expect("the server's binding opens");
    let ZenohBinding {
        transport: phone_transport,
        events: phone_events,
    } = open(config_on(phone_port), "phone").expect("the phone's binding opens");

    // Each learns the other is up, from the router's liveliness plane and nothing it was told. The
    // events are read here to prove it and the core is told the readiness by hand, as a host's pump
    // would have, since the proof consumed them.
    let mut server_ready = false;
    let mut phone_ready = false;
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline && !(server_ready && phone_ready) {
        if let Some(event) = server_events.next_within(Duration::from_millis(100)) {
            server_ready |= event == LinkEvent::Ready("phone".to_string());
        }
        if let Some(event) = phone_events.next_within(Duration::from_millis(100)) {
            phone_ready |= event == LinkEvent::Ready("server".to_string());
        }
    }
    assert!(server_ready, "the server learned the phone is up");
    assert!(phone_ready, "the phone learned the server is up");
    eprintln!(
        "server on router {server_port}, phone on router {phone_port}: both saw the other up \
         after {:?}",
        started.elapsed()
    );

    let mut server_endpoint = endpoint("server", "phone", server_transport);
    let mut phone_endpoint = endpoint("phone", "server", phone_transport);
    server_endpoint.peer_ready("phone");
    phone_endpoint.peer_ready("server");

    phone_endpoint.send(&send("note.added", "server"));
    assert!(
        pump_until(
            &server_events,
            &mut server_endpoint,
            Duration::from_secs(20),
            |events| events.iter().any(|event| event.name == "note.added"),
        ),
        "the envelope the phone sent reached the server's engine"
    );
    eprintln!(
        "the envelope reached the server's engine after {:?}",
        started.elapsed()
    );

    // The phone goes away: its endpoint owns the transport that holds the presence token, so
    // dropping both closes the session, and the server is told the link is lost.
    drop(phone_endpoint);
    drop(phone_events);
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut lost = false;
    while Instant::now() < deadline && !lost {
        if let Some(event) = server_events.next_within(Duration::from_millis(100)) {
            lost = matches!(&event, LinkEvent::Lost(peer, _) if peer == "phone");
        }
    }
    assert!(lost, "the server was told the phone is gone");
    eprintln!(
        "the server was told the phone is gone after {:?}",
        started.elapsed()
    );
}
