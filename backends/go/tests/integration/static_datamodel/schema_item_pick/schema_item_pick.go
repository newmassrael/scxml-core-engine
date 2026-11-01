// SCE-GENERATED — DO NOT EDIT
// source-hash: 24f1c455257ea04f598b006a589c3dea642c1ea2e58b2dcf4b49b8cebf3f207e
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
