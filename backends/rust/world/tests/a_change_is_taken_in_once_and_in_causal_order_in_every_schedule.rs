// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The cross product of docs/adr/0085 section 9.1, through the Mesh core's causal route: three
// copies of the standard shared list with no core, each served by a host that keeps a journal
// (common/journal.rs). Every change a copy makes is offered to its host on `#journal`, which
// gives it to the copy's layer, commits it with the dot the layer tagged it with, and gives the
// copy the verdict back (docs/adr/0093, section 3.1).
//
// One change C of three operations, two write-overs and a write: alice empties the register
// bob wrote and writes over the register carol wrote, in one change. Around it, over every
// product of:
//
// - the order the network gives C relative to the changes around it: before them or after;
// - concurrent clears: each write-over's target written over first by a change made without
//   seeing C (bob's clear of the register C empties, carol's of the one it writes), the four
//   subsets;
// - a later change, bob's write over C's write: none; one every copy is given after C; one
//   that reaches carol before C;
// - redelivery: none; C given twice; C given again once every copy holds it and the stable
//   cut has passed it;
// - a stable cut, at which every host folds its journal: none; one before C; one past C;
// - a rebuild of every copy in turn: none; by recall, from the snapshot of the last fold and
//   the changes after it; by restore, from a snapshot at the cut alone.
//
// The network is held while the changes are made (every link loses what it carries), and
// each change is given to each copy by hand in the order the schedule says; then the links
// carry again and the layers exchange their vectors. What every schedule ends with, by the
// rule of docs/adr/0085 sections 2 and 3: every copy holds the same values and adds, which are
// what the changes on the network make (the register C empties holds nothing, and the
// register it writes holds C's write, or the later change's when there is one, and nothing
// else); each copy was given each other copy's change once, and reported each of its own
// closed once; every copy's clock is at or past every stamp it holds; the layers are at rest;
// and a rebuild ends with the state before it.
//
// The negative arm is kept for good (docs/adr/0085, "Owner's answers", 4e): the same schedules
// on a route with no causal layer below the list. There a later change that reaches carol
// before C is taken first: its write-over names a write carol does not hold yet, and C's write,
// taken after it, stays live at carol alone, so the copies end apart; every other schedule ends
// alike, since the effect of concurrent changes does not depend on their order and the route's
// duplicate window drops an envelope given again. The difference is the layer's to remove.
//
// That a register is told to a parent once per change is held where a parent observes it, by
// the scenario `record_list_concurrent_clears`: a copy a host starts tells no parent. What is
// held here of it is the host's side: each change given to a copy once, each reported once.

mod common;

use common::journal::*;
use common::*;
use sce_rust_mesh::causal::Refusal;
use serde_json::Value;

/// The register C empties: bob's write before it.
const EMPTIED: (u64, u64) = (1, DUE);

/// The register C writes over and writes: carol's write before it.
const WRITTEN: (u64, u64) = (2, DUE);

/// What C writes, and what the later change writes over it.
const C_WRITES: i64 = 30;
const LATER_WRITES: i64 = 40;

/// Where C reaches a copy among the concurrent clears it is given.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Order {
    ChangeFirst,
    ChangeLast,
}

/// Which of C's write-over targets a change made without seeing C writes over first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Clears {
    emptied: bool,
    written: bool,
}

/// Bob's later change, which writes over C's write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Later {
    Absent,
    AfterIt,
    BeforeItAtCarol,
}

/// How often C is given to a copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Again {
    Once,
    Twice,
    AfterTheCut,
}

/// Where the hosts fold their journals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Cut {
    Never,
    BeforeIt,
    PastIt,
}

/// How every copy is built again at the end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Rebuild {
    Kept,
    ByRecall,
    ByRestore,
}

#[derive(Debug, Clone, Copy)]
struct Schedule {
    order: Order,
    clears: Clears,
    later: Later,
    again: Again,
    cut: Cut,
    rebuild: Rebuild,
}

