// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// What the tests of the shared to-do list's repair have in common: a copy whose links lost
// changes for good, and the causal route that finds them again (docs/adr/0085, section 3.2).
//
// The list keeps no digest and walks none: the Mesh core's causal layer below it numbers
// every change by its author and that author's count of changes, and each copy sends its
// delivered vector, on a period and whenever a link comes back. A copy that is sent a vector
// answers with every change of its log the vector lacks, as the envelope it was sent in, and
// the copy that lacked it delivers it once, after what it follows. A change is found by its
// number, so its wall time and the numbers of its row and field play no part; the tests of
// this harness hold that at the wall times and the numbers the list's first forms could not
// reach.
//
// A test takes an `epoch`, the wall time its afternoon starts at, in milliseconds. Every
// change is stamped `epoch + world.now_ms()`, so one afternoon can be played in 1970, on
// either side of 3^16 minutes (2051-11-05T13:21Z, where the first form of the digest's key
// ran out) or at the last minute a signed 64-bit clock holds, and the checks are the same.

use std::collections::BTreeMap;
use std::ops::Range;

use sce_forge_runtime::codec::SceCursor;
use sce_rust_mesh::causal::{envelope_tag, Replica};
use sce_rust_mesh::generated::envelope::Envelope;
use sce_rust_mesh::generated::pattern_kind::PatternKind;
use sce_rust_world::{Fate, Outcome, Rng, World};
use serde_json::Value;

use super::{
    agreed_todos, assert_layers_rest, links_of, todo_edit_at, todo_oracle, todo_variables_of,
    todo_world, todos_of, whole, Field, COPIES, DONE, DUE, PEOPLE, TITLE,
};

/// A minute of the world's time, in milliseconds.
pub const MINUTE: i64 = 60_000;

/// The first minute the reference's 16-digit key could not write, 3^16: 2051-11-05T13:21Z.
pub const OLD_LIMIT_MINUTES: i64 = 43_046_721;

/// The last wall time of an afternoon that starts at the last minute a signed 64-bit
/// millisecond clock holds, less two days: an afternoon is a few hours.
pub const LAST_EPOCH: i64 = i64::MAX / MINUTE * MINUTE - 2 * 24 * 60 * MINUTE;

/// Every link of `who` to another copy loses everything it carries, both ways, or carries
/// it again (`lose` false): a loss no retry repairs and no link edge reports.
pub fn lose_everything(world: &mut World, who: &str, lose: bool) {
    for other in COPIES.iter().filter(|other| **other != who) {
        world.degrade(who, other, if lose { 100 } else { 0 }, 0);
    }
}

/// One change as the network carried it: who sent it, to whom, its dot as the change states
/// it (`by`, `wallTime`, `counter`) and its data.
#[derive(Debug, Clone)]
pub struct Carried {
    pub from: String,
    pub to: String,
    pub dot: (i128, i128, i128),
    pub change: Value,
}

/// Every change put on a link from the trace line `mark` on, each time it was, whether the
/// link carried it or lost it: a change the layer sent again in a repair is one more line.
pub fn changes_carried_since(world: &World, mark: usize) -> Vec<Carried> {
    world
        .trace()
        .into_iter()
        .skip(mark)
        .filter(|line| matches!(line.fate, Fate::Sent { .. } | Fate::Lost))
        .filter_map(|line| {
            let envelope = Envelope::decode(&mut SceCursor::new(&line.bytes)).ok()?;
            if envelope.pattern == PatternKind::CausalVector
                || envelope.event_type != "record_list.change"
            {
                return None;
            }
            let change: Value = serde_json::from_slice(envelope.data.into_inner()).ok()?;
            let dot = (
                whole(&change["by"]),
                whole(&change["wallTime"]),
                whole(&change["counter"]),
            );
            Some(Carried {
                from: line.from.clone(),
                to: line.to.clone(),
                dot,
                change,
            })
        })
        .collect()
}

/// How many vectors each copy put on the network from the trace line `mark` on, by sender.
pub fn vectors_sent_since(world: &World, mark: usize) -> BTreeMap<String, usize> {
    let mut sent = BTreeMap::new();
    for line in world.trace().into_iter().skip(mark) {
        if !matches!(line.fate, Fate::Sent { .. }) {
            continue;
        }
        let Ok(envelope) = Envelope::decode(&mut SceCursor::new(&line.bytes)) else {
            continue;
        };
        if envelope.pattern == PatternKind::CausalVector {
            *sent.entry(line.from.clone()).or_insert(0) += 1;
        }
    }
    sent
}

/// The dots of every change a copy sent from the trace line `mark` on, once each.
pub fn dots_made_since(world: &World, mark: usize) -> Vec<(i128, i128, i128)> {
    let mut dots: Vec<(i128, i128, i128)> = changes_carried_since(world, mark)
        .into_iter()
        .map(|carried| carried.dot)
        .collect();
    dots.sort_unstable();
    dots.dedup();
    dots
}

