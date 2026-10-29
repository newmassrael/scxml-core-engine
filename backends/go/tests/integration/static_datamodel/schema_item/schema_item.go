// SCE-GENERATED — DO NOT EDIT
// source-hash: 75c0c87af90d82dec15f516a721dd486d47e1280589aefe06f15efdef5f4f850
// SCE-MAP: schema_item.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: item.added
// Do not edit — regenerate from the source SCXML file.

package schema_item
import (
	sce_enum_item_status "github.com/newmassrael/sce-go-tests/integration/static_datamodel/enum_item_status"
)

// EventName is the event this schema types (`sce:event-name`), as a host names it.
const EventName string = "item.added"

// Revision is the revision of the schema (`sce:revision`, 1 when the document declares none).
const Revision uint32 = 1

type SchemaItemPayload struct {
	Id uint32
	Status sce_enum_item_status.EnumItemStatus
}
