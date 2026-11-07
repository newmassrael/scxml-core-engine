// SCE-GENERATED — DO NOT EDIT
// source-hash: b2921c03007062914a1e69cc4069c6b54c8d66f97313a50f3246eed3d071c790
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
    out: &mut SceOwnedList<schema_item::SchemaItemPayload, 4>,
) -> Result<(), sce_forge_runtime::algorithm::AlgorithmError> {
    let moved = schema_item::SchemaItemPayload {
        id: id,
        status: status,
    };
    out.clear();
    out.reserve(4);
    for &e in items.iter() {
        if e.id == id {
            out.push(moved)?;
        } else {
            out.push(e)?;
        }
    }
    return Ok(());
}
