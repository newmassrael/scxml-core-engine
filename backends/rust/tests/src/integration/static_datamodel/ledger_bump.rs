// SCE-GENERATED — DO NOT EDIT
// source-hash: 1bdfca3db4bb33c8c293390903822d652850c4718faefdf3c98b4d7553b2c7ee
#![doc = "SCE-MAP: algorithm_ledger_bump.scxml:7 :: _forge_body"]
// SCE-MAP: algorithm_ledger_bump.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function, no instance state. `#![no_std]`-clean when no `bytes`
// parameter (this fixture: no_std_clean = true).

use super::schema_total;

#[allow(clippy::all)]
#[allow(unused_assignments)]
pub fn ledger_bump(
    total: schema_total::SchemaTotalPayload,
    delta: i32,
) -> Result<schema_total::SchemaTotalPayload, sce_forge_runtime::algorithm::AlgorithmError> {
    let next = schema_total::SchemaTotalPayload {
        count: sce_forge_runtime::algorithm::add::<u32>(total.count, 1)?,
        sum: sce_forge_runtime::algorithm::add::<i32>(total.sum, delta)?,
    };
    return Ok(next);
}
