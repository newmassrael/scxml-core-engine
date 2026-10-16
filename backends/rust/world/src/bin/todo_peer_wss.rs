// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! One copy of the shared to-do list (`todo_peer`) as a process of its own, on real
//! sockets: the same generated machine the simulated world runs, served by a Mesh
//! endpoint over the WebSocket binding (SCE_MESH.md §mesh-18) instead of the world's
//! network. A test starts three of them and holds what they agree on to what the
//! simulation says.
//!
//! ```text
//! todo_peer_wss server <node> <listen-address>        the hub: prints `LISTENING <port>`
//! todo_peer_wss <alice|bob> <node> <ws-url>           a person: dials the hub, again after a loss
//! ```
//!
//! The copy is driven by lines on its standard input, each a JSON object, and says what
//! it did on its standard output, one line each:
//!
//! ```text
//! {"raise": "<event>", "data": "<the event's JSON>"}   the host gives the machine an event
//! {"saved": true}                                      answered `SAVED <the machine's saved state>`
//! {"ping": <n>}                                        answered `PONG <n>`, once what is queued is taken
//! ```
//!
//! and `LINK <peer>` when a link to a peer is ready, `LOST <peer>` when it ends. The
//! machine is stepped by this process alone; what it sends a peer leaves through the
//! endpoint and what a peer sends it arrives as the endpoint's calls, which are applied
//! after the step that asked, as the world applies them.

use std::io::BufRead;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use sce_rust_mesh::endpoint::{self, Endpoint, Environment, SharedEndpoint};
use sce_rust_mesh::inbound::Delivery;
use sce_rust_mesh::outbound::{OutboundBuffer, RetryPolicy};
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

/// How often the endpoint is asked whether an envelope is due to be sent again.
const TICK: Duration = Duration::from_millis(25);

/// How long a person waits before it dials the hub again.
const REDIAL: Duration = Duration::from_millis(100);

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

/// How a peer is bound: envelopes delivered once, those written while the link is down
/// kept until it is back, and not ordered, as in the simulated world.
fn peer(responders: &'static [&'static str]) -> PeerConfig {
    PeerConfig {
        transport: "wss",
        buffer: Some(OutboundBuffer {
            max_pending: 1_024,
            max_age_ms: 0,
        }),
        retry: Some(RetryPolicy {
            max_retries: 50,
            initial_backoff_ms: 100,
            backoff_multiplier: 2.0,
            max_backoff_ms: 1_000,
            jitter_pct: 0,
        }),
        stamp_sequence: false,
        delivery: Delivery {
            dedup: true,
            ordered: false,
        },
        responders,
        deadline_ms: None,
        reply_events: &[],
    }
}

fn router_for(name: &str) -> Router {
    let mut router = Router::new(name, 256, 5_000).expect("a window the core can hold");
    if name == "server" {
        router.add_peer("alice", peer(&["alice"]));
        router.add_peer("bob", peer(&["bob"]));
    } else {
        router.add_peer("server", peer(&["server"]));
    }
    router
}

type Shared = SharedEndpoint<WssTransport, RealEnvironment>;

/// Let the machine take what its endpoint has for it, one call at a time and each to
/// rest before the next, until neither has more.
fn pump(engine: &mut Engine<TodoPeerPolicy>, shared: &Shared) {
    for _ in 0..PUMP_LIMIT {
        engine.step();
        let calls = shared
            .lock()
            .expect("a Mesh endpoint is never left mid-update")
            .take_calls();
        if calls.is_empty() {
            for error in shared
                .lock()
                .expect("a Mesh endpoint is never left mid-update")
                .take_host_errors()
            {
                eprintln!("host error: {error:?}");
            }
            return;
        }
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

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let [_, name, node, address] = args.as_slice() else {
        eprintln!("usage: todo_peer_wss <server|alice|bob> <node> <listen-address|ws-url>");
        std::process::exit(2);
    };
    let hub = name == "server";

    let transport = WssTransport::new();
    let environment = RealEnvironment {
        started: Instant::now(),
        counter: 0,
    };
    let shared: Shared = Arc::new(Mutex::new(Endpoint::new(
        router_for(name),
        transport.clone(),
        environment,
    )));
    let mut engine = Engine::new(TodoPeerPolicy::new());
    endpoint::register(&mut engine, &shared);
    engine.initialize();
    engine.raise_external_by_name("todo.join", &format!(r#"{{"node":"{node}","hub":{hub}}}"#));
    pump(&mut engine, &shared);

    let (link_tx, mut links) = mpsc::unbounded_channel::<LinkEvent>();
    if hub {
        let listener = TcpListener::bind(address.as_str())
            .await
            .expect("an address to listen on");
        println!(
            "LISTENING {}",
            listener.local_addr().expect("a bound address").port()
        );
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
                        |peer| matches!(peer, "alice" | "bob"),
                        transport,
                        link_tx,
                        DEFAULT_KEEPALIVE,
                    )
                    .await;
                });
            }
        });
    } else {
        let (base, own) = (address.clone(), name.clone());
        let transport = transport.clone();
        let link_tx = link_tx.clone();
        tokio::spawn(async move {
            loop {
                let _ = wss::connect(
                    &base,
                    &own,
                    "server",
                    transport.clone(),
                    link_tx.clone(),
                    DEFAULT_KEEPALIVE,
                )
                .await;
                tokio::time::sleep(REDIAL).await;
            }
        });
    }

    // The host's lines are read on a thread of their own: reading stdin blocks.
    let (command_tx, mut commands) = mpsc::unbounded_channel::<String>();
    std::thread::spawn(move || {
        for line in std::io::stdin().lock().lines().map_while(Result::ok) {
            if command_tx.send(line).is_err() {
                break;
            }
        }
    });

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
