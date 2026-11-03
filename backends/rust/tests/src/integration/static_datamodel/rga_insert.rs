// SCE-GENERATED — DO NOT EDIT
// source-hash: fb4f42a34e7ac40a28d8e5daee5d633c3c45dddac513816edf2bb83fdc85f91c
#![doc = "SCE-MAP: rga_insert.scxml:25 :: _forge_body"]
// SCE-MAP: rga_insert.scxml:25 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function, no instance state. `#![no_std]`-clean when no `bytes`
// parameter (this fixture: no_std_clean = true).

use super::rga_node;

use sce_portable_bytes::SceOwnedList;

#[allow(clippy::all)]
#[allow(unused_assignments)]
pub fn rga_insert(
    nodes: &[rga_node::RgaNodePayload],
    node: rga_node::RgaNodePayload,
    out: &mut SceOwnedList<rga_node::RgaNodePayload, 256>,
) -> Result<(), sce_forge_runtime::algorithm::AlgorithmError> {
    out.clear();
    let mut held: bool = false;
    for &probe in nodes.iter() {
        if probe.key == node.key {
            held = true;
        }
    }
    if held {
        for &same in nodes.iter() {
            out.push(same)?;
        }
        return Ok(());
    }
    let mut phase: u8 = if node.after == 0 { 1 } else { 0 };
    for &n in nodes.iter() {
        if phase == 0 {
            out.push(n)?;
            if n.key == node.after {
                phase = 1;
            }
        } else {
            if phase == 1 {
                if n.key > node.key {
                    out.push(n)?;
                } else {
                    out.push(node)?;
                    out.push(n)?;
                    phase = 2;
                }
            } else {
                out.push(n)?;
            }
        }
    }
    if phase != 2 {
        out.push(node)?;
    }
    return Ok(());
}
