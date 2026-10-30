// SCE-GENERATED — DO NOT EDIT
// source-hash: 1ba579cdc10cbe049a3766817dcc6397d58e8716381e63be0e21ee74788ac719
// SCE-MAP: validator_rpm_check.scxml:2 :: _forge_body

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

public final class ValidatorRpmCheck {
    private var scePrevRpm: UInt16 = 0

    public init() {}

    public func validate(_ rpm: UInt16, _ engineState: String) -> SceForgeRuntime.ValidationResult {
        if rpm > 8000 {
            return SceForgeRuntime.ValidationResult(valid: false, reason: "rpm_out_of_range")
        }
        if (rpm > scePrevRpm ? rpm - scePrevRpm : scePrevRpm - rpm) > 500 {
            return SceForgeRuntime.ValidationResult(valid: false, reason: "rpm_rate_of_change_exceeded")
        }
        if !(rpm == 0 || !engineState.utf8.elementsEqual("STOP".utf8)) {
            return SceForgeRuntime.ValidationResult(valid: false, reason: "plausibility_failed")
        }
        scePrevRpm = rpm
        return SceForgeRuntime.ValidationResult(valid: true, reason: "")
    }
}
