// SCE-GENERATED — DO NOT EDIT
// source-hash: a45b1edeebf5addf07d55990d33be593d588441249c6330a80971f056631ac90
#![doc = "SCE-MAP: orset_observed.scxml:15 :: _forge_body"]
// SCE-MAP: orset_observed.scxml:15 :: _forge_body

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
pub fn orset_observed(
    live: &[orset_entry::OrsetEntryPayload],
    element: u64,
) -> Result<
    SceOwnedList<orset_entry::OrsetEntryPayload, 256>,
    sce_forge_runtime::algorithm::AlgorithmError,
> {
    let mut out: SceOwnedList<orset_entry::OrsetEntryPayload, 256> = SceOwnedList::new();
    for &x in live.iter() {
        if x.element == element {
            out.push(x)?;
        }
    }
    return Ok(out);
}
