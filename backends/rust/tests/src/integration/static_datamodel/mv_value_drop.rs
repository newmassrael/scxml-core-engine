// SCE-GENERATED — DO NOT EDIT
// source-hash: 8149e47f56da0bbb92d7b1c4600956bbfd585c43526372f3618b2e600843296d
#![doc = "SCE-MAP: mv_value_drop.scxml:16 :: _forge_body"]
// SCE-MAP: mv_value_drop.scxml:16 :: _forge_body

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
pub fn mv_value_drop(
    values: &[mv_value::MvValuePayload],
    gone: mv_value::MvValuePayload,
) -> Result<SceOwnedList<mv_value::MvValuePayload, 512>, sce_forge_runtime::algorithm::AlgorithmError>
{
    let mut out: SceOwnedList<mv_value::MvValuePayload, 512> = SceOwnedList::new();
    for v in values.iter() {
        if v.id != gone.id
            || v.field != gone.field
            || v.wall_time != gone.wall_time
            || v.counter != gone.counter
            || v.node_id != gone.node_id
        {
            out.push(v.clone())?;
        }
    }
    return Ok(out);
}
