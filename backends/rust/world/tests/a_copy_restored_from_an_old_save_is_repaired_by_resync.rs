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
//
// The same is held for wall times past 2051 and for field numbers in the thousands in
// a_resync_reaches_every_wall_time_and_every_field_number.rs.

mod common;

use common::resync::*;
use common::*;
use sce_rust_runtime::saved_state::SavedState;
use sce_rust_world::{Outcome, World};

/// A person's copy brought back from a save older than what the others have been sent
/// holds less than they do, and stays that way: nothing the network does repairs it. Told
/// that its link is back, it ends with the list the others hold, and the copies exchange
/// the changes of the minutes they differed in and no others.
#[test]
fn a_person_restored_from_an_old_save_is_repaired_by_resync() {
    assert_alice_is_repaired(0);
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
    let (mut world, _): (World, SavedState) = alice_saved_then_both_write_on(0);
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
    let (mut world, old) = alice_saved_then_both_write_on(0);
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
    random_afternoon(&Afternoon {
        seeds: 0..30,
        ids: &[1, 2, 3, 4],
        fields: &[(DONE, 2), (DUE, 365), (REMOVED, 2)],
        late: (DUE, 400),
        epochs: &[0],
        reaches: 0,
        repaired_at_least: 20,
    });
}
