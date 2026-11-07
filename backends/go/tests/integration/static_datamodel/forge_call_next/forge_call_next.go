// SCE-GENERATED — DO NOT EDIT
// source-hash: 3e662d3bbddd658a2d3b6f6a012f7bf4d21db1a4e6f0de7d35c1bd71b309f5a0
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
