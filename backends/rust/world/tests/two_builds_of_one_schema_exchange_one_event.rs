// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Two builds of one deployment on one network, as during an upgrade
// (docs/adr/0044): `revision_older` is built with the first revision of the
// schema of `reading.taken`, the level alone, and `revision_newer` with the
// second, which added the urgency (default `true`). Each sends the other one
// reading over the simulated network, and what each took is read from the
// machine.
//
// What this holds is that two revisions of one schema exchange events with no
// refusal in either direction: the newer machine reads an older delivery's
// missing field as its default, and the older machine walks past the field a
// newer delivery adds.

use sce_rust_mesh::inbound::Delivery;
use sce_rust_mesh::outbound::RetryPolicy;
use sce_rust_mesh::router::{PeerConfig, Router};
use sce_rust_world::{Outcome, World};

include!(concat!(env!("OUT_DIR"), "/machines.rs"));

use revision_newer::RevisionNewerPolicy as Newer;
use revision_older::RevisionOlderPolicy as Older;

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

/// The older build and the newer one, joined by a link of 20 ms.
fn world() -> World {
    let mut world = World::new(1);
    world.add("older", Older::new(), router("older", &["newer"]));
    world.add("newer", Newer::new(), router("newer", &["older"]));
    world.link("older", "newer", 20);
    world
}

#[test]
fn a_newer_build_reads_an_older_delivery_with_the_default_of_the_field_it_lacks() {
    let mut world = world();
    world.raise("older", "report", "");
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);

    let newer = world.actor::<Newer>("newer").policy();
    assert_eq!(newer.heard(), 1, "the delivery was taken");
    assert_eq!(newer.refusals(), 0, "and not refused");
    let last = newer.last();
    assert_eq!(last.level, 4, "the level the older build sent");
    assert!(last.urgent, "the urgency it lacks, read as its default");
    assert!(world.host_errors().is_empty());
}

#[test]
fn an_older_build_reads_a_newer_delivery_without_the_field_it_does_not_know() {
    let mut world = world();
    world.raise("newer", "report", "");
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);

    let older = world.actor::<Older>("older").policy();
    assert_eq!(older.heard(), 1, "the delivery was taken");
    assert_eq!(older.refusals(), 0, "and not refused");
    assert_eq!(older.last().level, 7, "the level the newer build sent");
    assert!(world.host_errors().is_empty());
}
