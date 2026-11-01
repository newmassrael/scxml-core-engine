// SCE-GENERATED — DO NOT EDIT
// source-hash: 24f1c455257ea04f598b006a589c3dea642c1ea2e58b2dcf4b49b8cebf3f207e
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
