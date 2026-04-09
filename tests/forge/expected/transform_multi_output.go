// SCE Forge: Auto-generated from Extended SCXML (sce:kind="transform")
// Do not edit — regenerate from the source SCXML file.

package transform_multi_output

// ComputeFahrenheit computes the fahrenheit output.
func ComputeFahrenheit(celsius float64) float64 {
	return celsius * 9 / 5 + 32
}

// ComputeKelvin computes the kelvin output.
func ComputeKelvin(celsius float64) float64 {
	return celsius + 273.15
}
