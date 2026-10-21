// SCE-GENERATED — DO NOT EDIT
// source-hash: 09d8fec62ddcc806d17c4e1da241b802dbe4f5317b3bd415a705b2ad99d82f3e
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
