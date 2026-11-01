// SCE-GENERATED — DO NOT EDIT
// source-hash: 24f1c455257ea04f598b006a589c3dea642c1ea2e58b2dcf4b49b8cebf3f207e
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
