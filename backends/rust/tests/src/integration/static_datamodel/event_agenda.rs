// SCE-GENERATED — DO NOT EDIT
// source-hash: b9ab4ddbd49e8ce2a3b4c80917bc3eecdf0d5285c15974d4e0a00636f93fd573
#![doc = "SCE-MAP: event_agenda.scxml:54 :: _forge_body"]
// SCE-MAP: event_agenda.scxml:54 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function, no instance state. `#![no_std]`-clean when no `bytes`
// parameter (this fixture: no_std_clean = true).

use super::event_later;
use super::event_occurrence;
use super::mv_value;

use sce_portable_bytes::SceOwnedList;

#[allow(clippy::all)]
#[allow(unused_assignments)]
pub fn event_agenda(
    values: &[mv_value::MvValuePayload],
    id: u64,
    window_start: i64,
    window_end: i64,
) -> Result<
    SceOwnedList<event_occurrence::EventOccurrencePayload, 64>,
    sce_forge_runtime::algorithm::AlgorithmError,
> {
    let mut out: SceOwnedList<event_occurrence::EventOccurrencePayload, 64> = SceOwnedList::new();
    let mut emitted: i64 = 0;
    let day: i64 = 86400000;
    let mut start_at: i64 = -1;
    let mut gone: i64 = 0;
    let mut freq: i64 = 0;
    let mut length: i64 = 0;
    let mut every0: i64 = 1;
    let mut days: i64 = 0;
    let mut count: i64 = 0;
    let mut until: i64 = 0;
    let mut flag: u8 = 0;
    let mut found: bool = false;
    let mut held: i64 = 0;
    let mut bw: i64 = 0;
    let mut bc: u32 = 0;
    let mut bn: u64 = 0;
    found = false;
    held = 0;
    for w16 in values.iter() {
        if w16.id == id && w16.field == 16 {
            held = sce_forge_runtime::algorithm::add::<i64>(held, 1)?;
            if !found
                || event_later::event_later(w16.wall_time, w16.counter, w16.node_id, bw, bc, bn)
            {
                found = true;
                bw = w16.wall_time;
                bc = w16.counter;
                bn = w16.node_id;
                start_at = w16.value;
            }
        }
    }
    if held > 1 {
        flag = 1;
    }
    found = false;
    held = 0;
    for w9 in values.iter() {
        if w9.id == id && w9.field == 9 {
            held = sce_forge_runtime::algorithm::add::<i64>(held, 1)?;
            if !found || event_later::event_later(w9.wall_time, w9.counter, w9.node_id, bw, bc, bn)
            {
                found = true;
                bw = w9.wall_time;
                bc = w9.counter;
                bn = w9.node_id;
                gone = w9.value;
            }
        }
    }
    if held > 1 {
        flag = 1;
    }
    found = false;
    held = 0;
    for w18 in values.iter() {
        if w18.id == id && w18.field == 18 {
            held = sce_forge_runtime::algorithm::add::<i64>(held, 1)?;
            if !found
                || event_later::event_later(w18.wall_time, w18.counter, w18.node_id, bw, bc, bn)
            {
                found = true;
                bw = w18.wall_time;
                bc = w18.counter;
                bn = w18.node_id;
                freq = w18.value;
            }
        }
    }
    if held > 1 {
        flag = 1;
    }
    found = false;
    held = 0;
    for w17 in values.iter() {
        if w17.id == id && w17.field == 17 {
            held = sce_forge_runtime::algorithm::add::<i64>(held, 1)?;
            if !found
                || event_later::event_later(w17.wall_time, w17.counter, w17.node_id, bw, bc, bn)
            {
                found = true;
                bw = w17.wall_time;
                bc = w17.counter;
                bn = w17.node_id;
                length = w17.value;
            }
        }
    }
    if held > 1 {
        flag = 1;
    }
    found = false;
    held = 0;
    for w19 in values.iter() {
        if w19.id == id && w19.field == 19 {
            held = sce_forge_runtime::algorithm::add::<i64>(held, 1)?;
            if !found
                || event_later::event_later(w19.wall_time, w19.counter, w19.node_id, bw, bc, bn)
            {
                found = true;
                bw = w19.wall_time;
                bc = w19.counter;
                bn = w19.node_id;
                every0 = w19.value;
            }
        }
    }
    if held > 1 {
        flag = 1;
    }
    found = false;
    held = 0;
    for w20 in values.iter() {
        if w20.id == id && w20.field == 20 {
            held = sce_forge_runtime::algorithm::add::<i64>(held, 1)?;
            if !found
                || event_later::event_later(w20.wall_time, w20.counter, w20.node_id, bw, bc, bn)
            {
                found = true;
                bw = w20.wall_time;
                bc = w20.counter;
                bn = w20.node_id;
                days = w20.value;
            }
        }
    }
    if held > 1 {
        flag = 1;
    }
    found = false;
    held = 0;
    for w21 in values.iter() {
        if w21.id == id && w21.field == 21 {
            held = sce_forge_runtime::algorithm::add::<i64>(held, 1)?;
            if !found
                || event_later::event_later(w21.wall_time, w21.counter, w21.node_id, bw, bc, bn)
            {
                found = true;
                bw = w21.wall_time;
                bc = w21.counter;
                bn = w21.node_id;
                count = w21.value;
            }
        }
    }
    if held > 1 {
        flag = 1;
    }
    found = false;
    held = 0;
    for w22 in values.iter() {
        if w22.id == id && w22.field == 22 {
            held = sce_forge_runtime::algorithm::add::<i64>(held, 1)?;
            if !found
                || event_later::event_later(w22.wall_time, w22.counter, w22.node_id, bw, bc, bn)
            {
                found = true;
                bw = w22.wall_time;
                bc = w22.counter;
                bn = w22.node_id;
                until = w22.value;
            }
        }
    }
    if held > 1 {
        flag = 1;
    }
    if start_at >= 0
        && gone != 1
        && freq >= 0
        && freq <= 3
        && window_end > window_start
        && window_end > 0
    {
        let every: i64 = if every0 < 1 { 1 } else { every0 };
        let time_of_day: i64 = sce_forge_runtime::algorithm::rem::<i64>(start_at, day)?;
        let start_day: i64 = sce_forge_runtime::algorithm::div::<i64>(
            sce_forge_runtime::algorithm::sub::<i64>(start_at, time_of_day)?,
            day,
        )?;
        let end_day: i64 = sce_forge_runtime::algorithm::div::<i64>(
            sce_forge_runtime::algorithm::sub::<i64>(
                sce_forge_runtime::algorithm::add::<i64>(window_end, day)?,
                1,
            )?,
            day,
        )?;
        let start_weekday: i64 = sce_forge_runtime::algorithm::rem::<i64>(
            sce_forge_runtime::algorithm::add::<i64>(
                sce_forge_runtime::algorithm::rem::<i64>(start_day, 7)?,
                11,
            )?,
            7,
        )?;
        let week_start: i64 = sce_forge_runtime::algorithm::sub::<i64>(start_day, start_weekday)?;
        let one: i64 = 1;
        let weekdays: i64 = if days == 0 {
            one << start_weekday
        } else {
            days & 127
        };
        let z: i64 = sce_forge_runtime::algorithm::add::<i64>(start_day, 719468)?;
        let era: i64 = sce_forge_runtime::algorithm::div::<i64>(z, 146097)?;
        let doe: i64 = sce_forge_runtime::algorithm::sub::<i64>(
            z,
            sce_forge_runtime::algorithm::mul::<i64>(era, 146097)?,
        )?;
        let yoe: i64 = sce_forge_runtime::algorithm::div::<i64>(
            sce_forge_runtime::algorithm::sub::<i64>(
                sce_forge_runtime::algorithm::add::<i64>(
                    sce_forge_runtime::algorithm::sub::<i64>(
                        doe,
                        sce_forge_runtime::algorithm::div::<i64>(doe, 1460)?,
                    )?,
                    sce_forge_runtime::algorithm::div::<i64>(doe, 36524)?,
                )?,
                sce_forge_runtime::algorithm::div::<i64>(doe, 146096)?,
            )?,
            365,
        )?;
        let doy: i64 = sce_forge_runtime::algorithm::sub::<i64>(
            doe,
            sce_forge_runtime::algorithm::sub::<i64>(
                sce_forge_runtime::algorithm::add::<i64>(
                    sce_forge_runtime::algorithm::mul::<i64>(365, yoe)?,
                    sce_forge_runtime::algorithm::div::<i64>(yoe, 4)?,
                )?,
                sce_forge_runtime::algorithm::div::<i64>(yoe, 100)?,
            )?,
        )?;
        let mp: i64 = sce_forge_runtime::algorithm::div::<i64>(
            sce_forge_runtime::algorithm::add::<i64>(
                sce_forge_runtime::algorithm::mul::<i64>(5, doy)?,
                2,
            )?,
            153,
        )?;
        let start_of_month: i64 = sce_forge_runtime::algorithm::add::<i64>(
            sce_forge_runtime::algorithm::sub::<i64>(
                doy,
                sce_forge_runtime::algorithm::div::<i64>(
                    sce_forge_runtime::algorithm::add::<i64>(
                        sce_forge_runtime::algorithm::mul::<i64>(153, mp)?,
                        2,
                    )?,
                    5,
                )?,
            )?,
            1,
        )?;
        let start_month: i64 = if mp < 10 {
            sce_forge_runtime::algorithm::add::<i64>(mp, 3)?
        } else {
            sce_forge_runtime::algorithm::sub::<i64>(mp, 9)?
        };
        let start_year: i64 = if start_month <= 2 {
            sce_forge_runtime::algorithm::add::<i64>(
                sce_forge_runtime::algorithm::add::<i64>(
                    yoe,
                    sce_forge_runtime::algorithm::mul::<i64>(era, 400)?,
                )?,
                1,
            )?
        } else {
            sce_forge_runtime::algorithm::add::<i64>(
                yoe,
                sce_forge_runtime::algorithm::mul::<i64>(era, 400)?,
            )?
        };
        let months: i64 = sce_forge_runtime::algorithm::sub::<i64>(
            sce_forge_runtime::algorithm::add::<i64>(
                sce_forge_runtime::algorithm::mul::<i64>(start_year, 12)?,
                start_month,
            )?,
            1,
        )?;
        let mut cursor: i64 = 0;
        let mut nth: i64 = 0;
        let mut going: bool = true;
        while going {
            let mut d: i64 = sce_forge_runtime::algorithm::add::<i64>(start_day, cursor)?;
            let mut valid: bool = false;
            if freq == 0 {
                valid = cursor == 0;
            }
            if freq == 1 {
                valid = sce_forge_runtime::algorithm::rem::<i64>(cursor, every)? == 0;
            }
            if freq == 2 {
                valid = weekdays
                    >> sce_forge_runtime::algorithm::rem::<i64>(
                        sce_forge_runtime::algorithm::add::<i64>(
                            sce_forge_runtime::algorithm::rem::<i64>(d, 7)?,
                            11,
                        )?,
                        7,
                    )?
                    & 1
                    == 1
                    && sce_forge_runtime::algorithm::rem::<i64>(
                        sce_forge_runtime::algorithm::div::<i64>(
                            sce_forge_runtime::algorithm::sub::<i64>(d, week_start)?,
                            7,
                        )?,
                        every,
                    )? == 0;
            }
            if freq == 3 {
                let total: i64 = sce_forge_runtime::algorithm::add::<i64>(
                    months,
                    sce_forge_runtime::algorithm::mul::<i64>(cursor, every)?,
                )?;
                let land_year: i64 = sce_forge_runtime::algorithm::div::<i64>(total, 12)?;
                let land_month: i64 = sce_forge_runtime::algorithm::add::<i64>(
                    sce_forge_runtime::algorithm::rem::<i64>(total, 12)?,
                    1,
                )?;
                let land_leap: bool = sce_forge_runtime::algorithm::rem::<i64>(land_year, 4)? == 0
                    && sce_forge_runtime::algorithm::rem::<i64>(land_year, 100)? != 0
                    || sce_forge_runtime::algorithm::rem::<i64>(land_year, 400)? == 0;
                let last: i64 = if land_month == 2 {
                    if land_leap {
                        29
                    } else {
                        28
                    }
                } else {
                    if land_month == 4 || land_month == 6 || land_month == 9 || land_month == 11 {
                        30
                    } else {
                        31
                    }
                };
                let land_day: i64 = if start_of_month < last {
                    start_of_month
                } else {
                    last
                };
                let cy: i64 = if land_month <= 2 {
                    sce_forge_runtime::algorithm::sub::<i64>(land_year, 1)?
                } else {
                    land_year
                };
                let cera: i64 = sce_forge_runtime::algorithm::div::<i64>(cy, 400)?;
                let cyoe: i64 = sce_forge_runtime::algorithm::sub::<i64>(
                    cy,
                    sce_forge_runtime::algorithm::mul::<i64>(cera, 400)?,
                )?;
                let cmp: i64 = if land_month > 2 {
                    sce_forge_runtime::algorithm::sub::<i64>(land_month, 3)?
                } else {
                    sce_forge_runtime::algorithm::add::<i64>(land_month, 9)?
                };
                let cdoy: i64 = sce_forge_runtime::algorithm::sub::<i64>(
                    sce_forge_runtime::algorithm::add::<i64>(
                        sce_forge_runtime::algorithm::div::<i64>(
                            sce_forge_runtime::algorithm::add::<i64>(
                                sce_forge_runtime::algorithm::mul::<i64>(153, cmp)?,
                                2,
                            )?,
                            5,
                        )?,
                        land_day,
                    )?,
                    1,
                )?;
                let cdoe: i64 = sce_forge_runtime::algorithm::add::<i64>(
                    sce_forge_runtime::algorithm::sub::<i64>(
                        sce_forge_runtime::algorithm::add::<i64>(
                            sce_forge_runtime::algorithm::mul::<i64>(cyoe, 365)?,
                            sce_forge_runtime::algorithm::div::<i64>(cyoe, 4)?,
                        )?,
                        sce_forge_runtime::algorithm::div::<i64>(cyoe, 100)?,
                    )?,
                    cdoy,
                )?;
                d = sce_forge_runtime::algorithm::sub::<i64>(
                    sce_forge_runtime::algorithm::add::<i64>(
                        sce_forge_runtime::algorithm::mul::<i64>(cera, 146097)?,
                        cdoe,
                    )?,
                    719468,
                )?;
                valid = true;
            }
            let occ_start: i64 = sce_forge_runtime::algorithm::add::<i64>(
                sce_forge_runtime::algorithm::mul::<i64>(d, day)?,
                time_of_day,
            )?;
            let occ_end: i64 = sce_forge_runtime::algorithm::add::<i64>(occ_start, length)?;
            if d >= end_day || occ_start >= window_end || until > 0 && occ_start > until {
                going = false;
            } else {
                if valid {
                    if count > 0 && nth >= count {
                        going = false;
                    } else {
                        if (occ_end > window_start || occ_start >= window_start) && emitted < 64 {
                            let mut excluded: bool = false;
                            let mut mark: u8 = flag;
                            let day_no: i64 =
                                sce_forge_runtime::algorithm::div::<i64>(occ_start, day)?;
                            if day_no >= 18262
                                && sce_forge_runtime::algorithm::sub::<i64>(day_no, 18262)? < 65472
                            {
                                let xfield: u64 = sce_forge_runtime::algorithm::narrow::<u64, i64>(
                                    sce_forge_runtime::algorithm::add::<i64>(
                                        sce_forge_runtime::algorithm::sub::<i64>(day_no, 18262)?,
                                        64,
                                    )?,
                                )?;
                                let mut xfound: bool = false;
                                let mut xheld: i64 = 0;
                                let mut xw: i64 = 0;
                                let mut xc: u32 = 0;
                                let mut xn: u64 = 0;
                                let mut xv: i64 = 0;
                                for wx in values.iter() {
                                    if wx.id == id && wx.field == xfield {
                                        xheld = sce_forge_runtime::algorithm::add::<i64>(xheld, 1)?;
                                        if !xfound
                                            || event_later::event_later(
                                                wx.wall_time,
                                                wx.counter,
                                                wx.node_id,
                                                xw,
                                                xc,
                                                xn,
                                            )
                                        {
                                            xfound = true;
                                            xw = wx.wall_time;
                                            xc = wx.counter;
                                            xn = wx.node_id;
                                            xv = wx.value;
                                        }
                                    }
                                }
                                excluded = xfound && xv == 1;
                                if xheld > 1 {
                                    mark = 1;
                                }
                            }
                            if !excluded {
                                let line = event_occurrence::EventOccurrencePayload {
                                    id: id,
                                    start: occ_start,
                                    end: occ_end,
                                    nth: sce_forge_runtime::algorithm::narrow::<u32, i64>(nth)?,
                                    conflict: mark,
                                };
                                out.push(line)?;
                                emitted = sce_forge_runtime::algorithm::add::<i64>(emitted, 1)?;
                            }
                        }
                        nth = sce_forge_runtime::algorithm::add::<i64>(nth, 1)?;
                        if freq == 0 {
                            going = false;
                        }
                    }
                }
            }
            if emitted >= 64 {
                going = false;
            }
            cursor = sce_forge_runtime::algorithm::add::<i64>(cursor, 1)?;
        }
    }
    return Ok(out);
}
