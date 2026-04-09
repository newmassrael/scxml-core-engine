// SCE Forge: Auto-generated from Extended SCXML (sce:kind="condition")
// Do not edit — regenerate from the source SCXML file.

package condition_threshold

// ConditionThreshold evaluates the condition.
func ConditionThreshold(coolantTemp float64, oilTemp float64, maxTemp float64) bool {
	return coolantTemp > maxTemp || oilTemp > maxTemp
}