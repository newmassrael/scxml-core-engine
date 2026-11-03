// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A shared list whose every device holds it through the peer core (ADR 0079 section 8): alice's
// device and carol's are Rust, their cores in this process; bob's is Kotlin, his copy of the
// standard record list and his core in a JVM of their own, the core reached through its C ABI with
// JNA and every round run by the library's own driver (`sce_peer_run`) over a transport whose
// every exchange the world carries. Between the devices and the relay is a network that runs on the
// world's clock (`common::devices`), and the relay keeps its own allowance. Nothing here composes
// the trust layer, a courier or a group by hand: each device has its machine and its core, and its
// host does what an application's would, giving the core what the machine sends and the machine
// what a round brings.
//
// What is asked, of every device whichever language it is: that the copies come to hold one list,
// and the list that was written, over a network that loses, delays and reorders what it carries;
// that a device whose job's time ends in the middle of a round, and one whose answers come after
// its deadline, go on where they were and sign nothing twice; that the relay's allowance slows the
// devices and they still agree; that a relay that answers less than it holds leaves a device short
// and refuses it nothing, until a resync reads what the relay holds; that a write the relay made up
// is refused by the core of every device it is given to and reaches no copy; and that a list keyed
// by its group admits every device from its key package and is sealed at the relay from its first
// write, which every device admitted later opens.
//
// Properties, not bytes: a run is the same run every time it is run with the same seed, but the
// group's keys are drawn, so what is asked is what holds of any run.
//
// These tests need a JVM, the step server's build (`./gradlew :sce-kotlin-world:installDist`) and
// cargo, which builds the core's shared library. Without the JVM or the build they say so and run
// with bob's device Rust, unless `SCE_REQUIRE_TOOLS` is set, which fails them.

mod common;

use common::devices::{
    ample, kotlin_device, Begin, Devices, Dishonesty, Keying, Path, BOB, CAROL, FORGED_ROW,
};
use common::DUE;
use sce_rust_peer::{Kept, RoundKind};
use sce_rust_relay::Limits;

/// The words a transport's failure is told to the core in, whoever carried it.
const CANCELLED: &str = "the exchange was cancelled";
const DEADLINE: &str = "the deadline passed before the relay answered";

fn link_failure(reason: &str) -> Option<(String, String)> {
    Some(("link".to_string(), reason.to_string()))
}

/// What every device's core did with what its machine sent: each was signed and kept, or was
/// no event that is signed. None was held already (nothing was sent twice) and none refused.
fn every_send_was_taken_once(devices: &mut Devices) {
    for name in devices.names() {
        let kept = &devices.device_mut(name).kept;
        assert!(
            kept.iter()
                .all(|kept| matches!(kept, Kept::Taken | Kept::NotSigned)),
            "{name}: {kept:?}"
        );
        assert!(
            kept.contains(&Kept::Taken),
            "{name} wrote, and its core signed nothing"
        );
    }
}

/// The three devices write, each to rows of its own and all three to one row before any hears of
/// the others, while the network between them and the relay loses a tenth of what it carries each
/// way, spreads its delays wide enough to bring requests to the relay out of the order they were
/// sent, and gives a transport less time than some answers take. Every pass, every device runs a
/// round, all at once. Then the network heals, and every copy holds one list, which is what was
/// written; an honest relay refused nothing, and nothing it gave was refused.
#[test]
fn every_device_holds_one_list_over_a_network_that_loses_delays_and_reorders() {
    let bob = kotlin_device("bob");
    for seed in 1..=3 {
        let mut devices = Devices::new(seed, bob.as_ref(), Keying::Clear, ample());
        devices.settle();
        let bad = Path {
            latency_ms: 5,
            jitter_ms: 60,
            loss_pct: 10,
            timeout_ms: 100,
        };
        for name in devices.names() {
            devices.set_path(name, bad);
        }
        devices.set("alice", 1, 1_000);
        devices.set("bob", 1, 2_000);
        devices.set("carol", 1, 3_000);
        for step in 0..6i64 {
            let row = u64::try_from(step).expect("a step is a row");
            devices.set("alice", 10 + row, step);
            devices.set("bob", 20 + row, 100 + step);
            devices.set("carol", 30 + row, 200 + step);
            devices.pass();
        }
        assert!(
            devices.reordered > 0,
            "seed {seed}: no request reached the relay after one sent later"
        );
        assert!(
            devices.failed_exchanges > 0,
            "seed {seed}: the network failed no exchange"
        );

        devices.heal();
        devices.settle();
        devices.assert_holds_what_was_written();
        assert!(
            devices.refused().is_empty(),
            "seed {seed}: {:?}",
            devices.refused()
        );
        every_send_was_taken_once(&mut devices);
        let trust = devices.administrator().trust();
        assert!(
            trust.equivocating().is_empty(),
            "seed {seed}: no chain is frozen"
        );
        assert!(trust.divergences().is_empty(), "seed {seed}");
    }
}

