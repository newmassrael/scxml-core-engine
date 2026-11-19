// SCE-GENERATED — DO NOT EDIT
// source-hash: b831af570a95c29e02d85630bf21ceff4209902140e863b077b10ee2b7f756f8
// source: forge_call_named.scxml
// SCE-MAP: forge_call_named.scxml:6 :: _forge_body

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

public final class ForgeCallNamed {
    public init() {}

    public func validate(_ name: String) -> SceForgeRuntime.ValidationResult {
        if !(!name.utf8.elementsEqual("".utf8)) {
            return SceForgeRuntime.ValidationResult(valid: false, reason: "plausibility_failed")
        }
        return SceForgeRuntime.ValidationResult(valid: true, reason: "")
    }
}
