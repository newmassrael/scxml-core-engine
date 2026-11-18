// SCE-MAP: codec_zenoh_network_envelope:60 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_zenoh_network_envelope

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.Decoded
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
import kotlin.jvm.JvmInline

// RFC §synth-5-B variant primitive: discriminated-union body for the
// codec's tag-field suffix. Each arm wraps an imported codec's decoded
// value; the optional Default arm preserves the runtime tag value
// alongside its catch-all body. Arm body types are referenced by FQN
// (defensive — wildcard imports could otherwise surface an ambiguity if
// two imported codecs declare same-named inner classes).
sealed class CodecZenohNetworkEnvelopeVariant {
    data class CodecZenohInterest(val body: com.sce.generated.codec_zenoh_interest.CodecZenohInterestSource) : CodecZenohNetworkEnvelopeVariant()
    data class CodecZenohResponseFinal(val body: com.sce.generated.codec_zenoh_response_final.CodecZenohResponseFinalSource) : CodecZenohNetworkEnvelopeVariant()
    data class CodecZenohResponse(val body: com.sce.generated.codec_zenoh_response.CodecZenohResponseSource) : CodecZenohNetworkEnvelopeVariant()
    data class CodecZenohRequest(val body: com.sce.generated.codec_zenoh_request.CodecZenohRequestSource) : CodecZenohNetworkEnvelopeVariant()
    data class CodecZenohPush(val body: com.sce.generated.codec_zenoh_push.CodecZenohPushSource) : CodecZenohNetworkEnvelopeVariant()
    data class CodecZenohDeclare(val body: com.sce.generated.codec_zenoh_declare.CodecZenohDeclareSource) : CodecZenohNetworkEnvelopeVariant()
    data class CodecZenohOam(val body: com.sce.generated.codec_zenoh_oam.CodecZenohOamSource) : CodecZenohNetworkEnvelopeVariant()
    data class Default(val tag: CodecZenohNetworkEnvelopeUnknownTag, val body: com.sce.generated.codec_zenoh_oam.CodecZenohOamSource) : CodecZenohNetworkEnvelopeVariant()
}

/// The tag the catch-all arm of [CodecZenohNetworkEnvelope]'s variant holds: one the bits
/// the tag is read from hold and no arm names, held to that where a value of
/// it is made (docs/adr/0065 §6 R3, docs/adr/0081 §2), so the catch-all a value
/// holds is written and read back as the catch-all.
@JvmInline
value class CodecZenohNetworkEnvelopeUnknownTag private constructor(val tag: UByte) {
    /// The tag as a `ULong`, the type an encode derives a tag in.
    val wide: ULong
        get() = tag.toULong()

    companion object {
        /// `tag` as a value of the catch-all, or `null` where an arm names it or the
        /// bits the tag is read from do not hold it.
        fun of(tag: UByte): CodecZenohNetworkEnvelopeUnknownTag? =
            when (tag.toUInt()) {
                in 0x19u..0x1Fu -> null
                else -> if (tag.toUInt() <= 0x1Fu) CodecZenohNetworkEnvelopeUnknownTag(tag) else null
            }

        /// The least tag no arm names: what a catch-all made by default holds.
        fun fewest(): CodecZenohNetworkEnvelopeUnknownTag = CodecZenohNetworkEnvelopeUnknownTag(0x0u.toUByte())
    }
}

