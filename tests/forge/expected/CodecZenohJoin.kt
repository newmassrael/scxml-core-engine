// SCE-MAP: codec_zenoh_join:41 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_zenoh_join

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecZenohJoin()` before any encode()/decode() call. Defaults
// mirror the zero-initialized shape that decode() fills in on success.
data class CodecZenohJoin(
    var version: UByte = 0.toUByte(),
    var cbyte: UByte = 0.toUByte(),
    var zid: ByteArray = byteArrayOf(),
    var sn_res: UByte? = null,
    var batch_size: UShort? = null,
    var lease: ULong = 0uL,
    var next_sn_reliable: ULong = 0uL,
    var next_sn_best_effort: ULong = 0uL
) {
    // RFC §synth-5-B flags primitive: per-bit-range accessors over
    // the carrier field. Single-bit (width=1) reads as Boolean; multi-
    // bit (width>=2) reads as the smallest unsigned Kotlin type that
    // fits (UByte / UShort / UInt / ULong). UByte/UShort widen through
    // `.toInt()` and UInt/ULong through `.toLong()` for the bitwise
    // ops; the result narrows back via the carrier's `toU*` ctor.
    fun whatami(): UByte {
        val _carrier = this.cbyte.toInt()
        return ((_carrier shr 0) and 0x03).toUByte()
    }

    fun setWhatami(v: UByte) {
        val _carrier = this.cbyte.toInt()
        val _shifted_mask = 0x03 shl 0
        val _val = (v.toInt() and 0x03) shl 0
        this.cbyte = ((_carrier and _shifted_mask.inv()) or _val).toUByte()
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
    @Suppress("UNUSED_PARAMETER")
    fun encode(w: SceSink, S: UByte): CodecError? {
        // Streaming cursor encode (SSOT selection: `needs_streaming`).
        // Mirrors the streaming decode: every field appends its own bytes
        // in declaration order through the per-field encode blocks, so a
        // gated field skips its append when null, and a fixed field after
        // a variable-length payload lands after the payload (the positional
        // path appends variable fields last, placing it ahead on the wire).
        // Per-field `is_repeat` / `is_tlv_chain` / `is_embed` route to their
        // dedicated helpers; everything else uses `present_if_encode_block`.
        w.writeU8(this.version.toByte())?.let { return it }
        w.writeU8(this.cbyte.toByte())?.let { return it }
        w.writeBytes(this.zid)?.let { return it }
        this.sn_res?.let { _v ->
            w.writeU8(_v.toByte())?.let { return it }
        }
        this.batch_size?.let { _v ->
            w.writeU8((_v.toInt() and 0xFF).toByte())?.let { return it }
            w.writeU8((_v.toInt() ushr 8 and 0xFF).toByte())?.let { return it }
        }
        w.writeVleU64((lease).toULong())?.let { return it }
        w.writeVleU64((next_sn_reliable).toULong())?.let { return it }
        w.writeVleU64((next_sn_best_effort).toULong())?.let { return it }
        return null
    }

    /// Heap-backed convenience facade. Runs `encode` over a
    /// `MutableListSink` and returns the freshly-encoded ByteArray.
    /// Callers targeting zero-alloc hot paths should call `encode`
    /// directly against a caller-owned sink (e.g. `ByteArraySink`).
    fun encodeToByteArray(S: UByte): ByteArray {
        val _list = mutableListOf<Byte>()
        encode(MutableListSink(_list), S)
        return _list.toByteArray()
    }

    companion object {
        /// Decode the next frame from `cursor`. On success the cursor
        /// advances past the consumed bytes; returns `null` when the
        /// cursor's tail is shorter than the declared minimum frame
        /// (RFC §synth-5-B L494-519).
        @Suppress("UNUSED_PARAMETER")
        fun decode(cursor: SceCursor, S: UByte): CodecZenohJoin? {
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
                val raw = cursor.peekSlice(1) ?: return null
                val _v = raw[0].toUByte()
                if (!cursor.advance(1)) return null
                _v
            }
            val cbyte = run {
                val raw = cursor.peekSlice(1) ?: return null
                val _v = raw[0].toUByte()
                if (!cursor.advance(1)) return null
                _v
            }
            val zid = run {
                val _n = (((cbyte.toInt() shr 4) and 0xF) + 1)
                val raw = cursor.peekSlice(_n) ?: return null
                val _v = raw.copyOf()
                if (!cursor.advance(_n)) return null
                _v
            }
            val sn_res = if ((S.toInt() and 0x01) != 0) {
                val raw = cursor.peekSlice(1) ?: return null
                val _v = raw[0].toUByte()
                if (!cursor.advance(1)) return null
                _v
            } else {
                null
            }
            val batch_size = if ((S.toInt() and 0x01) != 0) {
                val raw = cursor.peekSlice(2) ?: return null
                val _v = ((raw[0].toInt() and 0xFF) or ((raw[1].toInt() and 0xFF) shl 8)).toUShort()
                if (!cursor.advance(2)) return null
                _v
            } else {
                null
            }
            val lease = cursor.readVleU64() ?: return null
            val next_sn_reliable = cursor.readVleU64() ?: return null
            val next_sn_best_effort = cursor.readVleU64() ?: return null
            return CodecZenohJoin(
                version = version,
                cbyte = cbyte,
                zid = zid,
                sn_res = sn_res,
                batch_size = batch_size,
                lease = lease,
                next_sn_reliable = next_sn_reliable,
                next_sn_best_effort = next_sn_best_effort
            )
        }
    }
}
