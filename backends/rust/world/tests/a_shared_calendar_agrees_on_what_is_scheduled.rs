// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A shared schedule: the to-do list of `a_shared_todo_list_converges.rs`, whose to-dos
// are rows of fields that merge one field at a time, used for events. An event is a
// to-do with a start, a length and a repetition among its fields, and a copy of the list
// shows what is scheduled in a window as its `agenda` (`todo.agenda`, the fixtures
// `event_agenda.scxml` and `todo_peer.scxml`), computed from the values it holds.
//
// What is asked here is what a calendar adds to a list: that two people who move or
// repeat the same event at once, on links that delay, reorder and cut what they carry,
// end with copies that show the same occurrences, that the occurrences are the ones the
// rules of repetition give (a month's last day, a week's turn, a count), and that a
// field two people changed at once is shown as the later write with a flag that the
// people have not agreed, until someone who has seen both writes over them.
//
// Equality of the copies says they agree, not that they are right, so each agenda is also
// held to an expansion written a second time in `common/calendar.rs`, from the schedule
// the copies' fields agree on.

mod common;

use common::calendar::*;
use common::*;
use sce_rust_world::{Outcome, Rng};

const WINDOW_JAN_TO_JUN: (i64, i64) = (1_767_225_600_000, 1_782_864_000_000);

fn jan_to_jun() -> (i64, i64) {
    let (from, to) = WINDOW_JAN_TO_JUN;
    assert_eq!(
        from,
        instant(2026, 1, 1, 0, 0),
        "the window starts on New Year"
    );
    assert_eq!(to, instant(2026, 7, 1, 0, 0), "the window ends with June");
    (from, to)
}

fn starts(agenda: &[Occurrence]) -> Vec<i64> {
    agenda.iter().map(|occurrence| occurrence.start).collect()
}

/// Monday and Wednesday, as the days of a weekly repetition (Sunday is bit 0).
const MONDAY_AND_WEDNESDAY: i64 = (1 << 1) | (1 << 3);
const FRIDAY: i64 = 1 << 5;

#[test]
fn a_weekly_meeting_is_one_agenda_on_every_copy() {
    let mut world = todo_world(1, 20);
    let plan = Plan {
        days: MONDAY_AND_WEDNESDAY,
        ..Plan::once(instant(2026, 1, 5, 10, 0), HOUR_MS).repeating(WEEKLY, 1)
    };
    schedule_todo(&mut world, "alice", 5, &plan);
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);

    let agenda = agreed_agenda(
        &mut world,
        5,
        instant(2026, 1, 5, 0, 0),
        instant(2026, 1, 19, 0, 0),
    );
    assert_eq!(
        starts(&agenda),
        [
            instant(2026, 1, 5, 10, 0),
            instant(2026, 1, 7, 10, 0),
            instant(2026, 1, 12, 10, 0),
            instant(2026, 1, 14, 10, 0),
        ]
    );
    assert!(agenda
        .iter()
        .all(|o| o.end - o.start == HOUR_MS && o.conflict == 0));
    assert_eq!(
        agenda.iter().map(|o| o.nth).collect::<Vec<_>>(),
        [0, 1, 2, 3]
    );
    assert!(world.host_errors().is_empty());
}

