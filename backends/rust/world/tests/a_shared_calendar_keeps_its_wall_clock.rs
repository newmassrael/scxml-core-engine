// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A shared schedule in a time zone: what a calendar needs beyond repeating by the day.
// An event that starts at 09:30 keeps starting at 09:30 when the clock is set forward or
// back; a repetition can end (`until`); and a day can be left out of it, which two people
// can do at once without one undoing the other.
//
// The time zone is the host's, as the operating system's rules are: each copy is told its
// table of transitions (`todo.zone`) and the schedule's `start` is a wall-clock reading in
// that zone. The copy reads the schedule in the wall clock and turns each reading into an
// instant by the standard `utc_from_local`, so a reading the clock skipped and one it
// repeated are resolved by one rule on every backend (RFC 5545 §3.3.5).
//
// Each agenda is held to the expansion written a second time in `common/calendar.rs`, which
// reads the zone's table by another construction.

mod common;

use common::calendar::*;
use common::*;
use sce_rust_world::{Outcome, Rng};

fn starts(agenda: &[Occurrence]) -> Vec<i64> {
    agenda.iter().map(|occurrence| occurrence.start).collect()
}

fn quiet(world: &mut sce_rust_world::World) {
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);
}

/// 09:30 on Mondays, in New York, keeps being 09:30 when the clock is set forward on
/// 8 March: the instant moves an hour earlier in UTC and the wall clock does not.
#[test]
fn a_weekly_meeting_stays_at_its_wall_clock_across_the_clock_going_forward() {
    let mut world = todo_world(1, 20);
    let zone = new_york();
    set_zone_everywhere(&mut world, &zone);
    schedule_todo(
        &mut world,
        "alice",
        1,
        &Plan::once(instant(2026, 3, 2, 9, 30), HOUR_MS).repeating(WEEKLY, 1),
    );
    quiet(&mut world);

    let agenda = agreed_agenda_in(
        &mut world,
        Some(&zone),
        1,
        instant(2026, 3, 1, 0, 0),
        instant(2026, 3, 20, 0, 0),
    );
    assert_eq!(
        starts(&agenda),
        [
            instant(2026, 3, 2, 14, 30),
            instant(2026, 3, 9, 13, 30),
            instant(2026, 3, 16, 13, 30),
        ],
        "14:30Z in winter, 13:30Z once the clock is set forward"
    );
    assert!(agenda.iter().all(|o| o.end - o.start == HOUR_MS));
    assert!(world.host_errors().is_empty());
}

/// A reading the clock skipped, 02:30 on 8 March, is read with the offset before the
/// change, which lands it at 03:30 after.
#[test]
fn a_reading_the_clock_skipped_is_read_with_the_offset_before_the_change() {
    let mut world = todo_world(1, 20);
    let zone = new_york();
    set_zone_everywhere(&mut world, &zone);
    schedule_todo(
        &mut world,
        "bob",
        2,
        &Plan::once(instant(2026, 3, 7, 2, 30), 0).repeating(DAILY, 1),
    );
    quiet(&mut world);

    let agenda = agreed_agenda_in(
        &mut world,
        Some(&zone),
        2,
        instant(2026, 3, 7, 0, 0),
        instant(2026, 3, 10, 0, 0),
    );
    assert_eq!(
        starts(&agenda),
        [
            instant(2026, 3, 7, 7, 30),
            // 02:30 does not exist on 8 March: 02:30 by the offset before, 07:30Z.
            instant(2026, 3, 8, 7, 30),
            instant(2026, 3, 9, 6, 30),
        ]
    );
}

/// A reading the clock repeated, 01:30 on 1 November, is the first of its two instants.
#[test]
fn a_reading_the_clock_repeated_is_its_first_instant() {
    let mut world = todo_world(1, 20);
    let zone = new_york();
    set_zone_everywhere(&mut world, &zone);
    schedule_todo(
        &mut world,
        "alice",
        3,
        &Plan::once(instant(2026, 10, 31, 1, 30), 0).repeating(DAILY, 1),
    );
    quiet(&mut world);

    let agenda = agreed_agenda_in(
        &mut world,
        Some(&zone),
        3,
        instant(2026, 10, 31, 0, 0),
        instant(2026, 11, 3, 0, 0),
    );
    assert_eq!(
        starts(&agenda),
        [
            instant(2026, 10, 31, 5, 30),
            // 01:30 happens twice on 1 November: the one before the clock is set back.
            instant(2026, 11, 1, 5, 30),
            instant(2026, 11, 2, 6, 30),
        ]
    );
}

