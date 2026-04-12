// SCE Forge: Auto-generated from Extended SCXML (sce:kind="lookup")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.

package lookup_state_action

import "github.com/newmassrael/sce-forge-runtime/lookup"

var keys = [4]int32{ 0, 1, 2, 3 }
var values = [4]int32{ 10, 20, 30, 40 }

// LookupAction returns the value paired with state, or (zero, false) on miss.
func LookupAction(state int32) (int32, bool) {
	return lookup.Lookup(keys[:], values[:], state)
}
