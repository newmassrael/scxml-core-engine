// SCE-MAP: codec_zenoh_scout:41 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_zenoh_scout

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.Decoded
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink
import kotlin.jvm.JvmInline

/// The `zid` element of [CodecZenohScout], 1 to 16 bytes, held to that bound where a
/// value of it is made (docs/adr/0077 §3, docs/adr/0081 §2): its constructor is
/// private and [of] makes a value only of bytes within the bound. The value it
/// holds cannot change after, so a value holds its bound for as long as it lives.
@JvmInline
value class CodecZenohScoutZidBytes private constructor(val bytes: ByteArray) {
    companion object {
        /// The fewest bytes a value holds.
        const val LEAST: Int = 1

        /// The most bytes a value holds.
        const val MOST: Int = 16

        /// `bytes` as a value of the bound, or `null` where it lies outside it.
        fun of(bytes: ByteArray): CodecZenohScoutZidBytes? =
            if (bytes.size in LEAST..MOST) CodecZenohScoutZidBytes(bytes) else null

        /// The fewest bytes the bound holds, each zero: what a value made by
        /// default holds.
        fun fewest(): CodecZenohScoutZidBytes = CodecZenohScoutZidBytes(ByteArray(LEAST))
    }
}

/// The elements a value of [CodecZenohScout] presents, which its encode reads
/// (docs/adr/0081 §2): any object that presents them is a source of it, which
/// [CodecZenohScout.encodeSource] writes. The encode derives every other element
/// of the frame.
interface CodecZenohScoutSource {
    val version: UByte
    val cbyte: UByte
    val zid: CodecZenohScoutZidBytes?
}

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecZenohScout()` before any encode()/decode() call. Each default
// is a value of that field's own type, which decode() then fills in on
// success — the carrier's zero for a number, and for an enum the first
// variant its document declares, since a closed set does not hold a
// value it never declared.
data class CodecZenohScout(
    override var version: UByte = 0.toUByte(),
    override var cbyte: UByte = 0.toUByte(),
    override var zid: CodecZenohScoutZidBytes? = null
) : CodecZenohScoutSource {
    // RFC §synth-5-B flags primitive: per-bit-range accessors over
    // the carrier field. Single-bit (width=1) reads as Boolean; multi-
    // bit (width>=2) reads as the smallest unsigned Kotlin type that
    // fits (UByte / UShort / UInt / ULong). UByte/UShort widen through
    // `.toInt()` and UInt/ULong through `.toLong()` for the bitwise
    // ops; the result narrows back via the carrier's `toU*` ctor.
    fun what(): UByte {
        val _carrier = this.cbyte.toInt()
        return ((_carrier shr 0) and 0x07).toUByte()
    }

    fun setWhat(v: UByte) {
        val _carrier = this.cbyte.toInt()
        val _shifted_mask = 0x07 shl 0
        val _val = (v.toInt() and 0x07) shl 0
        this.cbyte = ((_carrier and _shifted_mask.inv()) or _val).toUByte()
    }

    fun I(): Boolean = (this.cbyte.toInt() and 0x08) != 0

    fun setI(v: Boolean) {
        this.cbyte = if (v) {
            (this.cbyte.toInt() or 0x08).toUByte()
        } else {
            (this.cbyte.toInt() and 0x08.inv()).toUByte()
        }
    }

    fun zidLenM1(): UByte {
        val _carrier = this.cbyte.toInt()
        return ((_carrier shr 4) and 0x0F).toUByte()
    }

    fun setZidLenM1(v: UByte) {
        val _carrier = this.cbyte.toInt()
        val _shifted_mask = 0x0F shl 4
        val _val = (v.toInt() and 0x0F) shl 4
        this.cbyte = ((_carrier and _shifted_mask.inv()) or _val).toUByte()
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
        /// Write any source of [CodecZenohScout] into `w`: each element read once off
        /// `src` and written where the codec's encode plan lays it out
        /// (docs/adr/0081). It refuses no value: it answers `null`, or
        /// [CodecError.BufferOverflow] from a bounded sink with no room left.
        fun encodeSource(src: CodecZenohScoutSource, w: SceSink): CodecError? {
            val _e_version = src.version
            val _e_cbyte = src.cbyte
            val _e_zid = src.zid
            var _u_cbyte = _e_cbyte.toUInt()
            _u_cbyte = (_u_cbyte and 0x8u.inv()) or (if (_e_zid != null) 0x8u else 0x0u)
            if (_e_zid != null) {
                _u_cbyte = (_u_cbyte and 0xF0u.inv()) or ((_e_zid.bytes.size - 1).toUInt() shl 4)
            }
            val _c_cbyte = _u_cbyte.toUByte()
            w.writeU8(_e_version.toByte())?.let { return it }
            w.writeU8(_c_cbyte.toByte())?.let { return it }
            if (_e_zid != null) {
                w.writeBytes(_e_zid.bytes)?.let { return it }
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
        fun decode(cursor: SceCursor): Decoded<CodecZenohScout> {
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
            val version = run {
                val raw = cursor.peekSlice(1) ?: return Decoded.Refused(CodecError.NeedMoreBytes)
                val _v = raw[0].toUByte()
                if (!cursor.advance(1)) return Decoded.Refused(CodecError.NeedMoreBytes)
                _v
            }
            val cbyte = run {
                val raw = cursor.peekSlice(1) ?: return Decoded.Refused(CodecError.NeedMoreBytes)
                val _v = raw[0].toUByte()
                if (!cursor.advance(1)) return Decoded.Refused(CodecError.NeedMoreBytes)
                _v
            }
            val zid = if ((cbyte.toInt() and 0x08) != 0) {
                val _len = ((cbyte.toInt() shr 4) and 0xF)
                val _n = _len + 1
                val raw = cursor.peekSlice(_n) ?: return Decoded.Refused(CodecError.NeedMoreBytes)
                val _v = raw.copyOf()
                if (!cursor.advance(_n)) return Decoded.Refused(CodecError.NeedMoreBytes)
                _v
            } else {
                null
            }
            return Decoded.Ok(
                CodecZenohScout(
                    version = version,
                    cbyte = cbyte,
                    zid = zid?.let { CodecZenohScoutZidBytes.of(it) ?: return Decoded.Refused(CodecError.LengthOutOfRange) }
                )
            )
        }
    }
}
