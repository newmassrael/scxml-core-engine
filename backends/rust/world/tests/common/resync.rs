// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// What the tests of the to-do list's resync have in common: a copy that has lost changes
// for good and the digest walk that finds them, and the ways of holding the three copies
// to the list the network says they should end with and to a digest that is the digest of
// exactly what each holds.
//
// The digest is a trie over the minute of each change's dot, thirty base-3 digits deep
// (stdlib/merge/merkle_node.scxml), and the hash of a change is merge/merkle_change_hash.
// Both are written out here a second time, in Rust and over wide integers, so that what a
// copy keeps is checked against a model that shares no code with the statechart: a digest
// that is wrong at a wall time or a field number the statechart mishandles is wrong here
// the same way only if both made the same mistake.
//
// A test takes an `epoch`, the wall time its afternoon starts at, in milliseconds. Every
// change is stamped `epoch + world.now_ms()`, so one afternoon can be played in 1970, on
// either side of 3^16 minutes (2051-11-05T13:21Z, where the first form of the key ran out)
// or at the last minute a signed 64-bit clock holds, and the checks are the same ones.

use std::collections::BTreeMap;
use std::ops::Range;

use sce_forge_runtime::codec::SceCursor;
use sce_rust_mesh::generated::envelope::Envelope;
use sce_rust_runtime::saved_state::SavedState;
use sce_rust_runtime::Engine;
use sce_rust_world::{Fate, Outcome, Rng, World};
use serde_json::Value;

use super::{
    agreed_todos, todo_edit_at, todo_oracle, todo_variables_of, todo_world, todos_of, whole, Field,
    TodoPeer, TodoPeerPersist, DONE, DUE, TITLE,
};

/// A minute of the world's time, in milliseconds.
pub const MINUTE: i64 = 60_000;

/// The digits of a digest's key, and so the depth of its last level.
pub const DIGITS: u32 = 30;

/// The first minute the reference's 16-digit key could not write, 3^16: 2051-11-05T13:21Z.
pub const OLD_LIMIT_MINUTES: i64 = 43_046_721;

/// The last wall time of an afternoon that starts at the last minute a signed 64-bit
/// millisecond clock holds, less two days: an afternoon is a few hours.
pub const LAST_EPOCH: i64 = i64::MAX / MINUTE * MINUTE - 2 * 24 * 60 * MINUTE;

/// A register's element, as the document makes it: `id * 65536 + field`.
pub fn element_of(id: u64, field: u64) -> i128 {
    i128::from(id) * 65_536 + i128::from(field)
}

/// A copy as it stands, to be restored later.
pub fn save(world: &mut World, who: &str) -> SavedState {
    world
        .actor::<TodoPeer>(who)
        .engine()
        .save_at(0)
        .expect("a running machine saves")
}

/// The copy killed and started again from `saved`, which is older than what it held.
pub fn restore_from(world: &mut World, who: &str, saved: SavedState) {
    world.restart::<TodoPeer>(who, move |_, clock| {
        Engine::<TodoPeer>::restore_with(TodoPeer::new(), &saved, clock, 0)
            .expect("what a machine saved restores")
    });
}

/// The host tells the copy its link is back.
pub fn resync(world: &mut World, who: &str) {
    world.raise(who, "todo.resync", "");
}

/// Everything sent on the network from the trace line `mark` on: who sent it, to whom, the
/// event and its data.
pub fn sent_since(world: &World, mark: usize) -> Vec<(String, String, String, Value)> {
    world
        .trace()
        .into_iter()
        .skip(mark)
        .filter(|line| matches!(line.fate, Fate::Sent { .. }))
        .map(|line| {
            let envelope = Envelope::decode(&mut SceCursor::new(&line.bytes)).expect("an envelope");
            let data = serde_json::from_slice(envelope.data).unwrap_or(Value::Null);
            (
                line.from.clone(),
                line.to.clone(),
                envelope.event_type.to_string(),
                data,
            )
        })
        .collect()
}

/// The minutes of the writes and supersedes among `sent`: the changes that were sent again.
pub fn minutes_of_changes(sent: &[(String, String, String, Value)]) -> Vec<i128> {
    let mut minutes: Vec<i128> = sent
        .iter()
        .filter(|(_, _, event, _)| event == "todo.write" || event == "todo.supersede")
        .map(|(_, _, _, data)| whole(&data["wallTime"]) / i128::from(MINUTE))
        .collect();
    minutes.sort_unstable();
    minutes.dedup();
    minutes
}

