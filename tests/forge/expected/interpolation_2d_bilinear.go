// SCE Forge: Auto-generated from Extended SCXML (sce:kind="interpolation")
// Do not edit — regenerate from the source SCXML file.

package interpolation_2d_bilinear

var axisRpm = []float64{ 800.0, 1200.0, 2000.0, 3000.0 }
var axisLoad = []float64{ 10.0, 50.0, 100.0 }
var values = [][]float64{
	{ 2.1, 4.5, 7.0 },
	{ 2.5, 5.0, 8.0 },
	{ 3.0, 6.0, 9.5 },
	{ 3.5, 7.0, 11.0 },
}

func Lookup(rpm uint16, load uint8) float64 {
	return bilinearInterpolate(
		axisRpm, axisLoad, values,
		float64(rpm), float64(load))
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

func bilinearInterpolate(axisX, axisY []float64, table [][]float64, xIn, yIn float64) float64 {
	x := xIn; y := yIn
	if x <= axisX[0] { x = axisX[0] } else if x >= axisX[len(axisX)-1] { x = axisX[len(axisX)-1] }
	if y <= axisY[0] { y = axisY[0] } else if y >= axisY[len(axisY)-1] { y = axisY[len(axisY)-1] }
	ix := 0; iy := 0
	for i := 0; i < len(axisX)-1; i++ { if x <= axisX[i+1] { ix = i; break }; ix = i }
	for i := 0; i < len(axisY)-1; i++ { if y <= axisY[i+1] { iy = i; break }; iy = i }
	tx := (x - axisX[ix]) / (axisX[ix+1] - axisX[ix])
	ty := (y - axisY[iy]) / (axisY[iy+1] - axisY[iy])
	a := table[ix][iy] + tx*(table[ix+1][iy]-table[ix][iy])
	b := table[ix][iy+1] + tx*(table[ix+1][iy+1]-table[ix][iy+1])
	return a + ty*(b-a)
}
