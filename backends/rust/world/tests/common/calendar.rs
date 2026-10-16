// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// What the tests of a shared schedule have in common: a to-do that has a start, a length
// and a repetition is an event, a copy of the to-do list shows what is scheduled in a
// window (`todo.agenda`, the fixture `event_agenda.scxml`), and the ways of holding the
// three copies' agendas to the schedule the network says they should show.
//
// The expansion of a repetition is written out here a second time, in Rust, from the
// rules the document states and by another construction: the statechart scans the days
// or the months from the start and tests each, this generates the occurrences in order
// from the rules, and counts civil dates with a table of month lengths where the
// statechart uses Hinnant's day arithmetic. An agenda that is wrong at a month's end, a
// week's turn or a count is wrong here the same way only if both made the same mistake.

use std::collections::BTreeMap;

use sce_rust_world::World;
use serde_json::Value;

use super::{agreed_todos, todo_set, todo_variables_of, whole, Field};

/// The fields of a scheduled to-do, by the numbers the host gives them
/// (`event_agenda.scxml`).
pub const START: u64 = 16;
pub const LENGTH: u64 = 17;
pub const REPEAT: u64 = 18;
pub const EVERY: u64 = 19;
pub const DAYS: u64 = 20;
pub const COUNT: u64 = 21;
pub const GONE: u64 = 9;

/// The repetitions.
pub const ONCE: i64 = 0;
pub const DAILY: i64 = 1;
pub const WEEKLY: i64 = 2;
pub const MONTHLY: i64 = 3;

pub const DAY_MS: i64 = 86_400_000;
pub const HOUR_MS: i64 = 3_600_000;

/// The most occurrences an agenda holds.
pub const AGENDA_MAX: usize = 64;

/// One occurrence an agenda shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Occurrence {
    pub id: u64,
    pub start: i64,
    pub end: i64,
    pub nth: u32,
    pub conflict: u8,
}

fn leap(year: i64) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

fn month_length(year: i64, month: i64) -> i64 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ if leap(year) => 29,
        _ => 28,
    }
}

/// The day count since 1970-01-01 of a civil date from 1970 on, by walking the years and
/// the months.
pub fn day_number(year: i64, month: i64, day: i64) -> i64 {
    assert!(year >= 1970 && (1..=12).contains(&month) && day >= 1);
    let mut days = 0;
    for y in 1970..year {
        days += if leap(y) { 366 } else { 365 };
    }
    for m in 1..month {
        days += month_length(year, m);
    }
    days + day - 1
}

/// The civil date of a day count from 1970 on, by the same walk back.
fn civil_of(mut days: i64) -> (i64, i64, i64) {
    assert!(days >= 0);
    let mut year = 1970;
    loop {
        let in_year = if leap(year) { 366 } else { 365 };
        if days < in_year {
            break;
        }
        days -= in_year;
        year += 1;
    }
    let mut month = 1;
    while days >= month_length(year, month) {
        days -= month_length(year, month);
        month += 1;
    }
    (year, month, days + 1)
}

/// The instant a civil date and a time of day are, in UTC.
pub fn instant(year: i64, month: i64, day: i64, hour: i64, minute: i64) -> i64 {
    day_number(year, month, day) * DAY_MS + hour * HOUR_MS + minute * 60_000
}

/// What a to-do's fields say it is scheduled as, as the fields hold it once the copies
/// agree: each field read as the value written last.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Schedule {
    pub start: Option<i64>,
    pub length: i64,
    pub repeat: i64,
    pub every: i64,
    pub days: i64,
    pub count: i64,
    pub gone: bool,
    /// Two people changed one of the fields at once and no one has chosen.
    pub conflict: bool,
}

/// The schedule of to-do `id` in the list the copies agree on.
pub fn schedule_in(list: &BTreeMap<(u64, u64), Field>, id: u64) -> Schedule {
    let value = |field: u64, none: i64| list.get(&(id, field)).map_or(none, |held| held.value);
    let split = [START, LENGTH, REPEAT, EVERY, DAYS, COUNT, GONE]
        .iter()
        .any(|field| {
            list.get(&(id, *field))
                .is_some_and(|held| held.live.len() > 1)
        });
    Schedule {
        start: list.get(&(id, START)).map(|held| held.value),
        length: value(LENGTH, 0),
        repeat: value(REPEAT, 0),
        every: value(EVERY, 1).max(1),
        days: value(DAYS, 0),
        count: value(COUNT, 0),
        gone: value(GONE, 0) == 1,
        conflict: split,
    }
}

