// SCE-GENERATED — DO NOT EDIT
// source-hash: b67ac92c47dabab09bc66be64b0a4b4a6acfabe17f95412102ed640a717fc04f
// SCE-MAP: forge_call_ratio.scxml:6 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="lookup")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.

package forge_call_ratio

import (
	"github.com/newmassrael/sce-forge-runtime/lookup"
)

var sceKeys = [3]uint8{ 1, 2, 3 }
var sceValues = [3]int32{ 35, 20, 14 }

// LookupRatio returns the value paired with gear, or 0 on miss.
func LookupRatio(gear uint8) int32 {
	if v, ok := lookup.Lookup(sceKeys[:], sceValues[:], gear); ok {
		return v
	}
	return 0
}
