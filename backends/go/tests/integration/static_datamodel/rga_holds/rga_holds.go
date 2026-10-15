// SCE-GENERATED — DO NOT EDIT
// source-hash: a45b1edeebf5addf07d55990d33be593d588441249c6330a80971f056631ac90
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
