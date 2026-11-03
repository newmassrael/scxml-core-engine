// SCE-GENERATED — DO NOT EDIT
// source-hash: 848898b5bbb016bf799fdf5a1235f8e512e7516e53f20e55bfd5887716ace728
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
/// The most UTF-8 bytes `text` holds (the bound its type writes), as this build resolved it.
#[allow(dead_code)]
pub const TEXT_MAX_SIZE: u32 = 32;
/// The most bytes the event's `_event.data` JSON text takes: every list full, every
/// text at its bound, every number at its longest spelling.
#[allow(dead_code)]
pub const DATA_MAX_LEN: u64 = 254;

#[derive(Debug, Clone, PartialEq)]
pub struct TextSlotPayload {
    pub slot: u32,
    pub version: i64,
    pub text: ::sce_portable_bytes::SceString<32>,
}
