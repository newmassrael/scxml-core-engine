// SCE-MAP: lookup_severity_default:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="lookup")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.

package lookup_severity_default

import "github.com/newmassrael/sce-forge-runtime/lookup"

var keys = [5]int32{ 100, 200, 300, 400, 500 }
var values = [5]int32{ 1, 2, 3, 2, 4 }

// LookupSeverity returns the value paired with code, or 0 on miss.
func LookupSeverity(code int32) int32 {
	if v, ok := lookup.Lookup(keys[:], values[:], code); ok {
		return v
	}
	return 0
}
