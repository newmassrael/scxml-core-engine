// SCE-GENERATED — DO NOT EDIT
// source-hash: fb4f42a34e7ac40a28d8e5daee5d633c3c45dddac513816edf2bb83fdc85f91c
// SCE-MAP: mv_value.scxml:27 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: merge.mv_value
// Do not edit — regenerate from the source SCXML file.

package mv_value

// EventName is the event this schema types (`sce:event-name`), as a host names it.
const EventName string = "merge.mv_value"

// Revision is the revision of the schema (`sce:revision`, 1 when the document declares none).
const Revision uint32 = 1

// TextMaxSize is the most UTF-8 bytes `text` holds (`sce:max-size`, the bound `sce:limit="merge_text"` names), as this build resolved it.
const TextMaxSize uint32 = 96

type MvValuePayload struct {
	Id uint64
	Field uint64
	Value int64
	Text string
	WallTime int64
	Counter uint32
	NodeId uint64
}
