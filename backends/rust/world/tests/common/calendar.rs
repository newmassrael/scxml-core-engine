// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// What the tests of a shared schedule have in common: a to-do that has a start, a length
// and a repetition is an event, a copy of the to-do list shows what is scheduled in a
// window (`todo.agenda`, the fixtures `event_agenda.scxml` and `todo_peer.scxml`), and the
// ways of holding the three copies' agendas to the schedule the network says they should
// show.
//
// The expansion of a repetition is written out here a second time, in Rust, from the rules
// `event_agenda.scxml` states and by another construction: the statechart tests every day
// of a period against bit masks, this reads each field into the sets it stands for and
// asks of every day whether it is in them; the statechart counts civil dates with Hinnant's
// day arithmetic, this with a table of month lengths and a walk over the years; the
// statechart finds the week of a day by a closed formula, this by the day the first week
// begins on. An agenda that is wrong at a month's end, a week's turn, an ordinal weekday,
// a set position or a count is wrong here the same way only if both made the same mistake.

use std::collections::BTreeMap;

use sce_rust_world::World;
use serde_json::Value;

use super::{agreed_todos, todo_set, todo_variables_of, whole, Field};

/// The fields of a scheduled to-do, by the numbers the host gives them
/// (`event_agenda.scxml`).
pub const GONE: u64 = 9;
pub const ZONE: u64 = 15;
pub const START: u64 = 16;
pub const LENGTH: u64 = 17;
pub const REPEAT: u64 = 18;
pub const EVERY: u64 = 19;
pub const DAYS: u64 = 20;
pub const COUNT: u64 = 21;
pub const UNTIL: u64 = 22;
pub const BYMONTH: u64 = 23;
pub const BYMONTHDAY: u64 = 24;
pub const BYMONTHDAY_NEG: u64 = 25;
pub const BYDAY_A: u64 = 26;
pub const BYDAY_B: u64 = 27;
pub const BYSETPOS: u64 = 28;
pub const BYSETPOS_NEG: u64 = 29;
pub const WKST: u64 = 30;
pub const BYWEEKNO: u64 = 31;
pub const BYWEEKNO_NEG: u64 = 32;
pub const BYYEARDAY: u64 = 33;
pub const BYYEARDAY_NEG: u64 = 39;

/// The repetitions.
pub const ONCE: i64 = 0;
pub const DAILY: i64 = 1;
pub const WEEKLY: i64 = 2;
pub const MONTHLY: i64 = 3;
pub const YEARLY: i64 = 4;

/// The weekdays, Sunday as 0.
pub const SUN: i64 = 0;
pub const MON: i64 = 1;
pub const TUE: i64 = 2;
pub const WED: i64 = 3;
pub const THU: i64 = 4;
pub const FRI: i64 = 5;
pub const SAT: i64 = 6;

pub const DAY_MS: i64 = 86_400_000;
pub const HOUR_MS: i64 = 3_600_000;

/// The most occurrences an agenda holds.
pub const AGENDA_MAX: usize = 64;

/// A day left out of a repetition is a field of its own: 64 plus the day's count since
/// 1970-01-01, holding 1 to leave the day out and 0 to put it back.
pub const EXCEPT_BASE: u64 = 64;
pub const EXCEPT_DAYS: i64 = 65_472;

/// One occurrence an agenda shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Occurrence {
    pub id: u64,
    pub start: i64,
    pub end: i64,
    pub nth: u32,
    pub conflict: u8,
}

/// A page of an agenda and where the next begins (0 when there is no next).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    pub items: Vec<Occurrence>,
    pub next: u32,
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

fn year_length(year: i64) -> i64 {
    if leap(year) {
        366
    } else {
        365
    }
}

