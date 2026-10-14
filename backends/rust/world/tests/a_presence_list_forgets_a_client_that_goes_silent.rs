// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Who is online in a shared document: clients that say they are here by heartbeats, and
// the hub they send to, which lets a client go when it has been silent for a ttl and
// tells every copy (the fixture `presence_peer.scxml`). Presence is soft state: nothing
// says a client left, the entry of one that stopped just runs out, so what is asked here
// is when, and whether the copies come to the same list however the links treat what
// they carry.
//
// The hub's list is held to an oracle that does not read any copy: the heartbeats the
// network delivered to it and the world's clock say who it holds at any moment, and a
// client that is silent is gone at exactly its last heartbeat plus the ttl, not a moment
// before and not a moment after. A copy is held to the hub's list once the links have
// been quiet, and to what it must have forgotten by a time that its own ttl bounds.

mod common;

use std::collections::BTreeSet;

use common::presence::*;
use sce_rust_world::{Fate, Outcome, Rng, World};

const TTL_MS: u32 = 4_000;
const LATENCY_MS: i64 = 20;
const LEASE: Lease = Lease {
    ttl_ms: TTL_MS,
    slack_ms: 1_000,
};

fn names() -> [&'static str; 3] {
    ["alice", "bob", "carol"]
}

fn nodes(of: &[u32]) -> BTreeSet<u32> {
    of.iter().copied().collect()
}

/// Run the world on to `to_ms`, doing everything that falls due on the way.
fn run_to(world: &mut World, to_ms: i64) {
    let wait = to_ms - world.now_ms();
    assert!(wait >= 0, "{to_ms} is behind the world's time");
    world.advance_ms(wait);
}

/// Every one of `who` beats, on each of the seconds, at the start of it.
fn beat_each_second(world: &mut World, who: &[&str], seconds: std::ops::RangeInclusive<i64>) {
    for second in seconds {
        run_to(world, second * 1_000);
        for client in who {
            beat(world, client);
        }
    }
}

/// Who the hub lists, and whether it is who the heartbeats and the clock say.
fn hub_holds(world: &mut World, ttl_ms: u32) -> BTreeSet<u32> {
    let held = entries_of(world, HUB);
    assert_eq!(
        held,
        who_the_hub_holds(world, i64::from(ttl_ms)),
        "the hub holds what its heartbeats and the clock say at {} ms",
        world.now_ms()
    );
    held.into_keys().collect()
}

/// The names of the copies every one of which lists exactly `expected`, and the ones that do not.
fn copies_listing(
    world: &mut World,
    names: &[&str],
    expected: &BTreeSet<u32>,
) -> (Vec<String>, Vec<String>) {
    let mut agree = Vec::new();
    let mut differ = Vec::new();
    for who in names {
        if &online_of(world, who) == expected {
            agree.push((*who).to_string());
        } else {
            differ.push((*who).to_string());
        }
    }
    (agree, differ)
}

fn assert_every_copy_lists(world: &mut World, names: &[&str], expected: &BTreeSet<u32>) {
    let (_, differ) = copies_listing(world, names, expected);
    assert!(
        differ.is_empty(),
        "at {} ms {differ:?} do not list {expected:?}",
        world.now_ms()
    );
}

