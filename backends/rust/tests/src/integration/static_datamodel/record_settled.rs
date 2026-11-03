// SCE-GENERATED — DO NOT EDIT
// source-hash: 26e5bf730b3c2b04bc64b3f3339bd11d68362c63297741a6a051afa4c6cd0d1b
#![doc = "SCE-MAP: record_settled.scxml:21 :: _forge_body"]
// SCE-MAP: record_settled.scxml:21 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function, no instance state. `#![no_std]`-clean when no `bytes`
// parameter (this fixture: no_std_clean = true).

use super::merkle_change_hash;
use super::orset_entry;

#[allow(clippy::all)]
#[allow(unused_assignments)]
pub fn record_settled(
    adds: &[orset_entry::OrsetEntryPayload],
    tombstones: &[orset_entry::OrsetEntryPayload],
    before: i64,
) -> Result<u32, sce_forge_runtime::algorithm::AlgorithmError> {
    let mut h: u32 = 0;
    for &a in adds.iter() {
        if a.wall_time <= before {
            h = h ^ merkle_change_hash::merkle_change_hash(a, 0)?;
        }
    }
    for &t in tombstones.iter() {
        if t.wall_time <= before {
            h = h ^ merkle_change_hash::merkle_change_hash(t, 1)?;
        }
    }
    return Ok(h);
}
