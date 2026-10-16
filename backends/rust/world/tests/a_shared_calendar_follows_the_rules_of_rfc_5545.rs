// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A shared schedule that repeats by the rules of RFC 5545 §3.3.10: the period (a day, a
// week from WKST, a month, a year), INTERVAL, and the parts that select days of it
// (BYMONTH, BYMONTHDAY, BYDAY with and without an ordinal, BYYEARDAY, BYWEEKNO) and then
// pick among them (BYSETPOS), with COUNT, UNTIL and the days people leave out.
//
// The first tests are cases worked out by hand from the calendar: a Thanksgiving, the last
// weekday of the month, the last Friday, ISO week 20, and the example RFC 5545 itself gives
// of WKST changing what a fortnightly repetition is. The last is a differential: many
// events with random rules are written to the three copies, and each agenda is held to the
// expansion written a second time in `common/calendar.rs`, which shares no code with the
// statechart.

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

/// The occurrences the copies agree on for `plan`, in the window `[from, to)` of whole days.
fn shown(plan: &Plan, from: (i64, i64, i64), to: (i64, i64, i64)) -> Vec<i64> {
    let mut world = todo_world(1, 20);
    schedule_todo(&mut world, "alice", 1, plan);
    quiet(&mut world);
    let (from, to) = (
        instant(from.0, from.1, from.2, 0, 0),
        instant(to.0, to.1, to.2, 0, 0),
    );
    let agenda = agreed_agenda(&mut world, 1, from, to);
    assert!(world.host_errors().is_empty());
    starts(&agenda)
}

fn at(year: i64, month: i64, day: i64, hour: i64) -> i64 {
    instant(year, month, day, hour, 0)
}

#[test]
fn a_thanksgiving_is_the_fourth_thursday_of_november() {
    let plan = Plan::once(at(2026, 1, 1, 12), HOUR_MS)
        .repeating(YEARLY, 1)
        .in_months(&[11])
        .on_ordinals(&[(4, THU)]);
    assert_eq!(
        shown(&plan, (2026, 1, 1), (2029, 1, 1)),
        [
            at(2026, 11, 26, 12),
            at(2027, 11, 25, 12),
            at(2028, 11, 23, 12)
        ]
    );
}

#[test]
fn the_last_weekday_of_each_month_is_the_last_of_the_days_a_month_selects() {
    let plan = Plan::once(at(2026, 1, 1, 17), 0)
        .repeating(MONTHLY, 1)
        .on_weekdays(&[MON, TUE, WED, THU, FRI])
        .at_set_positions(&[-1]);
    assert_eq!(
        shown(&plan, (2026, 1, 1), (2026, 7, 1)),
        [
            at(2026, 1, 30, 17),
            at(2026, 2, 27, 17),
            at(2026, 3, 31, 17),
            at(2026, 4, 30, 17),
            at(2026, 5, 29, 17),
            at(2026, 6, 30, 17),
        ]
    );
}

#[test]
fn the_first_and_the_last_weekday_of_a_month_are_the_first_and_last_places() {
    let plan = Plan::once(at(2026, 1, 1, 8), 0)
        .repeating(MONTHLY, 1)
        .on_weekdays(&[MON, TUE, WED, THU, FRI])
        .at_set_positions(&[1, -1]);
    assert_eq!(
        shown(&plan, (2026, 1, 1), (2026, 3, 1)),
        [
            at(2026, 1, 1, 8),
            at(2026, 1, 30, 8),
            at(2026, 2, 2, 8),
            at(2026, 2, 27, 8),
        ]
    );
}

