// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A copy of the shared to-do list that has lost changes for good, and what finds them.
//
// A copy restarted from an OLDER save comes back without changes its peers hold, and
// nothing repairs that by itself: the Mesh retries what was not delivered, and these were
// delivered, so the peers believe the copy has them and send them no more. The host tells
// the copy its link is back (`todo.resync`) and the copies walk their merkle digests one
// node to a message to find the minutes they differ in, and send each other only the
// changes of those minutes, as the ordinary writes and supersedes they were. What is held
// here is that the copies then end with one list, that it is the list the changes meant
// (the oracle reads the network, not the copies), that each copy's digest is the digest of
// exactly what it holds, and that the exchange does not resend what the copies agreed on.

mod common;

use common::resync::*;
use common::*;
use sce_forge_runtime::codec::SceCursor;
use sce_rust_mesh::generated::envelope::Envelope;
use sce_rust_runtime::saved_state::SavedState;
use sce_rust_runtime::Engine;
use sce_rust_world::{Fate, Outcome, Rng, World};
use serde_json::Value;

/// A copy as it stands, to be restored later.
fn save(world: &mut World, who: &str) -> SavedState {
    world
        .actor::<TodoPeer>(who)
        .engine()
        .save_at(0)
        .expect("a running machine saves")
}

/// The copy killed and started again from `saved`, which is older than what it held.
fn restore_from(world: &mut World, who: &str, saved: SavedState) {
    world.restart::<TodoPeer>(who, move |_, clock| {
        Engine::<TodoPeer>::restore_with(TodoPeer::new(), &saved, clock, 0)
            .expect("what a machine saved restores")
    });
}

/// The host tells the copy its link is back.
fn resync(world: &mut World, who: &str) {
    world.raise(who, "todo.resync", "");
}

/// Everything sent on the network from the trace line `mark` on: who sent it, to whom, the
/// event and its data.
fn sent_since(world: &World, mark: usize) -> Vec<(String, String, String, Value)> {
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
fn minutes_of_changes(sent: &[(String, String, String, Value)]) -> Vec<i128> {
    let mut minutes: Vec<i128> = sent
        .iter()
        .filter(|(_, _, event, _)| event == "todo.write" || event == "todo.supersede")
        .map(|(_, _, _, data)| whole(&data["wallTime"]) / i128::from(MINUTE))
        .collect();
    minutes.sort_unstable();
    minutes.dedup();
    minutes
}

/// Some to-dos written across the first half hour, a save of alice's copy, and then more
/// changes by both people in the minutes after it: the changes alice's copy would lose
/// if it came back from the save.
fn alice_saved_then_both_write_on() -> (World, SavedState) {
    let mut world = todo_world(7, 20);
    assert_eq!(world.run_until_quiet(1), Outcome::Quiet);
    todo_title(&mut world, "alice", 1, "pay rent");
    todo_set(&mut world, "bob", 1, DUE, 5);
    todo_set(&mut world, "alice", 2, DONE, 0);
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);
    world.advance_ms(30 * MINUTE);

    let old = save(&mut world, "alice");

    world.advance_ms(7 * MINUTE);
    todo_title(&mut world, "bob", 1, "pay the rent");
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);
    world.advance_ms(5 * MINUTE);
    todo_set(&mut world, "alice", 3, DUE, 9);
    todo_title(&mut world, "alice", 2, "buy bread");
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);
    world.advance_ms(11 * MINUTE);
    todo_set(&mut world, "bob", 3, DONE, 1);
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);
    (world, old)
}

/// A person's copy brought back from a save older than what the others have been sent
/// holds less than they do, and stays that way: nothing the network does repairs it. Told
/// that its link is back, it ends with the list the others hold, and the copies exchange
/// the changes of the minutes they differed in and no others.
#[test]
fn a_person_restored_from_an_old_save_is_repaired_by_resync() {
    let (mut world, old) = alice_saved_then_both_write_on();
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
    let minutes = minutes_of_changes(&sent);
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

/// The hub restored from an old save has lost what both people sent it, and the two
/// people are what it finds it again from.
#[test]
fn the_hub_restored_from_an_old_save_is_repaired_by_resync() {
    let mut world = todo_world(11, 20);
    assert_eq!(world.run_until_quiet(1), Outcome::Quiet);
    todo_title(&mut world, "alice", 1, "tidy desk");
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);
    world.advance_ms(20 * MINUTE);
    let old = save(&mut world, "server");

    world.advance_ms(3 * MINUTE);
    todo_set(&mut world, "alice", 1, DUE, 40);
    todo_title(&mut world, "bob", 2, "water plants");
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);
    world.advance_ms(9 * MINUTE);
    todo_title(&mut world, "bob", 1, "tidy the desk");
    todo_set(&mut world, "alice", 2, DONE, 1);
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);
    let whole_list = assert_converged(&mut world, "before the restore");

    restore_from(&mut world, "server", old);
    assert_ne!(todos_of(&mut world, "server"), whole_list);
    assert_eq!(
        todo_variables_of(&mut world, "server")["hub"],
        true,
        "the hub came back as the hub"
    );
    assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet);
    assert_ne!(todos_of(&mut world, "server"), whole_list);

    resync(&mut world, "server");
    assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet);
    assert_eq!(assert_converged(&mut world, "after the resync"), whole_list);
}