/// A client that stops heartbeating is gone from the hub at its last heartbeat plus the
/// ttl, and from every copy right after, when the hub's word reaches them; a client that
/// goes on is never let go.
#[test]
fn a_client_that_stops_beating_is_gone_everywhere_after_the_ttl_and_not_before() {
    let clients = names();
    let mut world = presence_world(1, &clients, LATENCY_MS, LEASE, false);
    // Everyone says it is here; alice goes on, bob and carol say it once.
    for who in clients {
        beat(&mut world, who);
    }
    run_to(&mut world, 100);
    let everyone = nodes(&[1, 2, 3]);
    assert_eq!(hub_holds(&mut world, TTL_MS), everyone);
    assert_every_copy_lists(&mut world, &clients, &everyone);

    for second in 1..=3 {
        run_to(&mut world, second * 1_000);
        beat(&mut world, "alice");
    }
    // The heartbeats reached the hub at 20 ms, so bob and carol are held until 4_020.
    run_to(&mut world, 4_019);
    assert_eq!(
        hub_holds(&mut world, TTL_MS),
        everyone,
        "the ttl has not run out a millisecond early"
    );
    run_to(&mut world, 4_020);
    let alice_only = nodes(&[1]);
    assert_eq!(
        hub_holds(&mut world, TTL_MS),
        alice_only,
        "the ttl ran out at once"
    );
    // The copies are told at 4_040, a link's time after the hub let them go. Their own
    // entries for bob and carol would have run out only at 5_040, a slack later.
    run_to(&mut world, 4_039);
    assert_every_copy_lists(&mut world, &clients, &everyone);
    run_to(&mut world, 4_040);
    assert_every_copy_lists(&mut world, &clients, &alice_only);

    // Alice keeps beating, so she stays for as long as she does.
    for second in 5..=9 {
        run_to(&mut world, second * 1_000);
        beat(&mut world, "alice");
        run_to(&mut world, second * 1_000 + 500);
        assert_eq!(hub_holds(&mut world, TTL_MS), alice_only);
        assert_every_copy_lists(&mut world, &clients, &alice_only);
    }
    assert!(world.host_errors().is_empty());
}

/// A heartbeat extends a client's life by a ttl from the heartbeat: the expiry armed for
/// the one before finds the entry renewed and does nothing.
#[test]
fn a_heartbeat_extends_a_clients_life() {
    let clients = names();
    let mut world = presence_world(2, &clients, LATENCY_MS, LEASE, false);
    beat(&mut world, "bob");
    run_to(&mut world, 3_000);
    // Bob's first heartbeat would run out at 4_020; this one is held from 3_020.
    beat(&mut world, "bob");
    run_to(&mut world, 4_100);
    assert_eq!(
        hub_holds(&mut world, TTL_MS),
        nodes(&[2]),
        "the first expiry found bob renewed and let him be"
    );
    assert_every_copy_lists(&mut world, &clients, &nodes(&[2]));
    run_to(&mut world, 7_019);
    assert_eq!(hub_holds(&mut world, TTL_MS), nodes(&[2]));
    run_to(&mut world, 7_020);
    assert!(hub_holds(&mut world, TTL_MS).is_empty());
    run_to(&mut world, 7_100);
    assert_every_copy_lists(&mut world, &clients, &BTreeSet::new());
    // Nothing is left to happen: there is no entry anywhere to run out.
    assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet);
    assert!(world.host_errors().is_empty());
}

/// A heartbeat is not a request for anything: a client that said so twice in a row, or
/// whose heartbeat the link delivered late, leaves the entry as the latest stamp made it.
#[test]
fn a_heartbeat_older_than_the_one_held_renews_nothing() {
    let clients = names();
    let mut world = presence_world(3, &clients, LATENCY_MS, LEASE, false);
    // The host of bob reads a clock that is behind by the time its next heartbeat is
    // made: the stamp is older than the one the hub holds.
    beat_at(&mut world, "bob", 1_000);
    run_to(&mut world, 1_000);
    beat_at(&mut world, "bob", 500);
    run_to(&mut world, 1_100);
    let held = entries_of(&mut world, HUB);
    assert_eq!(held[&2], 1_000, "the older stamp renewed nothing");
    // And it did not extend his life: the expiry is 4_000 after the first.
    run_to(&mut world, 4_019);
    assert_eq!(hub_holds(&mut world, TTL_MS), nodes(&[2]));
    run_to(&mut world, 5_000);
    assert!(hub_holds(&mut world, TTL_MS).is_empty());
    assert_every_copy_lists(&mut world, &clients, &BTreeSet::new());
    assert!(world.host_errors().is_empty());
}

