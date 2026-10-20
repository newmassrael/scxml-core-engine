// SCE-GENERATED — DO NOT EDIT
// source-hash: 23b50ff6771257bf06531b96d0a47eeaa5c39fb71939d9bd5e537100a9105206
#![doc = "SCE-MAP: event_occurrence.scxml:12 :: _forge_body"]
// SCE-MAP: event_occurrence.scxml:12 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: calendar.occurrence
// Do not edit — regenerate from the source SCXML file.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EventOccurrencePayload {
    pub id: u64,
    pub start: i64,
    pub end: i64,
    pub nth: u32,
    pub conflict: u8,
}
