// SCE-GENERATED — DO NOT EDIT
// source-hash: fb4f42a34e7ac40a28d8e5daee5d633c3c45dddac513816edf2bb83fdc85f91c
// SCE-MAP: rga_gone.scxml:14 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: merge.rga_gone
// Do not edit — regenerate from the source SCXML file.

package rga_gone

// EventName is the event this schema types (`sce:event-name`), as a host names it.
const EventName string = "merge.rga_gone"

// Revision is the revision of the schema (`sce:revision`, 1 when the document declares none).
const Revision uint32 = 1

// DataMaxLen is the most bytes the event's `_event.data` JSON text takes: every list
// full, every text at its bound, every number at its longest spelling.
const DataMaxLen uint64 = 58

type RgaGonePayload struct {
	Key uint64
	By uint64
}
