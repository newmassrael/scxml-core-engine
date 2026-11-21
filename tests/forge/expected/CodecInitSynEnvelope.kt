// SCE-MAP: codec_init_syn_envelope:24 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_init_syn_envelope

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.Decoded
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink
import com.sce.generated.codec_init_syn_body.*
import kotlin.jvm.JvmInline

// RFC §synth-5-B variant primitive: discriminated-union body for the
// codec's tag-field suffix. Each arm wraps an imported codec's decoded
// value; the optional Default arm preserves the runtime tag value
// alongside its catch-all body. Arm body types are referenced by FQN
// (defensive — wildcard imports could otherwise surface an ambiguity if
// two imported codecs declare same-named inner classes).
sealed class CodecInitSynEnvelopeVariant {
    data class CodecInitSynBody(val body: com.sce.generated.codec_init_syn_body.CodecInitSynBodySource) : CodecInitSynEnvelopeVariant()
    data class Default(val tag: CodecInitSynEnvelopeUnknownTag, val body: com.sce.generated.codec_init_syn_body.CodecInitSynBodySource) : CodecInitSynEnvelopeVariant()
}

/// The tag the catch-all arm of [CodecInitSynEnvelope]'s variant holds: one the bits
/// the tag is read from hold and no arm names, held to that where a value of
/// it is made (docs/adr/0065 §6 R3, docs/adr/0081 §2), so the catch-all a value
/// holds is written and read back as the catch-all.
@JvmInline
value class CodecInitSynEnvelopeUnknownTag private constructor(val tag: UByte) {
    /// The tag as a `ULong`, the type an encode derives a tag in.
    val wide: ULong
        get() = tag.toULong()

    companion object {
        /// `tag` as a value of the catch-all, or `null` where an arm names it or the
        /// bits the tag is read from do not hold it.
        fun of(tag: UByte): CodecInitSynEnvelopeUnknownTag? =
            when (tag.toUInt()) {
                0x1u -> null
                else -> if (tag.toUInt() <= 0x1Fu) CodecInitSynEnvelopeUnknownTag(tag) else null
            }

        /// The least tag no arm names: what a catch-all made by default holds.
        fun fewest(): CodecInitSynEnvelopeUnknownTag = CodecInitSynEnvelopeUnknownTag(0x0u.toUByte())
    }
}

