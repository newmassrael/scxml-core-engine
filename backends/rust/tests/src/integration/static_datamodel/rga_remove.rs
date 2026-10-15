// SCE-GENERATED — DO NOT EDIT
// source-hash: a45b1edeebf5addf07d55990d33be593d588441249c6330a80971f056631ac90
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
) -> Result<SceOwnedList<rga_gone::RgaGonePayload, 256>, sce_forge_runtime::algorithm::AlgorithmError>
{
    let mut out: SceOwnedList<rga_gone::RgaGonePayload, 256> = SceOwnedList::new();
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
    return Ok(out);
}
