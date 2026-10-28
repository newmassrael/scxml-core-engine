// SCE-GENERATED — DO NOT EDIT
// source-hash: d11fb6db69c27fed25f47a2f22a5bf7fb78da524d08f0c299e9cc553d32a6375
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
/// The most UTF-8 bytes `title` holds (`sce:max-size`), as this build resolved it.
#[allow(dead_code)]
pub const TITLE_MAX_SIZE: u32 = 16;

#[derive(Debug, Clone, PartialEq)]
pub struct SchemaItemPickPayload {
    pub id: u32,
    pub title: ::sce_portable_bytes::SceString<16>,
    pub count: i32,
}
