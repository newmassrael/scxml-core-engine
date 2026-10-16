// SCE-GENERATED — DO NOT EDIT
// source-hash: b8448cd407b1027864e71ab2f9c10d7e6a28146f7cc75313b45ffc4e7287b0f4
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