/// The day count since 1970-01-01 of a civil date from 1970 on, by walking the years and
/// the months.
pub fn day_number(year: i64, month: i64, day: i64) -> i64 {
    assert!(year >= 1970 && (1..=12).contains(&month) && day >= 1);
    let mut days = 0;
    for y in 1970..year {
        days += year_length(y);
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
        let in_year = year_length(year);
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

/// The weekday of a day count, Sunday as 0.
fn weekday_of(day: i64) -> i64 {
    (day + 4).rem_euclid(7)
}

/// The instant a civil date and a time of day are, in UTC.
pub fn instant(year: i64, month: i64, day: i64, hour: i64, minute: i64) -> i64 {
    day_number(year, month, day) * DAY_MS + hour * HOUR_MS + minute * 60_000
}

/// The field that leaves out (or puts back) the occurrence of a day, by its count since
/// 1970-01-01.
pub fn except_field(day: i64) -> u64 {
    assert!(
        (0..EXCEPT_DAYS).contains(&day),
        "day {day} has no exception field"
    );
    day as u64 + EXCEPT_BASE
}

/// What the copies hold for the exception of one day.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DayMark {
    pub left_out: bool,
    /// Two people set the day at once and no one has chosen.
    pub split: bool,
}

/// A time zone as a host holds it: the instants (seconds) its offset changes at, and the
/// offset (seconds) in force from each.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Zone {
    pub starts: Vec<i64>,
    pub offsets: Vec<i32>,
}

/// The zones a host holds, by the numbers 1 to 4 an event names them by.
pub type Zones = BTreeMap<u64, Zone>;

impl Zone {
    /// The offset in force at the instant `t`, in seconds.
    pub fn offset_at(&self, t: i64) -> i64 {
        let mut offset = None;
        for (start, off) in self.starts.iter().zip(&self.offsets) {
            if *start <= t {
                offset = Some(i64::from(*off));
            }
        }
        offset.expect("the table covers the instant")
    }

    /// The instant (seconds) a wall-clock reading names: the earlier of two when the
    /// clock was set back over it, and, when the clock was set forward over it, the
    /// instant that is as far past the change as the reading is past the skipped hour,
    /// by the offset before it.
    pub fn utc_from_local(&self, local: i64) -> i64 {
        let mut chosen: Option<i64> = None;
        for off in &self.offsets {
            let candidate = local - i64::from(*off);
            if self.offset_at(candidate) != i64::from(*off) {
                continue;
            }
            match chosen {
                Some(earlier) if earlier <= candidate => {}
                _ => chosen = Some(candidate),
            }
        }
        if let Some(found) = chosen {
            return found;
        }
        // A reading the clock skipped: it lies between where the offset before a change
        // would put the change and where the offset after would.
        for k in 1..self.starts.len() {
            let before = local - i64::from(self.offsets[k - 1]);
            if before >= self.starts[k] && local - i64::from(self.offsets[k]) < self.starts[k] {
                return before;
            }
        }
        panic!("the zone names no instant for {local}");
    }
}

/// What a to-do's fields say it is scheduled as, as the fields hold it once the copies
/// agree: each field read as the value written last, and each set as the bits it holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Schedule {
    pub start: Option<i64>,
    pub zone: u64,
    pub length: i64,
    pub freq: i64,
    pub interval: i64,
    pub days: i64,
    pub count: i64,
    pub until: i64,
    pub bymonth: i64,
    pub md_pos: i64,
    pub md_neg: i64,
    pub day_a: i64,
    pub day_b: i64,
    pub sp_pos: i64,
    pub sp_neg: i64,
    pub wkst: i64,
    pub wn_pos: i64,
    pub wn_neg: i64,
    pub yd_pos: [i64; 6],
    pub yd_neg: [i64; 6],
    pub gone: bool,
    /// Two people changed one of the fields at once and no one has chosen.
    pub conflict: bool,
    /// The days the fields leave out or put back, by their count since 1970-01-01.
    pub except: BTreeMap<i64, DayMark>,
}

fn bit(mask: i64, n: i64) -> bool {
    (0..63).contains(&n) && (mask >> n) & 1 == 1
}

