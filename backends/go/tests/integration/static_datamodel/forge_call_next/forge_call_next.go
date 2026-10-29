// SCE-GENERATED — DO NOT EDIT
// source-hash: 1178ba59796ca1f7c46f8569fd4bfda85c2b97db65f51728f3615d990d97dd1c
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