#[test]
fn the_last_friday_and_the_second_tuesday_of_each_month() {
    let last_friday = Plan::once(at(2026, 1, 1, 18), 0)
        .repeating(MONTHLY, 1)
        .on_ordinals(&[(-1, FRI)]);
    assert_eq!(
        shown(&last_friday, (2026, 1, 1), (2026, 7, 1)),
        [
            at(2026, 1, 30, 18),
            at(2026, 2, 27, 18),
            at(2026, 3, 27, 18),
            at(2026, 4, 24, 18),
            at(2026, 5, 29, 18),
            at(2026, 6, 26, 18),
        ]
    );
    let second_tuesday = Plan::once(at(2026, 1, 1, 9), 0)
        .repeating(MONTHLY, 1)
        .on_ordinals(&[(2, TUE)]);
    assert_eq!(
        shown(&second_tuesday, (2026, 1, 1), (2026, 4, 1)),
        [at(2026, 1, 13, 9), at(2026, 2, 10, 9), at(2026, 3, 10, 9)]
    );
}

#[test]
fn the_fifteenth_and_the_last_day_of_each_month() {
    let plan = Plan::once(at(2026, 1, 1, 9), 0)
        .repeating(MONTHLY, 1)
        .on_month_days(&[15, -1]);
    assert_eq!(
        shown(&plan, (2026, 1, 1), (2026, 4, 1)),
        [
            at(2026, 1, 15, 9),
            at(2026, 1, 31, 9),
            at(2026, 2, 15, 9),
            at(2026, 2, 28, 9),
            at(2026, 3, 15, 9),
            at(2026, 3, 31, 9),
        ]
    );
}

/// ISO 8601 week 1 is the first with four days in the year: Monday 29 December 2025,
/// 4 January 2027 and 3 January 2028 begin the weeks 20 of which fall on 11 May, 17 May
/// and 15 May.
#[test]
fn week_twenty_is_counted_the_way_iso_8601_counts_it() {
    let plan = Plan::once(at(2026, 1, 1, 7), 0)
        .repeating(YEARLY, 1)
        .in_weeks(&[20])
        .on_weekdays(&[MON]);
    assert_eq!(
        shown(&plan, (2026, 1, 1), (2029, 1, 1)),
        [at(2026, 5, 11, 7), at(2027, 5, 17, 7), at(2028, 5, 15, 7)]
    );
    let last_week = Plan::once(at(2026, 1, 1, 7), 0)
        .repeating(YEARLY, 1)
        .in_weeks(&[-1])
        .on_weekdays(&[MON]);
    // 2026 has 53 weeks, its last beginning on Monday 28 December; 2027 has 52, its last
    // beginning on 27 December; 2028 has 52, its last beginning on 25 December.
    assert_eq!(
        shown(&last_week, (2026, 1, 1), (2029, 1, 1)),
        [
            at(2026, 12, 28, 7),
            at(2027, 12, 27, 7),
            at(2028, 12, 25, 7)
        ]
    );
}

#[test]
fn the_days_of_the_year_a_list_names_are_counted_from_either_end() {
    let plan = Plan::once(at(2026, 1, 1, 6), 0)
        .repeating(YEARLY, 1)
        .on_year_days(&[1, 100, -1]);
    assert_eq!(
        shown(&plan, (2026, 1, 1), (2029, 1, 1)),
        [
            at(2026, 1, 1, 6),
            at(2026, 4, 10, 6),
            at(2026, 12, 31, 6),
            at(2027, 1, 1, 6),
            at(2027, 4, 10, 6),
            at(2027, 12, 31, 6),
            at(2028, 1, 1, 6),
            at(2028, 4, 9, 6),
            at(2028, 12, 31, 6),
        ],
        "the 100th day is 10 April, and 9 April in a leap year"
    );
}

