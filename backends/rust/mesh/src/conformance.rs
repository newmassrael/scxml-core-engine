// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The behaviour every client of the Zenoh binding is held to (SCE_MESH.md section 20.4).
//!
//! The binding talks to its client through [`ZenohClient`], so a client is a choice. This module is
//! what a choice has to pass: functions that run machines through [`open_with`] on whatever
//! client a [`Harness`] connects, against a router the harness points it at, and assert what the
//! machines see. A client in this repository (upstream's `zenoh` crate, see the test target
//! `zenoh_through_a_wz_router`) and a client in another repository run the same functions, so
//! "does the same four operations" is a test and not a claim.
//!
//! A client's own test target calls them from `#[test]` functions:
//!
//! ```text
//! struct MyHarness { port: u16 }
//!
//! impl Harness for MyHarness {
//!     type Client = MyClient;
//!     fn connect(&self) -> MyClient { MyClient::dial(self.port) }
//! }
//!
//! #[test]
//! fn two_machines_meet() {
//!     conformance::two_machines_meet(&MyHarness { port: start_router() });
//! }
//! ```
//!
//! What they hold, through a real router:
//!
//! * [`two_machines_meet`]: two machines that only know each other's names learn each is up, from
//!   the router's presence plane and nothing they were told (SCE_MESH.md section 20.3), and an
//!   envelope one sends reaches the other's engine under the sender's name (sections 20.1 and
//!   20.2).
//! * [`a_machine_that_goes_away_is_told_as_lost`]: a machine whose transport is dropped is told to
//!   the other as a lost link (section 20.3).
//! * [`a_device_that_was_away_catches_up_from_a_hub_that_stayed_up`]: the shape of a shared list
//!   whose devices are on only while their user presses sync, which exercises all of it, with the
//!   same name coming back after it has gone.
//! * [`a_machine_that_loses_its_session_comes_back_and_resyncs`]: the router goes away and comes
//!   back on the same address, and the machines that reconnect (`open_reconnecting`) are told
//!   every peer they had ready is lost, then learn from the presence history who is still there,
//!   so a machine that left during the gap is never believed to be up (section 20.3). It needs a
//!   harness that can end its sessions, [`Disruptable`], and a client that tells the binding when
//!   its session ended ([`ZenohClient::on_session_end`]); a client that cannot is not held to it.
//!
//! The `_across` forms of the first two take a harness for each machine, for a harness that points
//! them at different routers of one federation. Nothing here is a timing claim; every wait has a
//! generous bound and the assertions are on what arrived. The router, and how a harness starts or
//! finds it, are the harness's. The functions name their machines ("server", "phone", "hub",
//! "alice", "bob"), so two of them must not run against one router at the same time, and a
//! harness that wants none of one function's closed sessions seen by the next gives each a router
//! of its own.
//!
//! A failed expectation panics, as a test's does.

use std::collections::BTreeSet;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::{Duration, Instant};

use crate::endpoint::{
    Endpoint, EngineCall, EngineEvent, Environment, Transport, MESH_PROCESSOR_TYPE,
};
use crate::inbound::Delivery;
use crate::inbox::{
    open_with, ClientError, ZenohBinding, ZenohClient, ZenohEvents, ZenohTransport,
};
use crate::link::{deliver, LinkEvent};
use crate::outbound::OutboundBuffer;
use crate::reconnect::{open_reconnecting, Backoff, ClientFactory, ReconnectingTransport};
use crate::router::{PeerConfig, Router};
use sce_rust_runtime::HostSendRequest;

/// What a client is run through: how to get one, for one machine, connected to the router the
/// harness stands for.
pub trait Harness {
    /// The client under test.
    type Client: ZenohClient;

    /// A new client for one machine: one call per machine, each a session of its own. The harness
    /// knows which router it points at, and decides whether a machine needs a session identity of
    /// its own.
    fn connect(&self) -> Self::Client;
}

