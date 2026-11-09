// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A copy of the shared to-do list restored from an older save, and what keeps its peers whole.
//
// A copy set up again from a save older than what it made comes back without knowing the
// highest number its causal layer gave a change of its own (docs/adr/0085, section 3.3). The
// change with that number may be held only by a peer that is offline, so the peers it can reach
// cannot say what the number was: a copy that took the highest number they report and went on
// from it would give its next change the number of a change it already made, one dot would
// name two contents, and the copies that took either would never take the other. The copy
// begins a new incarnation instead, with a replica identity of its own, so no number is used
// twice. Its old incarnation's changes stay delivered, and the old incarnation's column holds
// the stable cut where it stopped. Until view changes land (landing 7) the view of epoch 0 is
// the only one, and a second identity for a member is not delivered, so the copy is cut off
// from its group until then; what it makes stays its own.
//
// This is where the list's resync, which walked a digest to find what a restored copy lacked,
// was held: the copy's journal now says what it took in, a copy that has one continues from it
// (a restart), and one that has not begins anew (this file). The repair of what a link lost is
// held in a_lost_change_is_repaired_at_every_wall_time_and_field_number.rs.
//
// The negative arm is kept: the same run with the copy continuing from the number its
// reachable peer reports ends with one dot naming two contents and the copies apart.

mod common;

use common::repair::*;
use common::*;
use sce_rust_mesh::causal::{JournalRecord, Start};
use sce_rust_runtime::saved_state::SavedState;
use sce_rust_runtime::Engine;
use sce_rust_world::{Outcome, World};

/// A copy as it stands, to be restored later.
fn save(world: &mut World, who: &str) -> SavedState {
    world
        .actor::<TodoPeer>(who)
        .engine()
        .save_at(0)
        .expect("a running machine saves")
}

/// The machine of `who` killed and started again from `saved`, which is older than what it
/// held. Its endpoint and links are kept; its causal layer is the host's to start again.
fn restore_machine(world: &mut World, who: &str, saved: SavedState) {
    world.restart::<TodoPeer>(who, move |_, clock| {
        Engine::<TodoPeer>::restore_with(TodoPeer::new(), &saved, clock, 0)
            .expect("what a machine saved restores")
    });
}

/// How a copy restored from an old save starts its causal layer again.
#[derive(Clone, Copy)]
enum Again {
    /// As docs/adr/0085 section 3.3.2 says: a new incarnation, with an identity of its own.
    NewIncarnation,
    /// The rule refused: its old identity, from the highest number of its own the peer it
    /// can reach reports.
    FromWhatAPeerReports,
}

/// Alice writes a first title, which every copy takes in, and saves. Cut off from bob, she
/// changes a due day, which only carol takes in; then her device is lost and set up again
/// from the old save while carol is offline, and her layer starts as `again` says. She writes
/// one more change, which bob can be reached for. Carol comes back, and the links all carry.
fn a_device_lost_and_set_up_again(again: Again) -> World {
    let mut world = todo_world(17, 20);
    todo_title(&mut world, "alice", 1, "first");
    assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet);
    let old = save(&mut world, "alice");

    world.partition("alice", "bob");
    world.partition("carol", "bob");
    todo_set(&mut world, "alice", 2, DUE, 20);
    world.advance_ms(MINUTE);
    assert!(todos_of(&mut world, "carol").contains_key(&(2, DUE)));
    assert!(!todos_of(&mut world, "bob").contains_key(&(2, DUE)));

    world.partition("alice", "carol");
    world.heal("alice", "bob");
    restore_machine(&mut world, "alice", old);
    let start = match again {
        Again::NewIncarnation => Start::NewIncarnation(replica_of("alice", 1)),
        Again::FromWhatAPeerReports => {
            let reported = world
                .causal_layer("bob", PEOPLE)
                .expect("bob's layer")
                .delivered()
                .to_vec();
            Start::Journal(JournalRecord {
                replica: replica_of("alice", 0),
                delivered: reported,
            })
        }
    };
    world.start_causal("alice", PEOPLE, start, LAYER_ROOM, VECTOR_EVERY_MS);
    todo_set(&mut world, "alice", 3, DUE, 30);
    world.advance_ms(MINUTE);

    world.heal("carol", "bob");
    world.heal("alice", "carol");
    world.advance_ms(10 * MINUTE);
    world
}

