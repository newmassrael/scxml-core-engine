// SCE-GENERATED — DO NOT EDIT
// source-hash: 75c0c87af90d82dec15f516a721dd486d47e1280589aefe06f15efdef5f4f850
#![doc = "SCE-MAP: algorithm_item_status_count.scxml:8 :: _forge_body"]
// SCE-MAP: algorithm_item_status_count.scxml:8 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function, no instance state. `#![no_std]`-clean when no `bytes`
// parameter (this fixture: no_std_clean = true).

use super::enum_item_status;
use super::schema_item;

#[allow(clippy::all)]
#[allow(unused_assignments)]
pub fn item_status_count(
    items: &[schema_item::SchemaItemPayload],
    status: enum_item_status::EnumItemStatus,
) -> Result<u32, sce_forge_runtime::algorithm::AlgorithmError> {
    let mut count: u32 = 0;
    for &e in items.iter() {
        if e.status == status {
            count = sce_forge_runtime::algorithm::add::<u32>(count, 1)?;
        }
    }
    return Ok(count);
}
