// SCE-MAP: codec_zenoh_source_info:57 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_zenoh_source_info

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.Decoded
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink
import kotlin.jvm.JvmInline

/// The `zid` element of [CodecZenohSourceInfo], 1 to 16 bytes, held to that bound where a
/// value of it is made (docs/adr/0077 §3, docs/adr/0081 §2): its constructor is
/// private and [of] makes a value only of bytes within the bound. The value it
/// holds cannot change after, so a value holds its bound for as long as it lives.
@JvmInline
value class CodecZenohSourceInfoZidBytes private constructor(val bytes: ByteArray) {
    companion object {
        /// The fewest bytes a value holds.
        const val LEAST: Int = 1

        /// The most bytes a value holds.
        const val MOST: Int = 16

        /// `bytes` as a value of the bound, or `null` where it lies outside it.
        fun of(bytes: ByteArray): CodecZenohSourceInfoZidBytes? =
            if (bytes.size in LEAST..MOST) CodecZenohSourceInfoZidBytes(bytes) else null

        /// The fewest bytes the bound holds, each zero: what a value made by
        /// default holds.
        fun fewest(): CodecZenohSourceInfoZidBytes = CodecZenohSourceInfoZidBytes(ByteArray(LEAST))
    }
}

/// The elements a value of [CodecZenohSourceInfo] presents, which its encode reads
/// (docs/adr/0081 §2): any object that presents them is a source of it, which
/// [CodecZenohSourceInfo.encodeSource] writes. The encode derives every other element
/// of the frame.
interface CodecZenohSourceInfoSource {
    val header: UByte
    val zid: CodecZenohSourceInfoZidBytes
    val eid: UInt
    val sn: UInt
}

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecZenohSourceInfo()` before any encode()/decode() call. Each default
// is a value of that field's own type, which decode() then fills in on
// success — the carrier's zero for a number, and for an enum the first
// variant its document declares, since a closed set does not hold a
// value it never declared.
data class CodecZenohSourceInfo(
    override var header: UByte = 0.toUByte(),
    override var zid: CodecZenohSourceInfoZidBytes = CodecZenohSourceInfoZidBytes.fewest(),
    override var eid: UInt = 0u,
    override var sn: UInt = 0u
) : CodecZenohSourceInfoSource {
    // RFC §synth-5-B flags primitive: per-bit-range accessors over
    // the carrier field. Single-bit (width=1) reads as Boolean; multi-
    // bit (width>=2) reads as the smallest unsigned Kotlin type that
    // fits (UByte / UShort / UInt / ULong). UByte/UShort widen through
    // `.toInt()` and UInt/ULong through `.toLong()` for the bitwise
    // ops; the result narrows back via the carrier's `toU*` ctor.
    fun zidlenM1(): UByte {
        val _carrier = this.header.toInt()
        return ((_carrier shr 4) and 0x0F).toUByte()
    }

    fun setZidlenM1(v: UByte) {
        val _carrier = this.header.toInt()
        val _shifted_mask = 0x0F shl 4
        val _val = (v.toInt() and 0x0F) shl 4
        this.header = ((_carrier and _shifted_mask.inv()) or _val).toUByte()
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
        /// Write any source of [CodecZenohSourceInfo] into `w`: each element read once off
        /// `src` and written where the codec's encode plan lays it out
        /// (docs/adr/0081). It refuses no value: it answers `null`, or
        /// [CodecError.BufferOverflow] from a bounded sink with no room left.
        fun encodeSource(src: CodecZenohSourceInfoSource, w: SceSink): CodecError? {
            val _e_header = src.header
            val _e_zid = src.zid
            val _e_eid = src.eid
            val _e_sn = src.sn
            var _u_header = _e_header.toUInt()
            _u_header = (_u_header and 0xF0u.inv()) or ((_e_zid.bytes.size - 1).toUInt() shl 4)
            val _c_header = _u_header.toUByte()
            w.writeU8(_c_header.toByte())?.let { return it }
            w.writeBytes(_e_zid.bytes)?.let { return it }
            w.writeVleU32(_e_eid)?.let { return it }
            w.writeVleU32(_e_sn)?.let { return it }
            return null
        }

        /// Decode the next frame from `cursor`. On success the cursor
        /// advances past the consumed bytes and the value is `Decoded.Ok`;
        /// a refused frame is `Decoded.Refused` with the kind that refused
        /// it (docs/adr/0099, amendment of 2026-11-16): `NeedMoreBytes` when
        /// the cursor's tail is shorter than the frame (RFC §synth-5-B
        /// L494-519), which the caller resumes after appending bytes, and
        /// another kind for a frame no more bytes make whole.
        fun decode(cursor: SceCursor): Decoded<CodecZenohSourceInfo> {
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
            val header = run {
                val raw = cursor.peekSlice(1) ?: return Decoded.Refused(CodecError.NeedMoreBytes)
                val _v = raw[0].toUByte()
                if (!cursor.advance(1)) return Decoded.Refused(CodecError.NeedMoreBytes)
                _v
            }
            val zid = run {
                val _len = ((header.toInt() shr 4) and 0xF)
                val _n = _len + 1
                val raw = cursor.peekSlice(_n) ?: return Decoded.Refused(CodecError.NeedMoreBytes)
                val _v = raw.copyOf()
                if (!cursor.advance(_n)) return Decoded.Refused(CodecError.NeedMoreBytes)
                _v
            }
            val eid = cursor.readVleU32().valueOr { _refused -> return _refused }
            val sn = cursor.readVleU32().valueOr { _refused -> return _refused }
            return Decoded.Ok(
                CodecZenohSourceInfo(
                    header = header,
                    zid = CodecZenohSourceInfoZidBytes.of(zid) ?: return Decoded.Refused(CodecError.LengthOutOfRange),
                    eid = eid,
                    sn = sn
                )
            )
        }
    }
}
