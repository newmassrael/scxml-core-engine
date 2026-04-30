// SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package crossfile_validator_interpolation

import (
	"example.com/sce-forge/interpolation_1d_linear"
)

// ValidationResult holds the outcome of a validation check.
type ValidationResult struct {
	Valid  bool
	Reason string
}

// CrossfileValidatorInterpolation performs range, rate-of-change, and plausibility validation.
type CrossfileValidatorInterpolation struct {
	// Imported kinds (cross-file composition)
}

// Validate checks all validation rules and returns the result.
func (v *CrossfileValidatorInterpolation) Validate(rpm uint16) ValidationResult {
	if rpm < 500 || rpm > 7000 {
		return ValidationResult{Valid: false, Reason: "rpm_out_of_range"}
	}
	if !(interpolation_1d_linear.Lookup(rpm) > 200.0) {
		return ValidationResult{Valid: false, Reason: "plausibility_failed"}
	}
	return ValidationResult{Valid: true, Reason: ""}
}
