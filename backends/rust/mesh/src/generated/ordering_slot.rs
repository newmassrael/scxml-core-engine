// SCE-GENERATED — DO NOT EDIT
// source-hash: 58f5a17e479c571b0f1a95c9ced6b8f03b5d957325be03d16f15d59d99727b1b
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
