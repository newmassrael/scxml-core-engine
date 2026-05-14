// SCE-MAP: crossfile_validator_filter:14

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package crossfile_validator_filter

import (
	"example.com/sce-forge/filter_low_pass"
)

// ValidationResult holds the outcome of a validation check.
type ValidationResult struct {
	Valid  bool
	Reason string
}

// CrossfileValidatorFilter performs range, rate-of-change, and plausibility validation.
type CrossfileValidatorFilter struct {
	// Imported kinds (cross-file composition)
	Smoother filter_low_pass.FilterLowPass
}

// NewCrossfileValidatorFilter returns an initialized validator. Stateful imports
// whose Go zero-value is not a valid initial state (e.g. filter — its
// internal pointer to the runtime implementation is nil after zero-init
// and the first Update call would deref nil) opt in to an explicit
// factory call via `ImportContext::go_init_expr`. Codec imports leave
// their slot zero-initialized (the pure-data struct's natural empty
// state). Validators with no stateful imports get an empty literal,
// matching the legacy `var v X` zero-value path callers used before
// this constructor existed.
func NewCrossfileValidatorFilter() *CrossfileValidatorFilter {
	return &CrossfileValidatorFilter{
		Smoother: *filter_low_pass.NewFilterLowPass(),
	}
}

// Validate checks all validation rules and returns the result.
func (p *CrossfileValidatorFilter) Validate(rawSample float64, threshold float64) ValidationResult {
	if !(p.Smoother.Update(rawSample) < threshold) {
		return ValidationResult{Valid: false, Reason: "plausibility_failed"}
	}
	return ValidationResult{Valid: true, Reason: ""}
}
