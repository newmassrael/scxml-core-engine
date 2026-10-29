// SCE-GENERATED — DO NOT EDIT
// source-hash: 75c0c87af90d82dec15f516a721dd486d47e1280589aefe06f15efdef5f4f850
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

// TitleMaxSize is the most UTF-8 bytes `title` holds (`sce:max-size`), as this build resolved it.
const TitleMaxSize uint32 = 16

type SchemaItemPickPayload struct {
	Id uint32
	Title string
	Count int32
}