/// A copy restored from an old save begins a new incarnation: no dot names two contents, the
/// peers that held its old incarnation's changes end with one list, and what the new
/// incarnation makes is not delivered while the view of epoch 0 is the only one. Red if the
/// copy keeps its old identity, which the negative arm below runs.
#[test]
fn a_copy_restored_from_an_old_save_numbers_no_change_twice() {
    let mut world = a_device_lost_and_set_up_again(Again::NewIncarnation);
    no_dot_names_two_contents(&world).expect("one dot, one content");

    let list = agreed_todos_of(&mut world, &["bob", "carol"]);
    assert_eq!(list[&(1, TITLE)].text.as_deref(), Some("first"));
    assert_eq!(
        list[&(2, DUE)].value,
        20,
        "the old incarnation's last change reached bob from carol"
    );
    assert!(
        !list.contains_key(&(3, DUE)),
        "the new incarnation's change was delivered before view changes land"
    );
    assert!(todos_of(&mut world, "alice").contains_key(&(3, DUE)));
    for who in ["bob", "carol"] {
        assert_eq!(
            delivered_of(&mut world, who, replica_of("alice", 0)),
            2,
            "{who} took both of the old incarnation's changes"
        );
        assert_eq!(
            delivered_of(&mut world, who, replica_of("alice", 1)),
            0,
            "{who} took a change of an identity the view does not hold"
        );
    }
}

/// The old incarnation's column holds the stable cut where it stopped: its peers keep the
/// changes it never said it took in, for the view change that lands later to settle, and do
/// not forget them.
#[test]
fn an_old_incarnations_column_holds_the_stable_cut() {
    let mut world = a_device_lost_and_set_up_again(Again::NewIncarnation);
    for who in ["bob", "carol"] {
        let layer = world.causal_layer(who, PEOPLE).expect("a layer");
        assert!(
            layer.log_len() > 0,
            "{who} forgot a change the old incarnation never said it took in"
        );
    }
}

/// The negative arm: a copy that goes on under its old identity from the highest number the
/// peer it can reach reports gives its next change the dot of the change only carol holds, and
/// the copies that took either never take the other.
#[test]
fn a_copy_that_goes_on_from_what_a_peer_reports_numbers_a_change_twice() {
    let mut world = a_device_lost_and_set_up_again(Again::FromWhatAPeerReports);
    assert!(
        no_dot_names_two_contents(&world).is_err(),
        "the rule that a copy asks its peers for its number would leave one dot one content"
    );
    let bob = todos_of(&mut world, "bob");
    let carol = todos_of(&mut world, "carol");
    assert!(bob.contains_key(&(3, DUE)) && !bob.contains_key(&(2, DUE)));
    assert!(carol.contains_key(&(2, DUE)) && !carol.contains_key(&(3, DUE)));
    assert_ne!(
        bob, carol,
        "the copies agree though one dot names two changes"
    );
}

/// A copy that is killed and started again with its journal goes on where its layer was: the
/// machine comes back from what it saved last, its layer numbers its next change after the last
/// one it made, and the copies end with one list in which nothing is numbered twice.
#[test]
fn a_copy_restarted_with_its_journal_goes_on_from_its_own_number() {
    let mut world = todo_world(5, 20);
    todo_title(&mut world, "alice", 4, "first");
    assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet);
    let made = delivered_of(&mut world, "alice", replica_of("alice", 0));
    assert_eq!(made, 1);

    let saved = save(&mut world, "alice");
    restore_machine(&mut world, "alice", saved);
    todo_title(&mut world, "alice", 4, "second");
    assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet);

    let list = assert_converged(&mut world, "after the restart");
    assert_eq!(list[&(4, TITLE)].text.as_deref(), Some("second"));
    assert_eq!(
        list[&(4, TITLE)].live.len(),
        1,
        "the second wrote over the first"
    );
    for who in COPIES {
        assert_eq!(
            delivered_of(&mut world, who, replica_of("alice", 0)),
            2,
            "{who}"
        );
    }
    no_dot_names_two_contents(&world).expect("one dot, one content");
}
