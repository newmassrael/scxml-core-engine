// SCE-MAP: codec_chain_has_tagged_entry:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_chain_has_tagged_entry

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecChainHasTaggedEntry()` before any encode()/decode() call. Each default
// is a value of that field's own type, which decode() then fills in on
// success — the carrier's zero for a number, and for an enum the first
// variant its document declares, since a closed set does not hold a
// value it never declared.
data class CodecChainHasTaggedEntry(
    var entry_type: UByte = 0.toUByte(),
    var ctl: UByte = 0.toUByte(),
    var body: UByte = 0.toUByte()
) {
    // RFC §synth-5-B flags primitive: per-bit-range accessors over
    // the carrier field. Single-bit (width=1) reads as Boolean; multi-
    // bit (width>=2) reads as the smallest unsigned Kotlin type that
    // fits (UByte / UShort / UInt / ULong). UByte/UShort widen through
    // `.toInt()` and UInt/ULong through `.toLong()` for the bitwise
    // ops; the result narrows back via the carrier's `toU*` ctor.
    fun more(): Boolean = (this.ctl.toInt() and 0x01) != 0

    fun setMore(v: Boolean) {
        this.ctl = if (v) {
            (this.ctl.toInt() or 0x01).toUByte()
        } else {
            (this.ctl.toInt() and 0x01.inv()).toUByte()
        }
    }

    /// RFC §synth-5-B encode-side primary: write `self` into the
    /// caller-owned `w` sink. Returns `null` on success;
    /// `CodecError.BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable
    /// sinks (e.g. `MutableListSink`) are effectively infallible.
    fun encode(w: SceSink): CodecError? {
        w.writeU8(entry_type.toByte())?.let { return it }
        w.writeU8(ctl.toByte())?.let { return it }
        w.writeU8(body.toByte())?.let { return it }
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
        fun decode(cursor: SceCursor): CodecChainHasTaggedEntry? {
            val raw = cursor.peekSlice(3) ?: return null
            val entry_type = raw[0].toUByte()
            val ctl = raw[1].toUByte()
            val body = raw[2].toUByte()
            if (!cursor.advance(3)) return null
            // Construct in the `return` (no intermediate local) so a field
            // literally named `value` cannot shadow a result-struct local.
            return CodecChainHasTaggedEntry(
                entry_type = entry_type,
                ctl = ctl,
                body = body
            )
        }
    }
}
