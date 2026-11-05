// SCE-GENERATED — DO NOT EDIT
// source-hash: fb4f42a34e7ac40a28d8e5daee5d633c3c45dddac513816edf2bb83fdc85f91c
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
    out: &mut SceOwnedList<mv_value::MvValuePayload, 512>,
) -> Result<(), sce_forge_runtime::algorithm::AlgorithmError> {
    out.clear();
    out.reserve(512);
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
    return Ok(());
}