/// Seoul has no change of the clock: nine hours ahead all year.
#[test]
fn a_zone_without_a_change_of_the_clock_shifts_every_occurrence_alike() {
    let mut world = todo_world(1, 20);
    let zone = seoul();
    set_zone_everywhere(&mut world, &zone);
    schedule_todo(
        &mut world,
        "bob",
        4,
        &Plan::once(instant(2026, 6, 1, 8, 0), 30 * 60_000).repeating(DAILY, 1),
    );
    todo_set(&mut world, "bob", 4, COUNT, 3);
    quiet(&mut world);

    let agenda = agreed_agenda_in(
        &mut world,
        Some(&zone),
        4,
        instant(2026, 5, 31, 0, 0),
        instant(2026, 6, 30, 0, 0),
    );
    assert_eq!(
        starts(&agenda),
        [
            instant(2026, 5, 31, 23, 0),
            instant(2026, 6, 1, 23, 0),
            instant(2026, 6, 2, 23, 0),
        ]
    );
}

/// The zone is each host's. Two copies told different zones read one wall-clock reading
/// at different instants: an event is a reading in the zone of whoever shows it, and the
/// people of one calendar are told one zone.
#[test]
fn copies_told_different_zones_show_one_reading_at_different_instants() {
    let mut world = todo_world(1, 20);
    set_zone(&mut world, "alice", &seoul());
    set_zone(&mut world, "bob", &new_york());
    set_zone(&mut world, "server", &new_york());
    quiet(&mut world);
    schedule_todo(
        &mut world,
        "alice",
        5,
        &Plan::once(instant(2026, 4, 6, 9, 0), HOUR_MS),
    );
    quiet(&mut world);

    let (from, to) = (instant(2026, 4, 1, 0, 0), instant(2026, 5, 1, 0, 0));
    let in_seoul = agenda_of(&mut world, "alice", 5, from, to);
    let in_new_york = agenda_of(&mut world, "bob", 5, from, to);
    assert_eq!(starts(&in_seoul), [instant(2026, 4, 6, 0, 0)]);
    assert_eq!(starts(&in_new_york), [instant(2026, 4, 6, 13, 0)]);
}

/// A repetition ends at a reading, inclusive; the days after it are not occurrences.
#[test]
fn a_repetition_ends_at_its_until_reading_inclusive() {
    let mut world = todo_world(1, 20);
    schedule_todo(
        &mut world,
        "alice",
        6,
        &Plan::once(instant(2026, 1, 1, 8, 0), HOUR_MS).repeating(DAILY, 1),
    );
    todo_set(&mut world, "alice", 6, UNTIL, instant(2026, 1, 4, 8, 0));
    schedule_todo(
        &mut world,
        "bob",
        7,
        &Plan::once(instant(2026, 1, 1, 8, 0), HOUR_MS).repeating(DAILY, 1),
    );
    todo_set(&mut world, "bob", 7, UNTIL, instant(2026, 1, 4, 7, 59));
    quiet(&mut world);

    let (from, to) = (instant(2026, 1, 1, 0, 0), instant(2026, 2, 1, 0, 0));
    let inclusive = agreed_agenda(&mut world, 6, from, to);
    assert_eq!(inclusive.len(), 4, "the 4th is the until reading itself");
    let before = agreed_agenda(&mut world, 7, from, to);
    assert_eq!(
        before.len(),
        3,
        "a minute before the 4th's start excludes it"
    );
}

/// A day left out is not shown but still counts toward the count: a count of three from
/// the 1st with the 2nd left out shows the 1st and the 3rd, and no fourth.
#[test]
fn a_day_left_out_is_not_shown_and_still_counts() {
    let mut world = todo_world(1, 20);
    let plan = Plan {
        count: 3,
        ..Plan::once(instant(2026, 1, 1, 8, 0), HOUR_MS).repeating(DAILY, 1)
    };
    schedule_todo(&mut world, "alice", 8, &plan);
    quiet(&mut world);
    leave_out(&mut world, "bob", 8, day_number(2026, 1, 2), true);
    quiet(&mut world);

    let agenda = agreed_agenda(
        &mut world,
        8,
        instant(2026, 1, 1, 0, 0),
        instant(2026, 2, 1, 0, 0),
    );
    assert_eq!(
        starts(&agenda),
        [instant(2026, 1, 1, 8, 0), instant(2026, 1, 3, 8, 0)]
    );
    assert_eq!(agenda.iter().map(|o| o.nth).collect::<Vec<_>>(), [0, 2]);
    assert!(agenda.iter().all(|o| o.conflict == 0));
}

/// Two people leave out two different days at once: each is its own field, so both stay.
#[test]
fn two_days_left_out_at_once_are_both_left_out() {
    let mut world = todo_world(1, 20);
    schedule_todo(
        &mut world,
        "alice",
        9,
        &Plan::once(instant(2026, 5, 4, 18, 0), HOUR_MS).repeating(DAILY, 1),
    );
    quiet(&mut world);
    leave_out(&mut world, "alice", 9, day_number(2026, 5, 6), true);
    leave_out(&mut world, "bob", 9, day_number(2026, 5, 7), true);
    quiet(&mut world);

    let agenda = agreed_agenda(
        &mut world,
        9,
        instant(2026, 5, 4, 0, 0),
        instant(2026, 5, 10, 0, 0),
    );
    assert_eq!(
        starts(&agenda),
        [
            instant(2026, 5, 4, 18, 0),
            instant(2026, 5, 5, 18, 0),
            instant(2026, 5, 8, 18, 0),
            instant(2026, 5, 9, 18, 0),
        ]
    );
}