/// Every schedule of the product with the rebuild `rebuild`.
fn schedules(rebuild: Rebuild) -> Vec<Schedule> {
    let mut all = Vec::new();
    for order in [Order::ChangeFirst, Order::ChangeLast] {
        for (emptied, written) in [(false, false), (true, false), (false, true), (true, true)] {
            for later in [Later::Absent, Later::AfterIt, Later::BeforeItAtCarol] {
                for again in [Again::Once, Again::Twice, Again::AfterTheCut] {
                    for cut in [Cut::Never, Cut::BeforeIt, Cut::PastIt] {
                        all.push(Schedule {
                            order,
                            clears: Clears { emptied, written },
                            later,
                            again,
                            cut,
                            rebuild,
                        });
                    }
                }
            }
        }
    }
    all
}

/// What a schedule leaves: the world and its hosts, C, and the later change when there is one.
struct Played {
    hosted: Hosted,
    change: Made,
    later: Option<Made>,
}

/// Give `to` the change C `times` times, before or after `clear` as `order` says.
fn give_around(
    hosted: &mut Hosted,
    to: &str,
    change: &Made,
    times: usize,
    clear: Option<&Made>,
    order: Order,
) {
    if order == Order::ChangeLast {
        if let Some(clear) = clear {
            hosted.give(clear, to);
        }
    }
    for _ in 0..times {
        hosted.give(change, to);
    }
    if order == Order::ChangeFirst {
        if let Some(clear) = clear {
            hosted.give(clear, to);
        }
    }
}

/// Play `schedule` on `route` up to the rebuild, the world at rest at the end.
fn play(schedule: Schedule, route: Route, seed: u64) -> Played {
    let context = format!("{schedule:?}");
    let mut hosted = Hosted::new(seed, route);

    // Before C, with the links carrying: bob writes the register C empties, carol the one it
    // writes over, and every copy takes both in.
    hosted.edit("bob", EMPTIED, 1, false);
    hosted.quiet(&context);
    hosted.edit("carol", WRITTEN, 2, false);
    hosted.quiet(&context);
    if schedule.cut == Cut::BeforeIt {
        for who in COPIES {
            hosted.fold(who);
        }
    }

    // C and the concurrent clears, each made without seeing the others.
    hosted.hold(true);
    let change = hosted.make("alice", |hosted| {
        hosted.clear("alice", EMPTIED, true);
        hosted.edit("alice", WRITTEN, C_WRITES, false);
    });
    assert_eq!(
        change.change["ops"].as_array().map(Vec::len),
        Some(3),
        "{context}: C is two write-overs and a write: {}",
        change.change
    );
    let emptied = schedule
        .clears
        .emptied
        .then(|| hosted.make("bob", |hosted| hosted.clear("bob", EMPTIED, false)));
    let written = schedule
        .clears
        .written
        .then(|| hosted.make("carol", |hosted| hosted.clear("carol", WRITTEN, false)));

    // Each copy is given what the others made, in the schedule's order. Carol is not given C
    // yet when the later change is to reach her first.
    let times = if schedule.again == Again::Twice { 2 } else { 1 };
    give_around(
        &mut hosted,
        "bob",
        &change,
        times,
        written.as_ref(),
        schedule.order,
    );
    if schedule.later == Later::BeforeItAtCarol {
        if let Some(clear) = &emptied {
            hosted.give(clear, "carol");
        }
    } else {
        give_around(
            &mut hosted,
            "carol",
            &change,
            times,
            emptied.as_ref(),
            schedule.order,
        );
    }
    for clear in [&emptied, &written].into_iter().flatten() {
        hosted.give(clear, "alice");
    }

    // Bob, who holds C, writes over its write.
    let later = (schedule.later != Later::Absent).then(|| {
        let later = hosted.make("bob", |hosted| {
            hosted.edit("bob", WRITTEN, LATER_WRITES, false);
        });
        hosted.give(&later, "alice");
        hosted.give(&later, "carol");
        if schedule.later == Later::BeforeItAtCarol {
            for _ in 0..times {
                hosted.give(&change, "carol");
            }
        }
        later
    });

    // The links carry again, and the layers exchange their vectors until they rest.
    hosted.hold(false);
    hosted.quiet(&context);
    if schedule.cut == Cut::PastIt {
        for who in COPIES {
            hosted.fold(who);
        }
    }
    if schedule.again == Again::AfterTheCut {
        hosted.give(&change, "bob");
        hosted.give(&change, "carol");
        hosted.quiet(&context);
    }
    Played {
        hosted,
        change,
        later,
    }
}

