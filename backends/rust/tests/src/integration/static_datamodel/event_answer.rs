// SCE-GENERATED — DO NOT EDIT
// source-hash: 602cf8b2e076ca5de83736526fa544c7671ce4b8406bed6ee323cbe2df13499b
#![doc = "SCE-MAP: event_answer.scxml:8 :: _forge_body"]
// SCE-MAP: event_answer.scxml:8 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: calendar.answer
// Do not edit — regenerate from the source SCXML file.

/// The event this schema types (`sce:event-name`), as a host names it.
#[allow(dead_code)]
pub const EVENT_NAME: &str = "calendar.answer";
/// The revision of the schema (`sce:revision`, 1 when the document declares none).
#[allow(dead_code)]
pub const REVISION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EventAnswerPayload {
    pub sender: u64,
    pub hash: u32,
}
