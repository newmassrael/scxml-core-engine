// SCE-GENERATED — DO NOT EDIT
// source-hash: adbae20e9ca5d62b09870f4a9e5e88119b90dbcf69963958bb021c42c5c2c6a4
// SCE-MAP: algorithm_item_pick_first.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `item_pick_first`, no instance state. `bytes`
// parameters lower to `[]byte` (RFC §synth-5-J-5 emitter table).

package item_pick_first

import (
	"github.com/newmassrael/sce-forge-runtime/optional"
	sce_schema_item_pick "github.com/newmassrael/sce-go-tests/integration/static_datamodel/schema_item_pick"
)

func ItemPickFirst(items []sce_schema_item_pick.SchemaItemPickPayload, least int32) optional.Of[sce_schema_item_pick.SchemaItemPickPayload] {
    var found optional.Of[sce_schema_item_pick.SchemaItemPickPayload] = optional.None[sce_schema_item_pick.SchemaItemPickPayload]()
    for _, it := range items {
        if (!found.IsPresent() && it.Count >= least) {
            found = optional.Some(it);
        }
    }
    return found
}
