// SCE-GENERATED — DO NOT EDIT
// source-hash: 0a170bbc1ca3025d9cc4009c0838fff8e752b3322334de465b99571c709f4da7
#![doc = "SCE-MAP: lww_cell.scxml:21 :: _forge_body"]
// SCE-MAP: lww_cell.scxml:21 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: merge.lww_cell
// Do not edit — regenerate from the source SCXML file.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LwwCellPayload {
    pub row: u64,
    pub col: u64,
    pub value: i64,
    pub wall_time: i64,
    pub counter: u32,
    pub node_id: u64,
}
