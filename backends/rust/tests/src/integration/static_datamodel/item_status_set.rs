// SCE-GENERATED — DO NOT EDIT
// source-hash: 70f0d81e5e7371651dd76e3fbe2c4947104cadff25fef82829c73b390a84662d
#![doc = "SCE-MAP: algorithm_item_status_set.scxml:9 :: _forge_body"]
// SCE-MAP: algorithm_item_status_set.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function, no instance state. `#![no_std]`-clean when no `bytes`
// parameter (this fixture: no_std_clean = true).

use super::enum_item_status;
use super::schema_item;

use sce_portable_bytes::SceOwnedList;

#[allow(clippy::all)]
#[allow(unused_assignments)]
pub fn item_status_set(
    items: &[schema_item::SchemaItemPayload],
    id: u32,
    status: enum_item_status::EnumItemStatus,
) -> Result<
    SceOwnedList<schema_item::SchemaItemPayload, 4>,
    sce_forge_runtime::algorithm::AlgorithmError,
> {
    let moved = schema_item::SchemaItemPayload {
        id: id,
        status: status,
    };
    let mut out: SceOwnedList<schema_item::SchemaItemPayload, 4> = SceOwnedList::new();
    for &e in items.iter() {
        if e.id == id {
            out.push(moved)?;
        } else {
            out.push(e)?;
        }
    }
    return Ok(out);
}