/// The schedule of to-do `id` in the list the copies agree on.
pub fn schedule_in(list: &BTreeMap<(u64, u64), Field>, id: u64) -> Schedule {
    let value = |field: u64, none: i64| list.get(&(id, field)).map_or(none, |held| held.value);
    let split = std::iter::once(GONE)
        .chain(ZONE..=BYYEARDAY_NEG + 5)
        .any(|field| {
            list.get(&(id, field))
                .is_some_and(|held| held.live.len() > 1)
        });
    let mut except = BTreeMap::new();
    for ((todo, field), held) in list {
        if *todo == id && *field >= EXCEPT_BASE {
            except.insert(
                (*field - EXCEPT_BASE) as i64,
                DayMark {
                    left_out: held.value == 1,
                    split: held.live.len() > 1,
                },
            );
        }
    }
    let wkst = value(WKST, 1);
    let mut yd_pos = [0; 6];
    let mut yd_neg = [0; 6];
    for j in 0..6 {
        yd_pos[j] = value(BYYEARDAY + j as u64, 0);
        yd_neg[j] = value(BYYEARDAY_NEG + j as u64, 0);
    }
    Schedule {
        start: list.get(&(id, START)).map(|held| held.value),
        zone: value(ZONE, 0).max(0) as u64,
        length: value(LENGTH, 0),
        freq: value(REPEAT, 0),
        interval: value(EVERY, 1).max(1),
        days: value(DAYS, 0),
        count: value(COUNT, 0),
        until: value(UNTIL, 0),
        bymonth: value(BYMONTH, 0),
        md_pos: value(BYMONTHDAY, 0),
        md_neg: value(BYMONTHDAY_NEG, 0),
        day_a: value(BYDAY_A, 0),
        day_b: value(BYDAY_B, 0),
        sp_pos: value(BYSETPOS, 0),
        sp_neg: value(BYSETPOS_NEG, 0),
        wkst: if (0..=6).contains(&wkst) { wkst } else { 1 },
        wn_pos: value(BYWEEKNO, 0),
        wn_neg: value(BYWEEKNO_NEG, 0),
        yd_pos,
        yd_neg,
        gone: value(GONE, 0) == 1,
        conflict: split,
        except,
    }
}

/// The day the first week of `year` begins on, counting weeks from `wkst` as ISO 8601
/// does: the first week is the first that has four days in the year.
fn first_week_of(year: i64, wkst: i64) -> i64 {
    let jan1 = day_number(year, 1, 1);
    let lead = (weekday_of(jan1) - wkst).rem_euclid(7);
    if 7 - lead >= 4 {
        jan1 - lead
    } else {
        jan1 - lead + 7
    }
}

/// The week of a day, and how many weeks its year has; nothing for a day that is in the
/// calendar year next to the year of its week.
fn week_of(day: i64, year: i64, wkst: i64) -> Option<(i64, i64)> {
    let first = first_week_of(year, wkst);
    let weeks = (first_week_of(year + 1, wkst) - first) / 7;
    if day < first {
        return None;
    }
    let week = (day - first) / 7 + 1;
    (week <= weeks).then_some((week, weeks))
}

/// The first day of period `p` and how many days it has, for the repetition `s`.
fn period(s: &Schedule, p: i64, start_day: i64, week0_start: i64, y0: i64, m0: i64) -> (i64, i64) {
    match s.freq {
        ONCE | DAILY => (start_day + p * s.interval, 1),
        WEEKLY => (week0_start + p * s.interval * 7, 7),
        MONTHLY => {
            let months = y0 * 12 + (m0 - 1) + p * s.interval;
            let (y, m) = (months / 12, months % 12 + 1);
            (day_number(y, m, 1), month_length(y, m))
        }
        _ => {
            let y = y0 + p * s.interval;
            (day_number(y, 1, 1), year_length(y))
        }
    }
}

