// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The repair of the shared to-do list, at the wall times and the field numbers the list's
// first forms could not reach.
//
// A change a link lost for good is found again by the layer below the list, the Mesh core's
// causal route (`delivery: causal`, docs/adr/0085 section 3.2): every copy sends its
// delivered vector on a period and when a link comes back, and a copy sent a vector answers
// with every change of its log the vector lacks, which the copy that lacked it takes in once,
// after what it follows. What a resync and a walk of the list's digest did is done there,
// once, for every document on the route. The first form of the digest keyed a change by its
// minute written in 16 base-3 digits, which stopped at 3^16 minutes (2051-11-05T13:21Z), and
// a register by `id * 16 + field`, which ran field 16 of one to-do into field 0 of the next.
// The layer keys a change by its author and number, so no wall time and no number a row or a
// field is given can put a change out of its reach. What is held here, for afternoons played
// on either side of 3^16 minutes, far past it and at the last minute a clock holds, and for
// to-dos and fields numbered as a long list numbers them: the copies end with one list, the
// oracle's, with their layers at rest; what is sent again is what a copy lacked and nothing the
// copies held alike; copies that agree exchange vectors and no change; a change the network
// gives again is taken in once; and registers the first form ran together are told apart.
//
// Each test is red under a list with no causal layer below it: a change a link lost is then
// never sent again, and the copies end apart (`a_change_lost_on_a_plain_route_is_never_found`
// holds that, kept as the negative arm).

mod common;

use common::repair::*;
use common::*;
use sce_forge_runtime::codec::SceCursor;
use sce_rust_mesh::generated::envelope::Envelope;
use sce_rust_mesh::generated::pattern_kind::PatternKind;
use sce_rust_mesh::router::Router;
use sce_rust_world::{Fate, Outcome, World};

/// 2100-01-01T00:00Z, in milliseconds.
const YEAR_2100: i64 = 4_102_444_800_000;

/// A wall time rounded down to a whole minute, as an epoch is.
fn minutes(millis: i64) -> i64 {
    millis / MINUTE * MINUTE
}

/// The wall times the afternoons start at, each past 2051 or about to cross it: ninety
/// minutes before 3^16 minutes, so that an afternoon straddles it; just after it; 2100; the
/// minute where the old key's leading digit went from 1 to 2, 3^29 minutes in, straddled the
/// same way; a hundred and forty thousand years on; and the last two days a clock holds.
fn far_epochs() -> Vec<i64> {
    vec![
        (OLD_LIMIT_MINUTES - 90) * MINUTE,
        OLD_LIMIT_MINUTES * MINUTE,
        YEAR_2100,
        (2 * 3i64.pow(29) - 90) * MINUTE,
        minutes(4_000_000_000_000_000),
        LAST_EPOCH,
    ]
}

/// The fields of a long list: the host's own, and the numbers at which the first form of
/// the element (`id * 16 + field`) ran registers together and where the halves of the new one
/// meet.
const WIDE_FIELDS: [u64; 8] = [16, 17, 100, 300, 1000, 4096, 65_534, 65_535];

/// The to-dos of a long list: small, past the 2^21 at which a number is too large for the
/// Interpreter's integers when packed by 2^32, and the largest a uint32 holds.
const WIDE_IDS: [u64; 5] = [1, 2, 70_000, 3_000_000, 4_294_967_295];

/// An afternoon in 1970 in which alice's links lose everything for half an hour: once they
/// carry again she ends with the list the others hold, they with what she made, and what went
/// on the network again is what one of them lacked.
#[test]
fn a_person_whose_links_lost_everything_is_repaired() {
    assert_alice_is_repaired(0);
}

/// The same at every wall time: whether the changes are made before 3^16 minutes, after it,
/// or straddle it.
#[test]
fn a_person_whose_links_lost_everything_is_repaired_at_every_wall_time() {
    for epoch in far_epochs() {
        assert_alice_is_repaired(epoch);
    }
}

