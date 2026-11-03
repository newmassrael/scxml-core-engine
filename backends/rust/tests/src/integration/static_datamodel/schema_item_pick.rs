// SCE-GENERATED — DO NOT EDIT
// source-hash: ef94c112b4026dfab705629195b17ee265cdc572de6e8fceff328262c2af0613
#![doc = "SCE-MAP: schema_item_pick.scxml:10 :: _forge_body"]
// SCE-MAP: schema_item_pick.scxml:10 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: item.picked
// Do not edit — regenerate from the source SCXML file.

/// The event this schema types (`sce:event-name`), as a host names it.
#[allow(dead_code)]
pub const EVENT_NAME: &str = "item.picked";
/// The revision of the schema (`sce:revision`, 1 when the document declares none).
#[allow(dead_code)]
pub const REVISION: u32 = 1;
/// The most UTF-8 bytes `title` holds (the bound its type writes), as this build resolved it.
#[allow(dead_code)]
pub const TITLE_MAX_SIZE: u32 = 16;
/// The most bytes the event's `_event.data` JSON text takes: every list full, every
/// text at its bound, every number at its longest spelling.
#[allow(dead_code)]
pub const DATA_MAX_LEN: u64 = 144;

#[derive(Debug, Clone, PartialEq)]
pub struct SchemaItemPickPayload {
    pub id: u32,
    pub title: ::sce_portable_bytes::SceString<16>,
    pub count: i32,
}
