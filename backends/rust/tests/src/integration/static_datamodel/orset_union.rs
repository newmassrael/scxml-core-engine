// SCE-GENERATED — DO NOT EDIT
// source-hash: fb4f42a34e7ac40a28d8e5daee5d633c3c45dddac513816edf2bb83fdc85f91c
#![doc = "SCE-MAP: orset_union.scxml:23 :: _forge_body"]
// SCE-MAP: orset_union.scxml:23 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function, no instance state. `#![no_std]`-clean when no `bytes`
// parameter (this fixture: no_std_clean = true).

use super::orset_entry;

use sce_portable_bytes::SceOwnedList;

#[allow(clippy::all)]
#[allow(unused_assignments)]
pub fn orset_union(
    a: &[orset_entry::OrsetEntryPayload],
    b: &[orset_entry::OrsetEntryPayload],
    out: &mut SceOwnedList<orset_entry::OrsetEntryPayload, 1024>,
) -> Result<(), sce_forge_runtime::algorithm::AlgorithmError> {
    out.clear();
    for &x in a.iter() {
        out.push(x)?;
    }
    for &y in b.iter() {
        let mut held: bool = false;
        for &z in a.iter() {
            if z.element == y.element
                && z.wall_time == y.wall_time
                && z.counter == y.counter
                && z.node_id == y.node_id
            {
                held = true;
            }
        }
        if !held {
            out.push(y)?;
        }
    }
    return Ok(());
}
