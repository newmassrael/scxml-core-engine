// SCE-MAP: condition_programming:3

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="condition")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package condition_programming

// ConditionProgramming evaluates the condition.
func ConditionProgramming(engineStop bool, ignition bool) bool {
	return engineStop == true && ignition == true
}
