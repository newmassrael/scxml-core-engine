// SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package validator_plausibility_only

// ValidationResult holds the outcome of a validation check.
type ValidationResult struct {
	Valid  bool
	Reason string
}

// ValidatorPlausibilityOnly performs range, rate-of-change, and plausibility validation.
type ValidatorPlausibilityOnly struct {
}

// NewValidatorPlausibilityOnly returns an initialized validator. Stateful imports
// whose Go zero-value is not a valid initial state (e.g. filter — its
// internal pointer to the runtime implementation is nil after zero-init
// and the first Update call would deref nil) opt in to an explicit
// factory call via `ImportContext::go_init_expr`. Codec imports leave
// their slot zero-initialized (the pure-data struct's natural empty
// state). Validators with no stateful imports get an empty literal,
// matching the legacy `var v X` zero-value path callers used before
// this constructor existed.
func NewValidatorPlausibilityOnly() *ValidatorPlausibilityOnly {
	return &ValidatorPlausibilityOnly{
	}
}

// Validate checks all validation rules and returns the result.
func (p *ValidatorPlausibilityOnly) Validate(voltage float64, current float64) ValidationResult {
	if !(voltage * current <= 1000.0) {
		return ValidationResult{Valid: false, Reason: "plausibility_failed"}
	}
	return ValidationResult{Valid: true, Reason: ""}
}
