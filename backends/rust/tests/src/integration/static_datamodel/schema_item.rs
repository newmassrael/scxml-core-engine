// SCE-GENERATED — DO NOT EDIT
// source-hash: 70f0d81e5e7371651dd76e3fbe2c4947104cadff25fef82829c73b390a84662d
#![doc = "SCE-MAP: schema_item.scxml:9 :: _forge_body"]
// SCE-MAP: schema_item.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: item.added
// Do not edit — regenerate from the source SCXML file.

use super::enum_item_status;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SchemaItemPayload {
    pub id: u32,
    pub status: enum_item_status::EnumItemStatus,
}
