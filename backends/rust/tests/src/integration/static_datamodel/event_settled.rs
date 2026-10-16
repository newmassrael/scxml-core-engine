// SCE-GENERATED — DO NOT EDIT
// source-hash: 486a7ede17aec50321702f8e51d91329868b60553618b2452672e3f16a3437c4
#![doc = "SCE-MAP: event_settled.scxml:17 :: _forge_body"]
// SCE-MAP: event_settled.scxml:17 :: _forge_body

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
pub fn event_settled(
    adds: &[orset_entry::OrsetEntryPayload],
    tombstones: &[orset_entry::OrsetEntryPayload],
    before: i64,
) -> Result<u32, sce_forge_runtime::algorithm::AlgorithmError> {
    let mut h: u32 = 0;
    for &a in adds.iter() {
        if a.wall_time <= before {
            let mut named: bool = false;
            for &n in tombstones.iter() {
                if n.element == a.element
                    && n.wall_time == a.wall_time
                    && n.counter == a.counter
                    && n.node_id == a.node_id
                {
                    named = true;
                }
            }
            if !named {
                h = h ^ merkle_change_hash::merkle_change_hash(a, 0)?;
            }
        }
    }
    for &t in tombstones.iter() {
        if t.wall_time <= before {
            h = h ^ merkle_change_hash::merkle_change_hash(t, 1)?;
        }
    }
    return Ok(h);
}
