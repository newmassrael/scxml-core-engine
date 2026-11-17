// SCE-MAP: codec_zenoh_value_entry:14 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_zenoh_value_entry

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.Decoded
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink
import com.sce.generated.codec_zenoh_ext_unit.*
import com.sce.generated.codec_zenoh_ext_zint.*
import com.sce.generated.codec_zenoh_value_zbuf.*

// RFC §synth-5-B variant primitive: discriminated-union body for the
// codec's tag-field suffix. Each arm wraps an imported codec's decoded
// value; the optional Default arm preserves the runtime tag value
// alongside its catch-all body. Arm body types are referenced by FQN
// (defensive — wildcard imports could otherwise surface an ambiguity if
// two imported codecs declare same-named inner classes).
sealed class CodecZenohValueEntryVariant {
    data class CodecZenohExtUnit(val body: com.sce.generated.codec_zenoh_ext_unit.CodecZenohExtUnit) : CodecZenohValueEntryVariant()
    data class CodecZenohExtZint(val body: com.sce.generated.codec_zenoh_ext_zint.CodecZenohExtZint) : CodecZenohValueEntryVariant()
    data class CodecZenohValueZbuf(val body: com.sce.generated.codec_zenoh_value_zbuf.CodecZenohValueZbuf) : CodecZenohValueEntryVariant()
    data class Default(val tag: UByte, val body: com.sce.generated.codec_zenoh_ext_unit.CodecZenohExtUnit) : CodecZenohValueEntryVariant()
}

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecZenohValueEntry()` before any encode()/decode() call. Each default
// is a value of that field's own type, which decode() then fills in on
// success — the carrier's zero for a number, and for an enum the first
// variant its document declares, since a closed set does not hold a
// value it never declared.
data class CodecZenohValueEntry(
    var header: UByte = 0.toUByte(),
    // RFC variant-default-uniformity (Kotlin): pick the declared
    // default arm (`<sce:arm default="true"/>`) instead of the first
    // alternative so a freshly-constructed envelope round-trips byte-
    // exactly through `encode() -> decode()`. Paired with the inner
    // codec's `<sce:flag value=>`-baked default fields above.
    var body: CodecZenohValueEntryVariant = CodecZenohValueEntryVariant.CodecZenohExtUnit(com.sce.generated.codec_zenoh_ext_unit.CodecZenohExtUnit())
) {
    // RFC §synth-5-B flags primitive: per-bit-range accessors over
    // the carrier field. Single-bit (width=1) reads as Boolean; multi-
    // bit (width>=2) reads as the smallest unsigned Kotlin type that
    // fits (UByte / UShort / UInt / ULong). UByte/UShort widen through
    // `.toInt()` and UInt/ULong through `.toLong()` for the bitwise
    // ops; the result narrows back via the carrier's `toU*` ctor.
    fun extId(): UByte {
        val _carrier = this.header.toInt()
        return ((_carrier shr 0) and 0x0F).toUByte()
    }

    fun setExtId(v: UByte) {
        val _carrier = this.header.toInt()
        val _shifted_mask = 0x0F shl 0
        val _val = (v.toInt() and 0x0F) shl 0
        this.header = ((_carrier and _shifted_mask.inv()) or _val).toUByte()
    }

    fun M(): Boolean = (this.header.toInt() and 0x10) != 0

    fun setM(v: Boolean) {
        this.header = if (v) {
            (this.header.toInt() or 0x10).toUByte()
        } else {
            (this.header.toInt() and 0x10.inv()).toUByte()
        }
    }

    fun enc(): UByte {
        val _carrier = this.header.toInt()
        return ((_carrier shr 5) and 0x03).toUByte()
    }

    fun setEnc(v: UByte) {
        val _carrier = this.header.toInt()
        val _shifted_mask = 0x03 shl 5
        val _val = (v.toInt() and 0x03) shl 5
        this.header = ((_carrier and _shifted_mask.inv()) or _val).toUByte()
    }

    fun Z(): Boolean = (this.header.toInt() and 0x80) != 0

    fun setZ(v: Boolean) {
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
    @Suppress("UNUSED_PARAMETER")
    fun encode(w: SceSink, afterShm: UByte): CodecError? {
        // Encode fixed prefix (tag field bytes are part of the prefix).
        // The tag value is read from the struct field, NOT derived from
        // the body discriminant — keeping author-set tag / body in sync
        // is the caller's responsibility (v1 keeps the layout simple).
        w.writeU8(header.toByte())?.let { return it }
        // Append the active arm body's encoded bytes via the same sink.
        when (val _b = this.body) {
            is CodecZenohValueEntryVariant.CodecZenohExtUnit -> _b.body.encode(w)?.let { return it }
            is CodecZenohValueEntryVariant.CodecZenohExtZint -> _b.body.encode(w)?.let { return it }
            is CodecZenohValueEntryVariant.CodecZenohValueZbuf -> _b.body.encode(w, afterShm)?.let { return it }
            is CodecZenohValueEntryVariant.Default -> _b.body.encode(w)?.let { return it }
        }
        return null
    }

    /// Heap-backed convenience facade. Runs `encode` over a
    /// `MutableListSink` and returns the freshly-encoded ByteArray.
    /// Callers targeting zero-alloc hot paths should call `encode`
    /// directly against a caller-owned sink (e.g. `ByteArraySink`).
    fun encodeToByteArray(afterShm: UByte): ByteArray {
        val _list = mutableListOf<Byte>()
        encode(MutableListSink(_list), afterShm)
        return _list.toByteArray()
    }

    companion object {
        /// Decode the next frame from `cursor`. On success the cursor
        /// advances past the consumed bytes and the value is `Decoded.Ok`;
        /// a refused frame is `Decoded.Refused` with the kind that refused
        /// it (docs/adr/0099, amendment of 2026-11-16): `NeedMoreBytes` when
        /// the cursor's tail is shorter than the frame (RFC §synth-5-B
        /// L494-519), which the caller resumes after appending bytes, and
        /// another kind for a frame no more bytes make whole.
        @Suppress("UNUSED_PARAMETER")
        fun decode(cursor: SceCursor, afterShm: UByte): Decoded<CodecZenohValueEntry> {
            // Decode fixed prefix (RFC §synth-5-B variant: fields before tag suffix).
            val raw = cursor.peekSlice(1) ?: return Decoded.Refused(CodecError.NeedMoreBytes)
            val header = raw[0].toUByte()
            if (!cursor.advance(1)) return Decoded.Refused(CodecError.NeedMoreBytes)
            // Dispatch on the tag field; each arm decodes its body codec
            // from the cursor. The default arm (when declared) carries
            // the runtime tag value so encode can round-trip it back
            // onto the wire.
            val body: CodecZenohValueEntryVariant = when (((header.toInt() shr 5) and 0x03)) {
                0 -> {
                    val _arm = com.sce.generated.codec_zenoh_ext_unit.CodecZenohExtUnit.decode(cursor).valueOr { _refused -> return _refused }
                    CodecZenohValueEntryVariant.CodecZenohExtUnit(_arm)
                }
                1 -> {
                    val _arm = com.sce.generated.codec_zenoh_ext_zint.CodecZenohExtZint.decode(cursor).valueOr { _refused -> return _refused }
                    CodecZenohValueEntryVariant.CodecZenohExtZint(_arm)
                }
                2 -> {
                    val _arm = com.sce.generated.codec_zenoh_value_zbuf.CodecZenohValueZbuf.decode(cursor, afterShm).valueOr { _refused -> return _refused }
                    CodecZenohValueEntryVariant.CodecZenohValueZbuf(_arm)
                }
                else -> {
                    val _arm = com.sce.generated.codec_zenoh_ext_unit.CodecZenohExtUnit.decode(cursor).valueOr { _refused -> return _refused }
                    CodecZenohValueEntryVariant.Default(tag = ((header.toInt() shr 5) and 0x03).toUByte(), body = _arm)
                }
            }
            return Decoded.Ok(
                CodecZenohValueEntry(
                    header = header,
                    body = body
                )
            )
        }
    }
}
