// SCE-GENERATED — DO NOT EDIT
// source-hash: b2921c03007062914a1e69cc4069c6b54c8d66f97313a50f3246eed3d071c790
#![doc = "SCE-MAP: algorithm_ids_above.scxml:6 :: _forge_body"]
// SCE-MAP: algorithm_ids_above.scxml:6 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function, no instance state. `#![no_std]`-clean when no `bytes`
// parameter (this fixture: no_std_clean = true).

#[allow(clippy::all)]
#[allow(unused_assignments)]
pub fn ids_above(ids: &[u32], floor: u32) -> bool {
    let mut above: bool = false;
    for &id in ids.iter() {
        if id > floor {
            above = true;
        }
    }
    return above;
}
