// SCE-GENERATED — DO NOT EDIT
// source-hash: eeb0103caca15ce30c6db8ca9e98ae382eb1228e8e1148f2aec785510af4fbde
// SCE-MAP: validator_range_only.scxml:2 :: _forge_body

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

public final class ValidatorRangeOnly {
    public init() {}

    public func validate(_ temperature: Double) -> SceForgeRuntime.ValidationResult {
        if temperature < -40.0 || temperature > 150.0 {
            return SceForgeRuntime.ValidationResult(valid: false, reason: "temperature_out_of_range")
        }
        return SceForgeRuntime.ValidationResult(valid: true, reason: "")
    }
}
