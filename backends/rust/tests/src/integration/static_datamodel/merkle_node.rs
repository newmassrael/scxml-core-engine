// SCE-GENERATED — DO NOT EDIT
// source-hash: 0022d2f7097a4a1eb0e7ba717e1da8948cc1ba80ab5d45ced6c3b314777dd2db
#![doc = "SCE-MAP: merkle_node.scxml:22 :: _forge_body"]
// SCE-MAP: merkle_node.scxml:22 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: merge.merkle_node
// Do not edit — regenerate from the source SCXML file.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MerkleNodePayload {
    pub depth: u8,
    pub prefix: u64,
    pub hash: u32,
}
