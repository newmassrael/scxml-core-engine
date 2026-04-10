// SCE Forge: Auto-generated from Extended SCXML (sce:kind="interpolation")
// Do not edit — regenerate from the source SCXML file.

package interpolation_1d_linear

var axisRpm = []float64{ 800.0, 1200.0, 2000.0, 3000.0, 4000.0, 6000.0 }
var values = []float64{ 120.0, 145.0, 200.0, 230.0, 210.0, 180.0 }

func Lookup(rpm uint16) float64 {
	return linearInterpolate(
		axisRpm, values,
		float64(rpm))
}

func linearInterpolate(axis []float64, vals []float64, x float64) float64 {
	n := len(axis)
	if x <= axis[0] { return vals[0] }
	if x >= axis[n-1] { return vals[n-1] }
	for i := 0; i < n-1; i++ {
		if x <= axis[i+1] {
			t := (x - axis[i]) / (axis[i+1] - axis[i])
			return vals[i] + t*(vals[i+1]-vals[i])
		}
	}
	return vals[n-1]
}
