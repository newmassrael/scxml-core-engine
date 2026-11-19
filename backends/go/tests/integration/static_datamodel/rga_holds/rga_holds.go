// SCE-GENERATED — DO NOT EDIT
// source-hash: 838c899db5708c0e2d2504fe81af92ed2fc772239976321217260b01890a5853
// source: sce:std/merge/rga_holds.scxml
// SCE-MAP: rga_holds.scxml:15 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `rga_holds`, no instance state. `bytes`
// parameters lower to `[]byte` (RFC §synth-5-J-5 emitter table).

package rga_holds

import (
	sce_rga_node "github.com/newmassrael/sce-go-tests/integration/static_datamodel/rga_node"
)

func RgaHolds(nodes []sce_rga_node.RgaNodePayload, key uint64) bool {
    var found bool = false
    for _, n := range nodes {
        if (n.Key == key) {
            found = true;
        }
    }
    return found
}
