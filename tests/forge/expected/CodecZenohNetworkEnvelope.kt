// SCE-MAP: codec_zenoh_network_envelope:60

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_zenoh_network_envelope

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink
import com.sce.generated.codec_zenoh_interest.*
import com.sce.generated.codec_zenoh_response_final.*
import com.sce.generated.codec_zenoh_response.*
import com.sce.generated.codec_zenoh_request.*
import com.sce.generated.codec_zenoh_push.*
import com.sce.generated.codec_zenoh_declare.*
import com.sce.generated.codec_zenoh_oam.*

// RFC §5.B variant primitive (B1-β): discriminated-union body for the
// codec's tag-field suffix. Each arm wraps an imported codec's decoded
// value; the optional Default arm preserves the runtime tag value
// alongside its catch-all body. Arm body types are referenced by FQN
// (defensive — wildcard imports could otherwise surface an ambiguity if
// two imported codecs declare same-named inner classes).
sealed class CodecZenohNetworkEnvelopeVariant {
    data class CodecZenohInterest(val body: com.sce.generated.codec_zenoh_interest.CodecZenohInterest) : CodecZenohNetworkEnvelopeVariant()
    data class CodecZenohResponseFinal(val body: com.sce.generated.codec_zenoh_response_final.CodecZenohResponseFinal) : CodecZenohNetworkEnvelopeVariant()
    data class CodecZenohResponse(val body: com.sce.generated.codec_zenoh_response.CodecZenohResponse) : CodecZenohNetworkEnvelopeVariant()
    data class CodecZenohRequest(val body: com.sce.generated.codec_zenoh_request.CodecZenohRequest) : CodecZenohNetworkEnvelopeVariant()
    data class CodecZenohPush(val body: com.sce.generated.codec_zenoh_push.CodecZenohPush) : CodecZenohNetworkEnvelopeVariant()
    data class CodecZenohDeclare(val body: com.sce.generated.codec_zenoh_declare.CodecZenohDeclare) : CodecZenohNetworkEnvelopeVariant()
    data class CodecZenohOam(val body: com.sce.generated.codec_zenoh_oam.CodecZenohOam) : CodecZenohNetworkEnvelopeVariant()
    data class Default(val tag: UByte, val body: com.sce.generated.codec_zenoh_oam.CodecZenohOam) : CodecZenohNetworkEnvelopeVariant()
}

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecZenohNetworkEnvelope()` before any encode()/decode() call. Defaults
// mirror the zero-initialized shape that decode() fills in on success.
data class CodecZenohNetworkEnvelope(
    // RFC variant-default-uniformity Atomic β-kotlin: pick the declared
    // default arm (`<sce:arm default="true"/>`) instead of the first
    // alternative so a freshly-constructed envelope round-trips byte-
    // exactly through `encode() -> decode()`. Paired with the inner
    // codec's `<sce:flag value=>`-baked default fields above.
    var body: CodecZenohNetworkEnvelopeVariant = CodecZenohNetworkEnvelopeVariant.CodecZenohOam(com.sce.generated.codec_zenoh_oam.CodecZenohOam())
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
            is CodecZenohNetworkEnvelopeVariant.CodecZenohInterest -> _b.body.encode(w)?.let { return it }
            is CodecZenohNetworkEnvelopeVariant.CodecZenohResponseFinal -> _b.body.encode(w)?.let { return it }
            is CodecZenohNetworkEnvelopeVariant.CodecZenohResponse -> _b.body.encode(w)?.let { return it }
            is CodecZenohNetworkEnvelopeVariant.CodecZenohRequest -> _b.body.encode(w)?.let { return it }
            is CodecZenohNetworkEnvelopeVariant.CodecZenohPush -> _b.body.encode(w)?.let { return it }
            is CodecZenohNetworkEnvelopeVariant.CodecZenohDeclare -> _b.body.encode(w)?.let { return it }
            is CodecZenohNetworkEnvelopeVariant.CodecZenohOam -> _b.body.encode(w)?.let { return it }
            is CodecZenohNetworkEnvelopeVariant.Default -> _b.body.encode(w)?.let { return it }
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
        fun decode(cursor: SceCursor): CodecZenohNetworkEnvelope? {
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
            val body: CodecZenohNetworkEnvelopeVariant = when (((_peek.toInt() shr 0) and 0x1F)) {
                25 -> {
                    val _arm = com.sce.generated.codec_zenoh_interest.CodecZenohInterest.decode(cursor) ?: return null
                    CodecZenohNetworkEnvelopeVariant.CodecZenohInterest(_arm)
                }
                26 -> {
                    val _arm = com.sce.generated.codec_zenoh_response_final.CodecZenohResponseFinal.decode(cursor) ?: return null
                    CodecZenohNetworkEnvelopeVariant.CodecZenohResponseFinal(_arm)
                }
                27 -> {
                    val _arm = com.sce.generated.codec_zenoh_response.CodecZenohResponse.decode(cursor) ?: return null
                    CodecZenohNetworkEnvelopeVariant.CodecZenohResponse(_arm)
                }
                28 -> {
                    val _arm = com.sce.generated.codec_zenoh_request.CodecZenohRequest.decode(cursor) ?: return null
                    CodecZenohNetworkEnvelopeVariant.CodecZenohRequest(_arm)
                }
                29 -> {
                    val _arm = com.sce.generated.codec_zenoh_push.CodecZenohPush.decode(cursor) ?: return null
                    CodecZenohNetworkEnvelopeVariant.CodecZenohPush(_arm)
                }
                30 -> {
                    val _arm = com.sce.generated.codec_zenoh_declare.CodecZenohDeclare.decode(cursor) ?: return null
                    CodecZenohNetworkEnvelopeVariant.CodecZenohDeclare(_arm)
                }
                31 -> {
                    val _arm = com.sce.generated.codec_zenoh_oam.CodecZenohOam.decode(cursor) ?: return null
                    CodecZenohNetworkEnvelopeVariant.CodecZenohOam(_arm)
                }
                else -> {
                    val _arm = com.sce.generated.codec_zenoh_oam.CodecZenohOam.decode(cursor) ?: return null
                    CodecZenohNetworkEnvelopeVariant.Default(tag = ((_peek.toInt() shr 0) and 0x1F).toUByte(), body = _arm)
                }
            }
            return CodecZenohNetworkEnvelope(
                body = body
            )
        }
    }
}
