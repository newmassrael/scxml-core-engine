// SCE-GENERATED — DO NOT EDIT
// source-hash: ded882be73ace6b7753d0c32fcf4aa3290f87b63ea6e27cd33861b6073a63373
// source: forge_call_ratio.scxml
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
