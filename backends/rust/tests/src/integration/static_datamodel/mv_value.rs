// SCE-GENERATED — DO NOT EDIT
// source-hash: fb4f42a34e7ac40a28d8e5daee5d633c3c45dddac513816edf2bb83fdc85f91c
#![doc = "SCE-MAP: mv_value.scxml:27 :: _forge_body"]
// SCE-MAP: mv_value.scxml:27 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: merge.mv_value
// Do not edit — regenerate from the source SCXML file.

/// The event this schema types (`sce:event-name`), as a host names it.
#[allow(dead_code)]
pub const EVENT_NAME: &str = "merge.mv_value";
/// The revision of the schema (`sce:revision`, 1 when the document declares none).
#[allow(dead_code)]
pub const REVISION: u32 = 1;
/// The most UTF-8 bytes `text` holds (the bound its type writes, named `merge_text`), as this build resolved it.
#[allow(dead_code)]
pub const TEXT_MAX_SIZE: u32 = 96;
/// The most bytes the event's `_event.data` JSON text takes: every list full, every
/// text at its bound, every number at its longest spelling.
#[allow(dead_code)]
pub const DATA_MAX_LEN: u64 = 764;

#[derive(Debug, Clone, PartialEq)]
pub struct MvValuePayload {
    pub id: u64,
    pub field: u64,
    pub value: i64,
    pub text: ::sce_portable_bytes::SceString<96>,
    pub wall_time: i64,
    pub counter: u32,
    pub node_id: u64,
}
