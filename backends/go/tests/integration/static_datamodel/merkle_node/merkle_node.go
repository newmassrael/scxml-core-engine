// SCE-GENERATED — DO NOT EDIT
// source-hash: b3df842d016676bf8989832cb164de51027fadf6ca9e7b5543c7bac0c9a13d1a
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
