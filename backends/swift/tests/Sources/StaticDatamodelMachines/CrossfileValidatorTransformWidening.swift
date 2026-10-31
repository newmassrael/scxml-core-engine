// SCE-GENERATED — DO NOT EDIT
// source-hash: f6c860c2693fcb3366bc871ff801c9b0ee9e491a1759830f8d83238267cc3b45
// SCE-MAP: crossfile_validator_transform_widening.scxml:9 :: _forge_body

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

public final class CrossfileValidatorTransformWidening {
    public init() {}

    public func validate(_ rawByte: UInt8) -> SceForgeRuntime.ValidationResult {
        if rawByte > 200 {
            return SceForgeRuntime.ValidationResult(valid: false, reason: "raw_byte_out_of_range")
        }
        if !(TransformTemperature.computeTemperature(UInt16(rawByte)) > TransformTemperature.computeTemperature(0)) {
            return SceForgeRuntime.ValidationResult(valid: false, reason: "plausibility_failed")
        }
        return SceForgeRuntime.ValidationResult(valid: true, reason: "")
    }
}
