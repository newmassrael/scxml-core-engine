// SCE-GENERATED — DO NOT EDIT
// source-hash: 39c188440acd2a30e1c96c90e825a582fc7011a2c558c89e70ea674bb5bda40e
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
