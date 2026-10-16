// SCE-GENERATED — DO NOT EDIT
// source-hash: b8448cd407b1027864e71ab2f9c10d7e6a28146f7cc75313b45ffc4e7287b0f4
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
    SceOwnedList<orset_entry::OrsetEntryPayload, 1024>,
    sce_forge_runtime::algorithm::AlgorithmError,
> {
    let mut out: SceOwnedList<orset_entry::OrsetEntryPayload, 1024> = SceOwnedList::new();
    for &x in live.iter() {
        if x.element == element {
            out.push(x)?;
        }
    }
    return Ok(out);
}
