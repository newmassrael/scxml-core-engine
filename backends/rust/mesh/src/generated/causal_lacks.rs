// SCE-GENERATED — DO NOT EDIT
// source-hash: 687f4c0ae3532ecabd8fec6c87388a5c95d9c36571dc803cf9f1b82e7d19a7ab
#![doc = "SCE-MAP: causal_lacks.scxml:16 :: _forge_body"]
// SCE-MAP: causal_lacks.scxml:16 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function, no instance state. `#![no_std]`-clean when no `bytes`
// parameter (this fixture: no_std_clean = true).

use super::causal_entry;

#[allow(clippy::all)]
#[allow(unused_assignments)]
pub fn causal_lacks(
    vector: &[causal_entry::CausalEntryPayload],
    dot: causal_entry::CausalEntryPayload,
) -> bool {
    let mut seen: u64 = 0;
    for &entry in vector.iter() {
        if entry.hi == dot.hi && entry.lo == dot.lo {
            seen = entry.seq;
        }
    }
    return dot.seq > seen;
}
