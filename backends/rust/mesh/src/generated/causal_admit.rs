// SCE-GENERATED — DO NOT EDIT
// source-hash: f2f8682669670c7aaedbccb3102dd71d7b187eac29a32d3d20509a5bc070c12e
#![doc = "SCE-MAP: causal_admit.scxml:25 :: _forge_body"]
// SCE-MAP: causal_admit.scxml:25 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function, no instance state. `#![no_std]`-clean when no `bytes`
// parameter (this fixture: no_std_clean = true).

use super::causal_admission;
use super::causal_entry;

#[allow(clippy::all)]
#[allow(unused_assignments)]
pub fn causal_admit(
    delivered: &[causal_entry::CausalEntryPayload],
    dot: causal_entry::CausalEntryPayload,
    context: &[causal_entry::CausalEntryPayload],
) -> Result<causal_admission::CausalAdmission, sce_forge_runtime::algorithm::AlgorithmError> {
    let mut seen: u64 = 0;
    for &mine in delivered.iter() {
        if mine.hi == dot.hi && mine.lo == dot.lo {
            seen = mine.seq;
        }
    }
    let mut covered: bool = true;
    let mut have: u64 = 0;
    for &needed in context.iter() {
        have = 0;
        for &held in delivered.iter() {
            if held.hi == needed.hi && held.lo == needed.lo {
                have = held.seq;
            }
        }
        if (needed.hi != dot.hi || needed.lo != dot.lo) && needed.seq > have {
            covered = false;
        }
    }
    let mut result: causal_admission::CausalAdmission = causal_admission::CausalAdmission::Hold;
    if dot.seq <= seen {
        result = causal_admission::CausalAdmission::Drop;
    }
    if dot.seq == sce_forge_runtime::algorithm::add::<u64>(seen, 1)? && covered {
        result = causal_admission::CausalAdmission::Deliver;
    }
    return Ok(result);
}
