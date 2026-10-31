// SCE-GENERATED — DO NOT EDIT
// source-hash: 62067cbe5198ad923571ee5243de08cf004a9a483bf54ae3e6db2a14e8ad77a7
// SCE-MAP: validator_signed_roc.scxml:2 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// A validator: a class whose `validate` checks one set of inputs against the document's range,
// rate-of-change and plausibility rules and answers the verdict, the first rule an input fails.
// It keeps the last value of each input a rate-of-change rule reads, so each holder of one
// instance shares them. The verdict is the runtime's, named by its module, so one type serves
// every validator of the module.

import SceForgeRuntime

public final class ValidatorSignedRoc {
    private var scePrevSpeed: Int32 = 0
    private var scePrevAltitude: Double = 0.0

    public init() {}

    public func validate(_ speed: Int32, _ altitude: Double) -> SceForgeRuntime.ValidationResult {
        if speed < -100 || speed > 500 {
            return SceForgeRuntime.ValidationResult(valid: false, reason: "speed_out_of_range")
        }
        if altitude > 50000.0 {
            return SceForgeRuntime.ValidationResult(valid: false, reason: "altitude_out_of_range")
        }
        if (Int64(speed) > Int64(scePrevSpeed) ? UInt64(bitPattern: Int64(speed) &- Int64(scePrevSpeed)) : UInt64(bitPattern: Int64(scePrevSpeed) &- Int64(speed))) > 50 {
            return SceForgeRuntime.ValidationResult(valid: false, reason: "speed_rate_of_change_exceeded")
        }
        if abs(altitude - scePrevAltitude) > 100.0 {
            return SceForgeRuntime.ValidationResult(valid: false, reason: "altitude_rate_of_change_exceeded")
        }
        scePrevSpeed = speed
        scePrevAltitude = altitude
        return SceForgeRuntime.ValidationResult(valid: true, reason: "")
    }
}
