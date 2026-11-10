// SCE-GENERATED — DO NOT EDIT
// source-hash: f3a316aacf7403325399aec899293549220a4c98b78ad03825aff15e55acb6a9
// SCE-MAP: crossfile_validator_codec.scxml:4 :: _forge_body

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

public final class CrossfileValidatorCodec {
    private let frame = CodecSimpleFrame()

    public init() {}

    public func validate(_ msgId: UInt8, _ payload: UInt16) -> SceForgeRuntime.ValidationResult {
        if payload > 4095 {
            return SceForgeRuntime.ValidationResult(valid: false, reason: "payload_out_of_range")
        }
        if !(frame.msgId == msgId && frame.payload == payload) {
            return SceForgeRuntime.ValidationResult(valid: false, reason: "plausibility_failed")
        }
        return SceForgeRuntime.ValidationResult(valid: true, reason: "")
    }
}
