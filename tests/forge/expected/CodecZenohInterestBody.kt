// SCE-MAP: codec_zenoh_interest_body:56

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_zenoh_interest_body

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink
import com.sce.generated.codec_zenoh_wireexpr.*

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecZenohInterestBody()` before any encode()/decode() call. Defaults
// mirror the zero-initialized shape that decode() fills in on success.
data class CodecZenohInterestBody(
    var header: UByte = 0.toUByte(),
    var keyexpr: CodecZenohWireexpr? = null
) {
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
        // Streaming cursor encode (SSOT selection: `needs_streaming`).
        // Mirrors the streaming decode: every field appends its own bytes
        // in declaration order through the per-field encode blocks, so a
        // gated field skips its append when null, and a fixed field after
        // a variable-length payload lands after the payload (the positional
        // path appends variable fields last, placing it ahead on the wire).
        // Per-field `is_repeat` / `is_tlv_chain` / `is_embed` route to their
        // dedicated helpers; everything else uses `present_if_encode_block`.
        w.writeU8(this.header.toByte())?.let { return it }
        this.keyexpr?.let { _v ->
            _v.encode(w, (((this.header.toInt() shr 5) and 0x1).toUByte()))?.let { return it }
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
        /// (RFC §synth-5-B L494-519).
        fun decode(cursor: SceCursor): CodecZenohInterestBody? {
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
                val raw = cursor.peekSlice(1) ?: return null
                val _v = raw[0].toUByte()
                if (!cursor.advance(1)) return null
                _v
            }
            val keyexpr: CodecZenohWireexpr? = if ((header.toInt() and 0x10) != 0) {
                CodecZenohWireexpr.decode(cursor, (((header.toInt() shr 5) and 0x1).toUByte())) ?: return null
            } else {
                null
            }
            return CodecZenohInterestBody(
                header = header,
                keyexpr = keyexpr
            )
        }
    }
}
