// SCE-GENERATED — DO NOT EDIT
// source-hash: b2921c03007062914a1e69cc4069c6b54c8d66f97313a50f3246eed3d071c790
// SCE-MAP: algorithm_item_status_of.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `item_status_of`, no instance state. `bytes`
// parameters lower to `[]byte` (RFC §synth-5-J-5 emitter table).

package item_status_of

import (
	sce_schema_item "github.com/newmassrael/sce-go-tests/integration/static_datamodel/schema_item"
	sce_enum_item_status "github.com/newmassrael/sce-go-tests/integration/static_datamodel/enum_item_status"
)

func ItemStatusOf(items []sce_schema_item.SchemaItemPayload, id uint32) sce_enum_item_status.EnumItemStatus {
    var found sce_enum_item_status.EnumItemStatus = sce_enum_item_status.EnumItemStatusOpen
    for _, e := range items {
        if (e.Id == id) {
            found = e.Status;
        }
    }
    return found
}
