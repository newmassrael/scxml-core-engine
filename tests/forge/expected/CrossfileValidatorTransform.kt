// SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator")
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.crossfile_validator_transform
import com.sce.generated.transform_temperature.*

data class ValidationResult(val valid: Boolean, val reason: String)

class CrossfileValidatorTransform {

    // Imported kinds (cross-file composition)

    fun validate(rawTemp: UShort): ValidationResult {
        if (rawTemp.toInt() < 0 || rawTemp.toInt() > 4095)
            return ValidationResult(false, "raw_temp_out_of_range")
        if (!(computeTemperature(rawTemp) > -40.0 && computeTemperature(rawTemp) < 200.0))
            return ValidationResult(false, "plausibility_failed")
        return ValidationResult(true, "")
    }
}