/// Build every copy again as `rebuild` says, in turn; each ends with the state it had.
fn rebuild_each(hosted: &mut Hosted, rebuild: Rebuild, context: &str) {
    if rebuild == Rebuild::Kept {
        return;
    }
    for who in COPIES {
        let before = hosted.held(who);
        if rebuild == Rebuild::ByRestore {
            hosted.fold(who);
        }
        hosted.rebuild(who);
        assert_eq!(
            hosted.held(who),
            before,
            "{context}: {who} built again {rebuild:?} holds what it held before"
        );
    }
}

/// The stamp a register's one live value carries, as a change states its stamp.
fn stamp_held(field: &Field) -> (i128, i128, i128) {
    (
        i128::from(field.stamp.2),
        i128::from(field.stamp.0),
        i128::from(field.stamp.1),
    )
}

/// Every copy ends alike, as the rule says, and a rebuild changes nothing.
fn assert_ends_alike(schedule: Schedule, seed: u64) {
    let context = format!("{schedule:?}");
    let Played {
        mut hosted,
        change,
        later,
    } = play(schedule, Route::Causal { room: LAYER_ROOM }, seed);
    rebuild_each(&mut hosted, schedule.rebuild, &context);
    let world = &mut hosted.world;

    let list = agreed_records(world, &COPIES);
    assert!(
        !list.contains_key(&EMPTIED),
        "{context}: the register C emptied holds {:?}",
        list.get(&EMPTIED)
    );
    let (value, by) = match &later {
        None => (C_WRITES, &change),
        Some(later) => (LATER_WRITES, later),
    };
    let written = &list[&WRITTEN];
    assert_eq!(
        written.live.len(),
        1,
        "{context}: the register C wrote holds one value: {written:?}"
    );
    assert_eq!(written.value, value, "{context}");
    assert_eq!(stamp_held(written), stamp_of(&by.change), "{context}");
    assert_eq!(list.len(), 1, "{context}: {list:?}");
    let changes = changes_on_the_network(world);
    assert_eq!(list, list_oracle(&changes), "{context}");

    for who in COPIES {
        let variables = record_variables_of(world, who);
        for count in ["refusals", "denied", "full"] {
            assert_eq!(variables[count], 0, "{context}: {who}'s {count}");
        }
    }
    let errors = world.host_errors();
    assert!(errors.is_empty(), "{context}: {errors:?}");

    let alices = hosted.held("alice");
    for who in COPIES {
        let held = hosted.held(who);
        assert_eq!(held.adds, alices.adds, "{context}: {who}'s adds");
        for field in held.records.values() {
            for one in &field.live {
                assert!(
                    held.clock >= (i128::from(one.stamp.0), i128::from(one.stamp.1)),
                    "{context}: {who}'s clock {:?} is before a stamp it holds {:?}",
                    held.clock,
                    one.stamp
                );
            }
        }
    }

    for who in COPIES {
        let node = i128::from(record_node_of(who));
        let mut others: Vec<(i128, i128, i128)> = changes
            .iter()
            .filter(|made| whole(&made["by"]) != node)
            .map(stamp_of)
            .collect();
        others.sort_unstable();
        let own = changes
            .iter()
            .filter(|made| whole(&made["by"]) == node)
            .count();
        let book = hosted.book(who);
        let book = lock(&book);
        let mut given = book.given.clone();
        given.sort_unstable();
        assert_eq!(
            given, others,
            "{context}: {who} was given each other copy's change once"
        );
        assert_eq!(book.accepted, own, "{context}: {who}'s offers accepted");
        assert!(book.refused.is_empty(), "{context}: {:?}", book.refused);
        assert_eq!(
            book.closed.len(),
            own,
            "{context}: {who} reported each change once"
        );
        for closed in &book.closed {
            assert!(closed["reason"].is_null(), "{context}: {closed}");
            assert!(whole(&closed["operations"]) > 0, "{context}: {closed}");
        }
    }
}

