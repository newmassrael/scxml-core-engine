// SCE-GENERATED — DO NOT EDIT
// source-hash: 1178ba59796ca1f7c46f8569fd4bfda85c2b97db65f51728f3615d990d97dd1c
// SCE-MAP: forge_call_limit.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="interpolation")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.

package forge_call_limit

import "github.com/newmassrael/sce-forge-runtime/interpolation"

var sceAxisRpm = []float64{ 0.0, 1000.0, 2000.0 }
var sceValues = []float64{ 100.0, 150.0, 120.0 }

func Lookup(rpm uint16) float64 {
	return interpolation.Linear(
		sceAxisRpm, sceValues,
		float64(rpm),
	)
}
