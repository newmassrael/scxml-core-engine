// SCE-GENERATED — DO NOT EDIT
// source-hash: 9313ff05336b94eeb54552b43ab70c62d4e55585e0cedc6ece4c89ceb09077d5
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