/// Bob's job and carol's end with the second request of a round on its way, which the relay still
/// takes: each run stops on its cancellation, which for bob's device is the ABI's
/// `SCE_PEER_ERR_CANCELLED`. Alice's transport is then given less time than any answer takes, and
/// her round ends as its deadline passes. The network heals and each device's next rounds go on
/// from where its last stopped: the copies hold one list, what was written, and no core signed a
/// send twice.
#[test]
fn a_device_suspended_in_a_round_and_one_answered_too_late_go_on_where_they_were() {
    let bob = kotlin_device("bob");
    let mut devices = Devices::new(7, bob.as_ref(), Keying::Clear, ample());
    devices.settle();
    devices.set("bob", 1, 10);
    devices.set("bob", 2, 20);
    devices.set("carol", 3, 30);
    devices.give_sends();
    devices.suspend("bob", 2);
    devices.suspend("carol", 2);
    let ran = devices.rounds(&["bob", "carol"], Begin::Round(RoundKind::Sync));
    for name in ["bob", "carol"] {
        assert!(ran[name].cancelled, "{name}: {:?}", ran[name]);
        assert_eq!(ran[name].failed, link_failure(CANCELLED), "{name}");
    }

    devices.set("alice", 4, 40);
    devices.give_sends();
    devices.set_path(
        "alice",
        Path {
            latency_ms: 300,
            timeout_ms: 100,
            ..Path::default()
        },
    );
    let ran = devices.rounds(&["alice"], Begin::Round(RoundKind::Sync));
    assert_eq!(ran["alice"].failed, link_failure(DEADLINE));
    assert!(!ran["alice"].cancelled, "a deadline is not a cancellation");

    devices.heal();
    devices.settle();
    devices.assert_holds_what_was_written();
    assert!(devices.refused().is_empty(), "{:?}", devices.refused());
    every_send_was_taken_once(&mut devices);
}

/// The relay keeps the allowance it would keep for any list: a few requests at once, a few more
/// each second, a write dearer than a read. Every device writes more than a round's worth, so the
/// relay refuses rounds for the rate and says when to ask again; each device asks again then, and
/// the copies come to hold one list, the list that was written.
#[test]
fn the_relays_allowance_slows_every_device_and_they_still_agree() {
    let bob = kotlin_device("bob");
    let tight = Limits {
        burst: 6,
        refill_per_second: 2,
        write_cost: 2,
        ..Limits::default()
    };
    let mut devices = Devices::new(17, bob.as_ref(), Keying::Clear, tight);
    for row in 1..=3 {
        devices.set("alice", row, 10);
        devices.set("bob", 10 + row, 20);
        devices.set("carol", 20 + row, 30);
    }
    let began = std::time::Instant::now();
    devices.settle();
    let took = began.elapsed();
    assert!(
        devices.limited > 0,
        "the relay refused no round for its allowance"
    );
    // Bob's Kotlin device runs its rounds by the library's driver, which waits out a refusal for
    // the rate and asks again, on the clock of whoever carries the texts: the world's, through the
    // pause the step server gives its link. So bob asked again after waits the world's clock took,
    // and the run slept none of them: a run on the system's clock would sleep each wait in real
    // time and ask again of a relay whose clock had not moved, and be refused again for as long as
    // the world's paths took to move it.
    if devices.is_kotlin("bob") {
        let resent = devices.paused.get("bob").copied().unwrap_or(0);
        let waited_ms = devices.paused_ms.get("bob").copied().unwrap_or(0);
        assert!(
            resent > 0,
            "bob's run waited out no refusal for the rate on the world's clock"
        );
        assert!(
            waited_ms > 0,
            "a wait of the relay's is a wait of some time"
        );
        assert!(
            took < std::time::Duration::from_secs(120),
            "bob waited {resent} times, {waited_ms} ms of the world's time, and the settling took \
             {took:?} of real time"
        );
    }
    devices.assert_holds_what_was_written();
    every_send_was_taken_once(&mut devices);
}

/// A relay that leaves alice's write to row 5 out of everything it gives bob's device and carol's
/// (the feed's cursor goes past it all the same) leaves both copies without it, and with all else
/// alice and carol wrote: what was withheld is not refused, since nothing wrong was given. Once the
/// relay answers in full, a sync does not bring back what the feed passed, and a resync, which
/// reads everything the relay holds, does: the copies hold one list.
#[test]
fn a_relay_that_withholds_leaves_a_device_short_until_a_resync_reads_all_it_holds() {
    let bob = kotlin_device("bob");
    let mut devices = Devices::new(11, bob.as_ref(), Keying::Clear, ample());
    devices.settle();
    devices.dishonesty.withhold = vec![(BOB, 5), (CAROL, 5)];
    // Her last write is the one withheld, so that no write of hers after it waits on it.
    devices.set("alice", 6, 60);
    devices.set("carol", 7, 70);
    devices.settle();
    devices.set("alice", 5, 50);
    devices.settle();
    assert!(devices.records("alice").contains_key(&(5, DUE)));
    for name in ["bob", "carol"] {
        let held = devices.records(name);
        assert!(!held.contains_key(&(5, DUE)), "{name} was given row 5");
        assert!(
            held.contains_key(&(6, DUE)) && held.contains_key(&(7, DUE)),
            "{name} holds all else that was written: {held:?}"
        );
    }
    assert!(
        devices.refused().is_empty(),
        "what was withheld is not refused: {:?}",
        devices.refused()
    );

    devices.dishonesty = Dishonesty::default();
    devices.settle();
    for name in ["bob", "carol"] {
        assert!(
            !devices.records(name).contains_key(&(5, DUE)),
            "{name}: a sync reads past the cursor, which went past row 5"
        );
    }
    let read = devices.rounds(&["bob", "carol"], Begin::Round(RoundKind::Resync));
    for (name, ran) in &read {
        assert_eq!(ran.failed, None, "{name}");
    }
    devices.settle();
    devices.assert_holds_what_was_written();
}

