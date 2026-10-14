// SCE-GENERATED — DO NOT EDIT
// source-hash: c431fa9e45990e8b729e69a4bf81ba13f9cc4e8066ddecbd5ff6346ac56dd5f3
#![doc = "SCE-MAP: lww_write.scxml:24 :: _forge_body"]
// SCE-MAP: lww_write.scxml:24 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function, no instance state. `#![no_std]`-clean when no `bytes`
// parameter (this fixture: no_std_clean = true).

use super::hlc_compare;
use super::hlc_timestamp;
use super::lww_cell;

use sce_portable_bytes::SceOwnedList;

#[allow(clippy::all)]
#[allow(unused_assignments)]
pub fn lww_write(
    cells: &[lww_cell::LwwCellPayload],
    write: lww_cell::LwwCellPayload,
) -> Result<SceOwnedList<lww_cell::LwwCellPayload, 256>, sce_forge_runtime::algorithm::AlgorithmError>
{
    let mut out: SceOwnedList<lww_cell::LwwCellPayload, 256> = SceOwnedList::new();
    let mut held: bool = false;
    let incoming = hlc_timestamp::HlcTimestampPayload {
        wall_time: write.wall_time,
        counter: write.counter,
        node_id: write.node_id,
    };
    for &c in cells.iter() {
        if c.row == write.row && c.col == write.col {
            held = true;
            let stored = hlc_timestamp::HlcTimestampPayload {
                wall_time: c.wall_time,
                counter: c.counter,
                node_id: c.node_id,
            };
            if hlc_compare::hlc_compare(incoming, stored) > 0 {
                out.push(write)?;
            } else {
                out.push(c)?;
            }
        } else {
            out.push(c)?;
        }
    }
    if !held {
        out.push(write)?;
    }
    return Ok(out);
}