/// RFC 5545 §3.3.10: DTSTART 1997-08-05, FREQ=WEEKLY;INTERVAL=2;COUNT=4;BYDAY=TU,SU. With
/// WKST=MO the days are 5, 10, 19 and 24 August; with WKST=SU they are 5, 17, 19 and 31.
#[test]
fn the_week_a_fortnight_begins_on_changes_which_days_it_holds() {
    let fortnightly = Plan {
        count: 4,
        ..Plan::once(at(1997, 8, 5, 9), 0)
            .repeating(WEEKLY, 2)
            .on_weekdays(&[TUE, SUN])
    };
    assert_eq!(
        shown(&fortnightly, (1997, 8, 1), (1997, 10, 1)),
        [
            at(1997, 8, 5, 9),
            at(1997, 8, 10, 9),
            at(1997, 8, 19, 9),
            at(1997, 8, 24, 9)
        ],
        "a week begins on Monday when nothing says otherwise"
    );
    assert_eq!(
        shown(
            &fortnightly.week_begins_on(MON),
            (1997, 8, 1),
            (1997, 10, 1)
        ),
        starts_of(&[(8, 5), (8, 10), (8, 19), (8, 24)]),
    );
    assert_eq!(
        shown(
            &fortnightly.week_begins_on(SUN),
            (1997, 8, 1),
            (1997, 10, 1)
        ),
        starts_of(&[(8, 5), (8, 17), (8, 19), (8, 31)]),
    );
}

fn starts_of(days: &[(i64, i64)]) -> Vec<i64> {
    days.iter()
        .map(|(month, day)| at(1997, *month, *day, 9))
        .collect()
}

#[test]
fn a_yearly_event_on_the_29th_of_february_falls_on_the_28th_in_other_years() {
    let plan = Plan::once(at(2024, 2, 29, 10), 0).repeating(YEARLY, 1);
    assert_eq!(
        shown(&plan, (2024, 1, 1), (2029, 1, 1)),
        [
            at(2024, 2, 29, 10),
            at(2025, 2, 28, 10),
            at(2026, 2, 28, 10),
            at(2027, 2, 28, 10),
            at(2028, 2, 29, 10),
        ]
    );
}

/// RFC 5545 skips a month that has no such day as the start's; the schedule says whether it
/// does or takes the last day of the month instead.
#[test]
fn a_month_without_the_day_has_no_occurrence_when_invalid_dates_are_skipped() {
    let monthly = Plan::once(at(2026, 1, 31, 9), 0)
        .repeating(MONTHLY, 1)
        .skipping_invalid_dates();
    assert_eq!(
        shown(&monthly, (2026, 1, 1), (2026, 9, 1)),
        [
            at(2026, 1, 31, 9),
            at(2026, 3, 31, 9),
            at(2026, 5, 31, 9),
            at(2026, 7, 31, 9),
            at(2026, 8, 31, 9),
        ]
    );
    let leap_day = Plan::once(at(2024, 2, 29, 10), 0)
        .repeating(YEARLY, 1)
        .skipping_invalid_dates();
    assert_eq!(
        shown(&leap_day, (2024, 1, 1), (2034, 1, 1)),
        [
            at(2024, 2, 29, 10),
            at(2028, 2, 29, 10),
            at(2032, 2, 29, 10)
        ]
    );
}

/// The ordinal of a weekday in the year goes up to 53, as RFC 5545's does: the twentieth
/// Monday, the last Monday, and the 53rd, which only some years have.
#[test]
fn an_ordinal_weekday_of_the_year_goes_up_to_the_fifty_third() {
    let twentieth = Plan::once(at(2026, 1, 1, 12), 0)
        .repeating(YEARLY, 1)
        .on_ordinals(&[(20, MON)]);
    assert_eq!(
        shown(&twentieth, (2026, 1, 1), (2029, 1, 1)),
        [
            at(2026, 5, 18, 12),
            at(2027, 5, 17, 12),
            at(2028, 5, 15, 12)
        ]
    );
    let last = Plan::once(at(2026, 1, 1, 12), 0)
        .repeating(YEARLY, 1)
        .on_ordinals(&[(-1, MON)]);
    assert_eq!(
        shown(&last, (2026, 1, 1), (2029, 1, 1)),
        [
            at(2026, 12, 28, 12),
            at(2027, 12, 27, 12),
            at(2028, 12, 25, 12)
        ]
    );
    // 2024 began on a Monday and is a leap year, so it has 53 Mondays; 2025 has 52.
    let fifty_third = Plan::once(at(2024, 1, 1, 12), 0)
        .repeating(YEARLY, 1)
        .on_ordinals(&[(53, MON)]);
    assert_eq!(
        shown(&fifty_third, (2024, 1, 1), (2027, 1, 1)),
        [at(2024, 12, 30, 12)]
    );
}

