// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! One copy of the shared to-do list (`todo_peer`) as a process of its own, on real
//! sockets: the same generated machine the simulated world runs, served by a Mesh
//! endpoint over the WebSocket binding (SCE_MESH.md §mesh-18) instead of the world's
//! network, its route `#people` a causal group (`delivery: causal`, docs/adr/0085 section
//! 3.2). A WebSocket has no topic, so the group is realised by a relay node that forwards
//! every envelope one member sends to every other member, as it is, tags and all; the
//! relay runs no machine and reads nothing. A test starts a relay and three copies and
//! holds what they agree on to what the simulation says.
//!
//! ```text
//! todo_peer_wss relay <listen-address>                 the relay: prints `LISTENING <port>`
//! todo_peer_wss <alice|bob|carol> <node> <ws-url>      a copy: dials the relay as `#people`,
//!                                                      again after a loss
//! ```
//!
//! A copy is driven by lines on its standard input, each a JSON object, and says what it
//! did on its standard output, one line each:
//!
//! ```text
//! {"raise": "<event>", "data": "<the event's JSON>"}   the host gives the machine an event
//! {"saved": true}                                      answered `SAVED <the machine's saved state>`
//! {"layer": true}                                      answered `LAYER <log> <held>`: the changes its
//!                                                      causal layer keeps not yet stable, and holds
//! {"ping": <n>}                                        answered `PONG <n>`, once what is queued is taken
//! ```
//!
//! and `LINK <peer>` when a link to a peer is ready, `LOST <peer>` when it ends. The
//! machine is stepped by this process alone; what it sends a peer leaves through the
//! endpoint and what a peer sends it arrives as the endpoint's calls, which are applied
//! after the step that asked, as the world applies them. The host commits each change the
//! causal layer accepts at once, keeping no journal: a copy that is started again is a new
//! incarnation (docs/adr/0085, section 3.3). The relay answers `ping` alone.

use std::collections::BTreeSet;
use std::io::BufRead;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use sce_rust_mesh::causal::{CausalGroup, Replica, Start};
use sce_rust_mesh::endpoint::{
    self, CausalVerdict, Endpoint, Environment, SharedEndpoint, Transport,
};
use sce_rust_mesh::inbound::Delivery;
use sce_rust_mesh::router::{PeerConfig, Router};
use sce_rust_mesh::wss::{self, LinkEvent, WssTransport, DEFAULT_KEEPALIVE};
use sce_rust_runtime::Engine;
use serde_json::Value;
use tokio::net::TcpListener;
use tokio::sync::mpsc;

include!(concat!(env!("OUT_DIR"), "/machines.rs"));

use todo_peer::{TodoPeerPersist, TodoPeerPolicy};

/// The pump's bound: a machine that answers itself without end is a bug in the document,
/// not something to wait out.
const PUMP_LIMIT: usize = 10_000;

/// How often the endpoint is asked whether an envelope is due to be sent again, or a vector.
const TICK: Duration = Duration::from_millis(25);

/// How long a copy waits before it dials the relay again.
const REDIAL: Duration = Duration::from_millis(100);

/// The route every copy sends to and reads.
const PEOPLE: &str = "people";

/// The copies of the list, each a person's machine on a device of its own.
const COPIES: [&str; 3] = ["alice", "bob", "carol"];

/// The group of the copies, and the view of its epoch 0 (`<device>/<machine>`).
const TRIO: CausalGroup = CausalGroup {
    name: PEOPLE,
    epoch: 0,
    view: &["phone/alice", "laptop/bob", "tablet/carol"],
};

/// How often a causal layer sends its vector, in milliseconds.
const VECTOR_EVERY_MS: i64 = 200;

/// The most changes a causal layer keeps not yet stable.
const LAYER_ROOM: usize = 100_000;

/// The process's time and randomness: a monotonic clock from its start, the system's
/// clock for the envelope's stamp, and ids drawn from a counter mixed with both.
struct RealEnvironment {
    started: Instant,
    counter: u64,
}

