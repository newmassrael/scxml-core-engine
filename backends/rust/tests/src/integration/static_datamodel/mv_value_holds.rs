// SCE-GENERATED — DO NOT EDIT
// source-hash: 8149e47f56da0bbb92d7b1c4600956bbfd585c43526372f3618b2e600843296d
#![doc = "SCE-MAP: mv_value_holds.scxml:13 :: _forge_body"]
// SCE-MAP: mv_value_holds.scxml:13 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function, no instance state. `#![no_std]`-clean when no `bytes`
// parameter (this fixture: no_std_clean = true).

use super::mv_value;

#[allow(clippy::all)]
#[allow(unused_assignments)]
pub fn mv_value_holds(
    values: &[mv_value::MvValuePayload],
    probe: mv_value::MvValuePayload,
) -> bool {
    let mut found: bool = false;
    for v in values.iter() {
        if v.id == probe.id
            && v.field == probe.field
            && v.wall_time == probe.wall_time
            && v.counter == probe.counter
            && v.node_id == probe.node_id
        {
            found = true;
        }
    }
    return found;
}
