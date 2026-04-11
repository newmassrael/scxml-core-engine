// SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator")
// Do not edit — regenerate from the source SCXML file.

package crossfile_validator_transform

import (
	"example.com/sce-forge/transform_temperature"
)

// ValidationResult holds the outcome of a validation check.
type ValidationResult struct {
	Valid  bool
	Reason string
}

// CrossfileValidatorTransform performs range, rate-of-change, and plausibility validation.
type CrossfileValidatorTransform struct {
	// Imported kinds (cross-file composition)
}

// Validate checks all validation rules and returns the result.
func (v *CrossfileValidatorTransform) Validate(rawTemp uint16) ValidationResult {
	if rawTemp < 0 || rawTemp > 4095 {
		return ValidationResult{Valid: false, Reason: "raw_temp_out_of_range"}
	}
	if !(transform_temperature.ComputeTemperature(rawTemp) > -40.0 && transform_temperature.ComputeTemperature(rawTemp) < 200.0) {
		return ValidationResult{Valid: false, Reason: "plausibility_failed"}
	}
	return ValidationResult{Valid: true, Reason: ""}
}
