// SCE-GENERATED — DO NOT EDIT
// source-hash: 3e662d3bbddd658a2d3b6f6a012f7bf4d21db1a4e6f0de7d35c1bd71b309f5a0
#![doc = "SCE-MAP: algorithm_ledger_larger.scxml:7 :: _forge_body"]
// SCE-MAP: algorithm_ledger_larger.scxml:7 :: _forge_body

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
pub fn ledger_larger(
    a: schema_entry::SchemaEntryPayload,
    b: schema_entry::SchemaEntryPayload,
) -> schema_entry::SchemaEntryPayload {
    let mut out = schema_entry::SchemaEntryPayload {
        id: a.id,
        amount: a.amount,
    };
    if b.amount > a.amount {
        out.id = b.id;
        out.amount = b.amount;
    }
    return out;
}
