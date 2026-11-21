// SCE-MAP: codec_present_if_length_ref:16 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_present_if_length_ref

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.Decoded
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink
import kotlin.jvm.JvmInline

/// The `payload` element of [CodecPresentIfLengthRef], 0 to 32 bytes, held to that bound where a
/// value of it is made (docs/adr/0077 §3, docs/adr/0081 §2): its constructor is
/// private and [of] makes a value only of bytes within the bound. The value it
/// holds cannot change after, so a value holds its bound for as long as it lives.
@JvmInline
value class CodecPresentIfLengthRefPayloadBytes private constructor(val bytes: ByteArray) {
    companion object {
        /// The fewest bytes a value holds.
        const val LEAST: Int = 0

        /// The most bytes a value holds.
        const val MOST: Int = 32

        /// `bytes` as a value of the bound, or `null` where it lies outside it.
        fun of(bytes: ByteArray): CodecPresentIfLengthRefPayloadBytes? =
            if (bytes.size <= MOST) CodecPresentIfLengthRefPayloadBytes(bytes) else null

        /// The fewest bytes the bound holds, each zero: what a value made by
        /// default holds.
        fun fewest(): CodecPresentIfLengthRefPayloadBytes = CodecPresentIfLengthRefPayloadBytes(ByteArray(LEAST))
    }
}

/// The `flags_has_payload` group of [CodecPresentIfLengthRef]: the arm the value holds, each the
/// members present while the gates hold its values (docs/adr/0065 §6 R4).
sealed class CodecPresentIfLengthRefFlagsHasPayload {
    /// The `flags_has_payload` group of [CodecPresentIfLengthRef] where `flags.has_payload` holds (docs/adr/0065 §6 R4): the
    /// members present then.
    data class HasPayload(
        val payload: CodecPresentIfLengthRefPayloadBytes = CodecPresentIfLengthRefPayloadBytes.fewest()
    ) : CodecPresentIfLengthRefFlagsHasPayload()
    /// The `flags_has_payload` group of [CodecPresentIfLengthRef] where `flags.has_payload` does not hold (docs/adr/0065 §6 R4): the
    /// members present then.
    data class NotHasPayload(
        val payload_size: UByte = 0.toUByte()
    ) : CodecPresentIfLengthRefFlagsHasPayload()
}

