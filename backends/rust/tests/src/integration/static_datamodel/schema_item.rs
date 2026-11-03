// SCE-GENERATED — DO NOT EDIT
// source-hash: d947250ac8dbe681be6c82964fb8feadd2bb3cbd6b46adffc9d553a7fc0e7ce6
#![doc = "SCE-MAP: schema_item.scxml:9 :: _forge_body"]
// SCE-MAP: schema_item.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: item.added
// Do not edit — regenerate from the source SCXML file.

use super::enum_item_status;

/// The event this schema types (`sce:event-name`), as a host names it.
#[allow(dead_code)]
pub const EVENT_NAME: &str = "item.added";
/// The revision of the schema (`sce:revision`, 1 when the document declares none).
#[allow(dead_code)]
pub const REVISION: u32 = 1;
/// The most bytes the event's `_event.data` JSON text takes: every list full, every
/// text at its bound, every number at its longest spelling.
#[allow(dead_code)]
pub const DATA_MAX_LEN: u64 = 40;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SchemaItemPayload {
    pub id: u32,
    pub status: enum_item_status::EnumItemStatus,
}