/// A client that was let go and says it is here again is listed again, by the hub and by
/// every copy, as one who never was: there is nothing of its leaving to undo.
#[test]
fn a_client_that_rejoins_appears_again() {
    let clients = names();
    let mut world = presence_world(4, &clients, LATENCY_MS, LEASE, false);
    beat(&mut world, "alice");
    beat(&mut world, "bob");
    run_to(&mut world, 2_000);
    beat(&mut world, "alice");
    run_to(&mut world, 4_000);
    beat(&mut world, "alice");
    run_to(&mut world, 4_500);
    // Bob was heard at 20 ms and is gone at 4_020; alice, renewed at 4_020, is not.
    assert_eq!(hub_holds(&mut world, TTL_MS), nodes(&[1]));
    assert_every_copy_lists(&mut world, &clients, &nodes(&[1]));

    run_to(&mut world, 6_000);
    beat(&mut world, "alice");
    beat(&mut world, "bob");
    run_to(&mut world, 6_100);
    let both = nodes(&[1, 2]);
    assert_eq!(hub_holds(&mut world, TTL_MS), both);
    assert_every_copy_lists(&mut world, &clients, &both);
    assert_eq!(
        entries_of(&mut world, HUB)[&2],
        6_000,
        "the entry is the new one, not the one that ran out"
    );
    // And it runs out as the first did, a ttl after the new heartbeat reached the hub.
    run_to(&mut world, 8_000);
    beat(&mut world, "alice");
    run_to(&mut world, 10_019);
    assert!(hub_holds(&mut world, TTL_MS).contains(&2));
    run_to(&mut world, 10_020);
    assert!(!hub_holds(&mut world, TTL_MS).contains(&2));
    assert!(world.host_errors().is_empty());
}

/// A hub that cannot be reached lets go of its clients, whose heartbeats it is not
/// given, and a client that cannot hear the hub forgets everyone; when the link is back
/// the next heartbeat repairs both, with no message about the time in between.
#[test]
fn a_partition_of_the_hub_is_repaired_by_the_next_heartbeats() {
    let clients = names();
    let mut world = presence_world(5, &clients, LATENCY_MS, LEASE, false);
    let everyone = nodes(&[1, 2, 3]);
    beat_each_second(&mut world, &clients, 1..=3);
    run_to(&mut world, 3_500);
    assert_eq!(hub_holds(&mut world, TTL_MS), everyone);
    assert_every_copy_lists(&mut world, &clients, &everyone);

    // Bob and carol lose the hub, and go on beating into nothing.
    world.partition("bob", HUB);
    world.partition("carol", HUB);
    beat_each_second(&mut world, &clients, 4..=11);
    run_to(&mut world, 12_500);
    assert_eq!(
        hub_holds(&mut world, TTL_MS),
        nodes(&[1]),
        "the hub let go of the two it was not given heartbeats from"
    );
    assert!(
        lost_by(&mut world, "bob") >= 8,
        "bob's heartbeats did not go, and he was told so"
    );
    assert_eq!(online_of(&mut world, "alice"), nodes(&[1]));
    assert!(
        online_of(&mut world, "bob").is_empty(),
        "bob hears nothing, so what he held ran out"
    );
    assert!(online_of(&mut world, "carol").is_empty());

    world.heal("bob", HUB);
    world.heal("carol", HUB);
    beat_each_second(&mut world, &clients, 13..=14);
    run_to(&mut world, 14_500);
    assert_eq!(hub_holds(&mut world, TTL_MS), everyone);
    assert_every_copy_lists(&mut world, &clients, &everyone);
    assert!(world.host_errors().is_empty());
}

/// Names for `count` clients, in the order of their nodes.
fn crowd(count: usize) -> Vec<String> {
    (1..=count).map(|n| format!("c{n:03}")).collect()
}

