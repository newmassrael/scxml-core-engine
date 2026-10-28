// SCE-GENERATED — DO NOT EDIT
// source-hash: d11fb6db69c27fed25f47a2f22a5bf7fb78da524d08f0c299e9cc553d32a6375
// SCE-MAP: forge_call_next.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="transform")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package forge_call_next

import (
	scealgorithm "github.com/newmassrael/sce-forge-runtime/algorithm"
	sce_forge_call_status "github.com/newmassrael/sce-go-tests/integration/static_datamodel/forge_call_status"
)

// ComputeAfter computes the after output.
func ComputeAfter(status sce_forge_call_status.ForgeCallStatus) sce_forge_call_status.ForgeCallStatus {
	return scealgorithm.Choose(status == sce_forge_call_status.ForgeCallStatusOpen, sce_forge_call_status.ForgeCallStatusHeld, sce_forge_call_status.ForgeCallStatusDone)
}
