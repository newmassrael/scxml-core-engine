// SCE Forge: Auto-generated from Extended SCXML (sce:kind="interpolation")
// Do not edit — regenerate from the source SCXML file.

package interpolation_2d_bilinear

import "github.com/newmassrael/sce-forge-runtime/interpolation"

var axisRpm = []float64{ 800.0, 1200.0, 2000.0, 3000.0 }
var axisLoad = []float64{ 10.0, 50.0, 100.0 }
var values = [][]float64{
	{ 2.1, 4.5, 7.0 },
	{ 2.5, 5.0, 8.0 },
	{ 3.0, 6.0, 9.5 },
	{ 3.5, 7.0, 11.0 },
}

func Lookup(rpm uint16, load uint8) float64 {
	return interpolation.Bilinear(
		axisRpm, axisLoad, values,
		float64(rpm), float64(load),
	)
}
