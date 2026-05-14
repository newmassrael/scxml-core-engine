// SCE-MAP: transform_multi_output:3

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="transform")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package transform_multi_output

// ComputeFahrenheit computes the fahrenheit output.
func ComputeFahrenheit(celsius float64) float64 {
	return celsius * 9.0 / 5.0 + 32.0
}

// ComputeKelvin computes the kelvin output.
func ComputeKelvin(celsius float64) float64 {
	return celsius + 273.15
}
