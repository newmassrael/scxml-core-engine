// SCE-MAP: interpolation_2d_bilinear:1 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="interpolation")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.

package interpolation_2d_bilinear

import "github.com/newmassrael/sce-forge-runtime/interpolation"

var sceAxisRpm = []float64{ 800.0, 1200.0, 2000.0, 3000.0 }
var sceAxisLoad = []float64{ 10.0, 50.0, 100.0 }
var sceValues = [][]float64{
	{ 2.1, 4.5, 7.0 },
	{ 2.5, 5.0, 8.0 },
	{ 3.0, 6.0, 9.5 },
	{ 3.5, 7.0, 11.0 },
}

func Lookup(rpm uint16, load uint8) float64 {
	return interpolation.Bilinear(
		sceAxisRpm, sceAxisLoad, sceValues,
		float64(rpm), float64(load),
	)
}
