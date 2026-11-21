// SCE-MAP: codec_qos_byte:15 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_qos_byte

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.Decoded
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink

/// One-bit flags of [CodecQosByte] a parent may set in place of the ones a value
/// presents, such as the flag a chain whose entries are [CodecQosByte] ends them
/// on, which the chain derives from where each entry stands.
class CodecQosByteOverrides(
    /// The `reliable` flag of `qos`, where a parent sets it.
    val qos_reliable: Boolean? = null,
    /// The `express` flag of `qos`, where a parent sets it.
    val qos_express: Boolean? = null,
    /// The `reserved` flag of `qos`, where a parent sets it.
    val qos_reserved: Boolean? = null,
)

/// The elements a value of [CodecQosByte] presents, which its encode reads
/// (docs/adr/0081 §2): any object that presents them is a source of it, which
/// [CodecQosByte.encodeSource] writes. The encode derives every other element
/// of the frame.
interface CodecQosByteSource {
    val qos: UByte
}

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecQosByte()` before any encode()/decode() call. Each default
// is a value of that field's own type, which decode() then fills in on
// success — the carrier's zero for a number, and for an enum the first
// variant its document declares, since a closed set does not hold a
// value it never declared.
data class CodecQosByte(
    override var qos: UByte = 0.toUByte()
) : CodecQosByteSource {
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
        /// Write any source of [CodecQosByte] into `w`: each element read once off
        /// `src` and written where the codec's encode plan lays it out
        /// (docs/adr/0081). A chain that holds it
        /// sets the flag its entries end on by `overrides`, in place of the value's. It refuses no value: it answers `null`, or
        /// [CodecError.BufferOverflow] from a bounded sink with no room left.
        fun encodeSource(src: CodecQosByteSource, w: SceSink, overrides: CodecQosByteOverrides? = null): CodecError? {
            val _e_qos = src.qos
            var _u_qos = _e_qos.toUInt()
            overrides?.qos_reliable?.let { _u_qos = (_u_qos and 0x8u.inv()) or (if (it) 0x8u else 0x0u) }
            overrides?.qos_express?.let { _u_qos = (_u_qos and 0x40u.inv()) or (if (it) 0x40u else 0x0u) }
            overrides?.qos_reserved?.let { _u_qos = (_u_qos and 0x80u.inv()) or (if (it) 0x80u else 0x0u) }
            val _c_qos = _u_qos.toUByte()
            w.writeU8(_c_qos.toByte())?.let { return it }
            return null
        }

        /// Decode the next frame from `cursor`. On success the cursor
        /// advances past the consumed bytes and the value is `Decoded.Ok`;
        /// a refused frame is `Decoded.Refused` with the kind that refused
        /// it (docs/adr/0099, amendment of 2026-11-16): `NeedMoreBytes` when
        /// the cursor's tail is shorter than the frame (RFC
        /// §synth-5-B L494-519), which the caller resumes after appending
        /// bytes, and another kind for a frame no more bytes make whole.
        fun decode(cursor: SceCursor): Decoded<CodecQosByte> {
            val raw = cursor.peekSlice(1) ?: return Decoded.Refused(CodecError.NeedMoreBytes)
            val qos = raw[0].toUByte()
            if (!cursor.advance(1)) return Decoded.Refused(CodecError.NeedMoreBytes)
            // Construct in the `return` (no intermediate local) so a field
            // literally named `value` cannot shadow a result-struct local.
            return Decoded.Ok(
                CodecQosByte(
                    qos = qos
                )
            )
        }
    }
}
