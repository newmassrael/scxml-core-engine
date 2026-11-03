// SCE-GENERATED — DO NOT EDIT
// source-hash: fb4f42a34e7ac40a28d8e5daee5d633c3c45dddac513816edf2bb83fdc85f91c
// SCE-MAP: merkle_node.scxml:32 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: merge.merkle_node
// Do not edit — regenerate from the source SCXML file.

package merkle_node

// EventName is the event this schema types (`sce:event-name`), as a host names it.
const EventName string = "merge.merkle_node"

// Revision is the revision of the schema (`sce:revision`, 1 when the document declares none).
const Revision uint32 = 1

type MerkleNodePayload struct {
	Depth uint8
	Prefix uint64
	Hash uint32
}
