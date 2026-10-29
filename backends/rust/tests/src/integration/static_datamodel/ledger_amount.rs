// SCE-GENERATED — DO NOT EDIT
// source-hash: adbae20e9ca5d62b09870f4a9e5e88119b90dbcf69963958bb021c42c5c2c6a4
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