/// The last two changes a copy lacks are made in the last minute the old key could write and
/// the first it could not: the vectors find both, and send those two and no more.
#[test]
fn changes_on_either_side_of_3_to_the_16_minutes_are_both_found() {
    let before = (OLD_LIMIT_MINUTES - 100) * MINUTE;
    let mut world = todo_world(31, 20);
    todo_edit_at(&mut world, "alice", 1, TITLE, 0, "first", before);
    assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet);

    lose_everything(&mut world, "alice", true);
    let mark = world.trace().len();
    todo_edit_at(
        &mut world,
        "bob",
        2,
        DUE,
        30,
        "",
        OLD_LIMIT_MINUTES * MINUTE - 1,
    );
    todo_edit_at(
        &mut world,
        "carol",
        3,
        DUE,
        31,
        "",
        OLD_LIMIT_MINUTES * MINUTE,
    );
    world.advance_ms(10 * MINUTE);
    assert!(!todos_of(&mut world, "alice").contains_key(&(2, DUE)));
    let made = dots_made_since(&world, mark);
    assert_eq!(made.len(), 2, "{made:?}");

    lose_everything(&mut world, "alice", false);
    let repaired = world.trace().len();
    assert_eq!(world.run_until_quiet(600_000), Outcome::Quiet);
    let list = assert_converged(&mut world, "after the repair");
    assert_eq!(list[&(2, DUE)].value, 30);
    assert_eq!(list[&(3, DUE)].value, 31);
    let mut again = dots_made_since(&world, repaired);
    again.sort_unstable();
    assert_eq!(
        again, made,
        "what was sent again is the changes of those two minutes"
    );
    let minutes_again: Vec<i128> = again
        .iter()
        .map(|(_, wall, _)| wall / i128::from(MINUTE))
        .collect();
    assert_eq!(
        minutes_again,
        vec![
            i128::from(OLD_LIMIT_MINUTES - 1),
            i128::from(OLD_LIMIT_MINUTES)
        ]
    );
}

/// Two to-dos and fields that the first form of the element ran together are two registers:
/// field 16 of to-do 1 and field 0 of to-do 2, and the thousands beside them, each hold their
/// own value, a value written over in one leaves the other, and a copy whose links lost the
/// write-overs is repaired to all of them.
#[test]
fn registers_numbered_in_the_hundreds_and_thousands_are_each_their_own() {
    let epoch = YEAR_2100;
    let mut world = todo_world(21, 20);
    set_at(&mut world, "alice", 1, 16, 7, epoch);
    set_at(&mut world, "bob", 2, 0, 9, epoch);
    set_at(&mut world, "alice", 1, 300, 1, epoch);
    set_at(&mut world, "bob", 1, 1000, 2, epoch);
    set_at(&mut world, "alice", 70_000, 4096, 3, epoch);
    set_at(&mut world, "carol", 4_294_967_295, 65_535, 4, epoch);
    assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet);

    let list = assert_converged(&mut world, "before the loss");
    assert_eq!(
        list.len(),
        6,
        "six registers, none run into another: {list:?}"
    );
    for (key, value) in [
        ((1, 16), 7),
        ((2, 0), 9),
        ((1, 300), 1),
        ((1, 1000), 2),
        ((70_000, 4096), 3),
        ((4_294_967_295, 65_535), 4),
    ] {
        assert_eq!(list[&key].value, value, "{key:?}");
        assert_eq!(list[&key].live.len(), 1, "{key:?}");
    }

    world.advance_ms(10 * MINUTE);
    lose_everything(&mut world, "alice", true);
    world.advance_ms(5 * MINUTE);
    // Bob has alice's write of field 16 and writes over it; field 0 of the next to-do, which the
    // first form could not tell from it, is not touched.
    set_at(&mut world, "bob", 1, 16, 8, epoch);
    set_at(&mut world, "bob", 1, 4096, 5, epoch);
    world.advance_ms(MINUTE);
    set_at(&mut world, "bob", 70_000, 4096, 6, epoch);
    set_at(&mut world, "carol", 4_294_967_295, 65_534, 7, epoch);
    world.advance_ms(MINUTE);
    let lost = todos_of(&mut world, "alice");
    assert_eq!(lost[&(1, 16)].value, 7);
    assert!(!lost.contains_key(&(1, 4096)));

    lose_everything(&mut world, "alice", false);
    assert_eq!(world.run_until_quiet(600_000), Outcome::Quiet);
    let whole_list = assert_converged(&mut world, "after the repair");
    assert_eq!(whole_list[&(1, 16)].value, 8);
    assert_eq!(whole_list[&(1, 16)].live.len(), 1);
    assert_eq!(
        whole_list[&(2, 0)].value,
        9,
        "field 0 of to-do 2 was not written over"
    );
    assert_eq!(whole_list[&(70_000, 4096)].value, 6);
    assert_eq!(whole_list[&(70_000, 4096)].live.len(), 1);
    assert_eq!(whole_list.len(), 8);
}

