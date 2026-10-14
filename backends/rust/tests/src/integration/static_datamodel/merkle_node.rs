// SCE-GENERATED — DO NOT EDIT
// source-hash: c431fa9e45990e8b729e69a4bf81ba13f9cc4e8066ddecbd5ff6346ac56dd5f3
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
