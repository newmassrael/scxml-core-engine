// SCE-MAP: crossfile_validator_codec:4

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.crossfile_validator_codec
import com.sce.generated.codec_simple_frame.*

data class ValidationResult(val valid: Boolean, val reason: String)

class CrossfileValidatorCodec {

    // Imported kinds (cross-file composition)
    private val frame: CodecSimpleFrame = CodecSimpleFrame()

    fun validate(msgId: UByte, payload: UShort): ValidationResult {
        if (msgId.toInt() < 0 || msgId.toInt() > 255)
            return ValidationResult(false, "msg_id_out_of_range")
        if (payload.toInt() < 0 || payload.toInt() > 4095)
            return ValidationResult(false, "payload_out_of_range")
        if (!(frame.msgId == msgId && frame.payload == payload))
            return ValidationResult(false, "plausibility_failed")
        return ValidationResult(true, "")
    }
}
