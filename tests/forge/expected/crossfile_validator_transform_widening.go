// SCE-MAP: crossfile_validator_transform_widening:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package crossfile_validator_transform_widening

import (
	"example.com/sce-forge/transform_temperature"
)

// ValidationResult holds the outcome of a validation check.
type ValidationResult struct {
	Valid  bool
	Reason string
}

// CrossfileValidatorTransformWidening performs range, rate-of-change, and plausibility validation.
type CrossfileValidatorTransformWidening struct {
	// Imported kinds (cross-file composition)
}

// NewCrossfileValidatorTransformWidening returns an initialized validator. Stateful imports
// whose Go zero-value is not a valid initial state (e.g. filter — its
// internal pointer to the runtime implementation is nil after zero-init
// and the first Update call would deref nil) opt in to an explicit
// factory call via `ImportContext::go_init_expr`. Codec imports leave
// their slot zero-initialized (the pure-data struct's natural empty
// state). Validators with no stateful imports get an empty literal,
// matching the legacy `var v X` zero-value path callers used before
// this constructor existed.
func NewCrossfileValidatorTransformWidening() *CrossfileValidatorTransformWidening {
	return &CrossfileValidatorTransformWidening{
	}
}

// Validate checks all validation rules and returns the result.
func (p *CrossfileValidatorTransformWidening) Validate(rawByte uint8) ValidationResult {
	if rawByte > 200 {
		return ValidationResult{Valid: false, Reason: "raw_byte_out_of_range"}
	}
	if !(transform_temperature.ComputeTemperature(uint16(rawByte)) > transform_temperature.ComputeTemperature(0)) {
		return ValidationResult{Valid: false, Reason: "plausibility_failed"}
	}
	return ValidationResult{Valid: true, Reason: ""}
}
