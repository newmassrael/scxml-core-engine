// SCE-GENERATED — DO NOT EDIT
// source-hash: a92737f3147f50ca2cf6319055f29d7afe2c17766723037ae46a81e73e7a66df
#![doc = "SCE-MAP: algorithm_ledger_amount.scxml:6 :: _forge_body"]
// SCE-MAP: algorithm_ledger_amount.scxml:6 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function, no instance state. `#![no_std]`-clean when no `bytes`
// parameter (this fixture: no_std_clean = true).

use super::schema_entry;

#[allow(clippy::all)]
#[allow(unused_assignments)]
pub fn ledger_amount(line: schema_entry::SchemaEntryPayload) -> i32 {
    return line.amount;
}
