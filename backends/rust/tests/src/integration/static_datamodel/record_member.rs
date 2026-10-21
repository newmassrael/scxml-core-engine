// SCE-GENERATED — DO NOT EDIT
// source-hash: 52dec3eb3cccdf24690d474eaf44f8ac67bafec1b1ed5060c73db2a6256056db
#![doc = "SCE-MAP: record_member.scxml:13 :: _forge_body"]
// SCE-MAP: record_member.scxml:13 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: record_list.member
// Do not edit — regenerate from the source SCXML file.

/// The event this schema types (`sce:event-name`), as a host names it.
#[allow(dead_code)]
pub const EVENT_NAME: &str = "record_list.member";
/// The revision of the schema (`sce:revision`, 1 when the document declares none).
#[allow(dead_code)]
pub const REVISION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RecordMemberPayload {
    pub node: u64,
    pub role: u8,
}
