// SCE-GENERATED — DO NOT EDIT
// source-hash: b3df842d016676bf8989832cb164de51027fadf6ca9e7b5543c7bac0c9a13d1a
// SCE-MAP: hlc_timestamp.scxml:12 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: merge.hlc_timestamp
// Do not edit — regenerate from the source SCXML file.

package hlc_timestamp

// EventName is the event this schema types (`sce:event-name`), as a host names it.
const EventName string = "merge.hlc_timestamp"

// Revision is the revision of the schema (`sce:revision`, 1 when the document declares none).
const Revision uint32 = 1

type HlcTimestampPayload struct {
	WallTime int64
	Counter uint32
	NodeId uint64
}
