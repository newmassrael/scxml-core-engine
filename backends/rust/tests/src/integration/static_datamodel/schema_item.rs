// SCE-GENERATED — DO NOT EDIT
// source-hash: 438f3c44a0f49109d71c51cacca59b20d771e2147409672affa707c8d139f3c1
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