/// The hash of one change, written here a second time beside the library's
/// (merge/merkle_change_hash): the element, the wall time and the node each as a low and a
/// high 32-bit half, then the counter and the kind (0 for an add, 1 for a tombstone), one
/// step at a time, then the avalanche of MurmurHash3's fmix32.
pub fn change_hash(kind: i128, element: i128, wall: i128, counter: i128, node: i128) -> i128 {
    const M: i128 = 4_294_967_296;
    let mut h = 0;
    for part in [
        element % M,
        element / M,
        wall % M,
        wall / M,
        counter,
        node % M,
        node / M,
        kind + 1,
    ] {
        h = (h * 1_000_003 + part) % M;
    }
    h ^= h / 65_536;
    h = (h * 0x85eb_ca6b) % M;
    h ^= h / 8_192;
    h = (h * 0xc2b2_ae35) % M;
    h ^= h / 65_536;
    h
}

/// A digest as a map from (depth, prefix) to hash, without the nodes whose hash is 0: a
/// node that every change under it cancelled out is a node that holds nothing.
pub type Trie = BTreeMap<(u64, u64), i128>;

/// The digest a copy keeps.
pub fn digest_of(world: &mut World, who: &str) -> Trie {
    let variables = todo_variables_of(world, who);
    variables["digest"]
        .as_array()
        .expect("the digest is a list")
        .iter()
        .map(|node| {
            (
                (whole(&node["depth"]) as u64, whole(&node["prefix"]) as u64),
                whole(&node["hash"]),
            )
        })
        .filter(|(_, hash)| *hash != 0)
        .collect()
}

/// The digest of changes, each a (minute, hash), built as the definition says: the hash is
/// XORed into the root and into the node of each depth whose prefix is the minute divided
/// by 3^(30 - depth).
pub fn trie_of(changes: &[(i128, i128)]) -> Trie {
    let mut trie = Trie::new();
    for &(minute, hash) in changes {
        for depth in 0..=DIGITS {
            let prefix = (minute / 3i128.pow(DIGITS - depth)) as u64;
            *trie.entry((u64::from(depth), prefix)).or_insert(0) ^= hash;
        }
    }
    trie.retain(|_, hash| *hash != 0);
    trie
}

/// The digest of what a copy holds, built from its live writes and its tombstones and from
/// nothing the copy keeps as a digest.
pub fn digest_of_what_is_held(world: &mut World, who: &str) -> Trie {
    let variables = todo_variables_of(world, who);
    let mut changes: Vec<(i128, i128)> = Vec::new();
    for value in variables["values"].as_array().expect("values are a list") {
        let n = |name: &str| whole(&value[name]);
        let wall = n("wallTime");
        changes.push((
            wall / i128::from(MINUTE),
            change_hash(
                0,
                element_of(n("id") as u64, n("field") as u64),
                wall,
                n("counter"),
                n("nodeId"),
            ),
        ));
    }
    for tombstone in variables["tombstones"]
        .as_array()
        .expect("tombstones are a list")
    {
        let n = |name: &str| whole(&tombstone[name]);
        let wall = n("wallTime");
        changes.push((
            wall / i128::from(MINUTE),
            change_hash(1, n("element"), wall, n("counter"), n("nodeId")),
        ));
    }
    trie_of(&changes)
}

/// Every copy keeps the digest of what it holds, and the three keep one.
pub fn agreed_digest(world: &mut World) -> Trie {
    for who in ["alice", "bob", "server"] {
        assert_eq!(
            digest_of(world, who),
            digest_of_what_is_held(world, who),
            "{who} keeps a digest that is not the digest of what it holds"
        );
    }
    let server = digest_of(world, "server");
    for who in ["alice", "bob"] {
        assert_eq!(
            digest_of(world, who),
            server,
            "{who} and the hub keep different digests"
        );
    }
    server
}

pub fn refusals(world: &mut World, who: &str) -> Value {
    todo_variables_of(world, who)["refusals"].clone()
}

/// The three copies stand alike and as the oracle says, keep their digests, and refused
/// nothing; the list they agree on.
pub fn assert_converged(world: &mut World, context: &str) -> BTreeMap<(u64, u64), Field> {
    for who in ["alice", "bob", "server"] {
        assert_eq!(refusals(world, who), 0, "{context}: {who} refused a change");
    }
    let list = agreed_todos(world);
    assert_eq!(list, todo_oracle(world), "{context}");
    agreed_digest(world);
    assert!(world.host_errors().is_empty(), "{context}");
    list
}

/// A person gives a to-do a title, when the host's clock reads `epoch` and the world's time.
pub fn title_at(world: &mut World, who: &str, id: u64, title: &str, epoch: i64) {
    let now = epoch + world.now_ms();
    todo_edit_at(world, who, id, TITLE, 0, title, now);
}

/// A person changes the number of a field of a to-do, when the host's clock reads `epoch`
/// and the world's time.
pub fn set_at(world: &mut World, who: &str, id: u64, field: u64, value: i64, epoch: i64) {
    let now = epoch + world.now_ms();
    todo_edit_at(world, who, id, field, value, "", now);
}

