// SCE-GENERATED — DO NOT EDIT
// source-hash: c32a835d00dee2859b7af30a88ce4538c28fc16f7d6ecae66467e06268265dee
// source: gcra_admit.scxml
#![doc = "SCE-MAP: gcra_admit.scxml:35 :: _forge_body"]
// SCE-MAP: gcra_admit.scxml:35 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function, no instance state. `#![no_std]`-clean when no `bytes`
// parameter (this fixture: no_std_clean = true).

use super::gcra_decision;

#[allow(clippy::all)]
#[allow(unused_assignments)]
#[inline]
pub fn gcra_admit(
    tat: i64,
    now: i64,
    cost: u32,
    interval: i64,
    burst: u32,
) -> Result<gcra_decision::GcraDecisionPayload, sce_forge_runtime::algorithm::AlgorithmError> {
    if !(cost >= 1 && interval >= 1 && burst >= 1 && cost <= burst) {
        return Err(sce_forge_runtime::algorithm::AlgorithmError::Precondition);
    }
    let start: i64 = if tat > now { tat } else { now };
    let arrival: i64 = sce_forge_runtime::algorithm::add::<i64>(
        start,
        sce_forge_runtime::algorithm::mul::<i64>(cost as i64, interval)?,
    )?;
    let limit: i64 = sce_forge_runtime::algorithm::add::<i64>(
        now,
        sce_forge_runtime::algorithm::mul::<i64>(burst as i64, interval)?,
    )?;
    let decision = gcra_decision::GcraDecisionPayload {
        admitted: arrival <= limit,
        tat: if arrival <= limit { arrival } else { tat },
        retry_after: if arrival <= limit {
            0
        } else {
            sce_forge_runtime::algorithm::sub::<i64>(arrival, limit)?
        },
    };
    return Ok(decision);
}
