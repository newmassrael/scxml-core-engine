// SCE-GENERATED — DO NOT EDIT
// source-hash: 9ad3991e659415a87e89be4422403fddaea9105e2a522b988eeacca0944ca255
#![doc = "SCE-MAP: mv_value_keep.scxml:22 :: _forge_body"]
// SCE-MAP: mv_value_keep.scxml:22 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function, no instance state. `#![no_std]`-clean when no `bytes`
// parameter (this fixture: no_std_clean = true).

use super::mv_value;

use sce_portable_bytes::SceOwnedList;

#[allow(clippy::all)]
#[allow(unused_assignments)]
pub fn mv_value_keep(
    values: &[mv_value::MvValuePayload],
    held: mv_value::MvValuePayload,
) -> Result<SceOwnedList<mv_value::MvValuePayload, 512>, sce_forge_runtime::algorithm::AlgorithmError>
{
    let mut out: SceOwnedList<mv_value::MvValuePayload, 512> = SceOwnedList::new();
    let mut there: bool = false;
    for v in values.iter() {
        if v.id == held.id
            && v.field == held.field
            && v.wall_time == held.wall_time
            && v.counter == held.counter
            && v.node_id == held.node_id
        {
            there = true;
        }
        out.push(v.clone())?;
    }
    if !there {
        out.push(held.clone())?;
    }
    return Ok(out);
}