/// Some to-dos written across the first half hour, a save of alice's copy, and then more
/// changes by both people in the minutes after it: the changes alice's copy would lose
/// if it came back from the save.
pub fn alice_saved_then_both_write_on(epoch: i64) -> (World, SavedState) {
    let mut world = todo_world(7, 20);
    assert_eq!(world.run_until_quiet(1), Outcome::Quiet);
    title_at(&mut world, "alice", 1, "pay rent", epoch);
    set_at(&mut world, "bob", 1, DUE, 5, epoch);
    set_at(&mut world, "alice", 2, DONE, 0, epoch);
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);
    world.advance_ms(30 * MINUTE);

    let old = save(&mut world, "alice");

    world.advance_ms(7 * MINUTE);
    title_at(&mut world, "bob", 1, "pay the rent", epoch);
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);
    world.advance_ms(5 * MINUTE);
    set_at(&mut world, "alice", 3, DUE, 9, epoch);
    title_at(&mut world, "alice", 2, "buy bread", epoch);
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);
    world.advance_ms(11 * MINUTE);
    set_at(&mut world, "bob", 3, DONE, 1, epoch);
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);
    (world, old)
}

/// A person's copy brought back from a save older than what the others have been sent
/// holds less than they do, and stays that way: nothing the network does repairs it. Told
/// that its link is back, it ends with the list the others hold, and the copies exchange
/// the changes of the minutes they differed in and no others. All of it for an afternoon
/// that starts at wall time `epoch`, a whole number of minutes.
pub fn assert_alice_is_repaired(epoch: i64) {
    assert_eq!(epoch % MINUTE, 0, "an epoch is a whole minute");
    let (mut world, old) = alice_saved_then_both_write_on(epoch);
    let first = i128::from(epoch / MINUTE);
    let whole_list = assert_converged(&mut world, "before the restore");
    assert_eq!(
        whole_list[&(1, TITLE)].text.as_deref(),
        Some("pay the rent")
    );
    assert_eq!(whole_list[&(3, DONE)].value, 1);

    restore_from(&mut world, "alice", old);
    let lost = todos_of(&mut world, "alice");
    assert_ne!(lost, whole_list, "alice came back without what came after");
    assert_eq!(lost[&(1, TITLE)].text.as_deref(), Some("pay rent"));
    assert!(!lost.contains_key(&(3, DUE)));

    // The links are up and the others believe alice has everything they sent her, so
    // there is nothing left for the network to do, and alice stays as she came back.
    assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet);
    assert_eq!(
        todos_of(&mut world, "alice"),
        lost,
        "nothing repaired alice"
    );
    assert_ne!(
        digest_of(&mut world, "alice"),
        digest_of(&mut world, "server")
    );

    let mark = world.trace().len();
    resync(&mut world, "alice");
    assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet);

    let list = assert_converged(&mut world, "after the resync");
    assert_eq!(list, whole_list);
    assert_eq!(
        list[&(1, TITLE)].live.len(),
        1,
        "the old title was written over"
    );

    // What was sent again is what alice lacked, from the minutes after the save, and the
    // minute of the title bob wrote over: a tombstone is in the minute of the dot it names, so
    // the minute of the first title is one the copies differ in. The half hour between,
    // which the copies agreed on, is not sent.
    let sent = sent_since(&world, mark);
    let minutes: Vec<i128> = minutes_of_changes(&sent)
        .into_iter()
        .map(|minute| minute - first)
        .collect();
    assert!(!minutes.is_empty(), "alice was sent what she lacked");
    assert!(
        minutes.iter().all(|minute| *minute == 0 || *minute >= 30),
        "changes of the minutes the copies agreed on were sent again: {minutes:?}"
    );
    assert!(
        minutes.iter().any(|minute| *minute >= 30),
        "the changes made after the save were sent: {minutes:?}"
    );
    assert!(
        sent.iter()
            .all(|(from, _, _, _)| from == "alice" || from == "server"),
        "bob took no part: he agreed with the hub"
    );
}

/// What an afternoon of random changes is made of.
pub struct Afternoon<'a> {
    /// The seeds to play, one afternoon each.
    pub seeds: Range<u64>,
    /// The to-dos the people change.
    pub ids: &'a [u64],
    /// The numeric fields they change, each with the count its value is drawn below.
    pub fields: &'a [(u64, u64)],
    /// The field and value a copy is written to with before its link is back.
    pub late: (u64, i64),
    /// The wall times the afternoons start at; seed `n` takes entry `n % len`.
    pub epochs: &'a [i64],
    /// The latest wall time of the list each afternoon must have reached, in milliseconds.
    pub reaches: i64,
    /// How many restores of a copy that had lost changes the run must have made.
    pub repaired_at_least: usize,
}