/// No dot of the causal layer named two contents on the network: every tagged change that
/// carried one dot (its author's replica and number, docs/adr/0085 section 3) carried the same
/// data. What a copy that numbered a second change with a dot it had used would break.
pub fn no_dot_names_two_contents(world: &World) -> Result<(), String> {
    let mut named: BTreeMap<(u64, u64, u64), Vec<u8>> = BTreeMap::new();
    for line in world.trace() {
        if !matches!(line.fate, Fate::Sent { .. }) {
            continue;
        }
        let Ok(envelope) = Envelope::decode(&mut SceCursor::new(&line.bytes)) else {
            continue;
        };
        if envelope.pattern == PatternKind::CausalVector {
            continue;
        }
        let Ok(Some(tag)) = envelope_tag(&envelope) else {
            continue;
        };
        let Some(own) = tag.vector.first() else {
            continue;
        };
        let data = envelope.data.into_inner().to_vec();
        match named.get(&(own.hi, own.lo, own.seq)) {
            Some(held) if *held != data => {
                return Err(format!(
                    "the dot ({:016x}{:016x}, {}) names two contents",
                    own.hi, own.lo, own.seq
                ));
            }
            Some(_) => {}
            None => {
                named.insert((own.hi, own.lo, own.seq), data);
            }
        }
    }
    Ok(())
}

/// The number of `who`'s layer's delivered vector for the author `author`, 0 when it names
/// none: how many of that replica's changes the copy took in.
pub fn delivered_of(world: &mut World, who: &str, author: Replica) -> u64 {
    let layer = world
        .causal_layer(who, PEOPLE)
        .unwrap_or_else(|| panic!("{who} has no causal layer"));
    layer
        .delivered()
        .iter()
        .find(|entry| Replica::of(entry) == author)
        .map_or(0, |entry| entry.seq)
}

/// `who`'s layer's delivered vector, as `(hi, lo, seq)` entries in their order of replica: two
/// copies that took in the same changes give the same.
pub fn vector_of(world: &mut World, who: &str) -> Vec<(u64, u64, u64)> {
    let layer = world
        .causal_layer(who, PEOPLE)
        .unwrap_or_else(|| panic!("{who} has no causal layer"));
    let mut vector: Vec<(u64, u64, u64)> = layer
        .delivered()
        .iter()
        .filter(|entry| entry.seq > 0)
        .map(|entry| (entry.hi, entry.lo, entry.seq))
        .collect();
    vector.sort_unstable();
    vector
}

pub fn refusals(world: &mut World, who: &str) -> Value {
    todo_variables_of(world, who)["refusals"].clone()
}

/// The three copies stand alike and as the oracle says, refused nothing, their causal
/// layers are at rest, and no host met a fault; the list they agree on.
pub fn assert_converged(world: &mut World, context: &str) -> BTreeMap<(u64, u64), Field> {
    for who in COPIES {
        assert_eq!(refusals(world, who), 0, "{context}: {who} refused a change");
    }
    let list = agreed_todos(world);
    assert_eq!(list, todo_oracle(world), "{context}");
    assert_layers_rest(world, &COPIES, context);
    let errors = world.host_errors();
    assert!(errors.is_empty(), "{context}: {errors:?}");
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

/// Some to-dos written across the first half hour; then alice's links lose everything,
/// both ways, while both people write on in the minutes after: the changes she does not
/// get, and the ones of hers no one gets. Her links carry again at the end, and nothing has
/// repaired anything yet. Returns the world and the trace line the losses began at.
pub fn alice_cut_off_while_both_write_on(epoch: i64) -> (World, usize) {
    let mut world = todo_world(7, 20);
    assert_eq!(world.run_until_quiet(1), Outcome::Quiet);
    title_at(&mut world, "alice", 1, "pay rent", epoch);
    set_at(&mut world, "bob", 1, DUE, 5, epoch);
    set_at(&mut world, "alice", 2, DONE, 0, epoch);
    assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet);
    world.advance_ms(30 * MINUTE);

    let mark = world.trace().len();
    lose_everything(&mut world, "alice", true);
    world.advance_ms(7 * MINUTE);
    title_at(&mut world, "bob", 1, "pay the rent", epoch);
    world.advance_ms(5 * MINUTE);
    set_at(&mut world, "alice", 3, DUE, 9, epoch);
    title_at(&mut world, "alice", 2, "buy bread", epoch);
    world.advance_ms(11 * MINUTE);
    set_at(&mut world, "carol", 3, DONE, 1, epoch);
    world.advance_ms(MINUTE);
    lose_everything(&mut world, "alice", false);
    (world, mark)
}

