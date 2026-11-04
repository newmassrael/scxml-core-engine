// SCE-GENERATED — DO NOT EDIT
// source-hash: a959df7d40a6301aa78e8501513db4064b02c0155bca6d82b638d4f2f72d3815
#![doc = "SCE-MAP: algorithm_ledger_has.scxml:7 :: _forge_body"]
// SCE-MAP: algorithm_ledger_has.scxml:7 :: _forge_body

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
pub fn ledger_has(entries: &[schema_entry::SchemaEntryPayload], id: u32) -> bool {
    let mut found: bool = false;
    for &e in entries.iter() {
        if e.id == id {
            found = true;
        }
    }
    return found;
}
