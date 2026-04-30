// SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.crossfile_validator_condition
import com.sce.generated.condition_threshold.*

data class ValidationResult(val valid: Boolean, val reason: String)

class CrossfileValidatorCondition {

    // Imported kinds (cross-file composition)

    fun validate(coolantTemp: Double, oilTemp: Double, maxTemp: Double): ValidationResult {
        if (!(!conditionThreshold(coolantTemp, oilTemp, maxTemp)))
            return ValidationResult(false, "plausibility_failed")
        return ValidationResult(true, "")
    }
}
