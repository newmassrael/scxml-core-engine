// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The smallest world: two generated machines, `echo_client` and `echo_server`
// (backends/rust/world/machines), each on its own Mesh endpoint, joined by the
// simulated network. The client asks, the server answers, and what each machine
// counted is read from the machine and not from the network.
//
// What this holds is that the SEAMS the machines already have are enough to
// run a deployment in one process, and that doing so is deterministic: the
// same seed makes the same run, whatever the machine it runs on.

use sce_rust_mesh::inbound::Delivery;
use sce_rust_mesh::outbound::RetryPolicy;
use sce_rust_mesh::router::{PeerConfig, Router};
use sce_rust_world::{Fate, Outcome, World};

include!(concat!(env!("OUT_DIR"), "/machines.rs"));

use echo_client::EchoClientPolicy as Client;
use echo_server::EchoServerPolicy as Server;

/// How the binding to `peer` is configured: a WebSocket, which delivers in
/// order and once, with retries for a transmission its link refuses.
fn peer(responders: &'static [&'static str]) -> PeerConfig {
    PeerConfig {
        transport: "wss",
        buffer: None,
        retry: Some(RetryPolicy {
            max_retries: 5,
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

/// A router for `machine` with one peer.
fn router(machine: &str, other: &'static [&'static str]) -> Router {
    let mut router = Router::new(machine, 64, 50).expect("a window the core can hold");
    router.add_peer(other[0], peer(other));
    router
}

/// The pair, joined by a link of `latency_ms`.
fn world(seed: u64, latency_ms: i64) -> World {
    let mut world = World::new(seed);
    world.add("client", Client::new(), router("client", &["server"]));
    world.add("server", Server::new(), router("server", &["client"]));
    world.link("client", "server", latency_ms);
    world
}

fn counts(world: &mut World) -> (u32, u32, u32) {
    (
        world.actor::<Client>("client").policy().asked(),
        world.actor::<Client>("client").policy().heard(),
        world.actor::<Server>("server").policy().served(),
    )
}

#[test]
fn a_ping_reaches_the_server_and_the_pong_comes_back() {
    let mut world = world(1, 20);
    world.raise("client", "ask", "");

    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);

    assert_eq!(counts(&mut world), (1, 1, 1));
    // One way is 20 ms and the way back another 20: virtual time, not waited.
    assert_eq!(world.now_ms(), 40);
    assert!(world.host_errors().is_empty());
}

#[test]
fn a_link_that_is_down_is_retried_by_the_core_until_it_is_healed() {
    let mut world = world(1, 20);
    world.partition("client", "server");
    world.raise("client", "ask", "");

    // The client's first transmission is refused and backs off; with the link
    // down nothing reaches the server in the first half second.
    assert_eq!(world.run_until_quiet(500), Outcome::Limit);
    assert_eq!(counts(&mut world), (1, 0, 0));

    world.heal("client", "server");
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);

    assert_eq!(counts(&mut world), (1, 1, 1));
    assert!(world.host_errors().is_empty());
}

#[test]
fn an_envelope_in_flight_when_its_link_is_cut_is_sent_again_after_the_heal() {
    let mut world = world(1, 100);
    world.raise("client", "ask", "");
    // Settle the client's side only far enough for the ping to be on the wire.
    assert_eq!(world.run_until_quiet(10), Outcome::Limit);
    assert_eq!(counts(&mut world), (1, 0, 0));

    world.partition("client", "server");
    assert!(
        world.trace().iter().any(|line| line.fate == Fate::Cut),
        "the ping was in flight and was cut"
    );
    world.heal("client", "server");
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);

    assert_eq!(counts(&mut world), (1, 1, 1));
}

#[test]
fn an_envelope_the_link_delivers_twice_reaches_the_machine_once() {
    let mut world = world(7, 5);
    world.degrade("client", "server", 0, 100);
    world.raise("client", "ask", "");

    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);

    assert!(
        world
            .trace()
            .iter()
            .any(|line| matches!(line.fate, Fate::Duplicated { .. })),
        "the link did deliver the ping twice"
    );
    // The core's duplicate suppression (SCE_MESH.md §mesh-10.5) kept the second
    // from the machine: it served one ping, and the client heard one pong.
    assert_eq!(counts(&mut world), (1, 1, 1));
}

/// A run is a function of its seed: two worlds built alike, driven alike, leave
/// the same record on the network down to the bytes.
#[test]
fn the_same_seed_makes_the_same_run_and_another_seed_makes_another() {
    fn run(seed: u64) -> Vec<sce_rust_world::Traced> {
        let mut world = world(seed, 10);
        world.jitter("client", "server", 30);
        world.degrade("client", "server", 0, 50);
        for _ in 0..8 {
            world.raise("client", "ask", "");
        }
        assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet);
        world.trace()
    }

    assert_eq!(run(42), run(42), "the same seed, the same run");
    assert_ne!(run(42), run(43), "another seed, another run");
}

#[test]
fn many_pings_are_all_answered_once_whatever_the_delays() {
    for seed in 0..20 {
        let mut world = world(seed, 5);
        world.jitter("client", "server", 50);
        world.degrade("client", "server", 0, 30);
        world.degrade("server", "client", 0, 30);
        for _ in 0..16 {
            world.raise("client", "ask", "");
        }
        assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet, "seed {seed}");
        assert_eq!(counts(&mut world), (16, 16, 16), "seed {seed}");
        assert!(world.host_errors().is_empty(), "seed {seed}");
    }
}