/// A hundred clients on one hub over links that delay by a spread, lose a tenth of what
/// they carry and repeat a twentieth, each beating at a rhythm of its own. The hub holds,
/// at every moment that is looked at, exactly who its heartbeats and the clock say; once
/// the links are clean again every copy lists the hub's list; and a client that goes
/// silent is gone from the hub at the very time its last heartbeat and the ttl give.
#[test]
fn a_hundred_clients_over_lossy_links_converge_and_the_ttl_holds() {
    const CLIENTS: usize = 100;
    const PERIOD_MS: i64 = 3_000;
    const TTL: u32 = 10_000;
    // A copy outlasts the hub by more than a link can hold an envelope: the spread of
    // the delay, and what the hub's word waits for behind the one before it.
    const SLACK: u32 = 1_000;
    const SLOT_MS: i64 = 100;

    let crowd = crowd(CLIENTS);
    let lease = Lease {
        ttl_ms: TTL,
        slack_ms: SLACK,
    };
    let mut world = presence_world(7, &crowd, 15, lease, false);
    let names: Vec<&str> = crowd.iter().map(String::as_str).collect();
    for who in &names {
        world.jitter(who, HUB, 40);
        world.degrade(who, HUB, 10, 5);
    }

    // Each client beats at its own phase of the period, drawn from the seed.
    let mut rhythm = Rng::new(0x5EED);
    let phase: Vec<i64> = names
        .iter()
        .map(|_| {
            i64::try_from(rhythm.below(u64::try_from(PERIOD_MS / SLOT_MS).unwrap())).unwrap()
                * SLOT_MS
        })
        .collect();
    // The clients that are beating: all of them, until one by one a tenth go silent.
    let mut beating: BTreeSet<usize> = (0..CLIENTS).collect();
    let slot = |world: &mut World, beating: &BTreeSet<usize>| {
        let now = world.now_ms();
        for n in beating {
            if (now - phase[*n]).rem_euclid(PERIOD_MS) == 0 {
                beat(world, names[*n]);
            }
        }
        world.advance_ms(SLOT_MS);
    };

    // Twelve seconds of beating over the bad links, the hub looked at every second.
    for tick in 0..120 {
        slot(&mut world, &beating);
        if tick % 10 == 9 {
            hub_holds(&mut world, TTL);
        }
    }
    // A tenth fall silent for good.
    let silent: BTreeSet<usize> = (0..CLIENTS).filter(|n| n % 10 == 3).collect();
    for n in &silent {
        beating.remove(n);
    }
    for tick in 0..120 {
        slot(&mut world, &beating);
        if tick % 10 == 9 {
            hub_holds(&mut world, TTL);
        }
    }

    // The links are clean again. After a period and what one copy takes to arrive,
    // every client that beats is listed by every copy, and the silent are not.
    for who in &names {
        world.degrade(who, HUB, 0, 0);
    }
    for _ in 0..(PERIOD_MS / SLOT_MS + 6) {
        slot(&mut world, &beating);
    }
    let alive: BTreeSet<u32> = beating
        .iter()
        .map(|n| u32::try_from(n + 1).unwrap())
        .collect();
    assert_eq!(alive.len(), CLIENTS - silent.len());
    let held = hub_holds(&mut world, TTL);
    assert!(
        alive.is_subset(&held),
        "every client that beats through clean links is held by the hub"
    );
    // A silent client is let go by the time its own ttl has run out, so on a clean run
    // longer than that the hub holds exactly the clients that beat.
    assert_eq!(held, alive);
    assert_every_copy_lists(&mut world, &names, &alive);

    // Every client stops. The last heartbeat the hub was given is held for a ttl and not
    // a moment longer, and after that nothing is listed anywhere.
    let last_renewal = renewals_at_the_hub(&world, i64::from(TTL))
        .into_iter()
        .map(|(_, _, arrived)| arrived)
        .max()
        .expect("the hub was given heartbeats");
    run_to(&mut world, last_renewal + i64::from(TTL) - 1);
    assert!(
        !hub_holds(&mut world, TTL).is_empty(),
        "the last heartbeat is still held a millisecond before its ttl runs out"
    );
    run_to(&mut world, last_renewal + i64::from(TTL));
    assert!(
        hub_holds(&mut world, TTL).is_empty(),
        "and gone the moment it does"
    );
    // Every copy was told, or had its own entry run out besides, by the time the hub's
    // last announcement has arrived and the slack is over.
    run_to(
        &mut world,
        last_renewal + i64::from(TTL) + i64::from(SLACK) + 200,
    );
    assert_every_copy_lists(&mut world, &names, &BTreeSet::new());
    assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet);

    // The links did what they were asked to, and nothing went wrong that only a host
    // could act on.
    let trace = world.trace();
    let count = |wanted: fn(&Fate) -> bool| trace.iter().filter(|line| wanted(&line.fate)).count();
    assert!(count(|fate| matches!(fate, Fate::Lost)) > 100, "links lost");
    assert!(
        count(|fate| matches!(fate, Fate::Duplicated { .. })) > 20,
        "links repeated"
    );
    for who in names.iter().chain(std::iter::once(&HUB)) {
        assert_eq!(
            variables_of(&mut world, who)["refusals"],
            serde_json::json!(0),
            "{who} refused nothing"
        );
    }
    assert!(world.host_errors().is_empty());
}
