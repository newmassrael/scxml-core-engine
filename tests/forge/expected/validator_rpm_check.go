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

// Validate checks all validation rules and returns the result.
func (v *ValidatorRpmCheck) Validate(rpm uint16, engineState string) ValidationResult {
	if rpm < 0 || rpm > 8000 {
		return ValidationResult{Valid: false, Reason: "rpm_out_of_range"}
	}
	{
		var delta uint16
		if rpm > v.prevRpm {
			delta = rpm - v.prevRpm
		} else {
			delta = v.prevRpm - rpm
		}
		if delta > 500 {
			return ValidationResult{Valid: false, Reason: "rpm_rate_of_change_exceeded"}
		}
	}
	if !(rpm == 0 || engineState != "STOP") {
		return ValidationResult{Valid: false, Reason: "plausibility_failed"}
	}
	v.prevRpm = rpm
	return ValidationResult{Valid: true, Reason: ""}
}