/// Whether the repetition `s` selects day `d`, by the rules of `event_agenda.scxml`.
fn selects(s: &Schedule, d: i64, start_wd: i64, m0: i64, dom0: i64) -> bool {
    if s.freq == ONCE {
        return true;
    }
    let (year, month, dom) = civil_of(d);
    let last = month_length(year, month);
    let ylen = year_length(year);
    let yday = d - day_number(year, 1, 1) + 1;
    let wd = weekday_of(d);
    let has_month = s.bymonth != 0;
    let has_md = s.md_pos | s.md_neg != 0;
    let has_day = s.days | s.day_a | s.day_b != 0;
    let has_wn = s.wn_pos | s.wn_neg != 0;
    let has_yd = s.yd_pos.iter().chain(&s.yd_neg).any(|mask| *mask != 0);
    let md_match = bit(s.md_pos, dom) || bit(s.md_neg, last - dom + 1);
    let plain = bit(s.days, wd);
    // An ordinal counts in the month, or in the year under YEARLY with no BYMONTH.
    let in_month = s.freq == MONTHLY || has_month;
    let from_start = if in_month {
        (dom - 1) / 7 + 1
    } else {
        (yday - 1) / 7 + 1
    };
    let from_end = if in_month {
        (last - dom) / 7 + 1
    } else {
        (ylen - yday) / 7 + 1
    };
    let ordinals = if wd < 4 { s.day_a } else { s.day_b };
    let base = (wd % 4) * 10;
    let ordinal = (from_start <= 5 && bit(ordinals, base + from_start - 1))
        || (from_end <= 5 && bit(ordinals, base + 4 + from_end));
    let day_match = plain || ordinal;
    let clamped = dom == dom0.min(last);

    let mut ok = !has_month || bit(s.bymonth, month);
    match s.freq {
        DAILY => {
            if has_md {
                ok &= md_match;
            }
            if has_day {
                ok &= plain;
            }
        }
        WEEKLY => ok &= if has_day { plain } else { wd == start_wd },
        MONTHLY => {
            if has_md {
                ok &= md_match;
            }
            if has_day {
                ok &= day_match;
            }
            if !has_md && !has_day {
                ok &= clamped;
            }
        }
        _ => {
            if has_wn {
                ok &= week_of(d, year, s.wkst).is_some_and(|(week, weeks)| {
                    bit(s.wn_pos, week) || bit(s.wn_neg, weeks - week + 1)
                });
            }
            if has_yd {
                let (up, down) = (yday - 1, ylen - yday);
                ok &= bit(s.yd_pos[(up / 61) as usize], up % 61)
                    || bit(s.yd_neg[(down / 61) as usize], down % 61);
            }
            if has_md {
                ok &= md_match;
            }
            if has_day {
                ok &= day_match;
            }
            if has_wn && !has_day && !has_yd && !has_md {
                ok &= wd == start_wd;
            }
            if !has_wn && !has_yd && !has_md && !has_day {
                ok &= (has_month || month == m0) && clamped;
            }
        }
    }
    ok
}

/// The occurrences of a schedule whose wall-clock readings overlap `[from, to)`, in order,
/// the first `skip` of them (counted from the first there is) passed over, at most 64.
pub fn expand_readings(id: u64, s: &Schedule, from: i64, to: i64, skip: u32) -> Vec<Occurrence> {
    let Some(start) = s.start else {
        return Vec::new();
    };
    if start < 0 || s.gone || !(ONCE..=YEARLY).contains(&s.freq) || to <= from || to <= 0 {
        return Vec::new();
    }
    let time_of_day = start % DAY_MS;
    let start_day = start / DAY_MS;
    let (y0, m0, dom0) = civil_of(start_day);
    let start_wd = weekday_of(start_day);
    let end_day = (to + DAY_MS - 1) / DAY_MS;
    let week0_start = start_day - (start_wd - s.wkst).rem_euclid(7);
    let mut out = Vec::new();
    let mut nth: i64 = 0;
    let mut p = 0;
    'periods: loop {
        let (first, length) = period(s, p, start_day, week0_start, y0, m0);
        if first >= end_day || (s.until > 0 && first * DAY_MS > s.until) {
            break;
        }
        let chosen: Vec<i64> = (first..first + length)
            .filter(|d| selects(s, *d, start_wd, m0, dom0))
            .collect();
        let n = chosen.len() as i64;
        for (i, &day) in chosen.iter().enumerate() {
            if s.sp_pos | s.sp_neg != 0 {
                let kept = bit(s.sp_pos, i as i64 + 1) || bit(s.sp_neg, n - i as i64);
                if !kept {
                    continue;
                }
            }
            if day < start_day {
                continue;
            }
            let begins = day * DAY_MS + time_of_day;
            if begins >= to || (s.until > 0 && begins > s.until) {
                break 'periods;
            }
            if s.count > 0 && nth >= s.count {
                break 'periods;
            }
            let ends = begins + s.length;
            if (ends > from || begins >= from) && out.len() < AGENDA_MAX && nth >= i64::from(skip) {
                // A day left out still counts toward the count, and is not shown.
                let mark = s.except.get(&day).copied();
                if !mark.is_some_and(|m| m.left_out) {
                    out.push(Occurrence {
                        id,
                        start: begins,
                        end: ends,
                        nth: nth as u32,
                        conflict: u8::from(s.conflict || mark.is_some_and(|m| m.split)),
                    });
                }
            }
            nth += 1;
            if out.len() >= AGENDA_MAX {
                break 'periods;
            }
        }
        if s.freq == ONCE {
            break;
        }
        p += 1;
    }
    out
}

