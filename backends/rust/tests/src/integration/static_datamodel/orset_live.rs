// SCE-GENERATED — DO NOT EDIT
// source-hash: c431fa9e45990e8b729e69a4bf81ba13f9cc4e8066ddecbd5ff6346ac56dd5f3
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
) -> Result<
    SceOwnedList<orset_entry::OrsetEntryPayload, 256>,
    sce_forge_runtime::algorithm::AlgorithmError,
> {
    let mut out: SceOwnedList<orset_entry::OrsetEntryPayload, 256> = SceOwnedList::new();
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
    return Ok(out);
}
