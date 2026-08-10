// SCE-MAP: crossfile_validator_interpolation:9 :: _forge_body

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

// NewCrossfileValidatorInterpolation returns an initialized validator. Stateful imports
// whose Go zero-value is not a valid initial state (e.g. filter — its
// internal pointer to the runtime implementation is nil after zero-init
// and the first Update call would deref nil) opt in to an explicit
// factory call via `ImportContext::go_init_expr`. Codec imports leave
// their slot zero-initialized (the pure-data struct's natural empty
// state). Validators with no stateful imports get an empty literal,
// matching the legacy `var v X` zero-value path callers used before
// this constructor existed.
func NewCrossfileValidatorInterpolation() *CrossfileValidatorInterpolation {
	return &CrossfileValidatorInterpolation{
	}
}

// Validate checks all validation rules and returns the result.
func (p *CrossfileValidatorInterpolation) Validate(rpm uint16) ValidationResult {
	if rpm < 500 || rpm > 7000 {
		return ValidationResult{Valid: false, Reason: "rpm_out_of_range"}
	}
	if !(interpolation_1d_linear.Lookup(rpm) > 200.0) {
		return ValidationResult{Valid: false, Reason: "plausibility_failed"}
	}
	return ValidationResult{Valid: true, Reason: ""}
}
