// SCE-GENERATED — DO NOT EDIT
// source-hash: 23b50ff6771257bf06531b96d0a47eeaa5c39fb71939d9bd5e537100a9105206
// SCE-MAP: schema_item.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: item.added
// Do not edit — regenerate from the source SCXML file.

package schema_item
import (
	sce_enum_item_status "github.com/newmassrael/sce-go-tests/integration/static_datamodel/enum_item_status"
)

type SchemaItemPayload struct {
	Id uint32
	Status sce_enum_item_status.EnumItemStatus
}
