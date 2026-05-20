// SCE-MAP: codec_variant_peek_basic:29

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_variant_peek_basic

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink
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
    // RFC variant-default-uniformity Atomic β-kotlin: pick the declared
    // default arm (`<sce:arm default="true"/>`) instead of the first
    // alternative so a freshly-constructed envelope round-trips byte-
    // exactly through `encode() -> decode()`. Paired with the inner
    // codec's `<sce:flag value=>`-baked default fields above.
    var body: CodecVariantPeekBasicVariant = CodecVariantPeekBasicVariant.CodecPeekArmA(com.sce.generated.codec_peek_arm_a.CodecPeekArmA())
) {
    /// RFC §5.B B1-α encode-side primary: write `self` into the
    /// caller-owned `w` sink. Returns `null` on success;
    /// `CodecError.BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable
    /// sinks (e.g. `MutableListSink`) are effectively infallible.
    fun encode(w: SceSink): CodecError? {
        // RFC §5.B Y3 atomic 2b-ii peek-byte / 2b-iv streaming-prefix:
        // streaming prefix encode. Peek-byte mode: arm body's encode
        // prepends its own header byte (which the decoder peeked); no
        // separate tag byte here. Streaming-prefix mode (own-field):
        // carrier is part of the prefix fields and emits via the same
        // per-field path.
        // Append the active arm body's encoded bytes via the same sink.
        when (val _b = this.body) {
            is CodecVariantPeekBasicVariant.CodecPeekArmA -> _b.body.encode(w)?.let { return it }
            is CodecVariantPeekBasicVariant.CodecPeekArmB -> _b.body.encode(w)?.let { return it }
        }
        return null
    }

    /// Heap-backed convenience facade. Runs `encode` over a
    /// `MutableListSink` and returns the freshly-encoded ByteArray.
    /// Callers targeting zero-alloc hot paths should call `encode`
    /// directly against a caller-owned sink (e.g. `ByteArraySink`).
    fun encodeToByteArray(): ByteArray {
        val _list = mutableListOf<Byte>()
        encode(MutableListSink(_list))
        return _list.toByteArray()
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