/// The elements a value of [CodecPresentIfLengthRef] presents, which its encode reads
/// (docs/adr/0081 §2): any object that presents them is a source of it, which
/// [CodecPresentIfLengthRef.encodeSource] writes. The encode derives every other element
/// of the frame.
interface CodecPresentIfLengthRefSource {
    val flags: UByte
    val flags_has_payload: CodecPresentIfLengthRefFlagsHasPayload
}

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecPresentIfLengthRef()` before any encode()/decode() call. Each default
// is a value of that field's own type, which decode() then fills in on
// success — the carrier's zero for a number, and for an enum the first
// variant its document declares, since a closed set does not hold a
// value it never declared.
data class CodecPresentIfLengthRef(
    override var flags: UByte = 0.toUByte(),
    override var flags_has_payload: CodecPresentIfLengthRefFlagsHasPayload = CodecPresentIfLengthRefFlagsHasPayload.NotHasPayload()
) : CodecPresentIfLengthRefSource {
    // RFC §synth-5-B flags primitive: per-bit-range accessors over
    // the carrier field. Single-bit (width=1) reads as Boolean; multi-
    // bit (width>=2) reads as the smallest unsigned Kotlin type that
    // fits (UByte / UShort / UInt / ULong). UByte/UShort widen through
    // `.toInt()` and UInt/ULong through `.toLong()` for the bitwise
    // ops; the result narrows back via the carrier's `toU*` ctor.
    fun hasPayload(): Boolean = (this.flags.toInt() and 0x01) != 0

    fun setHasPayload(v: Boolean) {
        this.flags = if (v) {
            (this.flags.toInt() or 0x01).toUByte()
        } else {
            (this.flags.toInt() and 0x01.inv()).toUByte()
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
        /// Write any source of [CodecPresentIfLengthRef] into `w`: each element read once off
        /// `src` and written where the codec's encode plan lays it out
        /// (docs/adr/0081). It derives `payload_size` where what it measures is there, and reads what
        /// the source states where none is. It refuses no value: it answers `null`, or
        /// [CodecError.BufferOverflow] from a bounded sink with no room left.
        fun encodeSource(src: CodecPresentIfLengthRefSource, w: SceSink): CodecError? {
            val _e_flags = src.flags
            val _g_flags_has_payload = src.flags_has_payload
            val _e_payload = when (val _g = _g_flags_has_payload) { is CodecPresentIfLengthRefFlagsHasPayload.HasPayload -> _g.payload; else -> null }
            val _e_payload_size = when (val _g = _g_flags_has_payload) { is CodecPresentIfLengthRefFlagsHasPayload.NotHasPayload -> _g.payload_size; else -> null }
            val _d_payload_size = if (_e_payload != null) _e_payload.bytes.size.toUByte() else (_e_payload_size ?: 0.toUByte())
            var _u_flags = _e_flags.toUInt()
            _u_flags = (_u_flags and 0x1u.inv()) or (if (_e_payload != null) 0x1u else 0x0u)
            val _c_flags = _u_flags.toUByte()
            w.writeU8(_c_flags.toByte())?.let { return it }
            w.writeU8(_d_payload_size.toByte())?.let { return it }
            if (_e_payload != null) {
                w.writeBytes(_e_payload.bytes)?.let { return it }
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
        fun decode(cursor: SceCursor): Decoded<CodecPresentIfLengthRef> {
            // Streaming cursor decode (SSOT selection: `needs_streaming`).
            // The positional `raw[byte_off]` path is valid only when every
            // field's absolute offset is fixed at codegen time; this branch
            // handles every codec where it is not — present-if-gated fields
            // (runtime presence), VLE / repeat / TLV-chain / embed fields
            // (runtime width), string fields (UTF-8 decode), and a fixed
            // field after a variable-length payload (offset depends on the
            // payload length). Each field reads its own bytes from the
            // cursor and advances past what it consumed. Per-field
            // `is_repeat` / `is_tlv_chain` / `is_embed` route to their
            // dedicated helpers; every other field flows through
            // `present_if_decode_stmt`.
            val flags = run {
                val raw = cursor.peekSlice(1) ?: return Decoded.Refused(CodecError.NeedMoreBytes)
                val _v = raw[0].toUByte()
                if (!cursor.advance(1)) return Decoded.Refused(CodecError.NeedMoreBytes)
                _v
            }
            val payload_size = run {
                val raw = cursor.peekSlice(1) ?: return Decoded.Refused(CodecError.NeedMoreBytes)
                val _v = raw[0].toUByte()
                if (!cursor.advance(1)) return Decoded.Refused(CodecError.NeedMoreBytes)
                _v
            }
            val payload = if ((flags.toInt() and 0x01) != 0) {
                val _len = payload_size
                if (_len > 32u) {
                    return Decoded.Refused(CodecError.LengthOutOfRange)
                }
                val _n = _len.toInt()
                val raw = cursor.peekSlice(_n) ?: return Decoded.Refused(CodecError.NeedMoreBytes)
                val _v = raw.copyOf()
                if (!cursor.advance(_n)) return Decoded.Refused(CodecError.NeedMoreBytes)
                _v
            } else {
                null
            }
            return Decoded.Ok(
                CodecPresentIfLengthRef(
                    flags = flags,
                    flags_has_payload = if (((flags.toUInt() shr 0) and 0x1u) != 0x0u) CodecPresentIfLengthRefFlagsHasPayload.HasPayload(payload = CodecPresentIfLengthRefPayloadBytes.of(payload!!) ?: return Decoded.Refused(CodecError.LengthOutOfRange)) else CodecPresentIfLengthRefFlagsHasPayload.NotHasPayload(payload_size = payload_size)
                )
            )
        }
    }
}
