// SCE-GENERATED — DO NOT EDIT
// source-hash: dcc96ea8c5321d36e69f038b98535f0793d14bdbecc7d72d9cf5831d9f96378c
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
