// SCE-GENERATED — DO NOT EDIT
// source-hash: 75c0c87af90d82dec15f516a721dd486d47e1280589aefe06f15efdef5f4f850
// SCE-MAP: forge_call_code.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="lookup")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.

package forge_call_code

import (
	"github.com/newmassrael/sce-forge-runtime/lookup"
	"github.com/newmassrael/sce-forge-runtime/optional"
)

var sceKeys = [2]uint8{ 1, 2 }
var sceValues = [2]int32{ 100, 200 }

// LookupCode returns the value paired with gear, or the absent value on miss.
func LookupCode(gear uint8) optional.Of[int32] {
	return lookup.Find(sceKeys[:], sceValues[:], gear)
}
