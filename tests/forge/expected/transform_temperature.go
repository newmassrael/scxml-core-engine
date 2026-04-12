// SCE Forge: Auto-generated from Extended SCXML (sce:kind="transform")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package transform_temperature

// ComputeTemperature computes the temperature output.
func ComputeTemperature(raw uint16) float64 {
	return float64(raw) * 0.1 - 40.0
}
