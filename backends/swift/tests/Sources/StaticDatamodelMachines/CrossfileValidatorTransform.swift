// SCE-GENERATED — DO NOT EDIT
// source-hash: 7cb5c601bc5bd6648425cb4d614d7be9f82d1e41816b0f69f2fcd5bc5c43d40b
// SCE-MAP: crossfile_validator_transform.scxml:3 :: _forge_body

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

public final class CrossfileValidatorTransform {
    public init() {}

    public func validate(_ rawTemp: UInt16) -> SceForgeRuntime.ValidationResult {
        if rawTemp > 4095 {
            return SceForgeRuntime.ValidationResult(valid: false, reason: "raw_temp_out_of_range")
        }
        if !(TransformTemperature.computeTemperature(rawTemp) > -40.0 && TransformTemperature.computeTemperature(rawTemp) < 200.0) {
            return SceForgeRuntime.ValidationResult(valid: false, reason: "plausibility_failed")
        }
        return SceForgeRuntime.ValidationResult(valid: true, reason: "")
    }
}
