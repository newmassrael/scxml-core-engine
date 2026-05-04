// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_variant_peek_basic

import com.sce.forge.runtime.SceCursor
import com.sce.generated.codec_peek_arm_a.*
import com.sce.generated.codec_peek_arm_b.*

// RFC §5.B variant primitive (B1-β): discriminated-union body for the
// codec's tag-field suffix. Each arm wraps an imported codec's decoded
// value; the optional Default arm preserves the runtime tag value
// alongside its catch-all body. Arm body types are referenced by FQN
// (defensive — wildcard imports could otherwise surface an ambiguity if
// two imported codecs declare same-named inner classes).
sealed class CodecVariantPeekBasicVariant {
    data class CodecPeekArmA(val body: com.sce.generated.codec_peek_arm_a.CodecPeekArmA) : CodecVariantPeekBasicVariant()
    data class CodecPeekArmB(val body: com.sce.generated.codec_peek_arm_b.CodecPeekArmB) : CodecVariantPeekBasicVariant()
}

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecVariantPeekBasic()` before any encode()/decode() call. Defaults
// mirror the zero-initialized shape that decode() fills in on success.
data class CodecVariantPeekBasic(
    var body: CodecVariantPeekBasicVariant = CodecVariantPeekBasicVariant.CodecPeekArmA(com.sce.generated.codec_peek_arm_a.CodecPeekArmA())
) {
    fun encode(): ByteArray {
        // RFC §5.B Y3 atomic 2b-ii peek-byte / 2b-iv streaming-prefix:
        // streaming prefix encode. Peek-byte mode: arm body's encode
        // prepends its own header byte (which the decoder peeked); no
        // separate tag byte here. Streaming-prefix mode (own-field):
        // carrier is part of the prefix fields and emits via the same
        // per-field path.
        val r = mutableListOf<Byte>()
        // Append the active arm body's encoded bytes.
        when (val _b = this.body) {
            is CodecVariantPeekBasicVariant.CodecPeekArmA -> r.addAll(_b.body.encode().toList())
            is CodecVariantPeekBasicVariant.CodecPeekArmB -> r.addAll(_b.body.encode().toList())
        }
        return r.toByteArray()
    }

    companion object {
        /// Decode the next frame from `cursor`. On success the cursor
        /// advances past the consumed bytes; returns `null` when the
        /// cursor's tail is shorter than the declared minimum frame
        /// (RFC §5.B L494-519).
        fun decode(cursor: SceCursor): CodecVariantPeekBasic? {
            // RFC §5.B Y3 atomic 2b-ii peek-byte / 2b-iv streaming-
            // prefix: streaming prefix decode (variable-length fields
            // supported via per-field present_if/tlv-chain/embed/repeat
            // helpers). Peek-byte mode additionally peeks the cursor's
            // next byte for variant tag without advancing — arm body
            // decoder reads it as own header.
            val _peekRaw = cursor.peekSlice(1) ?: return null
        val _peek: UByte = _peekRaw[0].toUByte()
            // Dispatch on the tag field; each arm decodes its body codec
            // from the cursor. The default arm (when declared) carries
            // the runtime tag value so encode can round-trip it back
            // onto the wire.
            val body: CodecVariantPeekBasicVariant = when (((_peek.toInt() shr 0) and 0x01)) {
                0 -> {
                    val _arm = com.sce.generated.codec_peek_arm_a.CodecPeekArmA.decode(cursor) ?: return null
                    CodecVariantPeekBasicVariant.CodecPeekArmA(_arm)
                }
                1 -> {
                    val _arm = com.sce.generated.codec_peek_arm_b.CodecPeekArmB.decode(cursor) ?: return null
                    CodecVariantPeekBasicVariant.CodecPeekArmB(_arm)
                }
                else -> {
                    // codec/variant-arm-unreachable rejected this case at parse time.
                    return null
                }
            }
            return CodecVariantPeekBasic(
                body = body
            )
        }
    }
}
