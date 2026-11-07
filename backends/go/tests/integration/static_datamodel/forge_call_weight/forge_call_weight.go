// SCE-GENERATED — DO NOT EDIT
// source-hash: f114630dfb8a0ecf4088d08c68a678a96fcb72b20acb2b143f2f7cceae8a03de
// SCE-MAP: forge_call_weight.scxml:8 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="lookup")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.

package forge_call_weight

import (
	"github.com/newmassrael/sce-forge-runtime/lookup"
	"github.com/newmassrael/sce-forge-runtime/optional"
	sce_forge_call_status "github.com/newmassrael/sce-go-tests/integration/static_datamodel/forge_call_status"
	sce_forge_call_level "github.com/newmassrael/sce-go-tests/integration/static_datamodel/forge_call_level"
)

var sceKeys = [2]sce_forge_call_status.ForgeCallStatus{ sce_forge_call_status.ForgeCallStatusOpen, sce_forge_call_status.ForgeCallStatusHeld }
var sceValues = [2]sce_forge_call_level.ForgeCallLevel{ sce_forge_call_level.ForgeCallLevelLight, sce_forge_call_level.ForgeCallLevelHeavy }

// LookupWeight returns the value paired with status, or the absent value on miss.
func LookupWeight(status sce_forge_call_status.ForgeCallStatus) optional.Of[sce_forge_call_level.ForgeCallLevel] {
	return lookup.Find(sceKeys[:], sceValues[:], status)
}
