// SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator")
// Do not edit — regenerate from the source SCXML file.

package validator_range_only

// ValidationResult holds the outcome of a validation check.
type ValidationResult struct {
	Valid  bool
	Reason string
}

// ValidatorRangeOnly performs range, rate-of-change, and plausibility validation.
type ValidatorRangeOnly struct {
}

// Validate checks all validation rules and returns the result.
func (v *ValidatorRangeOnly) Validate(temperature float64) ValidationResult {
	if temperature < -40.0 || temperature > 150.0 {
		return ValidationResult{Valid: false, Reason: "temperature_out_of_range"}
	}
	return ValidationResult{Valid: true, Reason: ""}
}