/// The elements a value of [CodecZenohNetworkEnvelope] presents, which its encode reads
/// (docs/adr/0081 §2): any object that presents them is a source of it, which
/// [CodecZenohNetworkEnvelope.encodeSource] writes. The encode derives every other element
/// of the frame.
interface CodecZenohNetworkEnvelopeSource {
    val body: CodecZenohNetworkEnvelopeVariant
}

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecZenohNetworkEnvelope()` before any encode()/decode() call. Each default
// is a value of that field's own type, which decode() then fills in on
// success — the carrier's zero for a number, and for an enum the first
// variant its document declares, since a closed set does not hold a
// value it never declared.
data class CodecZenohNetworkEnvelope(
    // RFC variant-default-uniformity (Kotlin): pick the declared
    // default arm (`<sce:arm default="true"/>`) instead of the first
    // alternative so a freshly-constructed envelope round-trips byte-
    // exactly through `encode() -> decode()`. Paired with the inner
    // codec's `<sce:flag value=>`-baked default fields above.
    override var body: CodecZenohNetworkEnvelopeVariant = CodecZenohNetworkEnvelopeVariant.CodecZenohOam(com.sce.generated.codec_zenoh_oam.CodecZenohOam())
) : CodecZenohNetworkEnvelopeSource {
    /// RFC §synth-5-B encode-side primary: write `self` into the
    /// caller-owned `w` sink. Returns `null` on success;
    /// `CodecError.BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable
    /// sinks (e.g. `MutableListSink`) are effectively infallible.
    fun encode(w: SceSink): CodecError? {
        // docs/adr/0081 step 4: the encode over any source, which derives
        // every element the value does not present. It is handed the flag
        // inputs the value does not say; one the value says is read off it,
        // and what a caller hands this `encode` for it is held to that
        // (docs/adr/0065 §6 R4a).
        return encodeSource(this, w)
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
        /// Write any source of [CodecZenohNetworkEnvelope] into `w`: each element read once off
        /// `src` and written where the codec's encode plan lays it out
        /// (docs/adr/0081). It refuses no value: it answers `null`, or
        /// [CodecError.BufferOverflow] from a bounded sink with no room left.
        fun encodeSource(src: CodecZenohNetworkEnvelopeSource, w: SceSink): CodecError? {
            val _r_body = src.body
            when (val _b = _r_body) {
                is CodecZenohNetworkEnvelopeVariant.CodecZenohInterest -> CodecZenohInterest.encodeSource(_b.body, w)?.let { return it }
                is CodecZenohNetworkEnvelopeVariant.CodecZenohResponseFinal -> CodecZenohResponseFinal.encodeSource(_b.body, w)?.let { return it }
                is CodecZenohNetworkEnvelopeVariant.CodecZenohResponse -> CodecZenohResponse.encodeSource(_b.body, w)?.let { return it }
                is CodecZenohNetworkEnvelopeVariant.CodecZenohRequest -> CodecZenohRequest.encodeSource(_b.body, w)?.let { return it }
                is CodecZenohNetworkEnvelopeVariant.CodecZenohPush -> CodecZenohPush.encodeSource(_b.body, w)?.let { return it }
                is CodecZenohNetworkEnvelopeVariant.CodecZenohDeclare -> CodecZenohDeclare.encodeSource(_b.body, w)?.let { return it }
                is CodecZenohNetworkEnvelopeVariant.CodecZenohOam -> CodecZenohOam.encodeSource(_b.body, w)?.let { return it }
                is CodecZenohNetworkEnvelopeVariant.Default -> CodecZenohOam.encodeSource(_b.body, w)?.let { return it }
            }
            return null
        }

        /// Decode the next frame from `cursor`. On success the cursor
        /// advances past the consumed bytes and the value is `Decoded.Ok`;
        /// a refused frame is `Decoded.Refused` with the kind that refused
        /// it (docs/adr/0099, amendment of 2026-11-16): `NeedMoreBytes` when
        /// the cursor's tail is shorter than the frame (RFC §synth-5-B
        /// L494-519), which the caller resumes after appending bytes, and
        /// another kind for a frame no more bytes make whole.
        fun decode(cursor: SceCursor): Decoded<CodecZenohNetworkEnvelope> {
            // RFC §synth-5-B peek-byte / streaming-
            // prefix: streaming prefix decode (variable-length fields
            // supported via per-field present_if/tlv-chain/embed/repeat
            // helpers). Peek-byte mode additionally peeks the cursor's
            // next byte for variant tag without advancing — arm body
            // decoder reads it as own header.
            val _peekRaw = cursor.peekSlice(1) ?: return Decoded.Refused(CodecError.NeedMoreBytes)
        val _peek: UByte = _peekRaw[0].toUByte()
            // Dispatch on the tag field; each arm decodes its body codec
            // from the cursor. The default arm (when declared) carries
            // the runtime tag value so encode can round-trip it back
            // onto the wire.
            val body: CodecZenohNetworkEnvelopeVariant = when (((_peek.toInt() shr 0) and 0x1F)) {
                25 -> {
                    val _arm = com.sce.generated.codec_zenoh_interest.CodecZenohInterest.decode(cursor).valueOr { _refused -> return _refused }
                    CodecZenohNetworkEnvelopeVariant.CodecZenohInterest(_arm)
                }
                26 -> {
                    val _arm = com.sce.generated.codec_zenoh_response_final.CodecZenohResponseFinal.decode(cursor).valueOr { _refused -> return _refused }
                    CodecZenohNetworkEnvelopeVariant.CodecZenohResponseFinal(_arm)
                }
                27 -> {
                    val _arm = com.sce.generated.codec_zenoh_response.CodecZenohResponse.decode(cursor).valueOr { _refused -> return _refused }
                    CodecZenohNetworkEnvelopeVariant.CodecZenohResponse(_arm)
                }
                28 -> {
                    val _arm = com.sce.generated.codec_zenoh_request.CodecZenohRequest.decode(cursor).valueOr { _refused -> return _refused }
                    CodecZenohNetworkEnvelopeVariant.CodecZenohRequest(_arm)
                }
                29 -> {
                    val _arm = com.sce.generated.codec_zenoh_push.CodecZenohPush.decode(cursor).valueOr { _refused -> return _refused }
                    CodecZenohNetworkEnvelopeVariant.CodecZenohPush(_arm)
                }
                30 -> {
                    val _arm = com.sce.generated.codec_zenoh_declare.CodecZenohDeclare.decode(cursor).valueOr { _refused -> return _refused }
                    CodecZenohNetworkEnvelopeVariant.CodecZenohDeclare(_arm)
                }
                31 -> {
                    val _arm = com.sce.generated.codec_zenoh_oam.CodecZenohOam.decode(cursor).valueOr { _refused -> return _refused }
                    CodecZenohNetworkEnvelopeVariant.CodecZenohOam(_arm)
                }
                else -> {
                    val _arm = com.sce.generated.codec_zenoh_oam.CodecZenohOam.decode(cursor).valueOr { _refused -> return _refused }
                    CodecZenohNetworkEnvelopeVariant.Default(tag = CodecZenohNetworkEnvelopeUnknownTag.of(((_peek.toInt() shr 0) and 0x1F).toUByte()) ?: error("a tag no arm names is one CodecZenohNetworkEnvelopeUnknownTag holds"), body = _arm)
                }
            }
            return Decoded.Ok(
                CodecZenohNetworkEnvelope(
                    body = body
                )
            )
        }
    }
}
