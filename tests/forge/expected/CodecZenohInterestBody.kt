// SCE-MAP: codec_zenoh_interest_body:56 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_zenoh_interest_body

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.Decoded
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink
import com.sce.generated.codec_zenoh_wireexpr.*

/// One-bit flags of [CodecZenohInterestBody] a parent may set in place of the ones a value
/// presents, such as the flag a chain whose entries are [CodecZenohInterestBody] ends them
/// on, which the chain derives from where each entry stands.
class CodecZenohInterestBodyOverrides(
    /// The `keyexprs` flag of `header`, where a parent sets it.
    val header_keyexprs: Boolean? = null,
    /// The `subscribers` flag of `header`, where a parent sets it.
    val header_subscribers: Boolean? = null,
    /// The `queryables` flag of `header`, where a parent sets it.
    val header_queryables: Boolean? = null,
    /// The `tokens` flag of `header`, where a parent sets it.
    val header_tokens: Boolean? = null,
    /// The `M` flag of `header`, where a parent sets it.
    val header_M: Boolean? = null,
    /// The `aggregate` flag of `header`, where a parent sets it.
    val header_aggregate: Boolean? = null,
)

/// The elements a value of [CodecZenohInterestBody] presents, which its encode reads
/// (docs/adr/0081 §2): any object that presents them is a source of it, which
/// [CodecZenohInterestBody.encodeSource] writes. The encode derives every other element
/// of the frame.
interface CodecZenohInterestBodySource {
    val header: UByte
    val keyexpr: CodecZenohWireexprSource?
}

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecZenohInterestBody()` before any encode()/decode() call. Each default
// is a value of that field's own type, which decode() then fills in on
// success — the carrier's zero for a number, and for an enum the first
// variant its document declares, since a closed set does not hold a
// value it never declared.
data class CodecZenohInterestBody(
    override var header: UByte = 0.toUByte(),
    override var keyexpr: CodecZenohWireexpr? = null
) : CodecZenohInterestBodySource {
    // RFC §synth-5-B flags primitive: per-bit-range accessors over
    // the carrier field. Single-bit (width=1) reads as Boolean; multi-
    // bit (width>=2) reads as the smallest unsigned Kotlin type that
    // fits (UByte / UShort / UInt / ULong). UByte/UShort widen through
    // `.toInt()` and UInt/ULong through `.toLong()` for the bitwise
    // ops; the result narrows back via the carrier's `toU*` ctor.
    fun keyexprs(): Boolean = (this.header.toInt() and 0x01) != 0

    fun setKeyexprs(v: Boolean) {
        this.header = if (v) {
            (this.header.toInt() or 0x01).toUByte()
        } else {
            (this.header.toInt() and 0x01.inv()).toUByte()
        }
    }

    fun subscribers(): Boolean = (this.header.toInt() and 0x02) != 0

    fun setSubscribers(v: Boolean) {
        this.header = if (v) {
            (this.header.toInt() or 0x02).toUByte()
        } else {
            (this.header.toInt() and 0x02.inv()).toUByte()
        }
    }

    fun queryables(): Boolean = (this.header.toInt() and 0x04) != 0

    fun setQueryables(v: Boolean) {
        this.header = if (v) {
            (this.header.toInt() or 0x04).toUByte()
        } else {
            (this.header.toInt() and 0x04.inv()).toUByte()
        }
    }

    fun tokens(): Boolean = (this.header.toInt() and 0x08) != 0

    fun setTokens(v: Boolean) {
        this.header = if (v) {
            (this.header.toInt() or 0x08).toUByte()
        } else {
            (this.header.toInt() and 0x08.inv()).toUByte()
        }
    }

    fun restricted(): Boolean = (this.header.toInt() and 0x10) != 0

    fun setRestricted(v: Boolean) {
        this.header = if (v) {
            (this.header.toInt() or 0x10).toUByte()
        } else {
            (this.header.toInt() and 0x10.inv()).toUByte()
        }
    }

    fun N(): Boolean = (this.header.toInt() and 0x20) != 0

    fun setN(v: Boolean) {
        this.header = if (v) {
            (this.header.toInt() or 0x20).toUByte()
        } else {
            (this.header.toInt() and 0x20.inv()).toUByte()
        }
    }

    fun M(): Boolean = (this.header.toInt() and 0x40) != 0

    fun setM(v: Boolean) {
        this.header = if (v) {
            (this.header.toInt() or 0x40).toUByte()
        } else {
            (this.header.toInt() and 0x40.inv()).toUByte()
        }
    }

    fun aggregate(): Boolean = (this.header.toInt() and 0x80) != 0

    fun setAggregate(v: Boolean) {
        this.header = if (v) {
            (this.header.toInt() or 0x80).toUByte()
        } else {
            (this.header.toInt() and 0x80.inv()).toUByte()
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
        /// Write any source of [CodecZenohInterestBody] into `w`: each element read once off
        /// `src` and written where the codec's encode plan lays it out
        /// (docs/adr/0081). A chain that holds it
        /// sets the flag its entries end on by `overrides`, in place of the value's. It refuses no value: it answers `null`, or
        /// [CodecError.BufferOverflow] from a bounded sink with no room left.
        fun encodeSource(src: CodecZenohInterestBodySource, w: SceSink, overrides: CodecZenohInterestBodyOverrides? = null): CodecError? {
            val _e_header = src.header
            val _e_keyexpr = src.keyexpr
            var _u_header = _e_header.toUInt()
            _u_header = (_u_header and 0x10u.inv()) or (if (_e_keyexpr != null) 0x10u else 0x0u)
            if (_e_keyexpr != null) {
                _u_header = (_u_header and 0x20u.inv()) or ((CodecZenohWireexpr.inputN(_e_keyexpr).toInt()).toUInt() shl 5)
            }
            overrides?.header_keyexprs?.let { _u_header = (_u_header and 0x1u.inv()) or (if (it) 0x1u else 0x0u) }
            overrides?.header_subscribers?.let { _u_header = (_u_header and 0x2u.inv()) or (if (it) 0x2u else 0x0u) }
            overrides?.header_queryables?.let { _u_header = (_u_header and 0x4u.inv()) or (if (it) 0x4u else 0x0u) }
            overrides?.header_tokens?.let { _u_header = (_u_header and 0x8u.inv()) or (if (it) 0x8u else 0x0u) }
            overrides?.header_M?.let { _u_header = (_u_header and 0x40u.inv()) or (if (it) 0x40u else 0x0u) }
            overrides?.header_aggregate?.let { _u_header = (_u_header and 0x80u.inv()) or (if (it) 0x80u else 0x0u) }
            val _c_header = _u_header.toUByte()
            w.writeU8(_c_header.toByte())?.let { return it }
            if (_e_keyexpr != null) {
                CodecZenohWireexpr.encodeSource(_e_keyexpr, w)?.let { return it }
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
        fun decode(cursor: SceCursor): Decoded<CodecZenohInterestBody> {
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
            val keyexpr: CodecZenohWireexpr? = if ((header.toInt() and 0x10) != 0) {
                CodecZenohWireexpr.decode(cursor, (((header.toInt() shr 5) and 0x1).toUByte())).valueOr { _refused -> return _refused }
            } else {
                null
            }
            return Decoded.Ok(
                CodecZenohInterestBody(
                    header = header,
                    keyexpr = keyexpr
                )
            )
        }
    }
}
