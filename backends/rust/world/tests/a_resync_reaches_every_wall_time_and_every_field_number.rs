// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The resync of the shared to-do list (a_copy_restored_from_an_old_save_is_repaired_by_resync.rs)
// at the wall times and the field numbers its first form could not reach.
//
// The first form of the digest keyed a change by its minute written in 16 base-3 digits,
// which stops at 3^16 minutes (2051-11-05T13:21Z), and a to-do's register by
// `id * 16 + field`, which ran field 16 of one to-do into field 0 of the next. The digest
// now has 30 digits, which is every minute a signed 64-bit millisecond clock holds, and a
// register is `id * 65536 + field` over a uint32 and a uint16. What is held here is what
// the first tests hold, for afternoons played on either side of 3^16 minutes, far past it
// and at the last minute a clock holds, and for to-dos and fields numbered as a long list
// numbers them: the copies end with one list, the oracle's, each with the digest of what it
// holds, a resync sends the changes of the minutes the copies differ in and no others, and
// registers that the first form ran together are told apart.

mod common;

use common::resync::*;
use common::*;
use sce_rust_world::Outcome;

/// 2100-01-01T00:00Z, in milliseconds.
const YEAR_2100: i64 = 4_102_444_800_000;

/// A wall time rounded down to a whole minute, as an epoch is.
fn minutes(millis: i64) -> i64 {
    millis / MINUTE * MINUTE
}

/// The wall times the afternoons start at, each past 2051 or about to cross it: ninety
/// minutes before 3^16 minutes, so that an afternoon straddles it; just after it; 2100; the
/// minute where the key's leading digit goes from 1 to 2, 3^29 minutes in, straddled the
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

/// An afternoon in which alice's copy is restored from an old save, for every wall time:
/// she ends with the list the others hold, and is sent the changes of the minutes she lacked
/// and no others, whether they are made before 3^16 minutes, after it, or straddle it.
#[test]
fn a_person_restored_from_an_old_save_is_repaired_at_every_wall_time() {
    for epoch in far_epochs() {
        assert_alice_is_repaired(epoch);
    }
}

/// The last two changes a copy lacks are made in the last minute the reference's key could
/// write and the first it could not: the walk finds both, and sends the changes of those two
/// minutes and no more.
#[test]
fn changes_on_either_side_of_3_to_the_16_minutes_are_both_found() {
    let before = (OLD_LIMIT_MINUTES - 100) * MINUTE;
    let mut world = todo_world(31, 20);
    assert_eq!(world.run_until_quiet(1), Outcome::Quiet);
    todo_edit_at(&mut world, "alice", 1, TITLE, 0, "first", before);
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);
    let old = save(&mut world, "alice");

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
        "bob",
        3,
        DUE,
        31,
        "",
        OLD_LIMIT_MINUTES * MINUTE,
    );
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);
    let whole_list = assert_converged(&mut world, "before the restore");

    restore_from(&mut world, "alice", old);
    assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet);
    assert!(!todos_of(&mut world, "alice").contains_key(&(2, DUE)));

    let mark = world.trace().len();
    resync(&mut world, "alice");
    assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet);
    assert_eq!(assert_converged(&mut world, "after the resync"), whole_list);
    assert_eq!(
        minutes_of_changes(&sent_since(&world, mark)),
        vec![
            i128::from(OLD_LIMIT_MINUTES - 1),
            i128::from(OLD_LIMIT_MINUTES)
        ],
        "what was sent is the changes of those two minutes"
    );
}

