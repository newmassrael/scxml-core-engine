// SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.validator_signed_roc

data class ValidationResult(val valid: Boolean, val reason: String)

class ValidatorSignedRoc {
    private var prevSpeed: Int = 0
    private var prevAltitude: Double = 0.0

    fun validate(speed: Int, altitude: Double): ValidationResult {
        if (speed < -100 || speed > 500)
            return ValidationResult(false, "speed_out_of_range")
        if (altitude > 50000.0)
            return ValidationResult(false, "altitude_out_of_range")
        run {
            val delta = kotlin.math.abs(speed.toLong() - prevSpeed.toLong())
            if (delta > 50)
                return ValidationResult(false, "speed_rate_of_change_exceeded")
        }
        run {
            val delta = kotlin.math.abs(altitude - prevAltitude)
            if (delta > 100.0)
                return ValidationResult(false, "altitude_rate_of_change_exceeded")
        }
        prevSpeed = speed
        prevAltitude = altitude
        return ValidationResult(true, "")
    }
}
