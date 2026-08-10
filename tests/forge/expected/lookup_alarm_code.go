// SCE-MAP: lookup_alarm_code:6 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="lookup")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.

package lookup_alarm_code

import "github.com/newmassrael/sce-forge-runtime/lookup"

var keys = [5]int32{ 100, 200, 300, 400, 500 }
var values = [5]int32{ 1, 2, 3, 2, 4 }

// LookupSeverity returns the value paired with code, or (zero, false) on miss.
func LookupSeverity(code int32) (int32, bool) {
	return lookup.Lookup(keys[:], values[:], code)
}