/// Two to-dos and fields that the first form of the element ran together are two registers:
/// field 16 of to-do 1 and field 0 of to-do 2, and the thousands beside them, each hold their
/// own value, a value written over in one leaves the other, and a copy restored from an old
/// save is repaired to all of them.
#[test]
fn registers_numbered_in_the_hundreds_and_thousands_are_each_their_own() {
    let epoch = YEAR_2100;
    let mut world = todo_world(21, 20);
    assert_eq!(world.run_until_quiet(1), Outcome::Quiet);
    set_at(&mut world, "alice", 1, 16, 7, epoch);
    set_at(&mut world, "bob", 2, 0, 9, epoch);
    set_at(&mut world, "alice", 1, 300, 1, epoch);
    set_at(&mut world, "bob", 1, 1000, 2, epoch);
    set_at(&mut world, "alice", 70_000, 4096, 3, epoch);
    set_at(&mut world, "bob", 4_294_967_295, 65_535, 4, epoch);
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);

    let list = assert_converged(&mut world, "before the save");
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
    let old = save(&mut world, "alice");
    world.advance_ms(5 * MINUTE);
    // Bob has alice's write of field 16 and writes over it; field 0 of the next to-do, which the
    // first form could not tell from it, is not touched.
    set_at(&mut world, "bob", 1, 16, 8, epoch);
    set_at(&mut world, "bob", 1, 4096, 5, epoch);
    world.advance_ms(MINUTE);
    set_at(&mut world, "bob", 70_000, 4096, 6, epoch);
    set_at(&mut world, "bob", 4_294_967_295, 65_534, 7, epoch);
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);
    let whole_list = assert_converged(&mut world, "after bob wrote on");
    assert_eq!(whole_list[&(1, 16)].value, 8);
    assert_eq!(whole_list[&(1, 16)].live.len(), 1);
    assert_eq!(
        whole_list[&(2, 0)].value,
        9,
        "field 0 of to-do 2 was not written over"
    );
    assert_eq!(whole_list[&(70_000, 4096)].value, 6);
    assert_eq!(whole_list.len(), 8);

    restore_from(&mut world, "alice", old);
    let lost = todos_of(&mut world, "alice");
    assert_eq!(lost[&(1, 16)].value, 7);
    assert!(!lost.contains_key(&(1, 4096)));
    assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet);

    let mark = world.trace().len();
    resync(&mut world, "alice");
    assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet);
    assert_eq!(assert_converged(&mut world, "after the resync"), whole_list);

    let first = i128::from(epoch / MINUTE);
    let sent = minutes_of_changes(&sent_since(&world, mark));
    assert!(
        sent.iter()
            .all(|minute| *minute - first == 0 || *minute - first >= 15),
        "changes of the minutes the copies agreed on were sent again: {sent:?}"
    );
    assert!(
        sent.iter().any(|minute| *minute - first >= 15),
        "the changes made after the save were sent: {sent:?}"
    );
}

/// The hub restored from an old save, in the last two days a clock holds, with to-dos and
/// fields numbered at the top of what the types hold, finds what it lost again from the people.
#[test]
fn the_hub_restored_at_the_last_minutes_of_a_clock_is_repaired_by_resync() {
    let epoch = LAST_EPOCH;
    let mut world = todo_world(41, 20);
    assert_eq!(world.run_until_quiet(1), Outcome::Quiet);
    set_at(&mut world, "alice", 4_294_967_295, 65_535, 1, epoch);
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);
    world.advance_ms(20 * MINUTE);
    let old = save(&mut world, "server");

    world.advance_ms(3 * MINUTE);
    set_at(&mut world, "alice", 4_294_967_295, 65_535, 40, epoch);
    set_at(&mut world, "bob", 3_000_000, 1000, 5, epoch);
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);
    world.advance_ms(9 * MINUTE);
    title_at(&mut world, "bob", 2, "water plants", epoch);
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);
    let whole_list = assert_converged(&mut world, "before the restore");

    restore_from(&mut world, "server", old);
    assert_ne!(todos_of(&mut world, "server"), whole_list);
    assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet);
    resync(&mut world, "server");
    assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet);
    assert_eq!(assert_converged(&mut world, "after the resync"), whole_list);
}

/// Copies that agree, at a wall time past anything the first form held, say so with one
/// message: the root of the sender's digest, which the receiver finds equal to its own.
#[test]
fn copies_that_agree_far_past_2051_exchange_one_node_and_nothing_else() {
    let (mut world, _) = alice_saved_then_both_write_on(minutes(4_000_000_000_000_000));
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
}

/// The digest of a copy is the one the definition gives for what it holds, down to the
/// thirtieth level, at a wall time of every kind: at 1970, either side of 3^16 minutes, and at
/// the last minute a clock holds, where the path is the longest and its leading digit is 2.
#[test]
fn a_copy_keeps_the_digest_of_what_it_holds_at_every_wall_time() {
    for epoch in [0, minutes(YEAR_2100), LAST_EPOCH] {
        let (mut world, _) = alice_saved_then_both_write_on(epoch);
        let digest = agreed_digest(&mut world);
        let deepest = digest.keys().map(|(depth, _)| *depth).max();
        assert_eq!(deepest, Some(u64::from(DIGITS)), "epoch {epoch}");
        let last = digest
            .keys()
            .filter(|(depth, _)| *depth == u64::from(DIGITS))
            .map(|(_, prefix)| i128::from(*prefix))
            .max()
            .expect("a digest of changes has a last level");
        assert!(last >= i128::from(epoch / MINUTE), "epoch {epoch}");
    }
}

/// An afternoon of changes at random times, to to-dos and fields numbered as a long list
/// numbers them, played from seeds that start it on either side of 3^16 minutes, far past it
/// and at the last minutes a clock holds, with copies restored from old saves, links cut and
/// restored and envelopes delivered twice: for every seed the three copies end alike, as the
/// oracle says, each with the digest of what it holds.
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
        late: (WIDE_FIELDS[5], 400),
        epochs: &far_epochs(),
        reaches: OLD_LIMIT_MINUTES * MINUTE,
        repaired_at_least: 12,
    });
}
