// SCE-MAP: crossfile_validator_lookup:7 :: _forge_body

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

// NewCrossfileValidatorLookup returns an initialized validator. Stateful imports
// whose Go zero-value is not a valid initial state (e.g. filter — its
// internal pointer to the runtime implementation is nil after zero-init
// and the first Update call would deref nil) opt in to an explicit
// factory call via `ImportContext::go_init_expr`. Codec imports leave
// their slot zero-initialized (the pure-data struct's natural empty
// state). Validators with no stateful imports get an empty literal,
// matching the legacy `var v X` zero-value path callers used before
// this constructor existed.
func NewCrossfileValidatorLookup() *CrossfileValidatorLookup {
	return &CrossfileValidatorLookup{
	}
}

// Validate checks all validation rules and returns the result.
func (p *CrossfileValidatorLookup) Validate(code int32) ValidationResult {
	if code < 0 || code > 1000 {
		return ValidationResult{Valid: false, Reason: "code_out_of_range"}
	}
	if !(lookup_severity_default.LookupSeverity(code) > 0) {
		return ValidationResult{Valid: false, Reason: "plausibility_failed"}
	}
	return ValidationResult{Valid: true, Reason: ""}
}
