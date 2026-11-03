// SCE-GENERATED — DO NOT EDIT
// source-hash: d445777bd638d59bb235766496907e1278e8d6187af708d676fb6e02145c0dea
#![doc = "SCE-MAP: envelope_id.scxml:18 :: _forge_body"]
// SCE-MAP: envelope_id.scxml:18 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: mesh.envelope_id
// Do not edit — regenerate from the source SCXML file.

/// The event this schema types (`sce:event-name`), as a host names it.
#[allow(dead_code)]
pub const EVENT_NAME: &str = "mesh.envelope_id";
/// The revision of the schema (`sce:revision`, 1 when the document declares none).
#[allow(dead_code)]
pub const REVISION: u32 = 1;
/// The most bytes the event's `_event.data` JSON text takes: every list full, every
/// text at its bound, every number at its longest spelling.
#[allow(dead_code)]
pub const DATA_MAX_LEN: u64 = 57;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnvelopeIdPayload {
    pub hi: u64,
    pub lo: u64,
}
