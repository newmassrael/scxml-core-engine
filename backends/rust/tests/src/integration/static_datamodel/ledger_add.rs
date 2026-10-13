// SCE-GENERATED — DO NOT EDIT
// source-hash: 5d4270ae4c884e43e7a12f9a58bd5d012629cb9a3aef7240aa93f930de85d6c6
#![doc = "SCE-MAP: algorithm_ledger_add.scxml:8 :: _forge_body"]
// SCE-MAP: algorithm_ledger_add.scxml:8 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function, no instance state. `#![no_std]`-clean when no `bytes`
// parameter (this fixture: no_std_clean = true).

use super::schema_entry;

use sce_portable_bytes::SceOwnedList;

#[allow(clippy::all)]
#[allow(unused_assignments)]
pub fn ledger_add(
    entries: &[schema_entry::SchemaEntryPayload],
    id: u32,
    amount: i32,
) -> Result<
    SceOwnedList<schema_entry::SchemaEntryPayload, 8>,
    sce_forge_runtime::algorithm::AlgorithmError,
> {
    let fresh = schema_entry::SchemaEntryPayload {
        id: id,
        amount: amount,
    };
    let mut out: SceOwnedList<schema_entry::SchemaEntryPayload, 8> = SceOwnedList::new();
    let mut seen: bool = false;
    for &e in entries.iter() {
        if e.id == id {
            seen = true;
            out.push(fresh)?;
        } else {
            out.push(e)?;
        }
    }
    if !seen {
        out.push(fresh)?;
    }
    return Ok(out);
}