/// A harness that can lose its router: what the resync test needs beyond [`Harness`].
pub trait Disruptable: Harness {
    /// End every session of this harness's clients as a lost link does. A harness for a real router
    /// kills it and starts it again on the same address, and returns once it answers there again:
    /// the clients made before the call are the ones whose sessions ended, and those made after
    /// (by [`try_connect`](Self::try_connect)) reach the new router.
    fn drop_every_session(&self);

    /// [`connect`](Harness::connect), that can fail: the router is still down. A reconnecting
    /// binding retries it with a backoff.
    fn try_connect(&self) -> Result<Self::Client, ClientError>;
}

/// Makes a reconnecting binding's clients from a harness that can lose its router.
struct Redial<H>(H);

impl<H: Disruptable + Send + 'static> ClientFactory for Redial<H> {
    type Client = H::Client;

    fn connect(&mut self) -> Result<H::Client, ClientError> {
        self.0.try_connect()
    }
}

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

fn peer_config(peer: &'static str) -> PeerConfig {
    PeerConfig {
        transport: "zenoh_inbox",
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

fn endpoint<T: Transport>(
    machine: &str,
    peer: &'static str,
    transport: T,
) -> Endpoint<T, Counting> {
    endpoint_with_peers(machine, &[peer], transport)
}

fn endpoint_with_peers<T: Transport>(
    machine: &str,
    peers: &[&'static str],
    transport: T,
) -> Endpoint<T, Counting> {
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

fn raised<T: Transport>(endpoint: &mut Endpoint<T, Counting>) -> Vec<EngineEvent> {
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
fn pump_until<T: Transport>(
    events: &ZenohEvents,
    endpoint: &mut Endpoint<T, Counting>,
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

/// Two machines, "server" and "phone", each on a client of its own harness, that have seen each
/// other come up and been told so.
struct Meeting<S, P> {
    server_endpoint: Endpoint<ZenohTransport<S>, Counting>,
    server_events: ZenohEvents,
    phone_endpoint: Endpoint<ZenohTransport<P>, Counting>,
    phone_events: ZenohEvents,
    started: Instant,
}

/// Open "server" on `server`'s client and "phone" on `phone`'s, and wait until each has learned
/// the other is up.
fn meet<HS: Harness, HP: Harness>(server: &HS, phone: &HP) -> Meeting<HS::Client, HP::Client> {
    let started = Instant::now();

    let ZenohBinding {
        transport: server_transport,
        events: server_events,
    } = open_with(server.connect(), "server").expect("the server's binding opens");
    let ZenohBinding {
        transport: phone_transport,
        events: phone_events,
    } = open_with(phone.connect(), "phone").expect("the phone's binding opens");

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
    eprintln!("both saw the other up after {:?}", started.elapsed());

    let mut server_endpoint = endpoint("server", "phone", server_transport);
    let mut phone_endpoint = endpoint("phone", "server", phone_transport);
    server_endpoint.peer_ready("phone");
    phone_endpoint.peer_ready("server");

    Meeting {
        server_endpoint,
        server_events,
        phone_endpoint,
        phone_events,
        started,
    }
}

/// A machine "server" and a machine "phone", both on `harness`'s router: each learns the other is
/// up, and an envelope the phone sends reaches the server's engine.
pub fn two_machines_meet(harness: &impl Harness) {
    two_machines_meet_across(harness, harness);
}

/// [`two_machines_meet`] with "server" on `server`'s router and "phone" on `phone`'s (the same
/// router, or two federated): a device connects to whichever router it is assigned and the server
/// to its own, and nothing in the binding says which.
pub fn two_machines_meet_across(server: &impl Harness, phone: &impl Harness) {
    let Meeting {
        mut server_endpoint,
        server_events,
        mut phone_endpoint,
        phone_events: _phone_events,
        started,
    } = meet(server, phone);

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
}

/// Two machines meet, and then the phone goes away: the server is told the link is lost.
pub fn a_machine_that_goes_away_is_told_as_lost(harness: &impl Harness) {
    a_machine_that_goes_away_is_told_as_lost_across(harness, harness);
}

/// [`a_machine_that_goes_away_is_told_as_lost`] with the two machines on `server`'s and `phone`'s
/// routers.
pub fn a_machine_that_goes_away_is_told_as_lost_across(
    server: &impl Harness,
    phone: &impl Harness,
) {
    let Meeting {
        server_endpoint: _server_endpoint,
        server_events,
        phone_endpoint,
        phone_events,
        started,
    } = meet(server, phone);

    // The phone goes away: its endpoint owns the transport that holds the presence token, so
    // dropping both closes the session, and the server is told the link is lost. The server's
    // endpoint is kept, for it owns the transport that holds the server's own session.
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
struct Hub<T> {
    endpoint: Endpoint<T, Counting>,
    events: ZenohEvents,
    notes: Vec<String>,
    /// The machines it was told are gone, in the order it was told.
    lost: Vec<String>,
}

impl<T: Transport> Hub<T> {
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
                // How the core tells its engine a machine went away (SCE_MESH.md section 20.3):
                // the link is lost, and a send to it would find no transport. It names a machine
                // this hub was just told is gone, and nothing else is held against it.
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

/// One visit of a device: it comes up as `name` on a client from `harness`, sends `requests` to the
/// hub, and stays until `done` holds of what it raised, then goes and the hub is told it is gone.
/// The requests are sent at once, before the hub is known to be up, and wait in the device's buffer
/// for it.
fn visit<H: Harness, T: Transport>(
    harness: &H,
    hub: &mut Hub<T>,
    name: &'static str,
    requests: &[(&str, String)],
    done: impl Fn(&Hub<T>, &[EngineEvent]) -> bool,
) -> Vec<EngineEvent> {
    let ZenohBinding { transport, events } =
        open_with(harness.connect(), name).expect("the device's binding opens");
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
pub fn a_device_that_was_away_catches_up_from_a_hub_that_stayed_up(harness: &impl Harness) {
    let ZenohBinding { transport, events } =
        open_with(harness.connect(), "hub").expect("the hub's binding opens");
    let mut hub = Hub {
        endpoint: endpoint_with_peers("hub", &["alice", "bob"], transport),
        events,
        notes: Vec::new(),
        lost: Vec::new(),
    };

    visit(
        harness,
        &mut hub,
        "alice",
        &note_pushed("milk"),
        |hub, _| hub.notes == ["milk"],
    );
    visit(
        harness,
        &mut hub,
        "alice",
        &note_pushed("eggs"),
        |hub, _| hub.notes == ["milk", "eggs"],
    );
    assert_eq!(hub.lost, ["alice", "alice"], "both visits ended");

    let first = visit(
        harness,
        &mut hub,
        "bob",
        &sync_since("bob", 0),
        |_, seen| reply_data(seen).is_some(),
    );
    assert_eq!(
        reply_data(&first).as_deref(),
        Some(r#"{"notes":["milk","eggs"],"seen":2}"#),
        "bob, who was away for both pushes, is given both"
    );

    visit(
        harness,
        &mut hub,
        "alice",
        &note_pushed("bread"),
        |hub, _| hub.notes.len() == 3,
    );
    let second = visit(
        harness,
        &mut hub,
        "bob",
        &sync_since("bob", 2),
        |_, seen| reply_data(seen).is_some(),
    );
    assert_eq!(
        reply_data(&second).as_deref(),
        Some(r#"{"notes":["bread"],"seen":3}"#),
        "bob's second visit is given only what is new since the two he saw"
    );
}

/// A machine on a reconnecting binding, with the core it feeds, and what it was told and raised.
struct Watched {
    endpoint: Endpoint<ReconnectingTransport, Counting>,
    events: ZenohEvents,
    /// Every event the binding gave, in order.
    log: Vec<LinkEvent>,
    raised: Vec<EngineEvent>,
}

impl Watched {
    /// Open `name` through `open_reconnecting` on clients from `harness`, with a core that knows
    /// `peers`.
    fn open(
        harness: &(impl Disruptable + Clone + Send + 'static),
        name: &str,
        peers: &[&'static str],
        backoff: Backoff,
    ) -> Self {
        let binding = match open_reconnecting(Redial(harness.clone()), name, backoff) {
            Ok(binding) => binding,
            Err(why) => panic!("{name}'s reconnecting binding did not open: {why}"),
        };
        Watched {
            endpoint: endpoint_with_peers(name, peers, binding.transport),
            events: binding.events,
            log: Vec::new(),
            raised: Vec::new(),
        }
    }

    /// One turn: take what has arrived, waiting up to `wait` for the first, hand each event to the
    /// core as a host would and keep it, and let time pass for the core.
    fn step(&mut self, wait: Duration) {
        let mut arrived = Vec::new();
        if let Some(event) = self.events.next_within(wait) {
            arrived.push(event);
        }
        while let Some(event) = self.events.try_next() {
            arrived.push(event);
        }
        for event in arrived {
            self.log.push(event.clone());
            deliver(&mut self.endpoint, event);
        }
        self.endpoint.tick();
        self.raised.extend(raised(&mut self.endpoint));
    }

    /// The peers the log leaves ready: made ready by a `Ready` and not lost since.
    fn ready(&self) -> BTreeSet<&str> {
        let mut ready = BTreeSet::new();
        for event in &self.log {
            match event {
                LinkEvent::Ready(peer) => {
                    ready.insert(peer.as_str());
                }
                LinkEvent::Lost(peer, _) => {
                    ready.remove(peer.as_str());
                }
                _ => {}
            }
        }
        ready
    }

    /// The first thing the log says about `peer` from position `mark` on.
    fn first_since(&self, mark: usize, peer: &str) -> Option<&LinkEvent> {
        self.log[mark..].iter().find(|event| match event {
            LinkEvent::Ready(named)
            | LinkEvent::Lost(named, _)
            | LinkEvent::Received(named, _)
            | LinkEvent::Unsent(named, _) => named == peer,
        })
    }

    /// Whether the log from `mark` on says `peer` is ready.
    fn announced_since(&self, mark: usize, peer: &str) -> bool {
        self.log[mark..]
            .iter()
            .any(|event| matches!(event, LinkEvent::Ready(named) if named == peer))
    }
}

/// Run both machines until `done` holds of them, or `within` passes. Returns whether it did.
fn until(
    first: &mut Watched,
    second: &mut Watched,
    within: Duration,
    done: impl Fn(&Watched, &Watched) -> bool,
) -> bool {
    let deadline = Instant::now() + within;
    loop {
        if done(first, second) {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        first.step(Duration::from_millis(10));
        second.step(Duration::from_millis(10));
    }
}

fn names<const N: usize>(list: [&'static str; N]) -> BTreeSet<&'static str> {
    list.into_iter().collect()
}

/// The router goes away and comes back on the same address, and the machines that reconnect come
/// back with it, knowing only who is still there.
///
/// "hub" and "alice" are on `open_reconnecting` and "carol" is on the plain binding, all through
/// the harness's router, and all three see the others up. Then every session ends. Hub and alice
/// are each told every peer they had ready is lost, carol included, before anything else about
/// that peer. Once the router is back they see each other up again, from the presence history of
/// their new sessions, and nobody sees carol up again: carol's session ended with the rest, her
/// plain binding does not reconnect, and a client that told its binding the session ended does not
/// carry on by itself, so she was gone during the gap and never returned. That is the departure
/// nobody was told of, which a client that only replays what exists now would leave believed
/// ready for good. Last, an envelope alice sends reaches the hub's engine on the new sessions.
pub fn a_machine_that_loses_its_session_comes_back_and_resyncs(
    harness: &(impl Disruptable + Clone + Send + 'static),
) {
    let backoff = Backoff {
        initial: Duration::from_millis(50),
        max: Duration::from_millis(500),
        factor: 2,
        jitter: false,
    };
    let mut hub = Watched::open(harness, "hub", &["alice", "carol"], backoff);
    let mut alice = Watched::open(harness, "alice", &["hub", "carol"], backoff);
    let ZenohBinding {
        transport: carol_transport,
        events: carol_events,
    } = open_with(harness.connect(), "carol").expect("carol's binding opens");

    assert!(
        until(
            &mut hub,
            &mut alice,
            Duration::from_secs(30),
            |hub, alice| {
                hub.ready() == names(["alice", "carol"]) && alice.ready() == names(["hub", "carol"])
            }
        ),
        "hub and alice each saw the other two up: hub {:?}, alice {:?}",
        hub.ready(),
        alice.ready()
    );
    let mut carol_log: Vec<LinkEvent> = Vec::new();
    let both_up = |log: &[LinkEvent]| {
        ["hub", "alice"]
            .iter()
            .all(|peer| log.contains(&LinkEvent::Ready(peer.to_string())))
    };
    let deadline = Instant::now() + Duration::from_secs(30);
    while !both_up(&carol_log) && Instant::now() < deadline {
        carol_log.extend(carol_events.next_within(Duration::from_millis(20)));
    }
    assert!(both_up(&carol_log), "carol saw hub and alice up");
    let hub_mark = hub.log.len();
    let alice_mark = alice.log.len();

    harness.drop_every_session();

    // Carol's own binding is told her session ended, and her peers are lost to her.
    let deadline = Instant::now() + Duration::from_secs(30);
    while carol_events.session_ended().is_none() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(
        carol_events.session_ended().is_some(),
        "carol's binding was told her session ended"
    );
    carol_log.extend(std::iter::from_fn(|| carol_events.try_next()));
    let carol_lost: BTreeSet<String> = carol_log
        .into_iter()
        .filter_map(|event| match event {
            LinkEvent::Lost(peer, _) => Some(peer),
            _ => None,
        })
        .collect();
    assert_eq!(
        carol_lost,
        BTreeSet::from(["alice".to_string(), "hub".to_string()]),
        "carol was told the two she had ready are lost"
    );

    // Hub and alice are told every peer they had ready is lost, before anything else about it.
    assert!(
        until(
            &mut hub,
            &mut alice,
            Duration::from_secs(60),
            |hub, alice| {
                ["alice", "carol"]
                    .iter()
                    .all(|peer| hub.first_since(hub_mark, peer).is_some())
                    && ["hub", "carol"]
                        .iter()
                        .all(|peer| alice.first_since(alice_mark, peer).is_some())
            }
        ),
        "hub and alice were told something about every peer after the router went"
    );
    for peer in ["alice", "carol"] {
        assert!(
            matches!(hub.first_since(hub_mark, peer), Some(LinkEvent::Lost(..))),
            "the first thing hub was told of {peer} is that it is lost: {:?}",
            hub.first_since(hub_mark, peer)
        );
    }
    for peer in ["hub", "carol"] {
        assert!(
            matches!(
                alice.first_since(alice_mark, peer),
                Some(LinkEvent::Lost(..))
            ),
            "the first thing alice was told of {peer} is that it is lost: {:?}",
            alice.first_since(alice_mark, peer)
        );
    }

    // The router is back: hub and alice find each other from the presence history.
    assert!(
        until(
            &mut hub,
            &mut alice,
            Duration::from_secs(90),
            |hub, alice| { hub.ready().contains("alice") && alice.ready().contains("hub") }
        ),
        "hub and alice saw each other up again: hub {:?}, alice {:?}",
        hub.ready(),
        alice.ready()
    );
    // Let whatever else the router replays arrive; none of it may be carol.
    let _ = until(&mut hub, &mut alice, Duration::from_secs(2), |_, _| false);
    assert_eq!(hub.ready(), names(["alice"]), "hub is not told carol is up");
    assert_eq!(
        alice.ready(),
        names(["hub"]),
        "alice is not told carol is up"
    );
    assert!(
        !hub.announced_since(hub_mark, "carol") && !alice.announced_since(alice_mark, "carol"),
        "carol was never announced again"
    );

    // And the new sessions carry an envelope.
    alice.endpoint.send(&send("note.added", "hub"));
    assert!(
        until(&mut hub, &mut alice, Duration::from_secs(60), |hub, _| {
            hub.raised.iter().any(|event| event.name == "note.added")
        }),
        "the envelope alice sent after the router came back reached the hub's engine"
    );
    drop(carol_transport);
    drop(carol_events);
}
