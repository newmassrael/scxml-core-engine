// SCE-MAP: lookup_unit_scale:6 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="lookup")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.

package lookup_unit_scale

import (
	"github.com/newmassrael/sce-forge-runtime/lookup"
	"github.com/newmassrael/sce-forge-runtime/optional"
)

var sceKeys = [6]int32{ 1, 2, 3, 4, 5, 6 }
var sceValues = [6]float64{ 0.001, 0.01, 0.1, 1.0, 10.0, 100.0 }

// LookupScale returns the value paired with unit, or the absent value on miss.
func LookupScale(unit int32) optional.Of[float64] {
	return lookup.Find(sceKeys[:], sceValues[:], unit)
}