/// The elements a value of [CodecInitSynEnvelope] presents, which its encode reads
/// (docs/adr/0081 §2): any object that presents them is a source of it, which
/// [CodecInitSynEnvelope.encodeSource] writes. The encode derives every other element
/// of the frame.
interface CodecInitSynEnvelopeSource {
    val header: UByte
    val body: CodecInitSynEnvelopeVariant
}

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecInitSynEnvelope()` before any encode()/decode() call. Each default
// is a value of that field's own type, which decode() then fills in on
// success — the carrier's zero for a number, and for an enum the first
// variant its document declares, since a closed set does not hold a
// value it never declared.
data class CodecInitSynEnvelope(
    override var header: UByte = 0.toUByte(),
    // RFC variant-default-uniformity (Kotlin): pick the declared
    // default arm (`<sce:arm default="true"/>`) instead of the first
    // alternative so a freshly-constructed envelope round-trips byte-
    // exactly through `encode() -> decode()`. Paired with the inner
    // codec's `<sce:flag value=>`-baked default fields above.
    override var body: CodecInitSynEnvelopeVariant = CodecInitSynEnvelopeVariant.CodecInitSynBody(com.sce.generated.codec_init_syn_body.CodecInitSynBody())
) : CodecInitSynEnvelopeSource {
    // RFC §synth-5-B flags primitive: per-bit-range accessors over
    // the carrier field. Single-bit (width=1) reads as Boolean; multi-
    // bit (width>=2) reads as the smallest unsigned Kotlin type that
    // fits (UByte / UShort / UInt / ULong). UByte/UShort widen through
    // `.toInt()` and UInt/ULong through `.toLong()` for the bitwise
    // ops; the result narrows back via the carrier's `toU*` ctor.
    fun mid(): UByte {
        val _carrier = this.header.toInt()
        return ((_carrier shr 0) and 0x1F).toUByte()
    }

    fun setMid(v: UByte) {
        val _carrier = this.header.toInt()
        val _shifted_mask = 0x1F shl 0
        val _val = (v.toInt() and 0x1F) shl 0
        this.header = ((_carrier and _shifted_mask.inv()) or _val).toUByte()
    }

    fun S(): Boolean = (this.header.toInt() and 0x40) != 0

    fun setS(v: Boolean) {
        this.header = if (v) {
            (this.header.toInt() or 0x40).toUByte()
        } else {
            (this.header.toInt() and 0x40.inv()).toUByte()
        }
    }

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
        /// Write any source of [CodecInitSynEnvelope] into `w`: each element read once off
        /// `src` and written where the codec's encode plan lays it out
        /// (docs/adr/0081). It refuses no value: it answers `null`, or
        /// [CodecError.BufferOverflow] from a bounded sink with no room left.
        fun encodeSource(src: CodecInitSynEnvelopeSource, w: SceSink): CodecError? {
            val _e_header = src.header
            val _r_body = src.body
            val _t_body: ULong = when (val _b = _r_body) {
                is CodecInitSynEnvelopeVariant.CodecInitSynBody -> 0x1uL
                is CodecInitSynEnvelopeVariant.Default -> _b.tag.wide
            }
            var _u_header = _e_header.toUInt()
            _u_header = (_u_header and 0x1Fu.inv()) or ((_t_body and 0x1FuL).toUInt() shl 0)
            _u_header = (_u_header and 0x40u.inv()) or (((when (val _b = _r_body) { is CodecInitSynEnvelopeVariant.CodecInitSynBody -> CodecInitSynBody.inputS(_b.body).toInt(); is CodecInitSynEnvelopeVariant.Default -> CodecInitSynBody.inputS(_b.body).toInt() })).toUInt() shl 6)
            val _c_header = _u_header.toUByte()
            w.writeU8(_c_header.toByte())?.let { return it }
            when (val _b = _r_body) {
                is CodecInitSynEnvelopeVariant.CodecInitSynBody -> CodecInitSynBody.encodeSource(_b.body, w)?.let { return it }
                is CodecInitSynEnvelopeVariant.Default -> CodecInitSynBody.encodeSource(_b.body, w)?.let { return it }
            }
            return null
        }

        /// Decode the next frame from `cursor`. On success the cursor
        /// advances past the consumed bytes and the value is `Decoded.Ok`;
        /// a refused frame is `Decoded.Refused` with the kind that refused
        /// it (docs/adr/0099, amendment of 2026-11-16): `NeedMoreBytes` when
        /// the cursor's tail is shorter than the frame (RFC
        /// §synth-5-B L494-519), which the caller resumes after appending
        /// bytes, and another kind for a frame no more bytes make whole.
        fun decode(cursor: SceCursor): Decoded<CodecInitSynEnvelope> {
            // Decode fixed prefix (RFC §synth-5-B variant: fields before tag suffix).
            val raw = cursor.peekSlice(1) ?: return Decoded.Refused(CodecError.NeedMoreBytes)
            val header = raw[0].toUByte()
            if (!cursor.advance(1)) return Decoded.Refused(CodecError.NeedMoreBytes)
            // Dispatch on the tag field; each arm decodes its body codec
            // from the cursor. The default arm (when declared) carries
            // the runtime tag value so encode can round-trip it back
            // onto the wire.
            val body: CodecInitSynEnvelopeVariant = when (((header.toInt() shr 0) and 0x1F)) {
                1 -> {
                    val _arm = com.sce.generated.codec_init_syn_body.CodecInitSynBody.decode(cursor, (((header.toInt() shr 6) and 0x1).toUByte())).valueOr { _refused -> return _refused }
                    CodecInitSynEnvelopeVariant.CodecInitSynBody(_arm)
                }
                else -> {
                    val _arm = com.sce.generated.codec_init_syn_body.CodecInitSynBody.decode(cursor, (((header.toInt() shr 6) and 0x1).toUByte())).valueOr { _refused -> return _refused }
                    CodecInitSynEnvelopeVariant.Default(tag = CodecInitSynEnvelopeUnknownTag.of(((header.toInt() shr 0) and 0x1F).toUByte()) ?: error("a tag no arm names is one CodecInitSynEnvelopeUnknownTag holds"), body = _arm)
                }
            }
            return Decoded.Ok(
                CodecInitSynEnvelope(
                    header = header,
                    body = body
                )
            )
        }
    }
}
