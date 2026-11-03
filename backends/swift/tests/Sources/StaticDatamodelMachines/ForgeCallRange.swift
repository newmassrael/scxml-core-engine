// SCE-GENERATED — DO NOT EDIT
// source-hash: ef94c112b4026dfab705629195b17ee265cdc572de6e8fceff328262c2af0613
// SCE-MAP: forge_call_range.scxml:7 :: _forge_body

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

public final class ForgeCallRange {
    public init() {}

    public func validate(_ level: Int32, _ offset: Int32) -> SceForgeRuntime.ValidationResult {
        if level > 30 {
            return SceForgeRuntime.ValidationResult(valid: false, reason: "level_out_of_range")
        }
        if offset < 0 {
            return SceForgeRuntime.ValidationResult(valid: false, reason: "offset_out_of_range")
        }
        return SceForgeRuntime.ValidationResult(valid: true, reason: "")
    }
}
