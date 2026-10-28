// SCE-GENERATED — DO NOT EDIT
// source-hash: b67ac92c47dabab09bc66be64b0a4b4a6acfabe17f95412102ed640a717fc04f
#![doc = "SCE-MAP: algorithm_item_status_of.scxml:9 :: _forge_body"]
// SCE-MAP: algorithm_item_status_of.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function, no instance state. `#![no_std]`-clean when no `bytes`
// parameter (this fixture: no_std_clean = true).

use super::enum_item_status;
use super::schema_item;

#[allow(clippy::all)]
#[allow(unused_assignments)]
pub fn item_status_of(
    items: &[schema_item::SchemaItemPayload],
    id: u32,
) -> enum_item_status::EnumItemStatus {
    let mut found: enum_item_status::EnumItemStatus = enum_item_status::EnumItemStatus::Open;
    for &e in items.iter() {
        if e.id == id {
            found = e.status;
        }
    }
    return found;
}
