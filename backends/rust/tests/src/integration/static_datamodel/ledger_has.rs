// SCE-GENERATED — DO NOT EDIT
// source-hash: 9c4f25c71477b8168828f3c8f9ccad85a4253ff428db93b5b34a11bbacc1faa1
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