/// On a route with no causal layer, the copies end apart exactly where the network gave
/// carol the later change before C, and carol holds both writes of the register.
fn assert_apart_where_out_of_order(schedule: Schedule, seed: u64) {
    let context = format!("{schedule:?}");
    let Played { mut hosted, .. } = play(schedule, Route::Plain, seed);
    rebuild_each_plain(&mut hosted, schedule.rebuild);
    let world = &mut hosted.world;
    let alice = records_of(world, "alice");
    let bob = records_of(world, "bob");
    let carol = records_of(world, "carol");
    assert_eq!(alice, bob, "{context}: alice and bob took the same order");
    if schedule.later == Later::BeforeItAtCarol {
        assert_ne!(carol, alice, "{context}: the copies agree with no layer");
        assert_eq!(
            carol[&WRITTEN].live.len(),
            2,
            "{context}: carol keeps C's write beside the later one: {:?}",
            carol[&WRITTEN]
        );
    } else {
        assert_eq!(carol, alice, "{context}");
        assert_eq!(
            alice,
            list_oracle(&changes_on_the_network(world)),
            "{context}"
        );
    }
    let errors = world.host_errors();
    assert!(errors.is_empty(), "{context}: {errors:?}");
}

/// Build every copy of a route with no causal layer again as `rebuild` says: what it holds
/// is what its host gives it, the copies being apart or not.
fn rebuild_each_plain(hosted: &mut Hosted, rebuild: Rebuild) {
    if rebuild == Rebuild::Kept {
        return;
    }
    for who in COPIES {
        if rebuild == Rebuild::ByRestore {
            hosted.fold(who);
        }
        hosted.rebuild(who);
    }
}

fn every_schedule_ends_alike(rebuild: Rebuild) {
    for (seed, schedule) in (0u64..).zip(schedules(rebuild)) {
        assert_ends_alike(schedule, seed);
    }
}

fn without_a_layer_out_of_order_ends_apart(rebuild: Rebuild) {
    for (seed, schedule) in (0u64..).zip(schedules(rebuild)) {
        assert_apart_where_out_of_order(schedule, seed);
    }
}

/// Every schedule of the product ends with the copies alike, as the rule says.
#[test]
fn every_schedule_ends_alike_over_the_causal_route() {
    every_schedule_ends_alike(Rebuild::Kept);
}

/// And each copy built again from the snapshot of its host's last fold and the changes
/// after it, recalled in the order it took them in, holds what it held.
#[test]
fn every_schedule_ends_alike_and_a_rebuild_by_recall_ends_where_it_began() {
    every_schedule_ends_alike(Rebuild::ByRecall);
}

/// And each copy built again from a snapshot at the cut alone holds what it held: a live
/// value of a change whose write-overs the cut forgot is kept.
#[test]
fn every_schedule_ends_alike_and_a_rebuild_by_restore_ends_where_it_began() {
    every_schedule_ends_alike(Rebuild::ByRestore);
}

