// SCE-GENERATED — DO NOT EDIT
// source-hash: 3590e0da727b5c60e46e46571a637da3e7beb956941ee2491fcd012174a020af
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnvelopeIdPayload {
    pub hi: u64,
    pub lo: u64,
}
