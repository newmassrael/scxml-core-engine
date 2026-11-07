// SCE-GENERATED — DO NOT EDIT
// source-hash: f114630dfb8a0ecf4088d08c68a678a96fcb72b20acb2b143f2f7cceae8a03de
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