/// A change lost on a link that is cut, rather than one that loses silently, is repaired when
/// the link comes back, at the last minutes a clock holds, for to-dos and fields at the top of
/// what the types hold: the copy whose link came back sends its vector at once.
#[test]
fn what_a_cut_link_lost_is_found_when_it_comes_back_at_the_last_minutes_of_a_clock() {
    let epoch = LAST_EPOCH;
    let mut world = todo_world(41, 20);
    set_at(&mut world, "alice", 4_294_967_295, 65_535, 1, epoch);
    assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet);
    world.advance_ms(20 * MINUTE);

    world.partition("carol", "alice");
    world.partition("carol", "bob");
    set_at(&mut world, "alice", 4_294_967_295, 65_535, 40, epoch);
    set_at(&mut world, "bob", 3_000_000, 1000, 5, epoch);
    world.advance_ms(9 * MINUTE);
    title_at(&mut world, "bob", 2, "water plants", epoch);
    world.advance_ms(MINUTE);
    assert!(!todos_of(&mut world, "carol").contains_key(&(2, TITLE)));

    world.heal("carol", "alice");
    world.heal("carol", "bob");
    assert_eq!(world.run_until_quiet(600_000), Outcome::Quiet);
    let list = assert_converged(&mut world, "after the links came back");
    assert_eq!(list[&(4_294_967_295, 65_535)].value, 40);
    assert_eq!(list[&(4_294_967_295, 65_535)].live.len(), 1);
    assert_eq!(list[&(2, TITLE)].text.as_deref(), Some("water plants"));
}

/// Copies that agree, at a wall time past anything the first form held, say so with their
/// vectors and send no change: each finds the other's vector lacks nothing it holds.
#[test]
fn copies_that_agree_far_past_2051_exchange_vectors_and_no_change() {
    let (mut world, _) = alice_cut_off_while_both_write_on(minutes(4_000_000_000_000_000));
    assert_eq!(world.run_until_quiet(600_000), Outcome::Quiet);
    assert_converged(&mut world, "before the link comes back");

    // A link comes back, and both its ends send their vectors at once.
    world.partition("alice", "bob");
    let mark = world.trace().len();
    world.heal("alice", "bob");
    assert_eq!(world.run_until_quiet(600_000), Outcome::Quiet);

    let vectors = vectors_sent_since(&world, mark);
    for who in ["alice", "bob"] {
        assert!(
            vectors.get(who).copied().unwrap_or(0) > 0,
            "{who} sent no vector when its link came back: {vectors:?}"
        );
    }
    let again = changes_carried_since(&world, mark);
    assert!(
        again.is_empty(),
        "copies that agree sent changes: {again:?}"
    );
    assert_converged(&mut world, "after the vectors");
}

/// A repair is idempotent and takes no one's turn: vectors sent again and by every copy, over
/// links that delay, reorder and repeat what they carry, end with the copies alike, and a
/// change the network gives a copy again, long after it was taken in, changes nothing.
#[test]
fn vectors_and_changes_given_again_change_nothing() {
    let (mut world, mark) = alice_cut_off_while_both_write_on(0);
    for (a, b) in links_of(&COPIES) {
        world.jitter(a, b, 30);
        world.reorder(a, b);
        world.degrade(a, b, 0, 40);
    }
    for (a, b) in [("alice", "bob"), ("alice", "carol"), ("alice", "bob")] {
        world.partition(a, b);
        world.heal(a, b);
    }
    assert_eq!(world.run_until_quiet(600_000), Outcome::Quiet);
    let whole_list = assert_converged(&mut world, "after the repair");

    // Every change made while alice was cut off is given to every copy once more.
    let before: Vec<_> = COPIES.iter().map(|who| todos_of(&mut world, who)).collect();
    let trace = world.trace();
    for line in trace.iter().skip(mark) {
        if !matches!(line.fate, Fate::Sent { .. }) {
            continue;
        }
        world.deliver_again(&line.from, &line.to, PEOPLE, line.bytes.clone());
    }
    assert_eq!(world.run_until_quiet(600_000), Outcome::Quiet);
    let after: Vec<_> = COPIES.iter().map(|who| todos_of(&mut world, who)).collect();
    assert_eq!(after, before, "a change given again was taken in again");
    assert_eq!(
        assert_converged(&mut world, "after one more of everything"),
        whole_list
    );
}

/// A value written over while a copy's links lost everything is taken out of it when the
/// write-over reaches it: the write it names was delivered to it before, and the write-over is
/// delivered after it, so the value leaves the field and does not come back.
#[test]
fn a_value_written_over_while_a_copy_was_away_is_taken_out_of_it() {
    let mut world = todo_world(3, 20);
    todo_title(&mut world, "alice", 1, "draft");
    assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet);
    world.advance_ms(10 * MINUTE);
    lose_everything(&mut world, "alice", true);
    world.advance_ms(10 * MINUTE);
    todo_title(&mut world, "bob", 1, "final");
    world.advance_ms(MINUTE);
    assert_eq!(
        todos_of(&mut world, "alice")[&(1, TITLE)].text.as_deref(),
        Some("draft")
    );

    lose_everything(&mut world, "alice", false);
    assert_eq!(world.run_until_quiet(600_000), Outcome::Quiet);
    let list = assert_converged(&mut world, "after the repair");
    assert_eq!(list[&(1, TITLE)].live.len(), 1);
    assert_eq!(list[&(1, TITLE)].text.as_deref(), Some("final"));
}

