// SCE-GENERATED — DO NOT EDIT
// source-hash: b3df842d016676bf8989832cb164de51027fadf6ca9e7b5543c7bac0c9a13d1a
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
