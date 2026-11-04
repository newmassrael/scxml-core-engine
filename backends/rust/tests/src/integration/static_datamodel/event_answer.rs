// SCE-GENERATED — DO NOT EDIT
// source-hash: c4657e911a812ec796ce0b3c5be3eeef2b66326118a66c11d56342e555c6a85c
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
/// The most bytes the event's `_event.data` JSON text takes: every list full, every
/// text at its bound, every number at its longest spelling.
#[allow(dead_code)]
pub const DATA_MAX_LEN: u64 = 51;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EventAnswerPayload {
    pub sender: u64,
    pub hash: u32,
}