/// The 31st falls on the last day of a shorter month, and on the 29th of a leap February.
#[test]
fn a_monthly_event_on_the_31st_falls_on_the_last_day_of_short_months() {
    let mut world = todo_world(1, 20);
    schedule_todo(
        &mut world,
        "alice",
        1,
        &Plan::once(instant(2026, 1, 31, 9, 0), 0).repeating(MONTHLY, 1),
    );
    schedule_todo(
        &mut world,
        "bob",
        2,
        &Plan::once(instant(2028, 1, 31, 9, 0), 0).repeating(MONTHLY, 1),
    );
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);

    let plain = agreed_agenda(
        &mut world,
        1,
        instant(2026, 1, 1, 0, 0),
        instant(2026, 5, 1, 0, 0),
    );
    assert_eq!(
        starts(&plain),
        [
            instant(2026, 1, 31, 9, 0),
            instant(2026, 2, 28, 9, 0),
            instant(2026, 3, 31, 9, 0),
            instant(2026, 4, 30, 9, 0),
        ]
    );
    let leap = agreed_agenda(
        &mut world,
        2,
        instant(2028, 1, 1, 0, 0),
        instant(2028, 4, 1, 0, 0),
    );
    assert_eq!(
        starts(&leap),
        [
            instant(2028, 1, 31, 9, 0),
            instant(2028, 2, 29, 9, 0),
            instant(2028, 3, 31, 9, 0),
        ]
    );
    assert!(world.host_errors().is_empty());
}

#[test]
fn a_count_ends_a_repetition_wherever_the_window_starts() {
    let mut world = todo_world(1, 20);
    let plan = Plan {
        count: 4,
        ..Plan::once(instant(2026, 1, 1, 8, 0), HOUR_MS).repeating(DAILY, 3)
    };
    schedule_todo(&mut world, "alice", 3, &plan);
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);

    let (from, to) = jan_to_jun();
    let all = agreed_agenda(&mut world, 3, from, to);
    assert_eq!(
        starts(&all),
        [
            instant(2026, 1, 1, 8, 0),
            instant(2026, 1, 4, 8, 0),
            instant(2026, 1, 7, 8, 0),
            instant(2026, 1, 10, 8, 0),
        ]
    );
    // A window that begins after the second occurrence still counts from the first: the
    // occurrences it shows are the third and the fourth, and there is no fifth.
    let late = agreed_agenda(&mut world, 3, instant(2026, 1, 5, 0, 0), to);
    assert_eq!(
        late.iter().map(|o| o.nth).collect::<Vec<_>>(),
        [2, 3],
        "the count runs from the first occurrence and not from the window"
    );
    // A window before the start and one after the last show nothing.
    assert!(agreed_agenda(
        &mut world,
        3,
        instant(2025, 1, 1, 0, 0),
        instant(2026, 1, 1, 0, 0)
    )
    .is_empty());
    assert!(agreed_agenda(&mut world, 3, instant(2026, 2, 1, 0, 0), to).is_empty());
}

/// A weekly repetition every second week counts the weeks from the week of the start, and
/// its first occurrence is the first day its days name on or after the start.
#[test]
fn every_other_week_counts_the_weeks_from_the_week_of_the_start() {
    let mut world = todo_world(1, 20);
    let plan = Plan {
        days: FRIDAY,
        ..Plan::once(instant(2026, 1, 7, 18, 30), 45 * 60_000).repeating(WEEKLY, 2)
    };
    schedule_todo(&mut world, "bob", 4, &plan);
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);

    let agenda = agreed_agenda(
        &mut world,
        4,
        instant(2026, 1, 1, 0, 0),
        instant(2026, 2, 15, 0, 0),
    );
    assert_eq!(
        starts(&agenda),
        [
            instant(2026, 1, 9, 18, 30),
            instant(2026, 1, 23, 18, 30),
            instant(2026, 2, 6, 18, 30),
        ]
    );
}

/// A window never holds more than 64 occurrences.
#[test]
fn an_agenda_ends_at_the_sixty_fourth_occurrence() {
    let mut world = todo_world(1, 20);
    schedule_todo(
        &mut world,
        "alice",
        9,
        &Plan::once(instant(2026, 1, 1, 7, 0), 0).repeating(DAILY, 1),
    );
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);

    let (from, to) = jan_to_jun();
    let agenda = agreed_agenda(&mut world, 9, from, to);
    assert_eq!(agenda.len(), AGENDA_MAX);
    assert_eq!(agenda.last().map(|o| o.nth), Some(63));
}

