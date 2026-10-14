// SCE-GENERATED — DO NOT EDIT
// source-hash: 0a170bbc1ca3025d9cc4009c0838fff8e752b3322334de465b99571c709f4da7
#![doc = "SCE-MAP: hlc_timestamp.scxml:12 :: _forge_body"]
// SCE-MAP: hlc_timestamp.scxml:12 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: merge.hlc_timestamp
// Do not edit — regenerate from the source SCXML file.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HlcTimestampPayload {
    pub wall_time: i64,
    pub counter: u32,
    pub node_id: u64,
}
