// SCE-GENERATED — DO NOT EDIT
// source-hash: fb4f42a34e7ac40a28d8e5daee5d633c3c45dddac513816edf2bb83fdc85f91c
#![doc = "SCE-MAP: rga_remove.scxml:20 :: _forge_body"]
// SCE-MAP: rga_remove.scxml:20 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function, no instance state. `#![no_std]`-clean when no `bytes`
// parameter (this fixture: no_std_clean = true).

use super::rga_gone;

use sce_portable_bytes::SceOwnedList;

#[allow(clippy::all)]
#[allow(unused_assignments)]
pub fn rga_remove(
    gone: &[rga_gone::RgaGonePayload],
    removal: rga_gone::RgaGonePayload,
    out: &mut SceOwnedList<rga_gone::RgaGonePayload, 256>,
) -> Result<(), sce_forge_runtime::algorithm::AlgorithmError> {
    out.clear();
    out.reserve(256);
    let mut held: bool = false;
    for &g in gone.iter() {
        out.push(g)?;
        if g.key == removal.key {
            held = true;
        }
    }
    if !held {
        out.push(removal)?;
    }
    return Ok(());
}
