// SCE-GENERATED — DO NOT EDIT
// source-hash: 9ad3991e659415a87e89be4422403fddaea9105e2a522b988eeacca0944ca255
#![doc = "SCE-MAP: sheet_settle.scxml:63 :: _forge_body"]
// SCE-MAP: sheet_settle.scxml:63 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function, no instance state. `#![no_std]`-clean when no `bytes`
// parameter (this fixture: no_std_clean = true).

use super::lww_cell;
use super::rga_gone;
use super::rga_node;
use super::sheet_apply;
use super::sheet_value;

use sce_portable_bytes::SceOwnedList;

#[allow(clippy::all)]
#[allow(unused_assignments)]
pub fn sheet_settle(
    cells: &[lww_cell::LwwCellPayload],
    rows: &[rga_node::RgaNodePayload],
    gone: &[rga_gone::RgaGonePayload],
    cols: &[rga_node::RgaNodePayload],
    gone_cols: &[rga_gone::RgaGonePayload],
) -> Result<SceOwnedList<i64, 768>, sce_forge_runtime::algorithm::AlgorithmError> {
    let mut trail: SceOwnedList<i64, 768> = SceOwnedList::new();
    let mut moving: bool = true;
    let mut passes: i64 = 0;
    while moving && passes <= (cells).len() as i64 {
        moving = false;
        passes = sce_forge_runtime::algorithm::add::<i64>(passes, 1)?;
        let mut at: i64 = 0;
        for &c in cells.iter() {
            let mut settled: bool = false;
            let mut d: i64 = 0;
            while d < (trail).len() as i64 {
                if sce_forge_runtime::algorithm::at(&trail, d)? == at {
                    settled = true;
                }
                d = sce_forge_runtime::algorithm::add::<i64>(d, 3)?;
            }
            if !settled {
                let mut ready: bool = true;
                let mut v: i64 = 0;
                let mut f: i64 = 0;
                let mut va: i64 = 0;
                let mut fa: i64 = 0;
                let mut vb: i64 = 0;
                let mut fb: i64 = 0;
                let sides: i64 = if c.op >= 1 && c.op <= 5 {
                    if c.op == 1 {
                        1
                    } else {
                        2
                    }
                } else {
                    0
                };
                let mut side: i64 = 0;
                while side < sides {
                    let is_ref: bool = if side == 0 { c.a_ref } else { c.b_ref };
                    let want_row: u64 = if side == 0 { c.a_row } else { c.b_row };
                    let want_col: u64 = if side == 0 { c.a_col } else { c.b_col };
                    let mut xv: i64 = if side == 0 { c.a_val } else { c.b_val };
                    let mut xf: i64 = 0;
                    if is_ref {
                        xv = 0;
                        let mut found: bool = false;
                        let mut holder: i64 = 0;
                        let mut j: i64 = 0;
                        for &probe in cells.iter() {
                            if !found && probe.row == want_row && probe.col == want_col {
                                found = true;
                                holder = j;
                            }
                            j = sce_forge_runtime::algorithm::add::<i64>(j, 1)?;
                        }
                        if found {
                            let mut hit: bool = false;
                            let mut m: i64 = 0;
                            while m < (trail).len() as i64 {
                                if sce_forge_runtime::algorithm::at(&trail, m)? == holder {
                                    hit = true;
                                    xv = sce_forge_runtime::algorithm::at(
                                        &trail,
                                        sce_forge_runtime::algorithm::add::<i64>(m, 1)?,
                                    )?;
                                    xf = sce_forge_runtime::algorithm::at(
                                        &trail,
                                        sce_forge_runtime::algorithm::add::<i64>(m, 2)?,
                                    )?;
                                }
                                m = sce_forge_runtime::algorithm::add::<i64>(m, 3)?;
                            }
                            if !hit {
                                ready = false;
                            }
                        }
                    }
                    if side == 0 {
                        va = xv;
                        fa = xf;
                    } else {
                        vb = xv;
                        fb = xf;
                    }
                    side = sce_forge_runtime::algorithm::add::<i64>(side, 1)?;
                }
                if c.op == 0 {
                    v = c.value;
                }
                if c.op == 1 {
                    v = va;
                    f = fa;
                }
                if c.op >= 2 && c.op <= 5 {
                    if fa != 0 || fb != 0 {
                        f = if fa == 0 {
                            fb
                        } else {
                            if fb == 0 {
                                fa
                            } else {
                                if fa < fb {
                                    fa
                                } else {
                                    fb
                                }
                            }
                        };
                    } else {
                        let applied: sheet_value::SheetValuePayload =
                            sheet_apply::sheet_apply(c.op, va, vb)?;
                        v = applied.value;
                        f = applied.fault as i64;
                    }
                }
                if c.op > 6 {
                    f = 5;
                }
                if c.op == 6 {
                    let mut ra: i64 = 0;
                    let mut rb: i64 = 0;
                    let mut ca: i64 = 0;
                    let mut cb: i64 = 0;
                    let mut spin: i64 = 0;
                    for &rw in rows.iter() {
                        spin = sce_forge_runtime::algorithm::add::<i64>(spin, 1)?;
                        if rw.key == c.a_row {
                            ra = spin;
                        }
                        if rw.key == c.b_row {
                            rb = spin;
                        }
                    }
                    spin = 0;
                    for &cl in cols.iter() {
                        spin = sce_forge_runtime::algorithm::add::<i64>(spin, 1)?;
                        if cl.key == c.a_col {
                            ca = spin;
                        }
                        if cl.key == c.b_col {
                            cb = spin;
                        }
                    }
                    if !c.a_ref || !c.b_ref || ra == 0 || rb == 0 || ca == 0 || cb == 0 {
                        f = 4;
                    } else {
                        let row_low: i64 = if ra < rb { ra } else { rb };
                        let row_high: i64 = if ra < rb { rb } else { ra };
                        let col_low: i64 = if ca < cb { ca } else { cb };
                        let col_high: i64 = if ca < cb { cb } else { ca };
                        let mut above: i64 = 0;
                        let mut below: i64 = 0;
                        let mut spilled: bool = false;
                        let mut worst: i64 = 0;
                        let mut walked: i64 = 0;
                        for &q in cells.iter() {
                            let mut qr: i64 = 0;
                            let mut qc: i64 = 0;
                            let mut qs: i64 = 0;
                            for &rq in rows.iter() {
                                qs = sce_forge_runtime::algorithm::add::<i64>(qs, 1)?;
                                if rq.key == q.row {
                                    qr = qs;
                                }
                            }
                            qs = 0;
                            for &cq in cols.iter() {
                                qs = sce_forge_runtime::algorithm::add::<i64>(qs, 1)?;
                                if cq.key == q.col {
                                    qc = qs;
                                }
                            }
                            let mut qgone: bool = false;
                            for &gq in gone.iter() {
                                if gq.key == q.row {
                                    qgone = true;
                                }
                            }
                            for &hq in gone_cols.iter() {
                                if hq.key == q.col {
                                    qgone = true;
                                }
                            }
                            if !qgone
                                && qr >= row_low
                                && qr <= row_high
                                && qc >= col_low
                                && qc <= col_high
                            {
                                let mut seen: bool = false;
                                let mut w: i64 = 0;
                                while w < (trail).len() as i64 {
                                    if sce_forge_runtime::algorithm::at(&trail, w)? == walked {
                                        seen = true;
                                        let mv: i64 = sce_forge_runtime::algorithm::at(
                                            &trail,
                                            sce_forge_runtime::algorithm::add::<i64>(w, 1)?,
                                        )?;
                                        let mf: i64 = sce_forge_runtime::algorithm::at(
                                            &trail,
                                            sce_forge_runtime::algorithm::add::<i64>(w, 2)?,
                                        )?;
                                        if mf != 0 {
                                            worst = if worst == 0 {
                                                mf
                                            } else {
                                                if mf < worst {
                                                    mf
                                                } else {
                                                    worst
                                                }
                                            };
                                        }
                                        if mf == 0 && mv > 0 {
                                            let grown: sheet_value::SheetValuePayload =
                                                sheet_apply::sheet_apply(2, above, mv)?;
                                            if grown.fault != 0 {
                                                spilled = true;
                                            } else {
                                                above = grown.value;
                                            }
                                        }
                                        if mf == 0 && mv < 0 {
                                            let shrunk: sheet_value::SheetValuePayload =
                                                sheet_apply::sheet_apply(2, below, mv)?;
                                            if shrunk.fault != 0 {
                                                spilled = true;
                                            } else {
                                                below = shrunk.value;
                                            }
                                        }
                                    }
                                    w = sce_forge_runtime::algorithm::add::<i64>(w, 3)?;
                                }
                                if !seen {
                                    ready = false;
                                }
                            }
                            walked = sce_forge_runtime::algorithm::add::<i64>(walked, 1)?;
                        }
                        if worst != 0 {
                            f = worst;
                        } else {
                            if spilled {
                                f = 3;
                            } else {
                                v = sce_forge_runtime::algorithm::add::<i64>(above, below)?;
                            }
                        }
                    }
                }
                if ready {
                    trail.push(at)?;
                    trail.push(v)?;
                    trail.push(f)?;
                    moving = true;
                }
            }
            at = sce_forge_runtime::algorithm::add::<i64>(at, 1)?;
        }
    }
    return Ok(trail);
}
