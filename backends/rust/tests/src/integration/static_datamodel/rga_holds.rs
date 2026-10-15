// SCE-GENERATED — DO NOT EDIT
// source-hash: a45b1edeebf5addf07d55990d33be593d588441249c6330a80971f056631ac90
#![doc = "SCE-MAP: rga_holds.scxml:15 :: _forge_body"]
// SCE-MAP: rga_holds.scxml:15 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function, no instance state. `#![no_std]`-clean when no `bytes`
// parameter (this fixture: no_std_clean = true).

use super::rga_node;

#[allow(clippy::all)]
#[allow(unused_assignments)]
pub fn rga_holds(nodes: &[rga_node::RgaNodePayload], key: u64) -> bool {
    let mut found: bool = false;
    for &n in nodes.iter() {
        if n.key == key {
            found = true;
        }
    }
    return found;
}
