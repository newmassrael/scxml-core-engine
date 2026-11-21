// SCE-MAP: codec_zenoh_decl_ext_keyexpr:89 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_zenoh_decl_ext_keyexpr

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.Decoded
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink
import com.sce.generated.codec_zenoh_decl_ext_keyexpr_inner.*

/// One-bit flags of [CodecZenohDeclExtKeyexpr] a parent may set in place of the ones a value
/// presents, such as the flag a chain whose entries are [CodecZenohDeclExtKeyexpr] ends them
/// on, which the chain derives from where each entry stands.
class CodecZenohDeclExtKeyexprOverrides(
    /// The `M` flag of `outer_header`, where a parent sets it.
    val outer_header_M: Boolean? = null,
    /// The `Z` flag of `outer_header`, where a parent sets it.
    val outer_header_Z: Boolean? = null,
)

/// The elements a value of [CodecZenohDeclExtKeyexpr] presents, which its encode reads
/// (docs/adr/0081 §2): any object that presents them is a source of it, which
/// [CodecZenohDeclExtKeyexpr.encodeSource] writes. The encode derives every other element
/// of the frame.
interface CodecZenohDeclExtKeyexprSource {
    val outer_header: UByte
    val inner: CodecZenohDeclExtKeyexprInnerSource
}

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecZenohDeclExtKeyexpr()` before any encode()/decode() call. Each default
// is a value of that field's own type, which decode() then fills in on
// success — the carrier's zero for a number, and for an enum the first
// variant its document declares, since a closed set does not hold a
// value it never declared.
data class CodecZenohDeclExtKeyexpr(
    override var outer_header: UByte = 0.toUByte(),
    override var inner: CodecZenohDeclExtKeyexprInner = CodecZenohDeclExtKeyexprInner()
) : CodecZenohDeclExtKeyexprSource {
    // RFC §synth-5-B flags primitive: per-bit-range accessors over
    // the carrier field. Single-bit (width=1) reads as Boolean; multi-
    // bit (width>=2) reads as the smallest unsigned Kotlin type that
    // fits (UByte / UShort / UInt / ULong). UByte/UShort widen through
    // `.toInt()` and UInt/ULong through `.toLong()` for the bitwise
    // ops; the result narrows back via the carrier's `toU*` ctor.
    fun extId(): UByte {
        val _carrier = this.outer_header.toInt()
        return ((_carrier shr 0) and 0x0F).toUByte()
    }

    fun setExtId(v: UByte) {
        val _carrier = this.outer_header.toInt()
        val _shifted_mask = 0x0F shl 0
        val _val = (v.toInt() and 0x0F) shl 0
        this.outer_header = ((_carrier and _shifted_mask.inv()) or _val).toUByte()
    }

    fun M(): Boolean = (this.outer_header.toInt() and 0x10) != 0

    fun setM(v: Boolean) {
        this.outer_header = if (v) {
            (this.outer_header.toInt() or 0x10).toUByte()
        } else {
            (this.outer_header.toInt() and 0x10.inv()).toUByte()
        }
    }

    fun enc(): UByte {
        val _carrier = this.outer_header.toInt()
        return ((_carrier shr 5) and 0x03).toUByte()
    }

    fun setEnc(v: UByte) {
        val _carrier = this.outer_header.toInt()
        val _shifted_mask = 0x03 shl 5
        val _val = (v.toInt() and 0x03) shl 5
        this.outer_header = ((_carrier and _shifted_mask.inv()) or _val).toUByte()
    }

    fun Z(): Boolean = (this.outer_header.toInt() and 0x80) != 0

    fun setZ(v: Boolean) {
        this.outer_header = if (v) {
            (this.outer_header.toInt() or 0x80).toUByte()
        } else {
            (this.outer_header.toInt() and 0x80.inv()).toUByte()
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
        /// Write any source of [CodecZenohDeclExtKeyexpr] into `w`: each element read once off
        /// `src` and written where the codec's encode plan lays it out
        /// (docs/adr/0081). It derives `total_length` from what each measures, and never reads it off the
        /// source. A chain that holds it
        /// sets the flag its entries end on by `overrides`, in place of the value's. It refuses no value: it answers `null`, or
        /// [CodecError.BufferOverflow] from a bounded sink with no room left.
        fun encodeSource(src: CodecZenohDeclExtKeyexprSource, w: SceSink, overrides: CodecZenohDeclExtKeyexprOverrides? = null): CodecError? {
            val _e_outer_header = src.outer_header
            val _e_inner = src.inner
            var _u_outer_header = _e_outer_header.toUInt()
            overrides?.outer_header_M?.let { _u_outer_header = (_u_outer_header and 0x10u.inv()) or (if (it) 0x10u else 0x0u) }
            overrides?.outer_header_Z?.let { _u_outer_header = (_u_outer_header and 0x80u.inv()) or (if (it) 0x80u else 0x0u) }
            val _c_outer_header = _u_outer_header.toUByte()
            val _l_inner = mutableListOf<Byte>()
            CodecZenohDeclExtKeyexprInner.encodeSource(_e_inner, MutableListSink(_l_inner))?.let { return it }
            val _b_inner = _l_inner.toByteArray()
            val _d_total_length = _b_inner.size.toULong()
            w.writeU8(_c_outer_header.toByte())?.let { return it }
            w.writeVleU64(_d_total_length)?.let { return it }
            w.writeBytes(_b_inner)?.let { return it }
            return null
        }

        /// Decode the next frame from `cursor`. On success the cursor
        /// advances past the consumed bytes and the value is `Decoded.Ok`;
        /// a refused frame is `Decoded.Refused` with the kind that refused
        /// it (docs/adr/0099, amendment of 2026-11-16): `NeedMoreBytes` when
        /// the cursor's tail is shorter than the frame (RFC
        /// §synth-5-B L494-519), which the caller resumes after appending
        /// bytes, and another kind for a frame no more bytes make whole.
        fun decode(cursor: SceCursor): Decoded<CodecZenohDeclExtKeyexpr> {
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
            val outer_header = run {
                val raw = cursor.peekSlice(1) ?: return Decoded.Refused(CodecError.NeedMoreBytes)
                val _v = raw[0].toUByte()
                if (!cursor.advance(1)) return Decoded.Refused(CodecError.NeedMoreBytes)
                _v
            }
            val total_length = cursor.readVleU64().valueOr { _refused -> return _refused }
            val inner = run {
                val _len = (total_length).toInt()
                val _raw = cursor.peekSlice(_len) ?: return Decoded.Refused(CodecError.NeedMoreBytes)
                val _inner = SceCursor(_raw)
                val _v = CodecZenohDeclExtKeyexprInner.decode(_inner).valueOr { _refused -> return _refused }
                if (!cursor.advance(_len)) return Decoded.Refused(CodecError.NeedMoreBytes)
                _v
            }
            return Decoded.Ok(
                CodecZenohDeclExtKeyexpr(
                    outer_header = outer_header,
                    inner = inner
                )
            )
        }
    }
}
