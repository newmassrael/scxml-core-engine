// SCE-GENERATED — DO NOT EDIT
// source-hash: 8b4828d778fc1da05444ac6a052eb44c820b7a80e1f4075fc52bc6e97e8a070a
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