/// An afternoon of changes at random times across a few hours, a copy brought back now and
/// then from an older save of itself and told its link is back, links cut and restored,
/// delays spread out and envelopes delivered twice: for every seed the three copies end
/// alike, as the oracle says, each with the digest of what it holds.
///
/// A copy is restored only once the three agree, so the other two always hold what it lost.
pub fn random_afternoon(shape: &Afternoon) {
    const TITLES: [&str; 5] = ["milk", "post office", "\u{d560}\u{c77c}", "tax", "gym at 7"];
    let pick = |rng: &mut Rng, count: usize| usize::try_from(rng.below(count as u64)).unwrap();
    let mut repaired = 0;
    for seed in shape.seeds.clone() {
        let epoch = shape.epochs[usize::try_from(seed).unwrap() % shape.epochs.len()];
        let mut rng = Rng::new(seed);
        let mut world = todo_world(seed, 5);
        assert_eq!(world.run_until_quiet(1), Outcome::Quiet, "seed {seed}");
        for who in ["alice", "bob"] {
            world.jitter(who, "server", 40);
            world.reorder(who, "server");
            world.degrade(who, "server", 0, 20);
        }
        let names = ["alice", "bob", "server"];
        let mut saves: [Option<SavedState>; 3] = [None, None, None];
        let mut cut = [false, false];
        for round in 0..6 {
            for _ in 0..12 {
                world.advance_ms(i64::try_from(rng.below(4 * 60_000)).unwrap() + 1);
                let who = if rng.below(2) == 0 { "alice" } else { "bob" };
                let id = shape.ids[pick(&mut rng, shape.ids.len())];
                let op = pick(&mut rng, shape.fields.len() + 1);
                if op == 0 {
                    let title = TITLES[pick(&mut rng, TITLES.len())];
                    title_at(&mut world, who, id, title, epoch);
                } else {
                    let (field, count) = shape.fields[op - 1];
                    let value = i64::try_from(rng.below(count)).unwrap();
                    set_at(&mut world, who, id, field, value, epoch);
                }
                if rng.below(6) == 0 {
                    let which = usize::from(rng.below(2) == 1);
                    if cut[which] {
                        world.heal(names[which], "server");
                    } else {
                        world.partition(names[which], "server");
                    }
                    cut[which] = !cut[which];
                }
                world.run_until_quiet(i64::try_from(rng.below(60)).unwrap() + 1);
            }
            for (which, name) in names.iter().take(2).enumerate() {
                if cut[which] {
                    world.heal(name, "server");
                    cut[which] = false;
                }
            }
            assert_eq!(
                world.run_until_quiet(600_000),
                Outcome::Quiet,
                "seed {seed} round {round}"
            );
            let context = format!("seed {seed} round {round}, before a restore");
            assert_converged(&mut world, &context);

            // One copy comes back from the older save it kept, when it kept one.
            let victim = pick(&mut rng, 3);
            if let Some(old) = saves[victim].take() {
                let who = names[victim];
                let had = digest_of(&mut world, who);
                restore_from(&mut world, who, old);
                let lost = digest_of(&mut world, who) != had;
                // The copy is written to before its link is back, now and then.
                if rng.below(3) == 0 {
                    world.advance_ms(i64::try_from(rng.below(60_000)).unwrap() + 1);
                    if who != "server" {
                        let id = shape.ids[pick(&mut rng, shape.ids.len())];
                        set_at(&mut world, who, id, shape.late.0, shape.late.1, epoch);
                    }
                }
                resync(&mut world, who);
                if rng.below(4) == 0 {
                    resync(&mut world, names[pick(&mut rng, 3)]);
                }
                assert_eq!(
                    world.run_until_quiet(600_000),
                    Outcome::Quiet,
                    "seed {seed} round {round}"
                );
                repaired += usize::from(lost);
                let context = format!("seed {seed} round {round}, {who} restored");
                assert_converged(&mut world, &context);
            }
            for (which, save_of) in saves.iter_mut().enumerate() {
                if rng.below(2) == 0 {
                    *save_of = Some(save(&mut world, names[which]));
                }
            }
        }
        let list = agreed_todos(&mut world);
        assert!(!list.is_empty(), "seed {seed}: the run wrote something");
        let latest = list
            .values()
            .map(|field| field.stamp.0)
            .max()
            .expect("a list that is not empty has a latest stamp");
        assert!(
            i128::from(latest) >= i128::from(shape.reaches),
            "seed {seed}: the latest stamp is {latest}, before the {} the run was to reach",
            shape.reaches
        );
    }
    assert!(
        repaired >= shape.repaired_at_least,
        "the run restored copies that had lost changes only {repaired} times"
    );
}