/// One leaves a day out and the other puts it back at the same moment: neither saw the
/// other's, both stay, the later write is what every copy shows, and that occurrence says
/// the people have not agreed. Someone who has seen both settles it.
#[test]
fn a_day_left_out_and_put_back_at_once_is_a_conflict_until_someone_settles_it() {
    let mut world = todo_world(1, 20);
    schedule_todo(
        &mut world,
        "alice",
        10,
        &Plan::once(instant(2026, 5, 4, 18, 0), HOUR_MS).repeating(DAILY, 1),
    );
    quiet(&mut world);
    let day = day_number(2026, 5, 6);
    leave_out(&mut world, "alice", 10, day, true);
    leave_out(&mut world, "bob", 10, day, false);
    quiet(&mut world);

    let (from, to) = (instant(2026, 5, 4, 0, 0), instant(2026, 5, 9, 0, 0));
    let agenda = agreed_agenda(&mut world, 10, from, to);
    let sixth = agenda
        .iter()
        .find(|o| o.start == instant(2026, 5, 6, 18, 0))
        .expect("bob's later write puts the day back");
    assert_eq!(sixth.conflict, 1);
    assert!(
        agenda
            .iter()
            .filter(|o| o.start != sixth.start)
            .all(|o| o.conflict == 0),
        "only the day the people disagree about says so"
    );

    leave_out(&mut world, "alice", 10, day, true);
    quiet(&mut world);
    let settled = agreed_agenda(&mut world, 10, from, to);
    assert!(settled
        .iter()
        .all(|o| o.start != instant(2026, 5, 6, 18, 0)));
    assert!(settled.iter().all(|o| o.conflict == 0));
}

/// A month of changes to a few events in a zone, at random times, with links cut and
/// restored: at the end the three copies show one agenda for each event and it is the one
/// the schedule says, in instants.
#[test]
fn a_random_month_in_new_york_ends_with_one_agenda() {
    let zone = new_york();
    let mut shown = 0;
    for seed in 1..=8u64 {
        let mut world = todo_world(seed, 5);
        set_zone_everywhere(&mut world, &zone);
        let mut rng = Rng::new(seed);
        assert_eq!(world.run_until_quiet(1), Outcome::Quiet, "seed {seed}");
        world.jitter("alice", "server", 40);
        world.jitter("bob", "server", 40);
        world.reorder("alice", "server");
        world.reorder("bob", "server");
        world.degrade("alice", "server", 0, 20);
        world.degrade("bob", "server", 0, 20);
        // Three events that have a schedule to begin with, each repeating another way, so
        // that the changes below act on something that shows occurrences.
        for (id, repeat) in [(1, DAILY), (2, WEEKLY), (3, MONTHLY)] {
            let plan =
                Plan::once(instant(2026, 3, 1 + id as i64, 9, 30), HOUR_MS).repeating(repeat, 1);
            schedule_todo(&mut world, "alice", id, &plan);
        }
        assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet, "seed {seed}");
        let mut cut = [false, false];
        for _ in 0..70 {
            let who = if rng.below(2) == 0 { "alice" } else { "bob" };
            let id = 1 + rng.below(3);
            let (field, value) = match rng.below(9) {
                0 => {
                    let day = 1 + rng.below(28) as i64;
                    (START, instant(2026, 3, day, rng.below(24) as i64, 30))
                }
                1 => (LENGTH, [0, HOUR_MS, 90 * 60_000][rng.below(3) as usize]),
                2 => (REPEAT, rng.below(4) as i64),
                3 => (EVERY, rng.below(3) as i64),
                4 => (DAYS, rng.below(128) as i64),
                5 => (COUNT, [0, 0, 0, 5, 12][rng.below(5) as usize]),
                6 => {
                    let day = 1 + rng.below(28) as i64;
                    (
                        UNTIL,
                        [0, instant(2026, 3, day, 12, 0)][rng.below(2) as usize],
                    )
                }
                7 => (GONE, i64::from(rng.below(20) == 0)),
                _ => {
                    let day = day_number(2026, 3, 1 + rng.below(28) as i64);
                    (except_field(day), i64::from(rng.below(2) == 0))
                }
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
        let (from, to) = (instant(2026, 3, 1, 0, 0), instant(2026, 4, 10, 0, 0));
        for id in 1..=3 {
            shown += agreed_agenda_in(&mut world, Some(&zone), id, from, to).len();
        }
        assert!(world.host_errors().is_empty(), "seed {seed}");
    }
    // A seed may leave every event removed, ended or left out; the run as a whole must
    // have shown occurrences, or the agreement above says nothing.
    assert!(
        shown >= 40,
        "the eight seeds showed only {shown} occurrences between them"
    );
}
