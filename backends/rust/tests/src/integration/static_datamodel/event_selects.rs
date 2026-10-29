// SCE-GENERATED — DO NOT EDIT
// source-hash: 75c0c87af90d82dec15f516a721dd486d47e1280589aefe06f15efdef5f4f850
#![doc = "SCE-MAP: event_selects.scxml:19 :: _forge_body"]
// SCE-MAP: event_selects.scxml:19 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function, no instance state. `#![no_std]`-clean when no `bytes`
// parameter (this fixture: no_std_clean = true).

use super::event_schedule;

#[allow(clippy::all)]
#[allow(unused_assignments)]
pub fn event_selects(
    s: event_schedule::EventSchedulePayload,
    d: i64,
    start_day: i64,
) -> Result<bool, sce_forge_runtime::algorithm::AlgorithmError> {
    let freq: i64 = s.freq;
    let mut ok: bool = true;
    if freq != 0 {
        let start_wd: i64 = sce_forge_runtime::algorithm::rem::<i64>(
            sce_forge_runtime::algorithm::add::<i64>(
                sce_forge_runtime::algorithm::rem::<i64>(start_day, 7)?,
                11,
            )?,
            7,
        )?;
        let z0: i64 = sce_forge_runtime::algorithm::add::<i64>(start_day, 719468)?;
        let era0: i64 = sce_forge_runtime::algorithm::div::<i64>(z0, 146097)?;
        let doe0: i64 = sce_forge_runtime::algorithm::sub::<i64>(
            z0,
            sce_forge_runtime::algorithm::mul::<i64>(era0, 146097)?,
        )?;
        let yoe0: i64 = sce_forge_runtime::algorithm::div::<i64>(
            sce_forge_runtime::algorithm::sub::<i64>(
                sce_forge_runtime::algorithm::add::<i64>(
                    sce_forge_runtime::algorithm::sub::<i64>(
                        doe0,
                        sce_forge_runtime::algorithm::div::<i64>(doe0, 1460)?,
                    )?,
                    sce_forge_runtime::algorithm::div::<i64>(doe0, 36524)?,
                )?,
                sce_forge_runtime::algorithm::div::<i64>(doe0, 146096)?,
            )?,
            365,
        )?;
        let doy0: i64 = sce_forge_runtime::algorithm::sub::<i64>(
            doe0,
            sce_forge_runtime::algorithm::sub::<i64>(
                sce_forge_runtime::algorithm::add::<i64>(
                    sce_forge_runtime::algorithm::mul::<i64>(365, yoe0)?,
                    sce_forge_runtime::algorithm::div::<i64>(yoe0, 4)?,
                )?,
                sce_forge_runtime::algorithm::div::<i64>(yoe0, 100)?,
            )?,
        )?;
        let mp0: i64 = sce_forge_runtime::algorithm::div::<i64>(
            sce_forge_runtime::algorithm::add::<i64>(
                sce_forge_runtime::algorithm::mul::<i64>(5, doy0)?,
                2,
            )?,
            153,
        )?;
        let dom0: i64 = sce_forge_runtime::algorithm::add::<i64>(
            sce_forge_runtime::algorithm::sub::<i64>(
                doy0,
                sce_forge_runtime::algorithm::div::<i64>(
                    sce_forge_runtime::algorithm::add::<i64>(
                        sce_forge_runtime::algorithm::mul::<i64>(153, mp0)?,
                        2,
                    )?,
                    5,
                )?,
            )?,
            1,
        )?;
        let m0: i64 = if mp0 < 10 {
            sce_forge_runtime::algorithm::add::<i64>(mp0, 3)?
        } else {
            sce_forge_runtime::algorithm::sub::<i64>(mp0, 9)?
        };
        let dz: i64 = sce_forge_runtime::algorithm::add::<i64>(d, 719468)?;
        let dera: i64 = sce_forge_runtime::algorithm::div::<i64>(dz, 146097)?;
        let ddoe: i64 = sce_forge_runtime::algorithm::sub::<i64>(
            dz,
            sce_forge_runtime::algorithm::mul::<i64>(dera, 146097)?,
        )?;
        let dyoe: i64 = sce_forge_runtime::algorithm::div::<i64>(
            sce_forge_runtime::algorithm::sub::<i64>(
                sce_forge_runtime::algorithm::add::<i64>(
                    sce_forge_runtime::algorithm::sub::<i64>(
                        ddoe,
                        sce_forge_runtime::algorithm::div::<i64>(ddoe, 1460)?,
                    )?,
                    sce_forge_runtime::algorithm::div::<i64>(ddoe, 36524)?,
                )?,
                sce_forge_runtime::algorithm::div::<i64>(ddoe, 146096)?,
            )?,
            365,
        )?;
        let ddoy: i64 = sce_forge_runtime::algorithm::sub::<i64>(
            ddoe,
            sce_forge_runtime::algorithm::sub::<i64>(
                sce_forge_runtime::algorithm::add::<i64>(
                    sce_forge_runtime::algorithm::mul::<i64>(365, dyoe)?,
                    sce_forge_runtime::algorithm::div::<i64>(dyoe, 4)?,
                )?,
                sce_forge_runtime::algorithm::div::<i64>(dyoe, 100)?,
            )?,
        )?;
        let dmp: i64 = sce_forge_runtime::algorithm::div::<i64>(
            sce_forge_runtime::algorithm::add::<i64>(
                sce_forge_runtime::algorithm::mul::<i64>(5, ddoy)?,
                2,
            )?,
            153,
        )?;
        let dom: i64 = sce_forge_runtime::algorithm::add::<i64>(
            sce_forge_runtime::algorithm::sub::<i64>(
                ddoy,
                sce_forge_runtime::algorithm::div::<i64>(
                    sce_forge_runtime::algorithm::add::<i64>(
                        sce_forge_runtime::algorithm::mul::<i64>(153, dmp)?,
                        2,
                    )?,
                    5,
                )?,
            )?,
            1,
        )?;
        let mon: i64 = if dmp < 10 {
            sce_forge_runtime::algorithm::add::<i64>(dmp, 3)?
        } else {
            sce_forge_runtime::algorithm::sub::<i64>(dmp, 9)?
        };
        let yr: i64 = if mon <= 2 {
            sce_forge_runtime::algorithm::add::<i64>(
                sce_forge_runtime::algorithm::add::<i64>(
                    dyoe,
                    sce_forge_runtime::algorithm::mul::<i64>(dera, 400)?,
                )?,
                1,
            )?
        } else {
            sce_forge_runtime::algorithm::add::<i64>(
                dyoe,
                sce_forge_runtime::algorithm::mul::<i64>(dera, 400)?,
            )?
        };
        let leap: bool = sce_forge_runtime::algorithm::rem::<i64>(yr, 4)? == 0
            && sce_forge_runtime::algorithm::rem::<i64>(yr, 100)? != 0
            || sce_forge_runtime::algorithm::rem::<i64>(yr, 400)? == 0;
        let last: i64 = if mon == 2 {
            if leap {
                29
            } else {
                28
            }
        } else {
            if mon == 4 || mon == 6 || mon == 9 || mon == 11 {
                30
            } else {
                31
            }
        };
        let ylen: i64 = if leap { 366 } else { 365 };
        let wd: i64 = sce_forge_runtime::algorithm::rem::<i64>(
            sce_forge_runtime::algorithm::add::<i64>(
                sce_forge_runtime::algorithm::rem::<i64>(d, 7)?,
                11,
            )?,
            7,
        )?;
        let yday: i64 = sce_forge_runtime::algorithm::add::<i64>(
            sce_forge_runtime::algorithm::add::<i64>(
                sce_forge_runtime::algorithm::div::<i64>(
                    sce_forge_runtime::algorithm::sub::<i64>(
                        sce_forge_runtime::algorithm::mul::<i64>(367, mon)?,
                        362,
                    )?,
                    12,
                )?,
                if mon <= 2 {
                    0
                } else {
                    if leap {
                        -1
                    } else {
                        -2
                    }
                },
            )?,
            dom,
        )?;
        let has_month: bool = s.bymonth != 0;
        let has_md: bool = s.md_pos | s.md_neg != 0;
        let has_day: bool = s.days
            | s.op0
            | s.op1
            | s.op2
            | s.op3
            | s.op4
            | s.op5
            | s.op6
            | s.on0
            | s.on1
            | s.on2
            | s.on3
            | s.on4
            | s.on5
            | s.on6
            != 0;
        let has_wn: bool = s.wn_pos | s.wn_neg != 0;
        let has_yd: bool = s.yd0
            | s.yd1
            | s.yd2
            | s.yd3
            | s.yd4
            | s.yd5
            | s.ydn0
            | s.ydn1
            | s.ydn2
            | s.ydn3
            | s.ydn4
            | s.ydn5
            != 0;
        let md_match: bool = s.md_pos >> dom & 1 == 1
            || s.md_neg
                >> sce_forge_runtime::algorithm::add::<i64>(
                    sce_forge_runtime::algorithm::sub::<i64>(last, dom)?,
                    1,
                )?
                & 1
                == 1;
        let plain_day: bool = s.days >> wd & 1 == 1;
        let in_month: bool = freq == 3 || has_month;
        let ord_pos: i64 = if in_month {
            sce_forge_runtime::algorithm::add::<i64>(
                sce_forge_runtime::algorithm::div::<i64>(
                    sce_forge_runtime::algorithm::sub::<i64>(dom, 1)?,
                    7,
                )?,
                1,
            )?
        } else {
            sce_forge_runtime::algorithm::add::<i64>(
                sce_forge_runtime::algorithm::div::<i64>(
                    sce_forge_runtime::algorithm::sub::<i64>(yday, 1)?,
                    7,
                )?,
                1,
            )?
        };
        let ord_neg: i64 = if in_month {
            sce_forge_runtime::algorithm::add::<i64>(
                sce_forge_runtime::algorithm::div::<i64>(
                    sce_forge_runtime::algorithm::sub::<i64>(last, dom)?,
                    7,
                )?,
                1,
            )?
        } else {
            sce_forge_runtime::algorithm::add::<i64>(
                sce_forge_runtime::algorithm::div::<i64>(
                    sce_forge_runtime::algorithm::sub::<i64>(ylen, yday)?,
                    7,
                )?,
                1,
            )?
        };
        let word_first: i64 = if wd == 0 {
            s.op0
        } else {
            if wd == 1 {
                s.op1
            } else {
                if wd == 2 {
                    s.op2
                } else {
                    if wd == 3 {
                        s.op3
                    } else {
                        if wd == 4 {
                            s.op4
                        } else {
                            if wd == 5 {
                                s.op5
                            } else {
                                s.op6
                            }
                        }
                    }
                }
            }
        };
        let word_last: i64 = if wd == 0 {
            s.on0
        } else {
            if wd == 1 {
                s.on1
            } else {
                if wd == 2 {
                    s.on2
                } else {
                    if wd == 3 {
                        s.on3
                    } else {
                        if wd == 4 {
                            s.on4
                        } else {
                            if wd == 5 {
                                s.on5
                            } else {
                                s.on6
                            }
                        }
                    }
                }
            }
        };
        let ord_first: bool = ord_pos <= 53
            && word_first >> sce_forge_runtime::algorithm::sub::<i64>(ord_pos, 1)? & 1 == 1;
        let ord_last: bool = ord_neg <= 53
            && word_last >> sce_forge_runtime::algorithm::sub::<i64>(ord_neg, 1)? & 1 == 1;
        let day_match: bool = plain_day || ord_first || ord_last;
        let clamped: bool = if s.invalid == 1 {
            dom == dom0
        } else {
            dom == (if dom0 < last { dom0 } else { last })
        };
        ok = !has_month || s.bymonth >> mon & 1 == 1;
        if freq == 1 {
            if has_md {
                ok = ok && md_match;
            }
            if has_day {
                ok = ok && plain_day;
            }
        }
        if freq == 2 {
            ok = ok && (if has_day { plain_day } else { wd == start_wd });
        }
        if freq == 3 {
            if has_md {
                ok = ok && md_match;
            }
            if has_day {
                ok = ok && day_match;
            }
            if !has_md && !has_day {
                ok = ok && clamped;
            }
        }
        if freq == 4 {
            if has_wn {
                let wkst: i64 = if s.wkst >= 0 && s.wkst <= 6 {
                    s.wkst
                } else {
                    1
                };
                let wi: i64 = sce_forge_runtime::algorithm::add::<i64>(
                    sce_forge_runtime::algorithm::rem::<i64>(
                        sce_forge_runtime::algorithm::add::<i64>(
                            sce_forge_runtime::algorithm::sub::<i64>(wd, wkst)?,
                            7,
                        )?,
                        7,
                    )?,
                    1,
                )?;
                let wk: i64 = sce_forge_runtime::algorithm::div::<i64>(
                    sce_forge_runtime::algorithm::add::<i64>(
                        sce_forge_runtime::algorithm::sub::<i64>(yday, wi)?,
                        10,
                    )?,
                    7,
                )?;
                let wd28: i64 = sce_forge_runtime::algorithm::rem::<i64>(
                    sce_forge_runtime::algorithm::add::<i64>(
                        sce_forge_runtime::algorithm::sub::<i64>(
                            sce_forge_runtime::algorithm::sub::<i64>(
                                sce_forge_runtime::algorithm::add::<i64>(wd, ylen)?,
                                3,
                            )?,
                            yday,
                        )?,
                        7,
                    )?,
                    7,
                )?;
                let wi28: i64 = sce_forge_runtime::algorithm::add::<i64>(
                    sce_forge_runtime::algorithm::rem::<i64>(
                        sce_forge_runtime::algorithm::add::<i64>(
                            sce_forge_runtime::algorithm::sub::<i64>(wd28, wkst)?,
                            7,
                        )?,
                        7,
                    )?,
                    1,
                )?;
                let wks: i64 = sce_forge_runtime::algorithm::div::<i64>(
                    sce_forge_runtime::algorithm::add::<i64>(
                        sce_forge_runtime::algorithm::sub::<i64>(
                            sce_forge_runtime::algorithm::sub::<i64>(ylen, 3)?,
                            wi28,
                        )?,
                        10,
                    )?,
                    7,
                )?;
                let wk_ok: bool = wk >= 1
                    && wk <= wks
                    && (s.wn_pos >> wk & 1 == 1
                        || s.wn_neg
                            >> sce_forge_runtime::algorithm::add::<i64>(
                                sce_forge_runtime::algorithm::sub::<i64>(wks, wk)?,
                                1,
                            )?
                            & 1
                            == 1);
                ok = ok && wk_ok;
            }
            if has_yd {
                let yp: i64 = sce_forge_runtime::algorithm::sub::<i64>(yday, 1)?;
                let yn: i64 = sce_forge_runtime::algorithm::sub::<i64>(ylen, yday)?;
                let jp: i64 = sce_forge_runtime::algorithm::div::<i64>(yp, 61)?;
                let jn: i64 = sce_forge_runtime::algorithm::div::<i64>(yn, 61)?;
                let word_pos: i64 = if jp == 0 {
                    s.yd0
                } else {
                    if jp == 1 {
                        s.yd1
                    } else {
                        if jp == 2 {
                            s.yd2
                        } else {
                            if jp == 3 {
                                s.yd3
                            } else {
                                if jp == 4 {
                                    s.yd4
                                } else {
                                    s.yd5
                                }
                            }
                        }
                    }
                };
                let word_neg: i64 = if jn == 0 {
                    s.ydn0
                } else {
                    if jn == 1 {
                        s.ydn1
                    } else {
                        if jn == 2 {
                            s.ydn2
                        } else {
                            if jn == 3 {
                                s.ydn3
                            } else {
                                if jn == 4 {
                                    s.ydn4
                                } else {
                                    s.ydn5
                                }
                            }
                        }
                    }
                };
                let yd_ok: bool = word_pos >> sce_forge_runtime::algorithm::rem::<i64>(yp, 61)? & 1
                    == 1
                    || word_neg >> sce_forge_runtime::algorithm::rem::<i64>(yn, 61)? & 1 == 1;
                ok = ok && yd_ok;
            }
            if has_md {
                ok = ok && md_match;
            }
            if has_day {
                ok = ok && day_match;
            }
            if has_wn && !has_day && !has_yd && !has_md {
                ok = ok && wd == start_wd;
            }
            if !has_wn && !has_yd && !has_md && !has_day {
                ok = ok && (has_month || mon == m0) && clamped;
            }
        }
    }
    return Ok(ok);
}
