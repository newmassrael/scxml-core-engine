// SCE-MAP: codec_qos_byte:15

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_qos_byte

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecQosByte()` before any encode()/decode() call. Defaults
// mirror the zero-initialized shape that decode() fills in on success.
data class CodecQosByte(
    var qos: UByte = 0.toUByte()
) {
    // RFC §synth-5-B flags primitive: per-bit-range accessors over
    // the carrier field. Single-bit (width=1) reads as Boolean; multi-
    // bit (width>=2) reads as the smallest unsigned Kotlin type that
    // fits (UByte / UShort / UInt / ULong). UByte/UShort widen through
    // `.toInt()` and UInt/ULong through `.toLong()` for the bitwise
    // ops; the result narrows back via the carrier's `toU*` ctor.
    fun priority(): UByte {
        val _carrier = this.qos.toInt()
        return ((_carrier shr 0) and 0x07).toUByte()
    }

    fun setPriority(v: UByte) {
        val _carrier = this.qos.toInt()
        val _shifted_mask = 0x07 shl 0
        val _val = (v.toInt() and 0x07) shl 0
        this.qos = ((_carrier and _shifted_mask.inv()) or _val).toUByte()
    }

    fun reliable(): Boolean = (this.qos.toInt() and 0x08) != 0

    fun setReliable(v: Boolean) {
        this.qos = if (v) {
            (this.qos.toInt() or 0x08).toUByte()
        } else {
            (this.qos.toInt() and 0x08.inv()).toUByte()
        }
    }

    fun congestion(): UByte {
        val _carrier = this.qos.toInt()
        return ((_carrier shr 4) and 0x03).toUByte()
    }

    fun setCongestion(v: UByte) {
        val _carrier = this.qos.toInt()
        val _shifted_mask = 0x03 shl 4
        val _val = (v.toInt() and 0x03) shl 4
        this.qos = ((_carrier and _shifted_mask.inv()) or _val).toUByte()
    }

    fun express(): Boolean = (this.qos.toInt() and 0x40) != 0

    fun setExpress(v: Boolean) {
        this.qos = if (v) {
            (this.qos.toInt() or 0x40).toUByte()
        } else {
            (this.qos.toInt() and 0x40.inv()).toUByte()
        }
    }

    fun reserved(): Boolean = (this.qos.toInt() and 0x80) != 0

    fun setReserved(v: Boolean) {
        this.qos = if (v) {
            (this.qos.toInt() or 0x80).toUByte()
        } else {
            (this.qos.toInt() and 0x80.inv()).toUByte()
        }
    }

    /// RFC §synth-5-B encode-side primary: write `self` into the
    /// caller-owned `w` sink. Returns `null` on success;
    /// `CodecError.BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable
    /// sinks (e.g. `MutableListSink`) are effectively infallible.
    fun encode(w: SceSink): CodecError? {
        w.writeU8(qos.toByte())?.let { return it }
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
        fun decode(cursor: SceCursor): CodecQosByte? {
            val raw = cursor.peekSlice(1) ?: return null
            val qos = raw[0].toUByte()
            if (!cursor.advance(1)) return null
            // Construct in the `return` (no intermediate local) so a field
            // literally named `value` cannot shadow a result-struct local.
            return CodecQosByte(
                qos = qos
            )
        }
    }
}
