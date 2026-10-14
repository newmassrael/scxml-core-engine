// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// What the tests of the presence list have in common: the world of clients and the hub
// they send to (the fixture `presence_peer.scxml`), the ways of reading a copy back, and
// the oracle for what the hub must hold, which reads the network and not the copies.

use std::collections::{BTreeMap, BTreeSet};

use sce_forge_runtime::codec::SceCursor;
use sce_rust_mesh::generated::envelope::Envelope;
use sce_rust_mesh::inbound::Delivery;
use sce_rust_mesh::outbound::OutboundBuffer;
use sce_rust_mesh::router::{PeerConfig, Router};
use sce_rust_world::{Fate, World};
use serde_json::Value;

use super::presence_peer::PresencePeerPersist;
pub use super::presence_peer::PresencePeerPolicy as Presence;
use super::whole;

/// The hub's name, which the document writes as `#server`.
pub const HUB: &str = "server";

/// The topic the hub tells every client on, which the document writes as `#room`.
pub const ROOM: &str = "room";

/// How a binding of presence is configured: nothing is kept for a peer that is not
/// reachable and nothing is sent again, because the next heartbeat says what the one
/// that did not go would have said. A buffer would hand the hub, once a link is back, a
/// heartbeat from before the client was let go.
pub fn soft_peer() -> PeerConfig {
    PeerConfig {
        transport: "wss",
        buffer: None::<OutboundBuffer>,
        retry: None,
        stamp_sequence: false,
        delivery: Delivery {
            dedup: true,
            ordered: false,
        },
        responders: &[],
        deadline_ms: None,
        reply_events: &[],
    }
}

fn router(machine: &str, peers: impl IntoIterator<Item = String>) -> Router {
    let mut router = Router::new(machine, 256, 5_000).expect("a window the core can hold");
    for peer in peers {
        router.add_peer(&peer, soft_peer());
    }
    router
}

/// The text a copy of the presence list saves of itself.
fn saved_presence(engine: &sce_rust_runtime::Engine<Presence>) -> String {
    engine
        .save_at(0)
        .expect("a running machine saves")
        .to_json()
}

/// How long an entry lives: the hub keeps one `ttl_ms` from the heartbeat that renewed
/// it, and a client's copy `slack_ms` longer than that.
#[derive(Debug, Clone, Copy)]
pub struct Lease {
    pub ttl_ms: u32,
    pub slack_ms: u32,
}