#[test]
fn a_month_the_bymonth_does_not_name_is_passed_over_and_an_interval_steps_months() {
    let some_months = Plan::once(at(2026, 1, 15, 8), 0)
        .repeating(MONTHLY, 1)
        .in_months(&[3, 6]);
    assert_eq!(
        shown(&some_months, (2026, 1, 1), (2027, 1, 1)),
        [at(2026, 3, 15, 8), at(2026, 6, 15, 8)]
    );
    let every_other = Plan::once(at(2026, 1, 31, 8), 0).repeating(MONTHLY, 2);
    assert_eq!(
        shown(&every_other, (2026, 1, 1), (2026, 9, 1)),
        [
            at(2026, 1, 31, 8),
            at(2026, 3, 31, 8),
            at(2026, 5, 31, 8),
            at(2026, 7, 31, 8),
        ]
    );
}

/// An agenda holds 64 occurrences and says where the next page begins: the pages put
/// together are every occurrence there is, once and in order.
#[test]
fn an_agenda_of_more_than_a_page_is_read_page_by_page() {
    let mut world = todo_world(1, 20);
    schedule_todo(
        &mut world,
        "alice",
        1,
        &Plan::once(at(2026, 1, 1, 7), 0).repeating(DAILY, 1),
    );
    quiet(&mut world);
    let (from, to) = (at(2026, 1, 1, 0), at(2027, 1, 1, 0));
    let mut all = Vec::new();
    let mut skip = 0;
    let mut pages = 0;
    loop {
        let page = agreed_page(&mut world, &Zones::new(), 1, from, to, skip);
        pages += 1;
        all.extend(page.items.iter().map(|o| (o.nth, o.start)));
        if page.next == 0 {
            break;
        }
        assert_eq!(page.items.len(), 64, "a page that has a next is full");
        assert_eq!(page.next, skip + 64);
        skip = page.next;
    }
    assert_eq!(pages, 6, "365 days are five full pages and a short one");
    assert_eq!(all.len(), 365);
    for (n, (nth, start)) in all.iter().enumerate() {
        assert_eq!(*nth as usize, n);
        assert_eq!(*start, at(2026, 1, 1, 7) + n as i64 * DAY_MS);
    }
}

/// Between one and `most` random numbers from `low` to `high`.
fn some_of(rng: &mut Rng, low: i64, high: i64, most: u64) -> Vec<i64> {
    let k = 1 + rng.below(most);
    (0..k)
        .map(|_| low + rng.below((high - low + 1) as u64) as i64)
        .collect()
}