/// The page of an agenda a schedule gives for `[from, to)`: with a zone the readings are
/// those of the zone's wall clock over a window a day wider each side, each is turned
/// into an instant by the zone, and what overlaps `[from, to)` is kept; with none they
/// are the instants.
pub fn expand(id: u64, s: &Schedule, zones: &Zones, from: i64, to: i64, skip: u32) -> Page {
    let zone = zones.get(&s.zone);
    let raw = match zone {
        None => expand_readings(id, s, from, to, skip),
        Some(_) => expand_readings(id, s, from - DAY_MS, to + DAY_MS, skip),
    };
    let next = if raw.len() == AGENDA_MAX {
        raw.last().map_or(0, |o| o.nth + 1)
    } else {
        0
    };
    let Some(zone) = zone else {
        return Page { items: raw, next };
    };
    let items = raw
        .into_iter()
        .map(|reading| {
            let start = zone.utc_from_local(reading.start.div_euclid(1000)) * 1000
                + reading.start.rem_euclid(1000);
            Occurrence {
                start,
                end: start + (reading.end - reading.start),
                ..reading
            }
        })
        .filter(|o| (o.end > from || o.start >= from) && o.start < to)
        .collect();
    Page { items, next }
}

/// What a person writes into the fields of a schedule: what a repetition rule says, as
/// the sets its fields hold.
#[derive(Debug, Clone, Copy)]
pub struct Plan {
    pub start: i64,
    pub length: i64,
    pub repeat: i64,
    pub every: i64,
    pub days: i64,
    pub count: i64,
    pub until: i64,
    pub zone: i64,
    pub wkst: i64,
    pub bymonth: i64,
    pub bymonthday: i64,
    pub bymonthday_neg: i64,
    pub byday_a: i64,
    pub byday_b: i64,
    pub bysetpos: i64,
    pub bysetpos_neg: i64,
    pub byweekno: i64,
    pub byweekno_neg: i64,
    pub byyearday: [i64; 6],
    pub byyearday_neg: [i64; 6],
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
            until: 0,
            zone: 0,
            wkst: -1,
            bymonth: 0,
            bymonthday: 0,
            bymonthday_neg: 0,
            byday_a: 0,
            byday_b: 0,
            bysetpos: 0,
            bysetpos_neg: 0,
            byweekno: 0,
            byweekno_neg: 0,
            byyearday: [0; 6],
            byyearday_neg: [0; 6],
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

    /// The same, in the host's zone `zone` (1 to 4).
    pub fn in_zone(self, zone: i64) -> Self {
        Plan { zone, ..self }
    }

    /// BYDAY without an ordinal: these weekdays.
    pub fn on_weekdays(self, weekdays: &[i64]) -> Self {
        Plan {
            days: weekdays.iter().fold(0, |mask, wd| mask | 1 << wd),
            ..self
        }
    }

    /// BYDAY with ordinals: `(2, TUE)` the second Tuesday, `(-1, FRI)` the last Friday.
    pub fn on_ordinals(self, ordinals: &[(i64, i64)]) -> Self {
        let (mut a, mut b) = (0, 0);
        for (ordinal, weekday) in ordinals {
            assert!(
                (1..=5).contains(&ordinal.abs()),
                "no bit for ordinal {ordinal}"
            );
            let at = (weekday % 4) * 10
                + if *ordinal > 0 {
                    ordinal - 1
                } else {
                    4 + ordinal.abs()
                };
            if *weekday < 4 {
                a |= 1 << at;
            } else {
                b |= 1 << at;
            }
        }
        Plan {
            byday_a: a,
            byday_b: b,
            ..self
        }
    }

