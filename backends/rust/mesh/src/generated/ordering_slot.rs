// SCE-GENERATED — DO NOT EDIT
// source-hash: f3593d0da7c47c6e95770dd740ec6b7f10d9172b65629c75948b0c0e547691b7
#![doc = "SCE-MAP: ordering_slot.scxml:11 :: _forge_body"]
// SCE-MAP: ordering_slot.scxml:11 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: mesh.ordering_slot
// Do not edit — regenerate from the source SCXML file.

/// The event this schema types (`sce:event-name`), as a host names it.
#[allow(dead_code)]
pub const EVENT_NAME: &str = "mesh.ordering_slot";
/// The revision of the schema (`sce:revision`, 1 when the document declares none).
#[allow(dead_code)]
pub const REVISION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrderingSlotPayload {
    pub seq: u64,
    pub arrived_at_ms: i64,
}
