// SCE-GENERATED — DO NOT EDIT
// source-hash: 2ff8faab06d9203660f7aafa65ce8844767914ec1b20b0e6c7ae3897d6ed19f7
// source: forge_call_weight.scxml
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
