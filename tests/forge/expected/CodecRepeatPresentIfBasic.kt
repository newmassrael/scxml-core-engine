// SCE-MAP: codec_repeat_present_if_basic:37

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_repeat_present_if_basic

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink
import com.sce.generated.codec_repeat_elem.*

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecRepeatPresentIfBasic()` before any encode()/decode() call. Defaults
// mirror the zero-initialized shape that decode() fills in on success.
data class CodecRepeatPresentIfBasic(
    var carrier: UByte = 0.toUByte(),
    var num_elems: UByte? = null,
    var elems: MutableList<CodecRepeatElem>? = null
) {
    // RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors over
    // the carrier field. Single-bit (width=1) reads as Boolean; multi-
    // bit (width>=2) reads as the smallest unsigned Kotlin type that
    // fits (UByte / UShort / UInt / ULong). UByte/UShort widen through
    // `.toInt()` and UInt/ULong through `.toLong()` for the bitwise
    // ops; the result narrows back via the carrier's `toU*` ctor.
    fun hasList(): Boolean = (this.carrier.toInt() and 0x01) != 0

    fun setHasList(v: Boolean) {
        this.carrier = if (v) {
            (this.carrier.toInt() or 0x01).toUByte()
        } else {
            (this.carrier.toInt() and 0x01.inv()).toUByte()
        }
    }

    /// RFC §5.B B1-α encode-side primary: write `self` into the
    /// caller-owned `w` sink. Returns `null` on success;
    /// `CodecError.BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable
    /// sinks (e.g. `MutableListSink`) are effectively infallible.
    fun encode(w: SceSink): CodecError? {
        // RFC §5.B B1-δ + B2-β present-if encode: per-field byte
        // append. Gated fields skip the append when the optional is
        // null. Per-field `is_repeat` routes Repeat fields to the
        // dedicated helper. Branch fires before has_vle_fields so a
        // codec mixing VLE + present-if uses the unified encode path.
        w.writeU8(this.carrier.toByte())?.let { return it }
        this.num_elems?.let { _v ->
            w.writeU8(_v.toByte())?.let { return it }
        }
        this.elems?.let { _list ->
            for (_e in _list) {
                _e.encode(w)?.let { return it }
            }
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
        /// (RFC §5.B L494-519).
        fun decode(cursor: SceCursor): CodecRepeatPresentIfBasic? {
            // RFC §5.B B1-δ + B2-β present-if primitive: streaming
            // decode advances the cursor per field. Gated fields wrap
            // their read inside an `if predicate ... else null` block.
            // B2-β extends gating to Tail / LengthRef / Vle bit-sizes
            // via dispatch inside `present_if_decode_stmt`. Per-field
            // `is_repeat` routes Repeat fields to the dedicated
            // helper. Branch fires before has_vle_fields so a codec
            // mixing VLE + present-if uses the unified streaming path.
            val carrier = run {
                val raw = cursor.peekSlice(1) ?: return null
                val _v = raw[0].toUByte()
                if (!cursor.advance(1)) return null
                _v
            }
            val num_elems = if ((carrier.toInt() and 0x01) != 0) {
                val raw = cursor.peekSlice(1) ?: return null
                val _v = raw[0].toUByte()
                if (!cursor.advance(1)) return null
                _v
            } else {
                null
            }
            val elems: MutableList<CodecRepeatElem>? = if ((carrier.toInt() and 0x01) != 0) {
                val _n = num_elems!!
                mutableListOf<CodecRepeatElem>().apply {
                    repeat(_n.toInt()) {
                        add(CodecRepeatElem.decode(cursor) ?: return null)
                    }
                }
            } else null
            return CodecRepeatPresentIfBasic(
                carrier = carrier,
                num_elems = num_elems,
                elems = elems
            )
        }
    }
}
