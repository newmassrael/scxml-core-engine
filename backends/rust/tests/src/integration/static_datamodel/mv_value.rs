// SCE-GENERATED — DO NOT EDIT
// source-hash: 58e52c40de99a046030dfe178fb69ffa932b0a6a8146307d7430d1e6de80a470
#![doc = "SCE-MAP: mv_value.scxml:22 :: _forge_body"]
// SCE-MAP: mv_value.scxml:22 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: merge.mv_value
// Do not edit — regenerate from the source SCXML file.

#[derive(Debug, Clone, PartialEq)]
pub struct MvValuePayload {
    pub id: u64,
    pub field: u64,
    pub value: i64,
    pub text: ::sce_portable_bytes::SceString<32>,
    pub wall_time: i64,
    pub counter: u32,
    pub node_id: u64,
}
