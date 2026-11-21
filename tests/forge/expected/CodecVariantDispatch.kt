// SCE-MAP: codec_variant_dispatch:8 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_variant_dispatch

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.Decoded
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink
import com.sce.generated.codec_variant_session_open.*
import com.sce.generated.codec_variant_session_close.*
import kotlin.jvm.JvmInline

// RFC §synth-5-B variant primitive: discriminated-union body for the
// codec's tag-field suffix. Each arm wraps an imported codec's decoded
// value; the optional Default arm preserves the runtime tag value
// alongside its catch-all body. Arm body types are referenced by FQN
// (defensive — wildcard imports could otherwise surface an ambiguity if
// two imported codecs declare same-named inner classes).
sealed class CodecVariantDispatchVariant {
    data class CodecVariantSessionOpen(val body: com.sce.generated.codec_variant_session_open.CodecVariantSessionOpenSource) : CodecVariantDispatchVariant()
    data class CodecVariantSessionClose(val body: com.sce.generated.codec_variant_session_close.CodecVariantSessionCloseSource) : CodecVariantDispatchVariant()
    data class Default(val tag: CodecVariantDispatchUnknownTag, val body: com.sce.generated.codec_variant_session_close.CodecVariantSessionCloseSource) : CodecVariantDispatchVariant()
}

/// The tag the catch-all arm of [CodecVariantDispatch]'s variant holds: one the bits
/// the tag is read from hold and no arm names, held to that where a value of
/// it is made (docs/adr/0065 §6 R3, docs/adr/0081 §2), so the catch-all a value
/// holds is written and read back as the catch-all.
@JvmInline
value class CodecVariantDispatchUnknownTag private constructor(val tag: UByte) {
    /// The tag as a `ULong`, the type an encode derives a tag in.
    val wide: ULong
        get() = tag.toULong()

    companion object {
        /// `tag` as a value of the catch-all, or `null` where an arm names it or the
        /// bits the tag is read from do not hold it.
        fun of(tag: UByte): CodecVariantDispatchUnknownTag? =
            when (tag.toUInt()) {
                in 0x1u..0x2u -> null
                else -> CodecVariantDispatchUnknownTag(tag)
            }

        /// The least tag no arm names: what a catch-all made by default holds.
        fun fewest(): CodecVariantDispatchUnknownTag = CodecVariantDispatchUnknownTag(0x0u.toUByte())
    }
}

/// The elements a value of [CodecVariantDispatch] presents, which its encode reads
/// (docs/adr/0081 §2): any object that presents them is a source of it, which
/// [CodecVariantDispatch.encodeSource] writes. The encode derives every other element
/// of the frame.
interface CodecVariantDispatchSource {
    val body: CodecVariantDispatchVariant
}

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecVariantDispatch()` before any encode()/decode() call. Each default
// is a value of that field's own type, which decode() then fills in on
// success — the carrier's zero for a number, and for an enum the first
// variant its document declares, since a closed set does not hold a
// value it never declared.
data class CodecVariantDispatch(
    // RFC variant-default-uniformity (Kotlin): pick the declared
    // default arm (`<sce:arm default="true"/>`) instead of the first
    // alternative so a freshly-constructed envelope round-trips byte-
    // exactly through `encode() -> decode()`. Paired with the inner
    // codec's `<sce:flag value=>`-baked default fields above.
    override var body: CodecVariantDispatchVariant = CodecVariantDispatchVariant.CodecVariantSessionClose(com.sce.generated.codec_variant_session_close.CodecVariantSessionClose())
) : CodecVariantDispatchSource {
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
        /// Write any source of [CodecVariantDispatch] into `w`: each element read once off
        /// `src` and written where the codec's encode plan lays it out
        /// (docs/adr/0081). It refuses no value: it answers `null`, or
        /// [CodecError.BufferOverflow] from a bounded sink with no room left.
        fun encodeSource(src: CodecVariantDispatchSource, w: SceSink): CodecError? {
            val _r_body = src.body
            val _t_body: ULong = when (val _b = _r_body) {
                is CodecVariantDispatchVariant.CodecVariantSessionOpen -> 0x1uL
                is CodecVariantDispatchVariant.CodecVariantSessionClose -> 0x2uL
                is CodecVariantDispatchVariant.Default -> _b.tag.wide
            }
            val _d_msg_id = _t_body.toUByte()
            w.writeU8(_d_msg_id.toByte())?.let { return it }
            when (val _b = _r_body) {
                is CodecVariantDispatchVariant.CodecVariantSessionOpen -> CodecVariantSessionOpen.encodeSource(_b.body, w)?.let { return it }
                is CodecVariantDispatchVariant.CodecVariantSessionClose -> CodecVariantSessionClose.encodeSource(_b.body, w)?.let { return it }
                is CodecVariantDispatchVariant.Default -> CodecVariantSessionClose.encodeSource(_b.body, w)?.let { return it }
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
        fun decode(cursor: SceCursor): Decoded<CodecVariantDispatch> {
            // Decode fixed prefix (RFC §synth-5-B variant: fields before tag suffix).
            val raw = cursor.peekSlice(1) ?: return Decoded.Refused(CodecError.NeedMoreBytes)
            val msg_id = raw[0].toUByte()
            if (!cursor.advance(1)) return Decoded.Refused(CodecError.NeedMoreBytes)
            // Dispatch on the tag field; each arm decodes its body codec
            // from the cursor. The default arm (when declared) carries
            // the runtime tag value so encode can round-trip it back
            // onto the wire.
            val body: CodecVariantDispatchVariant = when (msg_id.toInt()) {
                1 -> {
                    val _arm = com.sce.generated.codec_variant_session_open.CodecVariantSessionOpen.decode(cursor).valueOr { _refused -> return _refused }
                    CodecVariantDispatchVariant.CodecVariantSessionOpen(_arm)
                }
                2 -> {
                    val _arm = com.sce.generated.codec_variant_session_close.CodecVariantSessionClose.decode(cursor).valueOr { _refused -> return _refused }
                    CodecVariantDispatchVariant.CodecVariantSessionClose(_arm)
                }
                else -> {
                    val _arm = com.sce.generated.codec_variant_session_close.CodecVariantSessionClose.decode(cursor).valueOr { _refused -> return _refused }
                    CodecVariantDispatchVariant.Default(tag = CodecVariantDispatchUnknownTag.of(msg_id) ?: error("a tag no arm names is one CodecVariantDispatchUnknownTag holds"), body = _arm)
                }
            }
            return Decoded.Ok(
                CodecVariantDispatch(
                    body = body
                )
            )
        }
    }
}
