// SCE-GENERATED — DO NOT EDIT
// source-hash: b3df842d016676bf8989832cb164de51027fadf6ca9e7b5543c7bac0c9a13d1a
#![doc = "SCE-MAP: mv_value.scxml:27 :: _forge_body"]
// SCE-MAP: mv_value.scxml:27 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: merge.mv_value
// Do not edit — regenerate from the source SCXML file.

#[derive(Debug, Clone, PartialEq)]
pub struct MvValuePayload {
    pub id: u64,
    pub field: u64,
    pub value: i64,
    pub text: ::sce_portable_bytes::SceString<96>,
    pub wall_time: i64,
    pub counter: u32,
    pub node_id: u64,
}
