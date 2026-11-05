// SCE-GENERATED — DO NOT EDIT
// source-hash: fb4f42a34e7ac40a28d8e5daee5d633c3c45dddac513816edf2bb83fdc85f91c
#![doc = "SCE-MAP: orset_live.scxml:31 :: _forge_body"]
// SCE-MAP: orset_live.scxml:31 :: _forge_body

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
pub fn orset_live(
    adds: &[orset_entry::OrsetEntryPayload],
    tombstones: &[orset_entry::OrsetEntryPayload],
    out: &mut SceOwnedList<orset_entry::OrsetEntryPayload, 1024>,
) -> Result<(), sce_forge_runtime::algorithm::AlgorithmError> {
    out.clear();
    out.reserve(1024);
    for &x in adds.iter() {
        let mut removed: bool = false;
        for &t in tombstones.iter() {
            if t.element == x.element
                && t.wall_time == x.wall_time
                && t.counter == x.counter
                && t.node_id == x.node_id
            {
                removed = true;
            }
        }
        if !removed {
            out.push(x)?;
        }
    }
    return Ok(());
}
