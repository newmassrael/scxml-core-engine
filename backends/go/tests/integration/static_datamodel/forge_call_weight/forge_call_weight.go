// SCE-GENERATED — DO NOT EDIT
// source-hash: 9ec1a747bc5501f93d66b3929221786d0a45ece3c23f323814ee50eb28f290d9
// SCE-MAP: forge_call_weight.scxml:8 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="lookup")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.

package forge_call_weight

import (
	"github.com/newmassrael/sce-forge-runtime/lookup"
	sce_forge_call_status "github.com/newmassrael/sce-go-tests/integration/static_datamodel/forge_call_status"
	sce_forge_call_level "github.com/newmassrael/sce-go-tests/integration/static_datamodel/forge_call_level"
)

var sceKeys = [2]sce_forge_call_status.ForgeCallStatus{ sce_forge_call_status.ForgeCallStatusOpen, sce_forge_call_status.ForgeCallStatusHeld }
var sceValues = [2]sce_forge_call_level.ForgeCallLevel{ sce_forge_call_level.ForgeCallLevelLight, sce_forge_call_level.ForgeCallLevelHeavy }

// LookupWeight returns the value paired with status, or (zero, false) on miss.
func LookupWeight(status sce_forge_call_status.ForgeCallStatus) (sce_forge_call_level.ForgeCallLevel, bool) {
	return lookup.Lookup(sceKeys[:], sceValues[:], status)
}
