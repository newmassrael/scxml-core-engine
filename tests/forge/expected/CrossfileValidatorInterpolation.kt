// SCE-MAP: crossfile_validator_interpolation:9

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.crossfile_validator_interpolation
import com.sce.generated.interpolation_1d_linear.*

data class ValidationResult(val valid: Boolean, val reason: String)

class CrossfileValidatorInterpolation {

    // Imported kinds (cross-file composition)

    fun validate(rpm: UShort): ValidationResult {
        if (rpm.toInt() < 500 || rpm.toInt() > 7000)
            return ValidationResult(false, "rpm_out_of_range")
        if (!(Interpolation1dLinear.lookup(rpm) > 200.0))
            return ValidationResult(false, "plausibility_failed")
        return ValidationResult(true, "")
    }
}
