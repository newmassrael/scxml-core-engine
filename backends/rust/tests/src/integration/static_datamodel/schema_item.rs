// SCE-GENERATED — DO NOT EDIT
// source-hash: ca67d4fa2cf4dbcd6617a3b32561c9b8257bc03aaa65978435303ccf35bae7b3
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
