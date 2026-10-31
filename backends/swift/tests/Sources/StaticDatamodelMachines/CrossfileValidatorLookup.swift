// SCE-GENERATED — DO NOT EDIT
// source-hash: 90dfacafb9c3b3dd4ea571704b12a814c54947805373c06b331ad43ea5f220d0
// SCE-MAP: crossfile_validator_lookup.scxml:7 :: _forge_body

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

public final class CrossfileValidatorLookup {
    public init() {}

    public func validate(_ code: Int32) -> SceForgeRuntime.ValidationResult {
        if code < 0 || code > 1000 {
            return SceForgeRuntime.ValidationResult(valid: false, reason: "code_out_of_range")
        }
        if !(LookupSeverityDefault.lookupSeverity(code) > 0) {
            return SceForgeRuntime.ValidationResult(valid: false, reason: "plausibility_failed")
        }
        return SceForgeRuntime.ValidationResult(valid: true, reason: "")
    }
}