    /// BYMONTH: these months, 1 to 12.
    pub fn in_months(self, months: &[i64]) -> Self {
        Plan {
            bymonth: months.iter().fold(0, |mask, m| mask | 1 << m),
            ..self
        }
    }

    /// BYMONTHDAY: these days of the month, a negative one counted from the end.
    pub fn on_month_days(self, days: &[i64]) -> Self {
        let (mut pos, mut neg) = (0, 0);
        for day in days {
            if *day > 0 {
                pos |= 1 << day;
            } else {
                neg |= 1 << -day;
            }
        }
        Plan {
            bymonthday: pos,
            bymonthday_neg: neg,
            ..self
        }
    }

    /// BYSETPOS: these places among the days a period selects, a negative one from the end.
    pub fn at_set_positions(self, places: &[i64]) -> Self {
        let (mut pos, mut neg) = (0, 0);
        for place in places {
            if *place > 0 {
                pos |= 1 << place;
            } else {
                neg |= 1 << -place;
            }
        }
        Plan {
            bysetpos: pos,
            bysetpos_neg: neg,
            ..self
        }
    }

    /// BYWEEKNO: these weeks of the year, a negative one from the end.
    pub fn in_weeks(self, weeks: &[i64]) -> Self {
        let (mut pos, mut neg) = (0, 0);
        for week in weeks {
            if *week > 0 {
                pos |= 1 << week;
            } else {
                neg |= 1 << -week;
            }
        }
        Plan {
            byweekno: pos,
            byweekno_neg: neg,
            ..self
        }
    }

    /// BYYEARDAY: these days of the year, a negative one from the end.
    pub fn on_year_days(self, days: &[i64]) -> Self {
        let (mut pos, mut neg) = ([0; 6], [0; 6]);
        for day in days {
            let (list, n) = if *day > 0 {
                (&mut pos, day - 1)
            } else {
                (&mut neg, -day - 1)
            };
            list[(n / 61) as usize] |= 1 << (n % 61);
        }
        Plan {
            byyearday: pos,
            byyearday_neg: neg,
            ..self
        }
    }

    /// WKST: the weekday a week begins on.
    pub fn week_begins_on(self, weekday: i64) -> Self {
        Plan {
            wkst: weekday,
            ..self
        }
    }
}

/// A person gives a to-do a schedule: every field of it, written in one go (a field that
/// holds nothing is not written).
pub fn schedule_todo(world: &mut World, who: &str, id: u64, plan: &Plan) {
    let mut fields = vec![
        (START, plan.start),
        (LENGTH, plan.length),
        (REPEAT, plan.repeat),
        (EVERY, plan.every),
        (DAYS, plan.days),
        (COUNT, plan.count),
    ];
    for (field, value) in [
        (ZONE, plan.zone),
        (UNTIL, plan.until),
        (BYMONTH, plan.bymonth),
        (BYMONTHDAY, plan.bymonthday),
        (BYMONTHDAY_NEG, plan.bymonthday_neg),
        (BYDAY_A, plan.byday_a),
        (BYDAY_B, plan.byday_b),
        (BYSETPOS, plan.bysetpos),
        (BYSETPOS_NEG, plan.bysetpos_neg),
        (BYWEEKNO, plan.byweekno),
        (BYWEEKNO_NEG, plan.byweekno_neg),
    ] {
        if value != 0 {
            fields.push((field, value));
        }
    }
    if plan.wkst >= 0 {
        fields.push((WKST, plan.wkst));
    }
    for j in 0..6 {
        if plan.byyearday[j] != 0 {
            fields.push((BYYEARDAY + j as u64, plan.byyearday[j]));
        }
        if plan.byyearday_neg[j] != 0 {
            fields.push((BYYEARDAY_NEG + j as u64, plan.byyearday_neg[j]));
        }
    }
    for (field, value) in fields {
        todo_set(world, who, id, field, value);
    }
}

