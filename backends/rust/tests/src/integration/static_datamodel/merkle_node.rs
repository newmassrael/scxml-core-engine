// SCE-GENERATED — DO NOT EDIT
// source-hash: a45b1edeebf5addf07d55990d33be593d588441249c6330a80971f056631ac90
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