impl RealEnvironment {
    fn unix_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |since| {
                u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
            })
    }

    /// SplitMix64: ids need only differ, and differ between processes.
    fn draw(&mut self) -> u64 {
        self.counter = self.counter.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self
            .counter
            .wrapping_add(u64::from(std::process::id()) << 32)
            .wrapping_add(Self::unix_ms());
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

impl Environment for RealEnvironment {
    fn now_ms(&mut self) -> i64 {
        i64::try_from(self.started.elapsed().as_millis()).unwrap_or(i64::MAX)
    }

    fn now_unix_ms(&mut self) -> u64 {
        Self::unix_ms()
    }

    /// A UUID v7 (§mesh-7.5): the millisecond clock, then drawn bits.
    fn envelope_id(&mut self) -> [u8; 16] {
        let ms = Self::unix_ms();
        let (a, b) = (self.draw(), self.draw());
        let mut id = [0u8; 16];
        id[..6].copy_from_slice(&ms.to_be_bytes()[2..]);
        id[6..14].copy_from_slice(&a.to_be_bytes());
        id[14..].copy_from_slice(&b.to_be_bytes()[..2]);
        id[6] = 0x70 | (id[6] & 0x0F);
        id[8] = 0x80 | (id[8] & 0x3F);
        id
    }

    fn jitter_draw(&mut self) -> i64 {
        0
    }
}

/// The binding of a copy to its group: `delivery: causal`, with no buffer and no retry,
/// since a change a link lost is repaired by the layer and not sent again by the binding.
fn people() -> PeerConfig {
    PeerConfig {
        transport: "wss",
        buffer: None,
        retry: None,
        stamp_sequence: false,
        delivery: Delivery {
            dedup: false,
            ordered: false,
        },
        responders: &[PEOPLE],
        deadline_ms: None,
        reply_events: &[],
        causal: Some(TRIO),
    }
}

/// The replica identity of this incarnation of the copy `name`: a UUID v7 drawn as it
/// starts, which no other incarnation shares.
fn new_replica(environment: &mut RealEnvironment) -> Replica {
    Replica(environment.envelope_id())
}

type Shared = SharedEndpoint<WssTransport, RealEnvironment>;

/// Let the machine take what its endpoint has for it, one call at a time and each to
/// rest before the next, until neither has more; each change the causal layer accepted
/// on the way is committed, which lets it leave.
fn pump(engine: &mut Engine<TodoPeerPolicy>, shared: &Shared) {
    for _ in 0..PUMP_LIMIT {
        engine.step();
        let mut endpoint = shared
            .lock()
            .expect("a Mesh endpoint is never left mid-update");
        for verdict in endpoint.take_verdicts() {
            match verdict {
                CausalVerdict::Accepted { peer, dot } => endpoint.committed(&peer, dot),
                CausalVerdict::Refused { peer, refusal } => {
                    eprintln!("the causal layer of {peer} refused a change: {refusal:?}");
                }
            }
        }
        let calls = endpoint.take_calls();
        if calls.is_empty() {
            for error in endpoint.take_host_errors() {
                eprintln!("host error: {error:?}");
            }
            return;
        }
        drop(endpoint);
        endpoint::apply_to(engine, calls);
    }
    panic!("the copy did not come to rest after {PUMP_LIMIT} rounds");
}

fn saved(engine: &Engine<TodoPeerPolicy>) -> String {
    let text = engine
        .save_at(0)
        .expect("a running machine saves")
        .to_json();
    // One line, whatever the machine's own layout.
    let value: Value = serde_json::from_str(&text).expect("a saved state is JSON");
    value.to_string()
}

/// The host's lines, read on a thread of their own: reading stdin blocks.
fn commands() -> mpsc::UnboundedReceiver<String> {
    let (command_tx, commands) = mpsc::unbounded_channel::<String>();
    std::thread::spawn(move || {
        for line in std::io::stdin().lock().lines().map_while(Result::ok) {
            if command_tx.send(line).is_err() {
                break;
            }
        }
    });
    commands
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.as_slice() {
        [_, relay, address] if relay == "relay" => run_relay(address).await,
        [_, name, node, address] if COPIES.contains(&name.as_str()) => {
            run_copy(name, node, address).await;
        }
        _ => {
            eprintln!(
                "usage: todo_peer_wss relay <listen-address> | todo_peer_wss <alice|bob|carol> <node> <ws-url>"
            );
            std::process::exit(2);
        }
    }
}

/// The relay node of the group: every envelope a member sends it goes to every other member
/// whose link is up, as it came, and nowhere else. A member whose link is down misses it,
/// and the layers repair that.
async fn run_relay(address: &str) {
    let transport = WssTransport::new();
    let (link_tx, mut links) = mpsc::unbounded_channel::<LinkEvent>();
    let listener = TcpListener::bind(address)
        .await
        .expect("an address to listen on");
    println!(
        "LISTENING {}",
        listener.local_addr().expect("a bound address").port()
    );
    {
        let transport = transport.clone();
        let link_tx = link_tx.clone();
        tokio::spawn(async move {
            loop {
                let Ok((stream, _)) = listener.accept().await else {
                    continue;
                };
                let (transport, link_tx) = (transport.clone(), link_tx.clone());
                tokio::spawn(async move {
                    let _ = wss::accept(
                        stream,
                        |peer| COPIES.contains(&peer),
                        transport,
                        link_tx,
                        DEFAULT_KEEPALIVE,
                    )
                    .await;
                });
            }
        });
    }

    let mut commands = commands();
    let mut members: BTreeSet<String> = BTreeSet::new();
    let mut transport = transport;
    loop {
        tokio::select! {
            Some(event) = links.recv() => match event {
                LinkEvent::Ready(peer) => {
                    println!("LINK {peer}");
                    members.insert(peer);
                }
                LinkEvent::Lost(peer, _) => {
                    println!("LOST {peer}");
                    members.remove(&peer);
                }
                LinkEvent::Received(from, bytes) => {
                    for other in members.iter().filter(|member| **member != from) {
                        // A member whose link closed in between misses it, as one whose
                        // link is down does.
                        let _ = transport.transmit(other, &bytes);
                    }
                }
                LinkEvent::Unsent(_, _) => {}
            },
            line = commands.recv() => {
                let Some(line) = line else { break };
                match serde_json::from_str::<Value>(&line) {
                    Ok(command) => match command["ping"].as_u64() {
                        Some(number) => println!("PONG {number}"),
                        None => println!("ERR the relay runs no machine: {line}"),
                    },
                    Err(_) => println!("ERR not JSON: {line}"),
                }
            }
        }
    }
}

/// A copy: its machine, its endpoint bound to the group, and its causal layer started as a
/// new incarnation, dialing the relay and again after every loss.
async fn run_copy(name: &str, node: &str, address: &str) {
    let transport = WssTransport::new();
    let mut environment = RealEnvironment {
        started: Instant::now(),
        counter: 0,
    };
    let replica = new_replica(&mut environment);
    let mut router = Router::new(name, 256, 5_000).expect("a window the core can hold");
    router.add_peer(PEOPLE, people());
    let shared: Shared = Arc::new(Mutex::new(Endpoint::new(
        router,
        transport.clone(),
        environment,
    )));
    shared
        .lock()
        .expect("a Mesh endpoint is never left mid-update")
        .start_causal(
            PEOPLE,
            Start::NewIncarnation(replica),
            LAYER_ROOM,
            VECTOR_EVERY_MS,
        )
        .expect("the group's binding is causal");
    let mut engine = Engine::new(TodoPeerPolicy::new());
    endpoint::register(&mut engine, &shared);
    engine.initialize();
    engine.raise_external_by_name("todo.join", &format!(r#"{{"node":"{node}"}}"#));
    pump(&mut engine, &shared);

    let (link_tx, mut links) = mpsc::unbounded_channel::<LinkEvent>();
    {
        let (base, own) = (address.to_string(), name.to_string());
        let transport = transport.clone();
        tokio::spawn(async move {
            loop {
                let _ = wss::connect(
                    &base,
                    &own,
                    PEOPLE,
                    transport.clone(),
                    link_tx.clone(),
                    DEFAULT_KEEPALIVE,
                )
                .await;
                tokio::time::sleep(REDIAL).await;
            }
        });
    }

    let mut commands = commands();
    let mut ticker = tokio::time::interval(TICK);
    loop {
        tokio::select! {
            Some(event) = links.recv() => {
                match &event {
                    LinkEvent::Ready(peer) => println!("LINK {peer}"),
                    LinkEvent::Lost(peer, _) => println!("LOST {peer}"),
                    _ => {}
                }
                wss::deliver(
                    &mut shared.lock().expect("a Mesh endpoint is never left mid-update"),
                    event,
                );
                pump(&mut engine, &shared);
            }
            _ = ticker.tick() => {
                shared.lock().expect("a Mesh endpoint is never left mid-update").tick();
                pump(&mut engine, &shared);
            }
            line = commands.recv() => {
                let Some(line) = line else { break };
                let Ok(command) = serde_json::from_str::<Value>(&line) else {
                    println!("ERR not JSON: {line}");
                    continue;
                };
                if let Some(event) = command["raise"].as_str() {
                    let data = command["data"].as_str().unwrap_or("");
                    engine.raise_external_by_name(event, data);
                    pump(&mut engine, &shared);
                } else if command["saved"].as_bool() == Some(true) {
                    println!("SAVED {}", saved(&engine));
                } else if command["layer"].as_bool() == Some(true) {
                    let endpoint = shared.lock().expect("a Mesh endpoint is never left mid-update");
                    let layer = endpoint.causal_layer(PEOPLE).expect("a started layer");
                    println!("LAYER {} {}", layer.log_len(), layer.held_len());
                } else if let Some(number) = command["ping"].as_u64() {
                    pump(&mut engine, &shared);
                    println!("PONG {number}");
                } else {
                    println!("ERR unknown command: {line}");
                }
            }
        }
    }
}
