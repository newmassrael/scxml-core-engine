// SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package validator_signed_roc

import "math"

// ValidationResult holds the outcome of a validation check.
type ValidationResult struct {
	Valid  bool
	Reason string
}

// ValidatorSignedRoc performs range, rate-of-change, and plausibility validation.
type ValidatorSignedRoc struct {
	prevSpeed int32
	prevAltitude float64
}

// Validate checks all validation rules and returns the result.
func (v *ValidatorSignedRoc) Validate(speed int32, altitude float64) ValidationResult {
	if speed < -100 || speed > 500 {
		return ValidationResult{Valid: false, Reason: "speed_out_of_range"}
	}
	if altitude > 50000.0 {
		return ValidationResult{Valid: false, Reason: "altitude_out_of_range"}
	}
	{
		delta := int64(speed) - int64(v.prevSpeed)
		if delta < 0 {
			delta = -delta
		}
		if delta > 50 {
			return ValidationResult{Valid: false, Reason: "speed_rate_of_change_exceeded"}
		}
	}
	if math.Abs(float64(altitude)-float64(v.prevAltitude)) > 100.0 {
		return ValidationResult{Valid: false, Reason: "altitude_rate_of_change_exceeded"}
	}
	v.prevSpeed = speed
	v.prevAltitude = altitude
	return ValidationResult{Valid: true, Reason: ""}
}
