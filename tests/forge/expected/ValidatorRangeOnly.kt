// SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator")
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.validator_range_only

data class ValidationResult(val valid: Boolean, val reason: String)

class ValidatorRangeOnly {

    fun validate(temperature: Double): ValidationResult {
        if (temperature < -40.0 || temperature > 150.0)
            return ValidationResult(false, "temperature_out_of_range")
        return ValidationResult(true, "")
    }
}