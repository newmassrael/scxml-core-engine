// SCE-GENERATED — DO NOT EDIT
// source-hash: b3df842d016676bf8989832cb164de51027fadf6ca9e7b5543c7bac0c9a13d1a
#![doc = "SCE-MAP: lww_cell.scxml:37 :: _forge_body"]
// SCE-MAP: lww_cell.scxml:37 :: _forge_body

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
    pub op: u8,
    pub a_ref: bool,
    pub a_row: u64,
    pub a_col: u64,
    pub a_val: i64,
    pub b_ref: bool,
    pub b_row: u64,
    pub b_col: u64,
    pub b_val: i64,
}