/// A copy that comes back holding a value the others have since written over is told the
/// tombstone, not only the new value: the old one leaves the field and the digest.
#[test]
fn a_value_written_over_while_a_copy_was_away_is_taken_out_of_it() {
    let mut world = todo_world(3, 20);
    assert_eq!(world.run_until_quiet(1), Outcome::Quiet);
    todo_title(&mut world, "alice", 1, "draft");
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);
    world.advance_ms(10 * MINUTE);
    let old = save(&mut world, "alice");
    world.advance_ms(10 * MINUTE);
    todo_title(&mut world, "bob", 1, "final");
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);

    restore_from(&mut world, "alice", old);
    assert_eq!(
        todos_of(&mut world, "alice")[&(1, TITLE)].text.as_deref(),
        Some("draft")
    );
    resync(&mut world, "alice");
    assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet);

    let list = assert_converged(&mut world, "after the resync");
    assert_eq!(list[&(1, TITLE)].live.len(), 1);
    assert_eq!(list[&(1, TITLE)].text.as_deref(), Some("final"));
}

/// A copy that has lost its own writes is sent them back, and its clock goes up to them: the
/// next thing it writes is stamped after every one it wrote before, and so writes over them.
#[test]
fn a_copy_that_lost_its_own_writes_is_stamped_after_them_once_they_are_back() {
    let mut world = todo_world(5, 20);
    assert_eq!(world.run_until_quiet(1), Outcome::Quiet);
    todo_title(&mut world, "alice", 4, "first");
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);
    let old = save(&mut world, "alice");
    world.advance_ms(MINUTE);
    todo_title(&mut world, "alice", 4, "second");
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);
    let written = todos_of(&mut world, "server")[&(4, TITLE)].stamp;

    restore_from(&mut world, "alice", old);
    resync(&mut world, "alice");
    assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet);
    assert_eq!(agreed_todos(&mut world)[&(4, TITLE)].stamp, written);

    // The host's clock is behind the stamps alice wrote; her own clock is not.
    todo_edit_at(&mut world, "alice", 4, TITLE, 0, "third", 0);
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);
    let list = assert_converged(&mut world, "after the third title");
    let title = &list[&(4, TITLE)];
    assert_eq!(title.live.len(), 1, "the third title wrote over the second");
    assert_eq!(title.text.as_deref(), Some("third"));
    assert!(
        title.stamp > written,
        "the third title is stamped {:?}, after the second {written:?}",
        title.stamp
    );
}

/// Copies that agree say so with one message and no more: the root of the sender's digest,
/// which the receiver finds equal to its own and does not answer.
#[test]
fn copies_that_agree_exchange_one_node_and_nothing_else() {
    let (mut world, _) = alice_saved_then_both_write_on();
    assert_converged(&mut world, "before the resync");
    let mark = world.trace().len();
    resync(&mut world, "alice");
    assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet);

    let sent = sent_since(&world, mark);
    assert_eq!(sent.len(), 1, "{sent:?}");
    let (from, to, event, data) = &sent[0];
    assert_eq!((from.as_str(), to.as_str()), ("alice", "server"));
    assert_eq!(event, "todo.node");
    assert_eq!((whole(&data["depth"]), whole(&data["prefix"])), (0, 0));
    assert_eq!(
        whole(&data["hash"]),
        digest_of(&mut world, "alice")[&(0, 0)],
        "it is the root of alice's digest"
    );
    assert_converged(&mut world, "after the resync");
}