/// A person's copy whose links lost everything for a while holds less than the others, and
/// the others lack what it made. Once its links carry again, the layers' vectors find what
/// each lacks, with no word from any host: the copies end with the list the oracle says, and
/// what went on the network again is the changes made while the links lost them and none of
/// those the copies agreed on. All of it for an afternoon that starts at wall time `epoch`.
pub fn assert_alice_is_repaired(epoch: i64) {
    assert_eq!(epoch % MINUTE, 0, "an epoch is a whole minute");
    let (mut world, mark) = alice_cut_off_while_both_write_on(epoch);
    let lacked = todos_of(&mut world, "alice");
    assert!(
        !lacked.contains_key(&(3, DONE)),
        "alice was not given carol's change while her links lost everything"
    );
    let made_while_cut = dots_made_since(&world, mark);
    assert!(
        made_while_cut.len() >= 4,
        "the changes made while alice was cut off: {made_while_cut:?}"
    );

    let repaired = world.trace().len();
    assert_eq!(world.run_until_quiet(600_000), Outcome::Quiet);
    let list = assert_converged(&mut world, &format!("epoch {epoch}, after the repair"));
    assert_eq!(list[&(1, TITLE)].text.as_deref(), Some("pay the rent"));
    assert_eq!(
        list[&(1, TITLE)].live.len(),
        1,
        "the old title was written over"
    );
    assert_eq!(list[&(3, DONE)].value, 1);
    assert_eq!(list[&(2, TITLE)].text.as_deref(), Some("buy bread"));

    // What went on the network again is what one copy lacked: the changes made while alice
    // was cut off, and none made before, which every copy held.
    let again = changes_carried_since(&world, repaired);
    assert!(!again.is_empty(), "nothing was sent again");
    for carried in &again {
        assert!(
            made_while_cut.contains(&carried.dot),
            "epoch {epoch}: a change every copy held was sent again: {carried:?}"
        );
    }
}

/// What an afternoon of random changes is made of.
pub struct Afternoon<'a> {
    /// The seeds to play, one afternoon each.
    pub seeds: Range<u64>,
    /// The to-dos the people change.
    pub ids: &'a [u64],
    /// The numeric fields they change, each with the count its value is drawn below.
    pub fields: &'a [(u64, u64)],
    /// The wall times the afternoons start at; seed `n` takes entry `n % len`.
    pub epochs: &'a [i64],
    /// The latest wall time of the list each afternoon must have reached, in milliseconds.
    pub reaches: i64,
    /// How many times a copy must have lacked a change when its links carried again.
    pub repaired_at_least: usize,
}

/// An afternoon of changes at random times across a few hours, by all three copies, now and
/// then one copy's links losing everything for a while, links cut and restored, delays
/// spread out, envelopes reordered and delivered twice: for every seed the three copies end
/// alike and as the oracle says, with their layers at rest.
pub fn random_afternoon(shape: &Afternoon) {
    const TITLES: [&str; 5] = ["milk", "post office", "\u{d560}\u{c77c}", "tax", "gym at 7"];
    let pick = |rng: &mut Rng, count: usize| usize::try_from(rng.below(count as u64)).unwrap();
    let mut repaired = 0;
    for seed in shape.seeds.clone() {
        let epoch = shape.epochs[usize::try_from(seed).unwrap() % shape.epochs.len()];
        let mut rng = Rng::new(seed);
        let mut world = todo_world(seed, 5);
        assert_eq!(world.run_until_quiet(1), Outcome::Quiet, "seed {seed}");
        let links = links_of(&COPIES);
        for &(a, b) in &links {
            world.jitter(a, b, 40);
            world.reorder(a, b);
            world.degrade(a, b, 0, 20);
        }
        let mut cut = vec![false; links.len()];
        for round in 0..6 {
            // One copy's links lose everything for the round, now and then.
            let victim = (rng.below(2) == 0).then(|| COPIES[pick(&mut rng, 3)]);
            if let Some(who) = victim {
                lose_everything(&mut world, who, true);
            }
            for _ in 0..12 {
                world.advance_ms(i64::try_from(rng.below(4 * 60_000)).unwrap() + 1);
                let who = COPIES[pick(&mut rng, 3)];
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
                    let which = pick(&mut rng, links.len());
                    let (a, b) = links[which];
                    if cut[which] {
                        world.heal(a, b);
                    } else {
                        world.partition(a, b);
                    }
                    cut[which] = !cut[which];
                }
                world.advance_ms(i64::try_from(rng.below(60)).unwrap() + 1);
            }
            for (which, &(a, b)) in links.iter().enumerate() {
                if cut[which] {
                    world.heal(a, b);
                    cut[which] = false;
                }
            }
            if let Some(who) = victim {
                let others: Vec<&str> = COPIES.iter().copied().filter(|o| *o != who).collect();
                let lacked = vector_of(&mut world, who) != vector_of(&mut world, others[0]);
                repaired += usize::from(lacked);
                for other in &others {
                    world.degrade(who, other, 0, 20);
                }
            }
            assert_eq!(
                world.run_until_quiet(600_000),
                Outcome::Quiet,
                "seed {seed} round {round}"
            );
            assert_converged(&mut world, &format!("seed {seed} round {round}"));
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
        "a copy lacked a change when its links carried again only {repaired} times"
    );
}
