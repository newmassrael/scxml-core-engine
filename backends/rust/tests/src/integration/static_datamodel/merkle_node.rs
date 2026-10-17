// SCE-GENERATED — DO NOT EDIT
// source-hash: 9ad3991e659415a87e89be4422403fddaea9105e2a522b988eeacca0944ca255
#![doc = "SCE-MAP: merkle_node.scxml:32 :: _forge_body"]
// SCE-MAP: merkle_node.scxml:32 :: _forge_body

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
