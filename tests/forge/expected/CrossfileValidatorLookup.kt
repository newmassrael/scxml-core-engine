// SCE-MAP: crossfile_validator_lookup:7

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.crossfile_validator_lookup
import com.sce.generated.lookup_severity_default.*

data class ValidationResult(val valid: Boolean, val reason: String)

class CrossfileValidatorLookup {

    // Imported kinds (cross-file composition)

    fun validate(code: Int): ValidationResult {
        if (code < 0 || code > 1000)
            return ValidationResult(false, "code_out_of_range")
        if (!(lookupSeverity(code) > 0))
            return ValidationResult(false, "plausibility_failed")
        return ValidationResult(true, "")
    }
}