/// Two people move one event at the same moment, neither having seen the other's move:
/// both starts stay, every copy shows the later one, and says the people have not agreed;
/// when someone who has seen both moves it again, the flag is gone.
#[test]
fn two_people_who_move_one_event_at_once_see_the_later_move_and_the_conflict() {
    let mut world = todo_world(1, 20);
    schedule_todo(
        &mut world,
        "alice",
        7,
        &Plan::once(instant(2026, 3, 2, 9, 0), HOUR_MS),
    );
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);
    let (from, to) = (instant(2026, 3, 1, 0, 0), instant(2026, 4, 1, 0, 0));
    assert!(agreed_agenda(&mut world, 7, from, to)
        .iter()
        .all(|o| o.conflict == 0));

    todo_set(&mut world, "alice", 7, START, instant(2026, 3, 3, 9, 0));
    todo_set(&mut world, "bob", 7, START, instant(2026, 3, 4, 14, 0));
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);

    let agenda = agreed_agenda(&mut world, 7, from, to);
    assert_eq!(agenda.len(), 1);
    // The stamps tie on time and counter, so the node id decides: bob's write is the
    // later one, and it is the one every copy shows.
    assert_eq!(agenda[0].start, instant(2026, 3, 4, 14, 0));
    assert_eq!(
        agenda[0].conflict, 1,
        "the people have not agreed on the start"
    );
    let list = agreed_todos(&mut world);
    assert_eq!(list[&(7, START)].live.len(), 2);

    // Alice has seen both, so what she writes replaces both.
    todo_set(&mut world, "alice", 7, START, instant(2026, 3, 5, 11, 0));
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);
    let agreed = agreed_agenda(&mut world, 7, from, to);
    assert_eq!(starts(&agreed), [instant(2026, 3, 5, 11, 0)]);
    assert_eq!(agreed[0].conflict, 0);
    assert!(world.host_errors().is_empty());
}

/// One person moves an event and another changes how it repeats, at the same moment: the
/// two fields are two registers and both changes stay, with nothing to choose between.
#[test]
fn a_move_and_a_new_repetition_made_at_once_are_both_kept() {
    let mut world = todo_world(1, 20);
    schedule_todo(
        &mut world,
        "alice",
        2,
        &Plan::once(instant(2026, 5, 4, 8, 0), HOUR_MS).repeating(DAILY, 1),
    );
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);

    todo_set(&mut world, "alice", 2, START, instant(2026, 5, 4, 9, 30));
    todo_set(&mut world, "bob", 2, REPEAT, WEEKLY);
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);

    let agenda = agreed_agenda(
        &mut world,
        2,
        instant(2026, 5, 1, 0, 0),
        instant(2026, 5, 19, 0, 0),
    );
    // Weekly on the start's own weekday (Monday), from 09:30.
    assert_eq!(
        starts(&agenda),
        [
            instant(2026, 5, 4, 9, 30),
            instant(2026, 5, 11, 9, 30),
            instant(2026, 5, 18, 9, 30),
        ]
    );
    assert!(agenda.iter().all(|o| o.conflict == 0));
}

/// One person removes an event while another changes its length: the removal is its own
/// field, it is kept, and the event shows nothing on any copy.
#[test]
fn an_event_removed_while_another_edits_it_shows_nothing() {
    let mut world = todo_world(1, 20);
    schedule_todo(
        &mut world,
        "alice",
        6,
        &Plan::once(instant(2026, 2, 10, 12, 0), HOUR_MS),
    );
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);
    let (from, to) = (instant(2026, 2, 1, 0, 0), instant(2026, 3, 1, 0, 0));
    assert_eq!(agreed_agenda(&mut world, 6, from, to).len(), 1);

    todo_set(&mut world, "bob", 6, GONE, 1);
    todo_set(&mut world, "alice", 6, LENGTH, 2 * HOUR_MS);
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);

    assert!(agreed_agenda(&mut world, 6, from, to).is_empty());
    assert_eq!(agreed_todos(&mut world), todo_oracle(&world));
}

