// SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package crossfile_validator_condition

import (
	"example.com/sce-forge/condition_threshold"
)

// ValidationResult holds the outcome of a validation check.
type ValidationResult struct {
	Valid  bool
	Reason string
}

// CrossfileValidatorCondition performs range, rate-of-change, and plausibility validation.
type CrossfileValidatorCondition struct {
	// Imported kinds (cross-file composition)
}

// Validate checks all validation rules and returns the result.
func (v *CrossfileValidatorCondition) Validate(coolantTemp float64, oilTemp float64, maxTemp float64) ValidationResult {
	if !(!condition_threshold.ConditionThreshold(coolantTemp, oilTemp, maxTemp)) {
		return ValidationResult{Valid: false, Reason: "plausibility_failed"}
	}
	return ValidationResult{Valid: true, Reason: ""}
}
