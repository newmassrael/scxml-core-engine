// SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package validator_rpm_check

// ValidationResult holds the outcome of a validation check.
type ValidationResult struct {
	Valid  bool
	Reason string
}

// ValidatorRpmCheck performs range, rate-of-change, and plausibility validation.
type ValidatorRpmCheck struct {
	prevRpm uint16
}

// NewValidatorRpmCheck returns an initialized validator. Stateful imports
// whose Go zero-value is not a valid initial state (e.g. filter — its
// internal pointer to the runtime implementation is nil after zero-init
// and the first Update call would deref nil) opt in to an explicit
// factory call via `ImportContext::go_init_expr`. Codec imports leave
// their slot zero-initialized (the pure-data struct's natural empty
// state). Validators with no stateful imports get an empty literal,
// matching the legacy `var v X` zero-value path callers used before
// this constructor existed.
func NewValidatorRpmCheck() *ValidatorRpmCheck {
	return &ValidatorRpmCheck{
	}
}

// Validate checks all validation rules and returns the result.
func (p *ValidatorRpmCheck) Validate(rpm uint16, engineState string) ValidationResult {
	if rpm < 0 || rpm > 8000 {
		return ValidationResult{Valid: false, Reason: "rpm_out_of_range"}
	}
	{
		var delta uint16
		if rpm > p.prevRpm {
			delta = rpm - p.prevRpm
		} else {
			delta = p.prevRpm - rpm
		}
		if delta > 500 {
			return ValidationResult{Valid: false, Reason: "rpm_rate_of_change_exceeded"}
		}
	}
	if !(rpm == 0 || engineState != "STOP") {
		return ValidationResult{Valid: false, Reason: "plausibility_failed"}
	}
	p.prevRpm = rpm
	return ValidationResult{Valid: true, Reason: ""}
}
