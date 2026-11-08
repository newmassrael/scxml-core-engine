// SCE-GENERATED — DO NOT EDIT
// source-hash: ea764ab5524bc7a7e7d4553640b3fb26547d13f0d2053c0f5e7214161efbc6d0
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
