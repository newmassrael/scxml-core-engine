// SCE-GENERATED — DO NOT EDIT
// source-hash: 8149e47f56da0bbb92d7b1c4600956bbfd585c43526372f3618b2e600843296d
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
