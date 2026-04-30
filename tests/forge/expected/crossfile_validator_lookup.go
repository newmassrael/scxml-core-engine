// SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package crossfile_validator_lookup

import (
	"example.com/sce-forge/lookup_severity_default"
)

// ValidationResult holds the outcome of a validation check.
type ValidationResult struct {
	Valid  bool
	Reason string
}

// CrossfileValidatorLookup performs range, rate-of-change, and plausibility validation.
type CrossfileValidatorLookup struct {
	// Imported kinds (cross-file composition)
}

// Validate checks all validation rules and returns the result.
func (v *CrossfileValidatorLookup) Validate(code int32) ValidationResult {
	if code < 0 || code > 1000 {
		return ValidationResult{Valid: false, Reason: "code_out_of_range"}
	}
	if !(lookup_severity_default.LookupSeverity(code) > 0) {
		return ValidationResult{Valid: false, Reason: "plausibility_failed"}
	}
	return ValidationResult{Valid: true, Reason: ""}
}
