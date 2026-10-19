// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What a Mesh envelope costs to cross a Zenoh network: a round trip from a device to the server
//! machine and back, through whatever routers lie between them.
//!
//! ```text
//! zenoh_probe server --router tcp/HOST:PORT [--name server] [--report-secs 10]
//! zenoh_probe client --router tcp/HOST:PORT --name dev-1 [--server server]
//!                    [--count 200] [--interval-ms 50] [--pad 64] [--wait-secs 30]
//! ```
//!
//! The server binds a device when it first hears of it (`Endpoint::add_peer`, which is what a shared
//! list's server does for a person who joins) and answers each `ping` with a `pong` carrying the same
//! data. The client sends `count` pings, one every `interval-ms`, and reports how many came back and
//! the round-trip time at the 50th, 90th and 99th percentile and the worst. Both ends are the Mesh
//! core over the Zenoh binding, so what is measured includes the core's own work, not only the wire.
//!
//! Run the two ends on different hosts and the routers between them are the network under test.

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use sce_rust_mesh::endpoint::{
    Endpoint, EngineCall, EngineEvent, Environment, MESH_ORIGIN_SCHEME, MESH_PROCESSOR_TYPE,
};
use sce_rust_mesh::inbound::Delivery;
use sce_rust_mesh::link::{deliver, LinkEvent};
use sce_rust_mesh::outbound::OutboundBuffer;
use sce_rust_mesh::router::{PeerConfig, Router};
use sce_rust_mesh::zenoh::{client_config, open, ZenohBinding, ZenohEvents, ZenohTransport};
use sce_rust_runtime::HostSendRequest;

/// A real clock, and envelope ids that cannot collide across machines: a salt drawn from the
/// start time, then a counter.
struct Clock {
    start: Instant,
    salt: u64,
    next: u64,
}

impl Clock {
    fn new() -> Self {
        let salt = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(1, |d| u64::try_from(d.as_nanos()).unwrap_or(1));
        Self {
            start: Instant::now(),
            salt,
            next: 0,
        }
    }
}

impl Environment for Clock {
    fn now_ms(&mut self) -> i64 {
        i64::try_from(self.start.elapsed().as_millis()).unwrap_or(i64::MAX)
    }
    fn now_unix_ms(&mut self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
    }
    fn envelope_id(&mut self) -> [u8; 16] {
        self.next += 1;
        let mut id = [0u8; 16];
        id[..8].copy_from_slice(&self.salt.to_be_bytes());
        id[8..].copy_from_slice(&self.next.to_be_bytes());
        id
    }
    fn jitter_draw(&mut self) -> i64 {
        0
    }
}

type Core = Endpoint<ZenohTransport, Clock>;

/// A peer that answers to itself alone, buffered until its presence is seen. The name is leaked
/// because `PeerConfig` carries `&'static` data, which a name learned at run time cannot be: one
/// small allocation per device the process ever hears of.
fn peer_config(peer: &str) -> PeerConfig {
    let name: &'static str = Box::leak(peer.to_string().into_boxed_str());
    PeerConfig {
        transport: "zenoh_inbox",
        buffer: Some(OutboundBuffer {
            max_pending: 64,
            max_age_ms: 0,
        }),
        retry: None,
        stamp_sequence: false,
        delivery: Delivery {
            dedup: true,
            ordered: false,
        },
        responders: Box::leak(Box::new([name])),
        deadline_ms: None,
        reply_events: &[],
    }
}

fn core(machine: &str, transport: ZenohTransport) -> Core {
    let router = Router::new(machine, 64, 50).expect("a router");
    Endpoint::new(router, transport, Clock::new())
}

