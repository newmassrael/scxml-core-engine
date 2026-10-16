// SCE-GENERATED — DO NOT EDIT
// source-hash: 4dd04931c1189858a076eb34dff5d0ac7c294ad18fa5b2c6b71a2b3fe53f7d49
#![doc = "SCE-MAP: algorithm_ledger_total.scxml:7 :: _forge_body"]
// SCE-MAP: algorithm_ledger_total.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function, no instance state. `#![no_std]`-clean when no `bytes`
// parameter (this fixture: no_std_clean = true).

use super::schema_entry;
use super::schema_total;

#[allow(clippy::all)]
#[allow(unused_assignments)]
pub fn ledger_total(
    entries: &[schema_entry::SchemaEntryPayload],
) -> Result<schema_total::SchemaTotalPayload, sce_forge_runtime::algorithm::AlgorithmError> {
    let mut t = schema_total::SchemaTotalPayload { count: 0, sum: 0 };
    for &e in entries.iter() {
        t.count = sce_forge_runtime::algorithm::add::<u32>(t.count, 1)?;
        t.sum = sce_forge_runtime::algorithm::add::<i32>(t.sum, e.amount)?;
    }
    return Ok(t);
}