/// A person on a cut link schedules an event and moves it twice; the other changes it as
/// well, from the hub's side. When the link is back every copy shows one agenda.
#[test]
fn an_event_scheduled_offline_is_shown_when_the_link_is_back() {
    let mut world = todo_world(5, 20);
    world.partition("alice", "server");
    schedule_todo(
        &mut world,
        "alice",
        8,
        &Plan::once(instant(2026, 4, 6, 9, 0), HOUR_MS).repeating(WEEKLY, 1),
    );
    assert_eq!(world.run_until_quiet(500), Outcome::Quiet);
    let (from, to) = (instant(2026, 4, 1, 0, 0), instant(2026, 5, 1, 0, 0));
    assert_eq!(
        agenda_of(&mut world, "alice", 8, from, to).len(),
        4,
        "alice sees her own event while cut off"
    );
    assert!(agenda_of(&mut world, "bob", 8, from, to).is_empty());

    world.heal("alice", "server");
    assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet);
    let agenda = agreed_agenda(&mut world, 8, from, to);
    assert_eq!(agenda.len(), 4);
}

/// A week of changes to a few events, at random times, with links cut and restored,
/// delays spread out and envelopes delivered twice: at the end the three copies show one
/// agenda for each event and it is the one the schedule says.
#[test]
fn a_random_week_of_schedule_changes_ends_with_one_agenda() {
    for seed in 1..=8u64 {
        let mut world = todo_world(seed, 5);
        let mut rng = Rng::new(seed);
        assert_eq!(world.run_until_quiet(1), Outcome::Quiet, "seed {seed}");
        world.jitter("alice", "server", 40);
        world.jitter("bob", "server", 40);
        world.reorder("alice", "server");
        world.reorder("bob", "server");
        world.degrade("alice", "server", 0, 20);
        world.degrade("bob", "server", 0, 20);
        let mut cut = [false, false];
        for _ in 0..60 {
            let who = if rng.below(2) == 0 { "alice" } else { "bob" };
            let id = 1 + rng.below(3);
            let (field, value) = match rng.below(7) {
                0 => {
                    let month = 1 + rng.below(6) as i64;
                    let day = 1 + rng.below(28) as i64;
                    let hour = rng.below(24) as i64;
                    (START, instant(2026, month, day, hour, 0))
                }
                1 => (LENGTH, [0, HOUR_MS, 90 * 60_000][rng.below(3) as usize]),
                2 => (REPEAT, rng.below(4) as i64),
                3 => (EVERY, rng.below(4) as i64),
                4 => (DAYS, rng.below(128) as i64),
                5 => (COUNT, rng.below(6) as i64),
                _ => (GONE, i64::from(rng.below(8) == 0)),
            };
            todo_set(&mut world, who, id, field, value);
            if rng.below(6) == 0 {
                let which = usize::from(rng.below(2) == 1);
                let name = ["alice", "bob"][which];
                if cut[which] {
                    world.heal(name, "server");
                } else {
                    world.partition(name, "server");
                }
                cut[which] = !cut[which];
            }
            world.run_until_quiet(rng.below(60) as i64 + 1);
        }
        for (which, name) in ["alice", "bob"].into_iter().enumerate() {
            if cut[which] {
                world.heal(name, "server");
            }
        }
        assert_eq!(
            world.run_until_quiet(600_000),
            Outcome::Quiet,
            "seed {seed}"
        );

        for who in ["alice", "bob", "server"] {
            assert_eq!(
                todo_variables_of(&mut world, who)["refusals"],
                0,
                "seed {seed}: {who} refused something"
            );
        }
        let (from, to) = jan_to_jun();
        let mut shown = 0;
        for id in 1..=3 {
            shown += agreed_agenda(&mut world, id, from, to).len();
        }
        assert!(world.host_errors().is_empty(), "seed {seed}");
        assert!(shown > 0, "seed {seed}: no event showed anything");
    }
}
