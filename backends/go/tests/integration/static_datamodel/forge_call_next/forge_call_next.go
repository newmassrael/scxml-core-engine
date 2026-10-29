// SCE-GENERATED — DO NOT EDIT
// source-hash: 75c0c87af90d82dec15f516a721dd486d47e1280589aefe06f15efdef5f4f850
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
