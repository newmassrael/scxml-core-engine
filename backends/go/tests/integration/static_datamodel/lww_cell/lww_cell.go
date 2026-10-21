// SCE-GENERATED — DO NOT EDIT
// source-hash: b3df842d016676bf8989832cb164de51027fadf6ca9e7b5543c7bac0c9a13d1a
// SCE-MAP: lww_cell.scxml:37 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: merge.lww_cell
// Do not edit — regenerate from the source SCXML file.

package lww_cell

// EventName is the event this schema types (`sce:event-name`), as a host names it.
const EventName string = "merge.lww_cell"

// Revision is the revision of the schema (`sce:revision`, 1 when the document declares none).
const Revision uint32 = 1

type LwwCellPayload struct {
	Row uint64
	Col uint64
	Value int64
	WallTime int64
	Counter uint32
	NodeId uint64
	Op uint8
	ARef bool
	ARow uint64
	ACol uint64
	AVal int64
	BRef bool
	BRow uint64
	BCol uint64
	BVal int64
}
