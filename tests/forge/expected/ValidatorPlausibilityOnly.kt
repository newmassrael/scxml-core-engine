// SCE-MAP: validator_plausibility_only:2

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.validator_plausibility_only

data class ValidationResult(val valid: Boolean, val reason: String)

class ValidatorPlausibilityOnly {

    fun validate(voltage: Double, current: Double): ValidationResult {
        if (!(voltage * current <= 1000.0))
            return ValidationResult(false, "plausibility_failed")
        return ValidationResult(true, "")
    }
}