fn arg(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

fn number(args: &[String], name: &str, default: u64) -> u64 {
    arg(args, name).map_or(default, |v| {
        v.parse()
            .unwrap_or_else(|_| panic!("{name} wants a number, got {v:?}"))
    })
}

fn raised(core: &mut Core) -> Vec<EngineEvent> {
    core.take_calls()
        .into_iter()
        .filter_map(|call| match call {
            EngineCall::Raise(event) => Some(event),
            _ => None,
        })
        .collect()
}

fn send(event: &str, target: &str, send_id: &str, data: &str) -> HostSendRequest {
    HostSendRequest {
        processor_type: MESH_PROCESSOR_TYPE.to_string(),
        event_name: event.to_string(),
        target: format!("#{target}"),
        send_id: send_id.to_string(),
        event_data: data.to_string(),
        ..HostSendRequest::default()
    }
}

/// The peer an event names, whatever kind it is.
fn peer_of(event: &LinkEvent) -> &str {
    match event {
        LinkEvent::Ready(p)
        | LinkEvent::Received(p, _)
        | LinkEvent::Unsent(p, _)
        | LinkEvent::Lost(p, _) => p,
    }
}

fn open_binding(args: &[String], name: &str) -> (ZenohTransport, ZenohEvents) {
    let router = arg(args, "--router").expect("--router tcp/HOST:PORT");
    let config = client_config(&[router.as_str()]).expect("a client config");
    let ZenohBinding { transport, events } = open(config, name).expect("the binding opens");
    (transport, events)
}

fn server(args: &[String]) {
    let name = arg(args, "--name").unwrap_or_else(|| "server".to_string());
    let report = Duration::from_secs(number(args, "--report-secs", 10));
    let (transport, events) = open_binding(args, &name);
    let mut core = core(&name, transport);
    eprintln!("server {name} is up");

    let (mut pings, mut pongs, mut devices) = (0u64, 0u64, 0u64);
    let mut next_report = Instant::now() + report;
    let mut reply_id = 0u64;
    loop {
        if let Some(event) = events.next_within(Duration::from_millis(20)) {
            let mut batch = vec![event];
            while let Some(more) = events.try_next() {
                batch.push(more);
            }
            for event in batch {
                let peer = peer_of(&event).to_string();
                // A peer is bound when it is first heard of, by its presence or by what it sent,
                // whichever the network delivers first.
                if !core.has_peer(&peer) && !matches!(event, LinkEvent::Lost(..)) {
                    core.add_peer(&peer, peer_config(&peer));
                    devices += 1;
                }
                let gone = matches!(event, LinkEvent::Lost(..));
                deliver(&mut core, event);
                if gone {
                    core.remove_peer(&peer);
                }
            }
        }
        core.tick();
        for event in raised(&mut core) {
            if event.name != "ping" {
                continue;
            }
            pings += 1;
            let origin = event.metadata.origin.to_string();
            let Some(source) = origin.strip_prefix(MESH_ORIGIN_SCHEME) else {
                continue;
            };
            reply_id += 1;
            core.send(&send(
                "pong",
                source,
                &format!("r-{reply_id}"),
                &event.metadata.data.to_string(),
            ));
            pongs += 1;
        }
        if Instant::now() >= next_report {
            eprintln!("server {name}: {devices} devices seen, {pings} pings, {pongs} pongs");
            next_report = Instant::now() + report;
        }
    }
}

fn client(args: &[String]) {
    let name = arg(args, "--name").expect("--name <machine name>");
    let server = arg(args, "--server").unwrap_or_else(|| "server".to_string());
    let count = number(args, "--count", 200) as usize;
    let interval = Duration::from_millis(number(args, "--interval-ms", 50));
    let pad = number(args, "--pad", 64) as usize;
    let wait = Duration::from_secs(number(args, "--wait-secs", 30));
    let (transport, events) = open_binding(args, &name);
    let mut core = core(&name, transport);
    core.add_peer(&server, peer_config(&server));

    // Wait for the server's presence: nothing is sent before the network says it is there.
    let deadline = Instant::now() + wait;
    let mut ready = false;
    while Instant::now() < deadline && !ready {
        if let Some(event) = events.next_within(Duration::from_millis(100)) {
            ready = event == LinkEvent::Ready(server.clone());
            deliver(&mut core, event);
        }
    }
    assert!(ready, "the server {server} did not appear within {wait:?}");
    eprintln!("client {name}: {server} is up, sending {count} pings");

    let filler = "x".repeat(pad);
    let mut sent_at: Vec<Option<Instant>> = vec![None; count];
    let mut round_trips: Vec<Duration> = Vec::with_capacity(count);
    let mut next_send = Instant::now();
    let mut sent = 0usize;
    let mut last_send_done: Option<Instant> = None;
    loop {
        if sent < count && Instant::now() >= next_send {
            let data = format!(r#"{{"seq":{sent},"pad":"{filler}"}}"#);
            sent_at[sent] = Some(Instant::now());
            core.send(&send("ping", &server, &format!("p-{sent}"), &data));
            sent += 1;
            next_send += interval;
            if sent == count {
                last_send_done = Some(Instant::now());
            }
        }
        if let Some(event) = events.next_within(Duration::from_millis(1)) {
            deliver(&mut core, event);
        }
        core.tick();
        for event in raised(&mut core) {
            if event.name != "pong" {
                continue;
            }
            let data = event.metadata.data.to_string();
            let seq = data
                .split("\"seq\":")
                .nth(1)
                .and_then(|rest| {
                    rest.chars()
                        .take_while(char::is_ascii_digit)
                        .collect::<String>()
                        .parse::<usize>()
                        .ok()
                })
                .filter(|seq| *seq < count);
            if let Some(at) = seq.and_then(|seq| sent_at[seq]) {
                round_trips.push(at.elapsed());
            }
        }
        // Done when everything came back, or the stragglers have had three seconds.
        let all_back = round_trips.len() >= count;
        let grace_over = last_send_done.is_some_and(|t| t.elapsed() > Duration::from_secs(3));
        if all_back || grace_over {
            break;
        }
    }

    round_trips.sort();
    let at = |p: f64| -> f64 {
        if round_trips.is_empty() {
            return f64::NAN;
        }
        let i = ((round_trips.len() as f64 - 1.0) * p).round() as usize;
        round_trips[i].as_secs_f64() * 1000.0
    };
    println!(
        "client {name}: sent {count}, back {} ({:.1}% lost); round trip ms p50 {:.2} p90 {:.2} p99 {:.2} max {:.2}",
        round_trips.len(),
        100.0 * (count - round_trips.len()) as f64 / count as f64,
        at(0.50),
        at(0.90),
        at(0.99),
        at(1.0),
    );
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("server") => server(&args),
        Some("client") => client(&args),
        _ => {
            eprintln!(
                "usage: zenoh_probe server --router tcp/HOST:PORT [--name N] [--report-secs S]\n       \
                 zenoh_probe client --router tcp/HOST:PORT --name N [--server S] [--count C] \
                 [--interval-ms MS] [--pad BYTES] [--wait-secs S]"
            );
            std::process::exit(2);
        }
    }
}