/// A world of the clients `names`, each `latency_ms` from the hub, with entries that
/// live as `lease` says. Client `n` of the list is node `n + 1`. The hub tells every
/// client on the topic [`ROOM`]. With `recorded` every copy keeps a record of what it
/// was given, held and sent ([`World::record_of`]).
pub fn presence_world<S: AsRef<str>>(
    seed: u64,
    names: &[S],
    latency_ms: i64,
    lease: Lease,
    recorded: bool,
) -> World {
    let Lease { ttl_ms, slack_ms } = lease;
    let mut world = World::new(seed);
    let clients: Vec<&str> = names.iter().map(AsRef::as_ref).collect();
    let add = |world: &mut World, who: &str, router: Router| {
        if recorded {
            world.add_read(who, Presence::new(), router, saved_presence);
        } else {
            world.add(who, Presence::new(), router);
        }
    };
    for who in &clients {
        add(&mut world, who, router(who, [HUB.to_string()]));
    }
    let hub = router(
        HUB,
        std::iter::once(ROOM.to_string()).chain(clients.iter().map(|who| (*who).to_string())),
    );
    add(&mut world, HUB, hub);
    for who in &clients {
        world.link(who, HUB, latency_ms);
    }
    world.topic(HUB, ROOM, &clients);
    for (n, who) in clients.iter().enumerate() {
        world.raise(
            who,
            "presence.join",
            &format!(
                r#"{{"node":{},"hub":false,"ttl":{ttl_ms},"slack":{slack_ms}}}"#,
                n + 1
            ),
        );
    }
    world.raise(
        HUB,
        "presence.join",
        &format!(r#"{{"node":100000,"hub":true,"ttl":{ttl_ms},"slack":{slack_ms}}}"#),
    );
    world
}

/// The node a client of `names` is: one more than its place in the list.
pub fn node_in<S: AsRef<str>>(names: &[S], who: &str) -> u32 {
    let place = names
        .iter()
        .position(|name| name.as_ref() == who)
        .unwrap_or_else(|| panic!("{who} is not a client of this world"));
    u32::try_from(place + 1).expect("a node fits what a node is")
}

/// The host of `who` says it is time to say it is here, by the world's clock.
pub fn beat(world: &mut World, who: &str) {
    let now = world.now_ms();
    beat_at(world, who, now);
}

/// The same, by a clock that reads `now`: a host's own, which is not the world's.
pub fn beat_at(world: &mut World, who: &str, now: i64) {
    // A 64-bit integer is written as the text of its digits, as it is on the wire.
    world.raise(who, "presence.beat", &format!(r#"{{"now":"{now}"}}"#));
}

/// What a copy saved of itself: its variables, by name.
pub fn variables_of(world: &mut World, who: &str) -> Value {
    let saved = world
        .actor::<Presence>(who)
        .engine()
        .save_at(0)
        .expect("a running machine saves");
    let saved: Value = serde_json::from_str(&saved.to_json()).expect("a saved state is JSON");
    saved["variables"].clone()
}

/// The entries a copy holds, by client, each with the latest heartbeat stamp it holds.
pub fn entries_of(world: &mut World, who: &str) -> BTreeMap<u32, i64> {
    let variables = variables_of(world, who);
    let mut entries = BTreeMap::new();
    for entry in variables["online"].as_array().expect("online is a list") {
        let client = u32::try_from(whole(&entry["client"])).expect("a client is a node");
        let last = i64::try_from(whole(&entry["last"])).expect("a stamp is 64 bits");
        assert!(
            entries.insert(client, last).is_none(),
            "{who} holds {client} twice"
        );
    }
    entries
}

/// The clients a copy lists as online.
pub fn online_of(world: &mut World, who: &str) -> BTreeSet<u32> {
    entries_of(world, who).into_keys().collect()
}

/// What a copy did not manage to send, as it counts them.
pub fn lost_by(world: &mut World, who: &str) -> u64 {
    u64::try_from(whole(&variables_of(world, who)["lost"])).expect("a count")
}

/// The heartbeats that reached the hub, in the order they did: the client, the stamp it
/// carried, and when the hub was given it, read off the envelopes the network
/// delivered and not off any copy.
pub fn heartbeats_at_the_hub(world: &World) -> Vec<(u32, i64, i64)> {
    let mut heard = Vec::new();
    for line in world.trace() {
        if line.fate != Fate::Delivered || line.to != HUB {
            continue;
        }
        let envelope = Envelope::decode(&mut SceCursor::new(&line.bytes)).expect("an envelope");
        if envelope.event_type != "presence.alive" {
            continue;
        }
        let data: Value = serde_json::from_slice(envelope.data).expect("an event's data is JSON");
        heard.push((
            u32::try_from(whole(&data["client"])).expect("a client is a node"),
            i64::try_from(whole(&data["at"])).expect("a stamp is 64 bits"),
            line.at_ms,
        ));
    }
    heard
}

/// The heartbeats that renewed an entry of the hub, worked out from the ones that
/// reached it: the client, the stamp and when the hub was given it. A heartbeat renews
/// unless the hub holds the client already, which it does for `ttl_ms` from the
/// heartbeat that renewed it last, and the stamp is no later than the one it holds.
pub fn renewals_at_the_hub(world: &World, ttl_ms: i64) -> Vec<(u32, i64, i64)> {
    // The latest renewal of each client: its stamp and when the hub was given it.
    let mut latest: BTreeMap<u32, (i64, i64)> = BTreeMap::new();
    let mut renewals = Vec::new();
    for (client, at, arrived) in heartbeats_at_the_hub(world) {
        let held = latest
            .get(&client)
            .is_some_and(|(last, since)| since + ttl_ms > arrived && at <= *last);
        if !held {
            latest.insert(client, (at, arrived));
            renewals.push((client, at, arrived));
        }
    }
    renewals
}

/// Who the hub holds at the world's time, and each one's latest stamp, worked out from
/// the heartbeats that reached it and the clock alone: what the hub holds of a client
/// lives `ttl_ms` from when the heartbeat that renewed it last reached the hub. Nothing
/// is read from any copy.
pub fn who_the_hub_holds(world: &World, ttl_ms: i64) -> BTreeMap<u32, i64> {
    let now = world.now_ms();
    let mut latest: BTreeMap<u32, (i64, i64)> = BTreeMap::new();
    for (client, at, arrived) in renewals_at_the_hub(world, ttl_ms) {
        latest.insert(client, (at, arrived));
    }
    latest
        .into_iter()
        .filter(|(_, (_, since))| since + ttl_ms > now)
        .map(|(client, (last, _))| (client, last))
        .collect()
}
