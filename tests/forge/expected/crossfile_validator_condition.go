// SCE-MAP: crossfile_validator_condition:3

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

// NewCrossfileValidatorCondition returns an initialized validator. Stateful imports
// whose Go zero-value is not a valid initial state (e.g. filter — its
// internal pointer to the runtime implementation is nil after zero-init
// and the first Update call would deref nil) opt in to an explicit
// factory call via `ImportContext::go_init_expr`. Codec imports leave
// their slot zero-initialized (the pure-data struct's natural empty
// state). Validators with no stateful imports get an empty literal,
// matching the legacy `var v X` zero-value path callers used before
// this constructor existed.
func NewCrossfileValidatorCondition() *CrossfileValidatorCondition {
	return &CrossfileValidatorCondition{
	}
}

// Validate checks all validation rules and returns the result.
func (p *CrossfileValidatorCondition) Validate(coolantTemp float64, oilTemp float64, maxTemp float64) ValidationResult {
	if !(!condition_threshold.ConditionThreshold(coolantTemp, oilTemp, maxTemp)) {
		return ValidationResult{Valid: false, Reason: "plausibility_failed"}
	}
	return ValidationResult{Valid: true, Reason: ""}
}