/// A resync is idempotent and takes no one's turn: told twice, by every copy, over links that
/// duplicate and delay what they carry, the copies end alike.
#[test]
fn a_resync_told_again_and_by_every_copy_changes_nothing() {
    let (mut world, old) = alice_saved_then_both_write_on();
    let whole_list = assert_converged(&mut world, "before the restore");
    for who in ["alice", "bob"] {
        world.jitter(who, "server", 30);
        world.reorder(who, "server");
        world.degrade(who, "server", 0, 40);
    }
    restore_from(&mut world, "alice", old);
    for who in ["alice", "alice", "server", "bob", "alice"] {
        resync(&mut world, who);
    }
    assert_eq!(world.run_until_quiet(600_000), Outcome::Quiet);
    assert_eq!(
        assert_converged(&mut world, "after the resyncs"),
        whole_list
    );

    let before = todos_of(&mut world, "alice");
    resync(&mut world, "alice");
    resync(&mut world, "server");
    assert_eq!(world.run_until_quiet(600_000), Outcome::Quiet);
    assert_eq!(todos_of(&mut world, "alice"), before);
    assert_converged(&mut world, "after one more");
}

/// Changes a lossy link dropped for good are the same kind of loss, and a resync finds
/// them as it finds those of a restore: the loss is on the link between alice and the hub,
/// so the hub and bob never had what alice wrote, and alice never had what bob wrote.
#[test]
fn what_a_lossy_link_dropped_is_found_by_resync() {
    let mut world = todo_world(13, 20);
    assert_eq!(world.run_until_quiet(1), Outcome::Quiet);
    todo_title(&mut world, "alice", 1, "start");
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);
    world.degrade("alice", "server", 100, 0);
    world.advance_ms(2 * MINUTE);
    todo_title(&mut world, "alice", 1, "alice's second");
    todo_set(&mut world, "alice", 2, DONE, 1);
    world.advance_ms(4 * MINUTE);
    todo_set(&mut world, "bob", 3, DUE, 12);
    assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet);
    assert_ne!(
        todos_of(&mut world, "alice"),
        todos_of(&mut world, "server")
    );

    world.degrade("alice", "server", 0, 0);
    resync(&mut world, "alice");
    assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet);
    let list = assert_converged(&mut world, "after the resync");
    assert_eq!(list[&(1, TITLE)].text.as_deref(), Some("alice's second"));
    assert_eq!(list[&(3, DUE)].value, 12);
}

/// An afternoon of changes at random times across a few hours, a copy brought back now and
/// then from an older save of itself and told its link is back, links cut and restored,
/// delays spread out and envelopes delivered twice: for every seed the three copies end
/// alike, as the oracle says, each with the digest of what it holds.
///
/// A copy is restored only once the three agree, so the other two always hold what it lost.
#[test]
fn a_random_afternoon_with_restores_from_old_saves_ends_with_one_list() {
    const TITLES: [&str; 5] = ["milk", "post office", "\u{d560}\u{c77c}", "tax", "gym at 7"];
    let mut repaired = 0;
    for seed in 0..30 {
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
                let id = 1 + rng.below(4);
                match rng.below(4) {
                    0 => {
                        let title = TITLES[usize::try_from(rng.below(5)).unwrap()];
                        todo_title(&mut world, who, id, title);
                    }
                    1 => todo_set(
                        &mut world,
                        who,
                        id,
                        DONE,
                        i64::try_from(rng.below(2)).unwrap(),
                    ),
                    2 => todo_set(
                        &mut world,
                        who,
                        id,
                        DUE,
                        i64::try_from(rng.below(365)).unwrap(),
                    ),
                    _ => todo_set(
                        &mut world,
                        who,
                        id,
                        REMOVED,
                        i64::try_from(rng.below(2)).unwrap(),
                    ),
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
            let victim = usize::try_from(rng.below(3)).unwrap();
            if let Some(old) = saves[victim].take() {
                let who = names[victim];
                let had = digest_of(&mut world, who);
                restore_from(&mut world, who, old);
                let lost = digest_of(&mut world, who) != had;
                // The copy is written to before its link is back, now and then.
                if rng.below(3) == 0 {
                    world.advance_ms(i64::try_from(rng.below(60_000)).unwrap() + 1);
                    if who != "server" {
                        todo_set(&mut world, who, 1 + rng.below(4), DUE, 400);
                    }
                }
                resync(&mut world, who);
                if rng.below(4) == 0 {
                    resync(&mut world, names[usize::try_from(rng.below(3)).unwrap()]);
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
        assert!(
            !agreed_todos(&mut world).is_empty(),
            "seed {seed}: the run wrote something"
        );
    }
    assert!(
        repaired >= 20,
        "the run restored copies that had lost changes only {repaired} times"
    );
}