/// A relay that gives bob's device and carol's, after each write, a copy it made up (the write
/// moved to another row, with the signature of the true one): the core of each refuses every copy
/// it is given, says so in the round's outcome, and tells its machine nothing of it, so that no copy
/// holds the row the relay made up and all hold what was written. Alice's device is given no forgery
/// and refuses nothing.
#[test]
fn a_write_the_relay_made_up_is_refused_by_the_core_of_every_device_given_it() {
    let bob = kotlin_device("bob");
    let mut devices = Devices::new(13, bob.as_ref(), Keying::Clear, ample());
    devices.settle();
    devices.dishonesty.forge_toward = vec![BOB, CAROL];
    devices.set("alice", 1, 10);
    devices.set("alice", 2, 20);
    devices.set("carol", 3, 30);
    devices.settle();
    devices.assert_holds_what_was_written();
    for name in devices.names() {
        assert!(
            !devices.records(name).contains_key(&(FORGED_ROW, DUE)),
            "{name} holds the row the relay made up"
        );
    }
    let refused = devices.refused();
    for (name, node) in [("bob", BOB), ("carol", CAROL)] {
        let forged = devices.forged.get(&node).copied().unwrap_or(0);
        assert!(forged > 0, "the relay forged nothing toward {name}");
        assert_eq!(
            refused.get(name).map_or(0, Vec::len),
            forged,
            "{name}'s core refused each copy the relay made up: {refused:?}"
        );
    }
    assert!(!refused.contains_key("alice"), "{refused:?}");
}

/// A list keyed by its group: every device holds a device of the group, and the list is sealed
/// from its first write, so what alice's device writes while she is alone in it reaches the relay
/// sealed. Bob's and carol's leave a key package with the relay, and alice's device admits each in
/// one act, bob first, giving each the keys of the epochs before its own. Each joins from the
/// Welcome a round brings it; bob's applies the commit that admits carol after it, so each holds
/// every key alice's holds, and every copy holds what alice wrote alone. What the three write then
/// is held alike by every copy and is sealed at the relay, which holds none of it in the clear. The
/// group's keys are drawn, so what is asked is what holds whatever they are.
#[test]
fn a_list_keyed_by_its_group_is_held_through_the_core_on_every_device() {
    let bob = kotlin_device("bob");
    let mut devices = Devices::new(19, bob.as_ref(), Keying::Group, ample());
    devices.title("alice", 4, "before anyone");
    devices.give_sends();
    let alone = devices.rounds(&["alice"], Begin::Round(RoundKind::Sync));
    assert_eq!(alone["alice"].failed, None, "{:?}", alone["alice"]);
    assert!(
        !devices.feed().contains("before anyone"),
        "the relay holds what alice wrote alone in the clear"
    );
    devices.admit_through_the_group();
    let keys = devices.keys("alice");
    assert_eq!(
        keys.len(),
        3,
        "the epoch the opening began, and one for each device admitted: {keys:?}"
    );
    assert_eq!(devices.keys("bob"), keys);
    assert_eq!(
        devices.keys("carol"),
        keys,
        "carol was given the keys of the epochs before hers"
    );
    let bob_steps = devices.device_mut("bob").steps.clone();
    let at = |step: &str| bob_steps.iter().position(|taken| taken == step);
    assert!(
        at("joined").is_some() && at("joined") < at("applied"),
        "bob joins, then applies the commit that admits carol: {bob_steps:?}"
    );
    let carol_steps = devices.device_mut("carol").steps.clone();
    assert!(
        carol_steps.iter().any(|step| step == "joined"),
        "{carol_steps:?}"
    );
    for steps in [&bob_steps, &carol_steps] {
        assert!(
            !steps
                .iter()
                .any(|step| step == "refused" || step == "removed"),
            "{steps:?}"
        );
    }

    devices.title("alice", 1, "buy milk");
    devices.set("bob", 2, 20);
    devices.set("carol", 3, 30);
    devices.settle();
    devices.assert_holds_what_was_written();
    assert!(devices.refused().is_empty(), "{:?}", devices.refused());
    let held = devices.feed();
    assert!(!held.contains("buy milk"), "the relay holds the text");
}
