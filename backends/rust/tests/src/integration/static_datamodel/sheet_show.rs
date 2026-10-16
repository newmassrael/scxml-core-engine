// SCE-GENERATED — DO NOT EDIT
// source-hash: 8149e47f56da0bbb92d7b1c4600956bbfd585c43526372f3618b2e600843296d
#![doc = "SCE-MAP: sheet_show.scxml:24 :: _forge_body"]
// SCE-MAP: sheet_show.scxml:24 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function, no instance state. `#![no_std]`-clean when no `bytes`
// parameter (this fixture: no_std_clean = true).

use super::lww_cell;
use super::sheet_shown;

use sce_portable_bytes::SceOwnedList;

#[allow(clippy::all)]
#[allow(unused_assignments)]
pub fn sheet_show(
    cells: &[lww_cell::LwwCellPayload],
    trail: &[i64],
) -> Result<
    SceOwnedList<sheet_shown::SheetShownPayload, 256>,
    sce_forge_runtime::algorithm::AlgorithmError,
> {
    let mut out: SceOwnedList<sheet_shown::SheetShownPayload, 256> = SceOwnedList::new();
    let mut started: bool = false;
    let mut last_row: u64 = 0;
    let mut last_col: u64 = 0;
    let mut more: bool = true;
    while more {
        let mut chosen: bool = false;
        let mut best_row: u64 = 0;
        let mut best_col: u64 = 0;
        let mut best_at: i64 = 0;
        let mut cursor: i64 = 0;
        for &s in cells.iter() {
            if (!started || s.row > last_row || s.row == last_row && s.col > last_col)
                && (!chosen || s.row < best_row || s.row == best_row && s.col < best_col)
            {
                chosen = true;
                best_row = s.row;
                best_col = s.col;
                best_at = cursor;
            }
            cursor = sce_forge_runtime::algorithm::add::<i64>(cursor, 1)?;
        }
        if chosen {
            let mut shown_value: i64 = 0;
            let mut shown_fault: u8 = 1;
            let mut u: i64 = 0;
            while u < (trail).len() as i64 {
                if sce_forge_runtime::algorithm::at(&trail, u)? == best_at {
                    shown_value = sce_forge_runtime::algorithm::at(
                        &trail,
                        sce_forge_runtime::algorithm::add::<i64>(u, 1)?,
                    )?;
                    shown_fault = sce_forge_runtime::algorithm::narrow::<u8, i64>(
                        sce_forge_runtime::algorithm::at(
                            &trail,
                            sce_forge_runtime::algorithm::add::<i64>(u, 2)?,
                        )?,
                    )?;
                }
                u = sce_forge_runtime::algorithm::add::<i64>(u, 3)?;
            }
            let line = sheet_shown::SheetShownPayload {
                row: best_row,
                col: best_col,
                value: shown_value,
                fault: shown_fault,
            };
            out.push(line)?;
            started = true;
            last_row = best_row;
            last_col = best_col;
        } else {
            more = false;
        }
    }
    return Ok(out);
}
