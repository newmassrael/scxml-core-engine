// SCE Forge: Auto-generated from Extended SCXML (sce:kind="condition")
// Do not edit — regenerate from the source SCXML file.

package condition_range

// ConditionRange evaluates the condition.
func ConditionRange(rpm uint32, minRpm uint32, maxRpm uint32) bool {
	return rpm >= minRpm && rpm <= maxRpm
}
