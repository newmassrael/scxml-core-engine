// SCE-GENERATED — DO NOT EDIT
// source-hash: 451ed84f90f73b42f48909eca6f7a302d6cb0b852dd68c846f5e713406957876
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
    out: &mut SceOwnedList<orset_entry::OrsetEntryPayload, 1024>,
) -> Result<(), sce_forge_runtime::algorithm::AlgorithmError> {
    out.clear();
    out.reserve(1024);
    for &x in live.iter() {
        if x.element == element {
            out.push(x)?;
        }
    }
    return Ok(());
}
