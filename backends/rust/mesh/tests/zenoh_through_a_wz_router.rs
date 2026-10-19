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
use std::sync::atomic::{AtomicU8, Ordering};
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

/// A clock that counts, and ids that count. Each one starts its ids where no earlier one reached:
/// a real envelope id is a UUID v7 and never repeats, and a machine that comes back under the same
/// name must not send a first envelope the receiver has already seen.
struct Counting {
    now: i64,
    ids: u8,
}

impl Default for Counting {
    fn default() -> Self {
        static NEXT_SEED: AtomicU8 = AtomicU8::new(0);
        Counting {
            now: 0,
            ids: NEXT_SEED.fetch_add(8, Ordering::Relaxed),
        }
    }
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
    endpoint_with_peers(machine, &[peer], transport)
}

fn endpoint_with_peers(
    machine: &str,
    peers: &[&'static str],
    transport: ZenohTransport,
) -> Endpoint<ZenohTransport, Counting> {
    let mut router = Router::new(machine, 8, 50).expect("a router");
    for peer in peers {
        router.add_peer(peer, peer_config(peer));
    }
    Endpoint::new(router, transport, Counting::default())
}

fn send(event: &str, target: &str) -> HostSendRequest {
    send_with(event, target, r#"{"v":1}"#)
}

fn send_with(event: &str, target: &str, data: &str) -> HostSendRequest {
    HostSendRequest {
        processor_type: MESH_PROCESSOR_TYPE.to_string(),
        event_name: event.to_string(),
        target: format!("#{target}"),
        send_id: "s-1".to_string(),
        event_data: data.to_string(),
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

/// The machine that stays up and holds the state, which is what a device that is on only while its
/// user presses sync catches up from. The router keeps nothing for an absent machine, so the
/// hub's own `notes` are the whole of what a later visit can be given.
struct Hub {
    endpoint: Endpoint<ZenohTransport, Counting>,
    events: ZenohEvents,
    notes: Vec<String>,
    /// The machines it was told are gone, in the order it was told.
    lost: Vec<String>,
}

impl Hub {
    /// One turn: take what has arrived, keep a pushed note, and answer a request for the notes after
    /// the number the asker has seen.
    fn step(&mut self) {
        let mut arrived = Vec::new();
        if let Some(event) = self.events.next_within(Duration::from_millis(10)) {
            arrived.push(event);
        }
        while let Some(event) = self.events.next_within(Duration::ZERO) {
            arrived.push(event);
        }
        for event in arrived {
            if let LinkEvent::Lost(peer, _) = &event {
                self.lost.push(peer.clone());
            }
            deliver(&mut self.endpoint, event);
        }
        self.endpoint.tick();
        for event in raised(&mut self.endpoint) {
            let data = event.metadata.data.to_string();
            match event.name.as_str() {
                "note.added" => self.notes.push(json_text(&data, "note").expect("a note")),
                "sync.since" => {
                    let from = json_text(&data, "from").expect("who asks");
                    let since: usize = json_text(&data, "since")
                        .and_then(|n| n.parse().ok())
                        .expect("what they have seen");
                    let after = self.notes.get(since..).unwrap_or_default();
                    let reply = format!(
                        r#"{{"notes":[{}],"seen":{}}}"#,
                        after
                            .iter()
                            .map(|note| format!("\"{note}\""))
                            .collect::<Vec<_>>()
                            .join(","),
                        self.notes.len()
                    );
                    self.endpoint.send(&send_with("sync.reply", &from, &reply));
                }
                // How the core tells its engine a machine went away (§mesh-20.3): the link is lost,
                // and a send to it would find no transport. It names a machine this hub was just
                // told is gone, and nothing else is held against it.
                "error.communication" => {
                    let target = json_text(&data, "target").expect("who is gone");
                    assert!(data.contains("TRANSPORT_UNAVAILABLE"), "{data}");
                    assert!(self.lost.contains(&target), "{target} was not lost: {data}");
                }
                other => panic!("the hub was sent an event it does not know: {other} {data}"),
            }
        }
    }
}

/// The text of `key` in a flat JSON object this file wrote: a quoted string, or a bare number.
fn json_text(data: &str, key: &str) -> Option<String> {
    let after = data.split(&format!("\"{key}\":")).nth(1)?;
    let after = after.trim_start();
    match after.strip_prefix('"') {
        Some(quoted) => quoted.split('"').next().map(str::to_string),
        None => Some(
            after
                .chars()
                .take_while(char::is_ascii_digit)
                .collect::<String>(),
        ),
    }
}

/// One visit of a device: it comes up as `name`, sends `requests` to the hub, and stays until
/// `done` holds of what it raised, then goes and the hub is told it is gone. The requests are sent
/// at once, before the hub is known to be up, and wait in the device's buffer for it.
fn visit(
    port: u16,
    hub: &mut Hub,
    name: &'static str,
    requests: &[(&str, String)],
    done: impl Fn(&Hub, &[EngineEvent]) -> bool,
) -> Vec<EngineEvent> {
    let ZenohBinding { transport, events } = open(
        client_config(&[&format!("tcp/127.0.0.1:{port}")]).expect("a config"),
        name,
    )
    .expect("the device's binding opens");
    let mut device = endpoint(name, "hub", transport);
    for (event, data) in requests {
        device.send(&send_with(event, "hub", data));
    }

    let lost_before = hub.lost.iter().filter(|peer| *peer == name).count();
    let mut seen: Vec<EngineEvent> = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(30);
    while Instant::now() < deadline && !done(hub, &seen) {
        if let Some(event) = events.next_within(Duration::from_millis(10)) {
            deliver(&mut device, event);
            events.pump(&mut device);
        }
        device.tick();
        seen.extend(raised(&mut device));
        hub.step();
    }
    assert!(done(hub, &seen), "{name}'s visit did not complete in time");

    drop(device);
    drop(events);
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline
        && hub.lost.iter().filter(|peer| *peer == name).count() == lost_before
    {
        hub.step();
    }
    assert!(
        hub.lost.iter().filter(|peer| *peer == name).count() > lost_before,
        "the hub was told {name} is gone"
    );
    seen
}

fn note_pushed(note: &str) -> Vec<(&'static str, String)> {
    vec![("note.added", format!(r#"{{"note":"{note}"}}"#))]
}

fn sync_since(from: &str, seen: usize) -> Vec<(&'static str, String)> {
    vec![(
        "sync.since",
        format!(r#"{{"from":"{from}","since":{seen}}}"#),
    )]
}

fn reply_data(events: &[EngineEvent]) -> Option<String> {
    events
        .iter()
        .find(|event| event.name == "sync.reply")
        .map(|event| event.metadata.data.to_string())
}

/// A device that was not there when a note was pushed asks the hub, on its own visit, for what it
/// has not seen, and gets it: the shape of a shared list whose devices are on only while their
/// user presses sync. Alice comes, pushes, and goes; she comes again and pushes another; Bob, who
/// was away for both, comes and asks for everything, goes, and comes back to ask for what is new.
/// The router holds nothing for an absent machine; what Bob is given is the hub's state.
#[test]
#[ignore = "needs WZ_ROUTER_BIN: a built wz-ap-demo with router-hat-router and routing-token-tables"]
fn a_device_that_was_away_catches_up_from_a_hub_that_stayed_up() {
    let router = start_router(None);
    let port = router.port;
    let ZenohBinding { transport, events } = open(
        client_config(&[&format!("tcp/127.0.0.1:{port}")]).expect("a config"),
        "hub",
    )
    .expect("the hub's binding opens");
    let mut hub = Hub {
        endpoint: endpoint_with_peers("hub", &["alice", "bob"], transport),
        events,
        notes: Vec::new(),
        lost: Vec::new(),
    };

    visit(port, &mut hub, "alice", &note_pushed("milk"), |hub, _| {
        hub.notes == ["milk"]
    });
    visit(port, &mut hub, "alice", &note_pushed("eggs"), |hub, _| {
        hub.notes == ["milk", "eggs"]
    });
    assert_eq!(hub.lost, ["alice", "alice"], "both visits ended");

    let first = visit(port, &mut hub, "bob", &sync_since("bob", 0), |_, seen| {
        reply_data(seen).is_some()
    });
    assert_eq!(
        reply_data(&first).as_deref(),
        Some(r#"{"notes":["milk","eggs"],"seen":2}"#),
        "bob, who was away for both pushes, is given both"
    );

    visit(port, &mut hub, "alice", &note_pushed("bread"), |hub, _| {
        hub.notes.len() == 3
    });
    let second = visit(port, &mut hub, "bob", &sync_since("bob", 2), |_, seen| {
        reply_data(seen).is_some()
    });
    assert_eq!(
        reply_data(&second).as_deref(),
        Some(r#"{"notes":["bread"],"seen":3}"#),
        "bob's second visit is given only what is new since the two he saw"
    );
}