/// A random repetition rule, the way a person might write one: a period and a step, and
/// any of the parts that select and pick, some of them not meaning anything under the
/// period (which the rules say to ignore), all of them from the sets the fields can hold.
fn random_plan(rng: &mut Rng) -> Plan {
    let start = instant(
        2026,
        1 + rng.below(12) as i64,
        1 + rng.below(28) as i64,
        rng.below(24) as i64,
        [0, 30][rng.below(2) as usize],
    );
    let mut plan = Plan::once(start, [0, HOUR_MS, 90 * 60_000][rng.below(3) as usize])
        .repeating(rng.below(5) as i64, rng.below(4) as i64);
    if rng.below(3) == 0 {
        plan = plan.on_weekdays(&some_of(rng, 0, 6, 4));
    }
    if rng.below(3) == 0 {
        let ordinals: Vec<(i64, i64)> = (0..1 + rng.below(3))
            .map(|_| {
                // Mostly the ordinals of a month, now and then one that only a year has.
                let most = if rng.below(4) == 0 { 53 } else { 5 };
                let n = 1 + rng.below(most) as i64;
                (if rng.below(2) == 0 { n } else { -n }, rng.below(7) as i64)
            })
            .collect();
        plan = plan.on_ordinals(&ordinals);
    }
    if rng.below(4) == 0 {
        plan = plan.skipping_invalid_dates();
    }
    if rng.below(4) == 0 {
        plan = plan.in_months(&some_of(rng, 1, 12, 4));
    }
    if rng.below(4) == 0 {
        let days: Vec<i64> = some_of(rng, 1, 31, 3)
            .into_iter()
            .map(|d| if rng.below(3) == 0 { -d } else { d })
            .collect();
        plan = plan.on_month_days(&days);
    }
    if rng.below(5) == 0 {
        let places: Vec<i64> = some_of(rng, 1, 6, 2)
            .into_iter()
            .map(|p| if rng.below(2) == 0 { -p } else { p })
            .collect();
        plan = plan.at_set_positions(&places);
    }
    if rng.below(5) == 0 {
        let weeks: Vec<i64> = some_of(rng, 1, 53, 3)
            .into_iter()
            .map(|w| if rng.below(4) == 0 { -(w.min(52)) } else { w })
            .collect();
        plan = plan.in_weeks(&weeks);
    }
    if rng.below(5) == 0 {
        let days: Vec<i64> = some_of(rng, 1, 366, 4)
            .into_iter()
            .map(|d| if rng.below(3) == 0 { -d } else { d })
            .collect();
        plan = plan.on_year_days(&days);
    }
    if rng.below(3) == 0 {
        plan = plan.week_begins_on(rng.below(7) as i64);
    }
    if rng.below(3) == 0 {
        plan.count = 1 + rng.below(20) as i64;
    }
    if rng.below(5) == 0 {
        plan.until = instant(
            2026 + rng.below(3) as i64,
            1 + rng.below(12) as i64,
            15,
            0,
            0,
        );
    }
    plan
}

/// Many events with random rules, written at random times to the three copies, and each
/// agenda read page by page, held to the expansion of the same fields written a second
/// time. The rules reach combinations nobody wrote a case for.
#[test]
fn random_rules_are_expanded_as_the_second_expansion_expands_them() {
    let (from, to) = (at(2026, 1, 1, 0), at(2030, 1, 1, 0));
    let mut occurrences = 0;
    let mut rules_that_showed_something = 0;
    // A copy holds 128 values (mv_value_keep) and a rule writes up to 19 of them, so a world
    // holds five events and there are forty worlds.
    for seed in 1..=40u64 {
        let mut world = todo_world(seed, 5);
        let mut rng = Rng::new(seed * 7919);
        for id in 1..=5u64 {
            let who = if rng.below(2) == 0 { "alice" } else { "bob" };
            schedule_todo(&mut world, who, id, &random_plan(&mut rng));
            world.run_until_quiet(rng.below(30) as i64 + 1);
        }
        assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet, "seed {seed}");
        for who in ["alice", "bob", "server"] {
            assert_eq!(
                todo_variables_of(&mut world, who)["refusals"],
                0,
                "seed {seed}: {who} refused something"
            );
        }
        for id in 1..=5u64 {
            let mut skip = 0;
            let mut total = 0;
            loop {
                let page = agreed_page(&mut world, &Zones::new(), id, from, to, skip);
                total += page.items.len();
                if page.next == 0 {
                    break;
                }
                skip = page.next;
            }
            occurrences += total;
            rules_that_showed_something += usize::from(total > 0);
        }
        assert!(world.host_errors().is_empty(), "seed {seed}");
    }
    assert!(
        rules_that_showed_something >= 100,
        "only {rules_that_showed_something} of 200 random rules showed an occurrence"
    );
    assert!(occurrences > 1_000, "only {occurrences} occurrences shown");
}
