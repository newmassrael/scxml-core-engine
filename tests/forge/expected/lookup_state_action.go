// SCE-MAP: lookup_state_action:5 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="lookup")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.

package lookup_state_action

import (
	"github.com/newmassrael/sce-forge-runtime/lookup"
	"github.com/newmassrael/sce-forge-runtime/optional"
)

var sceKeys = [4]int32{ 0, 1, 2, 3 }
var sceValues = [4]int32{ 10, 20, 30, 40 }

// LookupAction returns the value paired with state, or the absent value on miss.
func LookupAction(state int32) optional.Of[int32] {
	return lookup.Find(sceKeys[:], sceValues[:], state)
}
