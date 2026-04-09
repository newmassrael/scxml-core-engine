// SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator")
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.validator_rpm_check

data class ValidationResult(val valid: Boolean, val reason: String)

class ValidatorRpmCheck {
    private var prevRpm: UShort = 0u.toUShort()

    fun validate(rpm: UShort, engineState: String): ValidationResult {
        if (rpm.toInt() < 0 || rpm.toInt() > 8000)
            return ValidationResult(false, "rpm_out_of_range")
        run {
            val delta = if (rpm.toInt() > prevRpm.toInt()) rpm.toInt() - prevRpm.toInt() else prevRpm.toInt() - rpm.toInt()
            if (delta > 500)
                return ValidationResult(false, "rpm_rate_of_change_exceeded")
        }
        if (!(rpm.toInt() == 0 || engineState != "STOP"))
            return ValidationResult(false, "plausibility_failed")
        prevRpm = rpm
        return ValidationResult(true, "")
    }
}