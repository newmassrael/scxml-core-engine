// SCE-GENERATED — DO NOT EDIT
// source-hash: 9aa28cb8a6152625fb0eb1560e8ab3014da5864745e85b6f3b6e37672c04dcf6
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
