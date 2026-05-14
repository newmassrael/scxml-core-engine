// SCE-MAP: crossfile_validator_filter:14

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.crossfile_validator_filter
import com.sce.generated.filter_low_pass.*

data class ValidationResult(val valid: Boolean, val reason: String)

class CrossfileValidatorFilter {

    // Imported kinds (cross-file composition)
    private val smoother: FilterLowPass = FilterLowPass()

    fun validate(rawSample: Double, threshold: Double): ValidationResult {
        if (!(smoother.update(rawSample) < threshold))
            return ValidationResult(false, "plausibility_failed")
        return ValidationResult(true, "")
    }
}
