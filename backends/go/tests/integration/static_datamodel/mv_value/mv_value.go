// SCE-GENERATED — DO NOT EDIT
// source-hash: b3df842d016676bf8989832cb164de51027fadf6ca9e7b5543c7bac0c9a13d1a
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