/// Ask a copy for the page of the agenda of to-do `id` in `[from, to)` that begins after
/// the first `skip` occurrences and read what it shows.
pub fn agenda_page(world: &mut World, who: &str, id: u64, from: i64, to: i64, skip: u32) -> Page {
    world.raise(
        who,
        "todo.agenda",
        &format!(r#"{{"id":{id},"windowStart":"{from}","windowEnd":"{to}","skip":{skip}}}"#),
    );
    world.run_until_quiet(10);
    let variables = todo_variables_of(world, who);
    let items = variables["agenda"]
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
        .collect();
    Page {
        items,
        next: whole(&variables["agendaNext"]) as u32,
    }
}

/// The first page of the agenda of to-do `id` in `[from, to)`.
pub fn agenda_of(world: &mut World, who: &str, id: u64, from: i64, to: i64) -> Vec<Occurrence> {
    agenda_page(world, who, id, from, to, 0).items
}

/// The three copies show one agenda for the to-do; which. And it is the one the schedule
/// the copies agree on says it is, with no time zone.
pub fn agreed_agenda(world: &mut World, id: u64, from: i64, to: i64) -> Vec<Occurrence> {
    agreed_page(world, &Zones::new(), id, from, to, 0).items
}

/// The same under the zones every copy was given (`set_zone_everywhere`).
pub fn agreed_agenda_in(
    world: &mut World,
    zones: &Zones,
    id: u64,
    from: i64,
    to: i64,
) -> Vec<Occurrence> {
    agreed_page(world, zones, id, from, to, 0).items
}

/// A page of the agenda that the three copies show alike and that is the page the
/// schedule says, with the place the next begins.
pub fn agreed_page(
    world: &mut World,
    zones: &Zones,
    id: u64,
    from: i64,
    to: i64,
    skip: u32,
) -> Page {
    let server = agenda_page(world, "server", id, from, to, skip);
    for who in ["alice", "bob"] {
        assert_eq!(
            agenda_page(world, who, id, from, to, skip),
            server,
            "{who} and the hub show different agendas for to-do {id}"
        );
    }
    let list = agreed_todos(world);
    assert_eq!(
        server,
        expand(id, &schedule_in(&list, id), zones, from, to, skip),
        "the agenda of to-do {id} is not what its schedule says"
    );
    server
}

/// Tell a copy's host its time zone `number` (1 to 4): its transitions, one to a message.
/// The zones a copy holds are cleared first by `clear_zones`.
pub fn set_zone(world: &mut World, who: &str, number: u64, zone: &Zone) {
    for (start, offset) in zone.starts.iter().zip(&zone.offsets) {
        world.raise(
            who,
            "todo.zone",
            &format!(r#"{{"zone":{number},"start":"{start}","offset":{offset}}}"#),
        );
    }
}

/// Every copy's host has the same time zone under `number`: the one a group of people
/// share.
pub fn set_zone_everywhere(world: &mut World, number: u64, zone: &Zone) {
    for who in ["alice", "bob", "server"] {
        set_zone(world, who, number, zone);
    }
    world.run_until_quiet(10);
}

/// A person leaves out (`true`) or puts back (`false`) the occurrence of a day, by the
/// day's count since 1970-01-01.
pub fn leave_out(world: &mut World, who: &str, id: u64, day: i64, out: bool) {
    todo_set(world, who, id, except_field(day), i64::from(out));
}

/// New York in 2026: UTC-5 until the clock is set forward at 02:00 on 8 March (07:00Z)
/// and back at 02:00 on 1 November (06:00Z).
pub fn new_york() -> Zone {
    Zone {
        starts: vec![
            instant(2025, 11, 2, 6, 0) / 1000,
            instant(2026, 3, 8, 7, 0) / 1000,
            instant(2026, 11, 1, 6, 0) / 1000,
        ],
        offsets: vec![-18_000, -14_400, -18_000],
    }
}

/// Seoul: UTC+9, no change of the clock.
pub fn seoul() -> Zone {
    Zone {
        starts: vec![instant(2020, 1, 1, 0, 0) / 1000],
        offsets: vec![32_400],
    }
}
