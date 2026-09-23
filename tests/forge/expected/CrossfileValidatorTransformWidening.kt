// SCE-MAP: crossfile_validator_transform_widening:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.crossfile_validator_transform_widening
import com.sce.generated.transform_temperature.*

data class ValidationResult(val valid: Boolean, val reason: String)

class CrossfileValidatorTransformWidening {

    // Imported kinds (cross-file composition)

    fun validate(rawByte: UByte): ValidationResult {
        if (rawByte.toInt() > 200)
            return ValidationResult(false, "raw_byte_out_of_range")
        if (!(computeTemperature(rawByte.toUShort()) > computeTemperature(0.toUShort())))
            return ValidationResult(false, "plausibility_failed")
        return ValidationResult(true, "")
    }
}
