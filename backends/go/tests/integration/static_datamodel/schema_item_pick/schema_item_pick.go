// SCE-GENERATED — DO NOT EDIT
// source-hash: d947250ac8dbe681be6c82964fb8feadd2bb3cbd6b46adffc9d553a7fc0e7ce6
// SCE-MAP: schema_item_pick.scxml:10 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: item.picked
// Do not edit — regenerate from the source SCXML file.

package schema_item_pick

// EventName is the event this schema types (`sce:event-name`), as a host names it.
const EventName string = "item.picked"

// Revision is the revision of the schema (`sce:revision`, 1 when the document declares none).
const Revision uint32 = 1

// TitleMaxSize is the most UTF-8 bytes `title` holds (the bound its type writes), as this build resolved it.
const TitleMaxSize uint32 = 16

// DataMaxLen is the most bytes the event's `_event.data` JSON text takes: every list
// full, every text at its bound, every number at its longest spelling.
const DataMaxLen uint64 = 144

type SchemaItemPickPayload struct {
	Id uint32
	Title string
	Count int32
}
