// SCE-MAP: codec_little_endian:3 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_little_endian

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.Decoded
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink

/// The elements a value of [CodecLittleEndian] presents, which its encode reads
/// (docs/adr/0081 §2): any object that presents them is a source of it, which
/// [CodecLittleEndian.encodeSource] writes. The encode derives every other element
/// of the frame.
interface CodecLittleEndianSource {
    val sensorId: UByte
    val value: UShort
    val status: UByte
}

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecLittleEndian()` before any encode()/decode() call. Each default
// is a value of that field's own type, which decode() then fills in on
// success — the carrier's zero for a number, and for an enum the first
// variant its document declares, since a closed set does not hold a
// value it never declared.
data class CodecLittleEndian(
    override var sensorId: UByte = 0.toUByte(),
    override var value: UShort = 0.toUShort(),
    override var status: UByte = 0.toUByte()
) : CodecLittleEndianSource {
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
        /// Write any source of [CodecLittleEndian] into `w`: each element read once off
        /// `src` and written where the codec's encode plan lays it out
        /// (docs/adr/0081). It refuses no value: it answers `null`, or
        /// [CodecError.BufferOverflow] from a bounded sink with no room left.
        fun encodeSource(src: CodecLittleEndianSource, w: SceSink): CodecError? {
            val _e_sensorId = src.sensorId
            val _e_value = src.value
            val _e_status = src.status
            w.writeU8(_e_sensorId.toByte())?.let { return it }
            w.writeU8((_e_value.toInt() and 0xFF).toByte())?.let { return it }
            w.writeU8((_e_value.toInt() ushr 8 and 0xFF).toByte())?.let { return it }
            w.writeU8(_e_status.toByte())?.let { return it }
            return null
        }

        /// Decode the next frame from `cursor`. On success the cursor
        /// advances past the consumed bytes and the value is `Decoded.Ok`;
        /// a refused frame is `Decoded.Refused` with the kind that refused
        /// it (docs/adr/0099, amendment of 2026-11-16): `NeedMoreBytes` when
        /// the cursor's tail is shorter than the frame (RFC §synth-5-B
        /// L494-519), which the caller resumes after appending bytes, and
        /// another kind for a frame no more bytes make whole.
        fun decode(cursor: SceCursor): Decoded<CodecLittleEndian> {
            val raw = cursor.peekSlice(4) ?: return Decoded.Refused(CodecError.NeedMoreBytes)
            val sensorId = raw[0].toUByte()
            val value = ((raw[1].toInt() and 0xFF) or ((raw[2].toInt() and 0xFF) shl 8)).toUShort()
            val status = raw[3].toUByte()
            if (!cursor.advance(4)) return Decoded.Refused(CodecError.NeedMoreBytes)
            // Construct in the `return` (no intermediate local) so a field
            // literally named `value` cannot shadow a result-struct local.
            return Decoded.Ok(
                CodecLittleEndian(
                    sensorId = sensorId,
                    value = value,
                    status = status
                )
            )
        }
    }
}
