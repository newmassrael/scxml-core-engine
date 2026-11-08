// SCE-GENERATED — DO NOT EDIT
// source-hash: f2f8682669670c7aaedbccb3102dd71d7b187eac29a32d3d20509a5bc070c12e
#![doc = "SCE-MAP: causal_raise.scxml:21 :: _forge_body"]
// SCE-MAP: causal_raise.scxml:21 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function, no instance state. `#![no_std]`-clean when no `bytes`
// parameter (this fixture: no_std_clean = true).

use super::causal_entry;

use sce_portable_bytes::{CapacityExceeded, SceOwnedList};

#[allow(clippy::all)]
#[allow(unused_assignments)]
pub fn causal_raise(
    vector: &[causal_entry::CausalEntryPayload],
    other: &[causal_entry::CausalEntryPayload],
    out: &mut SceOwnedList<causal_entry::CausalEntryPayload, 64>,
) -> Result<(), CapacityExceeded> {
    out.clear();
    out.reserve(64);
    let mut top: u64 = 0;
    for &mine in vector.iter() {
        top = mine.seq;
        for &theirs in other.iter() {
            if theirs.hi == mine.hi && theirs.lo == mine.lo && theirs.seq > top {
                top = theirs.seq;
            }
        }
        let raised = causal_entry::CausalEntryPayload {
            hi: mine.hi,
            lo: mine.lo,
            seq: top,
        };
        out.push(raised)?;
    }
    let mut named: bool = false;
    for &added in other.iter() {
        named = false;
        for &known in vector.iter() {
            if known.hi == added.hi && known.lo == added.lo {
                named = true;
            }
        }
        if !named {
            out.push(added)?;
        }
    }
    return Ok(());
}
