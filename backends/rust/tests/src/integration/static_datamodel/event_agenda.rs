// SCE-GENERATED — DO NOT EDIT
// source-hash: 3370c0559f35f8781a10b50965704050ad622ba4d76c05954cfcb6b09c63c208
#![doc = "SCE-MAP: event_agenda.scxml:71 :: _forge_body"]
// SCE-MAP: event_agenda.scxml:71 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function, no instance state. `#![no_std]`-clean when no `bytes`
// parameter (this fixture: no_std_clean = true).

use super::event_later;
use super::event_occurrence;
use super::event_schedule;
use super::event_selects;
use super::mv_value;

use sce_portable_bytes::SceOwnedList;

#[allow(clippy::all)]
#[allow(unused_assignments)]
pub fn event_agenda(
    s: event_schedule::EventSchedulePayload,
    values: &[mv_value::MvValuePayload],
    id: u64,
    window_start: i64,
    window_end: i64,
    skip: u32,
) -> Result<
    SceOwnedList<event_occurrence::EventOccurrencePayload, 64>,
    sce_forge_runtime::algorithm::AlgorithmError,
> {
    let mut out: SceOwnedList<event_occurrence::EventOccurrencePayload, 64> = SceOwnedList::new();
    let mut emitted: i64 = 0;
    let day: i64 = 86400000;
    let skipped: i64 = skip as i64;
    let start_at: i64 = s.start;
    let freq: i64 = s.freq;
    if start_at >= 0
        && s.gone != 1
        && freq >= 0
        && freq <= 4
        && window_end > window_start
        && window_end > 0
    {
        let every: i64 = if s.interval < 1 { 1 } else { s.interval };
        let length: i64 = s.length;
        let count: i64 = s.count;
        let until: i64 = s.until;
        let has_sp: bool = s.sp_pos | s.sp_neg != 0;
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
        let wkst0: i64 = s.wkst;
        let wkst: i64 = if wkst0 >= 0 && wkst0 <= 6 { wkst0 } else { 1 };
        let week0_start: i64 = sce_forge_runtime::algorithm::sub::<i64>(
            start_day,
            sce_forge_runtime::algorithm::rem::<i64>(
                sce_forge_runtime::algorithm::add::<i64>(
                    sce_forge_runtime::algorithm::sub::<i64>(start_weekday, wkst)?,
                    7,
                )?,
                7,
            )?,
        )?;
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
        let m0: i64 = if mp < 10 {
            sce_forge_runtime::algorithm::add::<i64>(mp, 3)?
        } else {
            sce_forge_runtime::algorithm::sub::<i64>(mp, 9)?
        };
        let y0: i64 = if m0 <= 2 {
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
        let months0: i64 = sce_forge_runtime::algorithm::sub::<i64>(
            sce_forge_runtime::algorithm::add::<i64>(
                sce_forge_runtime::algorithm::mul::<i64>(y0, 12)?,
                m0,
            )?,
            1,
        )?;
        let mut p: i64 = 0;
        let mut nth: i64 = 0;
        let mut going: bool = true;
        while going {
            let mut ps: i64 = start_day;
            let mut plen: i64 = 1;
            if freq == 1 {
                ps = sce_forge_runtime::algorithm::add::<i64>(
                    start_day,
                    sce_forge_runtime::algorithm::mul::<i64>(p, every)?,
                )?;
            }
            if freq == 2 {
                ps = sce_forge_runtime::algorithm::add::<i64>(
                    week0_start,
                    sce_forge_runtime::algorithm::mul::<i64>(
                        sce_forge_runtime::algorithm::mul::<i64>(p, every)?,
                        7,
                    )?,
                )?;
                plen = 7;
            }
            if freq == 3 || freq == 4 {
                let pty: i64 = if freq == 3 {
                    sce_forge_runtime::algorithm::div::<i64>(
                        sce_forge_runtime::algorithm::add::<i64>(
                            months0,
                            sce_forge_runtime::algorithm::mul::<i64>(p, every)?,
                        )?,
                        12,
                    )?
                } else {
                    sce_forge_runtime::algorithm::add::<i64>(
                        y0,
                        sce_forge_runtime::algorithm::mul::<i64>(p, every)?,
                    )?
                };
                let ptm: i64 = if freq == 3 {
                    sce_forge_runtime::algorithm::add::<i64>(
                        sce_forge_runtime::algorithm::rem::<i64>(
                            sce_forge_runtime::algorithm::add::<i64>(
                                months0,
                                sce_forge_runtime::algorithm::mul::<i64>(p, every)?,
                            )?,
                            12,
                        )?,
                        1,
                    )?
                } else {
                    1
                };
                let pleap: bool = sce_forge_runtime::algorithm::rem::<i64>(pty, 4)? == 0
                    && sce_forge_runtime::algorithm::rem::<i64>(pty, 100)? != 0
                    || sce_forge_runtime::algorithm::rem::<i64>(pty, 400)? == 0;
                let pcy: i64 = if ptm <= 2 {
                    sce_forge_runtime::algorithm::sub::<i64>(pty, 1)?
                } else {
                    pty
                };
                let pcera: i64 = sce_forge_runtime::algorithm::div::<i64>(pcy, 400)?;
                let pcyoe: i64 = sce_forge_runtime::algorithm::sub::<i64>(
                    pcy,
                    sce_forge_runtime::algorithm::mul::<i64>(pcera, 400)?,
                )?;
                let pcmp: i64 = if ptm > 2 {
                    sce_forge_runtime::algorithm::sub::<i64>(ptm, 3)?
                } else {
                    sce_forge_runtime::algorithm::add::<i64>(ptm, 9)?
                };
                let pcdoy: i64 = sce_forge_runtime::algorithm::div::<i64>(
                    sce_forge_runtime::algorithm::add::<i64>(
                        sce_forge_runtime::algorithm::mul::<i64>(153, pcmp)?,
                        2,
                    )?,
                    5,
                )?;
                ps = sce_forge_runtime::algorithm::sub::<i64>(
                    sce_forge_runtime::algorithm::add::<i64>(
                        sce_forge_runtime::algorithm::sub::<i64>(
                            sce_forge_runtime::algorithm::add::<i64>(
                                sce_forge_runtime::algorithm::add::<i64>(
                                    sce_forge_runtime::algorithm::mul::<i64>(pcera, 146097)?,
                                    sce_forge_runtime::algorithm::mul::<i64>(pcyoe, 365)?,
                                )?,
                                sce_forge_runtime::algorithm::div::<i64>(pcyoe, 4)?,
                            )?,
                            sce_forge_runtime::algorithm::div::<i64>(pcyoe, 100)?,
                        )?,
                        pcdoy,
                    )?,
                    719468,
                )?;
                plen = if freq == 4 {
                    if pleap {
                        366
                    } else {
                        365
                    }
                } else {
                    if ptm == 2 {
                        if pleap {
                            29
                        } else {
                            28
                        }
                    } else {
                        if ptm == 4 || ptm == 6 || ptm == 9 || ptm == 11 {
                            30
                        } else {
                            31
                        }
                    }
                };
            }
            if ps >= end_day
                || until > 0 && sce_forge_runtime::algorithm::mul::<i64>(ps, day)? > until
            {
                going = false;
            } else {
                let mut n: i64 = 0;
                let mut k: i64 = 0;
                while k < plen {
                    if event_selects::event_selects(
                        s,
                        sce_forge_runtime::algorithm::add::<i64>(ps, k)?,
                        start_day,
                    )? {
                        n = sce_forge_runtime::algorithm::add::<i64>(n, 1)?;
                    }
                    k = sce_forge_runtime::algorithm::add::<i64>(k, 1)?;
                }
                let mut i: i64 = 0;
                let mut rank: i64 = 0;
                while i < plen && going {
                    let dd: i64 = sce_forge_runtime::algorithm::add::<i64>(ps, i)?;
                    if event_selects::event_selects(s, dd, start_day)? {
                        rank = sce_forge_runtime::algorithm::add::<i64>(rank, 1)?;
                        let mut keep: bool = true;
                        if has_sp {
                            keep = rank <= 62 && s.sp_pos >> rank & 1 == 1
                                || sce_forge_runtime::algorithm::add::<i64>(
                                    sce_forge_runtime::algorithm::sub::<i64>(n, rank)?,
                                    1,
                                )? <= 62
                                    && s.sp_neg
                                        >> sce_forge_runtime::algorithm::add::<i64>(
                                            sce_forge_runtime::algorithm::sub::<i64>(n, rank)?,
                                            1,
                                        )?
                                        & 1
                                        == 1;
                        }
                        if keep && dd >= start_day {
                            let occ_start: i64 = sce_forge_runtime::algorithm::add::<i64>(
                                sce_forge_runtime::algorithm::mul::<i64>(dd, day)?,
                                time_of_day,
                            )?;
                            let occ_end: i64 =
                                sce_forge_runtime::algorithm::add::<i64>(occ_start, length)?;
                            if occ_start >= window_end || until > 0 && occ_start > until {
                                going = false;
                            } else {
                                if count > 0 && nth >= count {
                                    going = false;
                                } else {
                                    if (occ_end > window_start || occ_start >= window_start)
                                        && emitted < 64
                                        && nth >= skipped
                                    {
                                        let mut excluded: bool = false;
                                        let mut mark: u8 =
                                            sce_forge_runtime::algorithm::narrow::<u8, i64>(
                                                s.flag,
                                            )?;
                                        if dd >= 0 && dd < 65472 {
                                            let xfield: u64 = sce_forge_runtime::algorithm::narrow::<
                                                u64,
                                                i64,
                                            >(
                                                sce_forge_runtime::algorithm::add::<i64>(dd, 64)?,
                                            )?;
                                            let mut xfound: bool = false;
                                            let mut xheld: i64 = 0;
                                            let mut xw: i64 = 0;
                                            let mut xc: u32 = 0;
                                            let mut xn: u64 = 0;
                                            let mut xv: i64 = 0;
                                            for wx in values.iter() {
                                                if wx.id == id && wx.field == xfield {
                                                    xheld = sce_forge_runtime::algorithm::add::<i64>(
                                                        xheld, 1,
                                                    )?;
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
                                                nth: sce_forge_runtime::algorithm::narrow::<
                                                    u32,
                                                    i64,
                                                >(
                                                    nth
                                                )?,
                                                conflict: mark,
                                            };
                                            out.push(line)?;
                                            emitted = sce_forge_runtime::algorithm::add::<i64>(
                                                emitted, 1,
                                            )?;
                                        }
                                    }
                                    nth = sce_forge_runtime::algorithm::add::<i64>(nth, 1)?;
                                }
                            }
                        }
                    }
                    if emitted >= 64 {
                        going = false;
                    }
                    i = sce_forge_runtime::algorithm::add::<i64>(i, 1)?;
                }
                if freq == 0 {
                    going = false;
                }
            }
            p = sce_forge_runtime::algorithm::add::<i64>(p, 1)?;
        }
    }
    return Ok(out);
}