/// What a copy's layer says it took in is exactly what the copy holds, at every wall time: its
/// vector gives each author the number of changes that author made, no more and no fewer, so
/// that a vector a peer is sent lacks exactly what the copy lacks.
#[test]
fn a_copys_vector_names_what_it_took_in_at_every_wall_time() {
    for epoch in [0, minutes(YEAR_2100), LAST_EPOCH] {
        let (mut world, _) = alice_cut_off_while_both_write_on(epoch);
        assert_eq!(world.run_until_quiet(600_000), Outcome::Quiet);
        assert_converged(&mut world, &format!("epoch {epoch}"));
        let changes = changes_sent(&world);
        for author in COPIES {
            let made = changes
                .iter()
                .filter(|change| whole(&change["by"]) == i128::from(todo_node_of(author)))
                .count();
            for who in COPIES {
                assert_eq!(
                    delivered_of(&mut world, who, replica_of(author, 0)),
                    made as u64,
                    "epoch {epoch}: {who}'s vector for {author}"
                );
            }
        }
    }
}

/// The negative arm: the same loss on a route with no causal layer below the list, a plain
/// binding to each other copy, and nothing finds what the link lost. The copies end apart for
/// good, which is what the list's own resync once repaired, and what the layer repairs now.
#[test]
fn a_change_lost_on_a_plain_route_is_never_found() {
    let mut world = World::new(9);
    for who in COPIES {
        let mut router = Router::new(who, 256, 5_000).expect("a window the core can hold");
        router.add_peer(PEOPLE, peer(&[PEOPLE], false));
        world.add(who, TodoPeer::new(), router);
    }
    for (a, b) in links_of(&COPIES) {
        world.link(a, b, 20);
    }
    world.group(PEOPLE, &COPIES);
    for who in COPIES {
        world.raise(
            who,
            "todo.join",
            &format!(r#"{{"node":"{}"}}"#, todo_node_of(who)),
        );
    }
    assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet);
    lose_everything(&mut world, "alice", true);
    todo_title(&mut world, "bob", 1, "lost to alice");
    assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet);
    lose_everything(&mut world, "alice", false);
    assert_eq!(world.run_until_quiet(600_000), Outcome::Quiet);
    assert!(
        !todos_of(&mut world, "alice").contains_key(&(1, TITLE)),
        "a plain route repaired a loss"
    );
    assert_ne!(
        todos_of(&mut world, "alice"),
        todos_of(&mut world, "bob"),
        "the copies agree with no layer to repair them"
    );
    let vectors = vectors_sent_since(&world, 0);
    assert!(
        vectors.is_empty(),
        "a plain route sent vectors: {vectors:?}"
    );
    // A plain route's envelopes carry no causal tag.
    for line in world.trace() {
        let envelope = Envelope::decode(&mut SceCursor::new(&line.bytes)).expect("an envelope");
        assert_ne!(envelope.pattern, PatternKind::CausalVector);
        assert!(envelope.causal_epoch.is_none());
    }
}

/// An afternoon of changes at random times, to to-dos and fields numbered as a long list
/// numbers them, played from seeds that start it on either side of 3^16 minutes, far past it
/// and at the last minutes a clock holds, with a copy's links losing everything now and then,
/// links cut and restored and envelopes reordered and delivered twice: for every seed the
/// three copies end alike, as the oracle says, with their layers at rest.
#[test]
fn a_random_afternoon_past_2051_with_wide_fields_ends_with_one_list() {
    random_afternoon(&Afternoon {
        seeds: 0..24,
        ids: &WIDE_IDS,
        fields: &[
            (WIDE_FIELDS[0], 2),
            (WIDE_FIELDS[2], 365),
            (WIDE_FIELDS[4], 2),
            (WIDE_FIELDS[5], 9),
            (WIDE_FIELDS[7], 3),
            (WIDE_FIELDS[1], 5),
            (WIDE_FIELDS[3], 7),
            (WIDE_FIELDS[6], 11),
        ],
        epochs: &far_epochs(),
        reaches: OLD_LIMIT_MINUTES * MINUTE,
        repaired_at_least: 12,
    });
}

/// The same in 1970, for the to-dos and fields of a short list.
#[test]
fn a_random_afternoon_with_losses_ends_with_one_list() {
    random_afternoon(&Afternoon {
        seeds: 0..30,
        ids: &[1, 2, 3, 4],
        fields: &[(DONE, 2), (DUE, 365), (REMOVED, 2)],
        epochs: &[0],
        reaches: 0,
        repaired_at_least: 15,
    });
}