/// The occurrences of a schedule that overlap `[from, to)`, in order, by the rules of
/// `event_agenda.scxml`, generated from the first occurrence on.
pub fn expand(id: u64, schedule: &Schedule, from: i64, to: i64) -> Vec<Occurrence> {
    let Some(start) = schedule.start else {
        return Vec::new();
    };
    if start < 0
        || schedule.gone
        || !(ONCE..=MONTHLY).contains(&schedule.repeat)
        || to <= from
        || to <= 0
    {
        return Vec::new();
    }
    let time_of_day = start % DAY_MS;
    let first_day = start / DAY_MS;
    let every = schedule.every;
    let flag = u8::from(schedule.conflict);
    // The days of the schedule's occurrences, in order, generated as far as there is a
    // reason to look: a day is a candidate while it begins before the window ends.
    let mut days: Vec<i64> = Vec::new();
    let limit = (to + DAY_MS - 1) / DAY_MS;
    match schedule.repeat {
        ONCE => days.push(first_day),
        DAILY => {
            let mut day = first_day;
            while day < limit {
                days.push(day);
                day += every;
            }
        }
        WEEKLY => {
            let weekday = (first_day + 4).rem_euclid(7);
            let mask = if schedule.days == 0 {
                1 << weekday
            } else {
                schedule.days & 127
            };
            let week_start = first_day - weekday;
            let mut week = 0;
            while week_start + week * 7 < limit {
                for wd in 0..7 {
                    let day = week_start + week * 7 + wd;
                    if (mask >> wd) & 1 == 1 && day >= first_day {
                        days.push(day);
                    }
                }
                week += every;
            }
        }
        _ => {
            let (year, month, day_of_month) = civil_of(first_day);
            let mut at = year * 12 + (month - 1);
            loop {
                let (y, m) = (at / 12, at % 12 + 1);
                let day = day_number(y, m, day_of_month.min(month_length(y, m)));
                if day >= limit {
                    break;
                }
                days.push(day);
                at += every;
            }
        }
    }
    let mut out = Vec::new();
    for (nth, day) in days.into_iter().enumerate() {
        if schedule.count > 0 && nth as i64 >= schedule.count {
            break;
        }
        let begins = day * DAY_MS + time_of_day;
        if begins >= to {
            break;
        }
        let ends = begins + schedule.length;
        if (ends > from || begins >= from) && out.len() < AGENDA_MAX {
            out.push(Occurrence {
                id,
                start: begins,
                end: ends,
                nth: nth as u32,
                conflict: flag,
            });
        }
    }
    out
}

/// What a person writes into the fields of a schedule.
#[derive(Debug, Clone, Copy)]
pub struct Plan {
    pub start: i64,
    pub length: i64,
    pub repeat: i64,
    pub every: i64,
    pub days: i64,
    pub count: i64,
}

impl Plan {
    /// A schedule that happens once at `start`, for `length`.
    pub fn once(start: i64, length: i64) -> Self {
        Plan {
            start,
            length,
            repeat: ONCE,
            every: 1,
            days: 0,
            count: 0,
        }
    }

    /// The same, repeating `repeat` every `every`.
    pub fn repeating(self, repeat: i64, every: i64) -> Self {
        Plan {
            repeat,
            every,
            ..self
        }
    }
}

/// A person gives a to-do a schedule: every field of it, written in one go.
pub fn schedule_todo(world: &mut World, who: &str, id: u64, plan: &Plan) {
    for (field, value) in [
        (START, plan.start),
        (LENGTH, plan.length),
        (REPEAT, plan.repeat),
        (EVERY, plan.every),
        (DAYS, plan.days),
        (COUNT, plan.count),
    ] {
        todo_set(world, who, id, field, value);
    }
}

/// Ask a copy for the agenda of to-do `id` in `[from, to)` and read what it shows.
pub fn agenda_of(world: &mut World, who: &str, id: u64, from: i64, to: i64) -> Vec<Occurrence> {
    world.raise(
        who,
        "todo.agenda",
        &format!(r#"{{"id":{id},"windowStart":"{from}","windowEnd":"{to}"}}"#),
    );
    world.run_until_quiet(10);
    let variables = todo_variables_of(world, who);
    variables["agenda"]
        .as_array()
        .expect("an agenda is a list")
        .iter()
        .map(|line: &Value| Occurrence {
            id: whole(&line["id"]) as u64,
            start: whole(&line["start"]) as i64,
            end: whole(&line["end"]) as i64,
            nth: whole(&line["nth"]) as u32,
            conflict: whole(&line["conflict"]) as u8,
        })
        .collect()
}

/// The three copies show one agenda for the to-do; which. And it is the one the schedule
/// the copies agree on says it is.
pub fn agreed_agenda(world: &mut World, id: u64, from: i64, to: i64) -> Vec<Occurrence> {
    let server = agenda_of(world, "server", id, from, to);
    for who in ["alice", "bob"] {
        assert_eq!(
            agenda_of(world, who, id, from, to),
            server,
            "{who} and the hub show different agendas for to-do {id}"
        );
    }
    let list = agreed_todos(world);
    assert_eq!(
        server,
        expand(id, &schedule_in(&list, id), from, to),
        "the agenda of to-do {id} is not what its schedule says"
    );
    server
}