/// The negative arm: on a route with no causal layer, a later change that reaches a copy
/// before the change it writes over leaves that copy apart for good.
#[test]
fn without_a_causal_layer_a_change_given_before_the_one_it_follows_leaves_the_copies_apart() {
    without_a_layer_out_of_order_ends_apart(Rebuild::Kept);
}

/// And a copy built again by recall is built as it was taken in, apart with it.
#[test]
fn without_a_causal_layer_a_rebuild_by_recall_leaves_the_copies_apart() {
    without_a_layer_out_of_order_ends_apart(Rebuild::ByRecall);
}

/// And a copy built again from a snapshot holds the state it was apart in.
#[test]
fn without_a_causal_layer_a_rebuild_by_restore_leaves_the_copies_apart() {
    without_a_layer_out_of_order_ends_apart(Rebuild::ByRestore);
}

/// An offer the layer has no room for is refused (docs/adr/0085, section 3, "offer and
/// verdict"): it takes no number, the host journals nothing of it, no copy takes it in, and
/// the copy reports it closed `noRoomForHistory` at the stamp it was closed at, which its next
/// change is stamped after. That next change takes the number the refused one did not, and
/// every copy takes it in.
#[test]
fn a_change_the_layer_has_no_room_for_is_refused_and_takes_no_number() {
    let mut hosted = Hosted::new(5, Route::Causal { room: 1 });
    hosted.hold(true);
    let kept = hosted.make("alice", |hosted| hosted.edit("alice", EMPTIED, 1, false));
    // Alice's log keeps that change until every copy holds it, and has room for no other.
    hosted.edit("alice", WRITTEN, 2, false);
    hosted.step();
    hosted.hold(false);
    hosted.quiet("after the refusal");
    let next = hosted.make("alice", |hosted| hosted.edit("alice", WRITTEN, 3, false));
    hosted.quiet("after the next change");

    let book = hosted.book("alice");
    let (refused, seqs, closed) = {
        let book = lock(&book);
        let seqs: Vec<u64> = book
            .journal
            .iter()
            .map(|own| own.dot.expect("a change the layer accepted has a dot").seq)
            .collect();
        (book.refused.clone(), seqs, book.closed.clone())
    };
    assert_eq!(refused, vec![Refusal::NoRoomForHistory]);
    assert_eq!(seqs, vec![1, 2], "the refused change took no number");
    assert_eq!(closed.len(), 3, "{closed:?}");
    assert!(closed[0]["reason"].is_null(), "{}", closed[0]);
    assert_eq!(closed[1]["reason"], "noRoomForHistory", "{}", closed[1]);
    assert_eq!(closed[1]["operations"], 0, "{}", closed[1]);
    assert!(closed[2]["reason"].is_null(), "{}", closed[2]);
    let refused_at = (whole(&closed[1]["wallTime"]), whole(&closed[1]["counter"]));
    let next_at = (
        whole(&next.change["wallTime"]),
        whole(&next.change["counter"]),
    );
    assert!(
        next_at > refused_at,
        "the next change {next_at:?} is stamped after the refused one {refused_at:?}"
    );

    let changes = changes_on_the_network(&hosted.world);
    let stamps: Vec<(i128, i128, i128)> = changes.iter().map(stamp_of).collect();
    assert_eq!(
        stamps,
        vec![stamp_of(&kept.change), stamp_of(&next.change)],
        "the refused change never reached the network"
    );
    let list = agreed_records(&mut hosted.world, &COPIES);
    assert_eq!(list[&EMPTIED].value, 1);
    assert_eq!(list[&WRITTEN].value, 3);
    assert_eq!(list[&WRITTEN].live.len(), 1, "{:?}", list[&WRITTEN]);
    assert_eq!(list, list_oracle(&changes));
    let alice: Value = record_variables_of(&mut hosted.world, "alice");
    assert_eq!(
        alice["refusals"], 0,
        "a refusal by the layer is no host's fault"
    );
    let errors = hosted.world.host_errors();
    assert!(errors.is_empty(), "{errors:?}");
}
