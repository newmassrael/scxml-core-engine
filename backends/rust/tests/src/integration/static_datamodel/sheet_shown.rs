// SCE-GENERATED — DO NOT EDIT
// source-hash: a45b1edeebf5addf07d55990d33be593d588441249c6330a80971f056631ac90
#![doc = "SCE-MAP: sheet_shown.scxml:16 :: _forge_body"]
// SCE-MAP: sheet_shown.scxml:16 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: merge.sheet_shown
// Do not edit — regenerate from the source SCXML file.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SheetShownPayload {
    pub row: u64,
    pub col: u64,
    pub value: i64,
    pub fault: u8,
}
