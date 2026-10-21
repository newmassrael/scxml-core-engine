// SCE-GENERATED — DO NOT EDIT
// source-hash: f7347a8b98455a65e31ae2b3af12ef65d68a284242bd71a8102ab373827910fd
#![doc = "SCE-MAP: text_slot.scxml:18 :: _forge_body"]
// SCE-MAP: text_slot.scxml:18 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: text.slot
// Do not edit — regenerate from the source SCXML file.

/// The event this schema types (`sce:event-name`), as a host names it.
#[allow(dead_code)]
pub const EVENT_NAME: &str = "text.slot";
/// The revision of the schema (`sce:revision`, 1 when the document declares none).
#[allow(dead_code)]
pub const REVISION: u32 = 1;
/// The most UTF-8 bytes `text` holds (`sce:max-size`), as this build resolved it.
#[allow(dead_code)]
pub const TEXT_MAX_SIZE: u32 = 32;

#[derive(Debug, Clone, PartialEq)]
pub struct TextSlotPayload {
    pub slot: u32,
    pub version: i64,
    pub text: ::sce_portable_bytes::SceString<32>,
}
