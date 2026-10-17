// SCE-GENERATED — DO NOT EDIT
// source-hash: 9ad3991e659415a87e89be4422403fddaea9105e2a522b988eeacca0944ca255
#![doc = "SCE-MAP: mv_value.scxml:24 :: _forge_body"]
// SCE-MAP: mv_value.scxml:24 :: _forge_body

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
