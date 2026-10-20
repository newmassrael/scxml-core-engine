// SCE-GENERATED — DO NOT EDIT
// source-hash: b3df842d016676bf8989832cb164de51027fadf6ca9e7b5543c7bac0c9a13d1a
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
