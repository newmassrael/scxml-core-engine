// SCE-MAP: codec_zenoh_oam:56

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_zenoh_oam

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink
import com.sce.generated.codec_zenoh_ext_entry.*
import com.sce.generated.codec_zenoh_ext_unit.*
import com.sce.generated.codec_zenoh_ext_zint.*
import com.sce.generated.codec_zenoh_ext_zbuf.*

// RFC §synth-5-B variant primitive: discriminated-union body for the
// codec's tag-field suffix. Each arm wraps an imported codec's decoded
// value; the optional Default arm preserves the runtime tag value
// alongside its catch-all body. Arm body types are referenced by FQN
// (defensive — wildcard imports could otherwise surface an ambiguity if
// two imported codecs declare same-named inner classes).
sealed class CodecZenohOamVariant {
    data class CodecZenohExtUnit(val body: com.sce.generated.codec_zenoh_ext_unit.CodecZenohExtUnit) : CodecZenohOamVariant()
    data class CodecZenohExtZint(val body: com.sce.generated.codec_zenoh_ext_zint.CodecZenohExtZint) : CodecZenohOamVariant()
    data class CodecZenohExtZbuf(val body: com.sce.generated.codec_zenoh_ext_zbuf.CodecZenohExtZbuf) : CodecZenohOamVariant()
    data class Default(val tag: UByte, val body: com.sce.generated.codec_zenoh_ext_unit.CodecZenohExtUnit) : CodecZenohOamVariant()
}

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecZenohOam()` before any encode()/decode() call. Defaults
// mirror the zero-initialized shape that decode() fills in on success.
data class CodecZenohOam(
    var header: UByte = 0x1f.toUByte(),
    var id: UShort = 0.toUShort(),
    var extensions: MutableList<CodecZenohExtEntry>? = null,
    // RFC variant-default-uniformity (Kotlin): pick the declared
    // default arm (`<sce:arm default="true"/>`) instead of the first
    // alternative so a freshly-constructed envelope round-trips byte-
    // exactly through `encode() -> decode()`. Paired with the inner
    // codec's `<sce:flag value=>`-baked default fields above.
    var body: CodecZenohOamVariant = CodecZenohOamVariant.CodecZenohExtUnit(com.sce.generated.codec_zenoh_ext_unit.CodecZenohExtUnit())
) {
    // RFC §synth-5-B flags primitive: per-bit-range accessors over
    // the carrier field. Single-bit (width=1) reads as Boolean; multi-
    // bit (width>=2) reads as the smallest unsigned Kotlin type that
    // fits (UByte / UShort / UInt / ULong). UByte/UShort widen through
    // `.toInt()` and UInt/ULong through `.toLong()` for the bitwise
    // ops; the result narrows back via the carrier's `toU*` ctor.
    fun mid(): UByte {
        val _carrier = this.header.toInt()
        return ((_carrier shr 0) and 0x1F).toUByte()
    }

    fun setMid(v: UByte) {
        val _carrier = this.header.toInt()
        val _shifted_mask = 0x1F shl 0
        val _val = (v.toInt() and 0x1F) shl 0
        this.header = ((_carrier and _shifted_mask.inv()) or _val).toUByte()
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
    fun encode(w: SceSink): CodecError? {
        // RFC §synth-5-B peek-byte / streaming-prefix:
        // streaming prefix encode. Peek-byte mode: arm body's encode
        // prepends its own header byte (which the decoder peeked); no
        // separate tag byte here. Streaming-prefix mode (own-field):
        // carrier is part of the prefix fields and emits via the same
        // per-field path.
        w.writeU8(this.header.toByte())?.let { return it }
        w.writeVleU16((id).toUShort())?.let { return it }
        this.extensions?.let { _list ->
            for (_e in _list) {
                _e.encode(w)?.let { return it }
            }
        }
        // Append the active arm body's encoded bytes via the same sink.
        when (val _b = this.body) {
            is CodecZenohOamVariant.CodecZenohExtUnit -> _b.body.encode(w)?.let { return it }
            is CodecZenohOamVariant.CodecZenohExtZint -> _b.body.encode(w)?.let { return it }
            is CodecZenohOamVariant.CodecZenohExtZbuf -> _b.body.encode(w)?.let { return it }
            is CodecZenohOamVariant.Default -> _b.body.encode(w)?.let { return it }
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
        fun decode(cursor: SceCursor): CodecZenohOam? {
            // RFC §synth-5-B peek-byte / streaming-
            // prefix: streaming prefix decode (variable-length fields
            // supported via per-field present_if/tlv-chain/embed/repeat
            // helpers). Peek-byte mode additionally peeks the cursor's
            // next byte for variant tag without advancing — arm body
            // decoder reads it as own header.
            val header = run {
                val raw = cursor.peekSlice(1) ?: return null
                val _v = raw[0].toUByte()
                if (!cursor.advance(1)) return null
                _v
            }
            val id = cursor.readVleU16() ?: return null
            val extensions: MutableList<CodecZenohExtEntry>? = if ((header.toInt() and 0x80) != 0) {
            mutableListOf<CodecZenohExtEntry>().also {
                var _more = false
                for (_i in 0 until 4) {
                    if (cursor.remaining() == 0) break
                    val _entry = CodecZenohExtEntry.decode(cursor) ?: return null
                    _more = _entry.Z()
                    it.add(_entry)
                    if (!_more) break
                }
                if (_more) return null
            }
        } else {
            null
        }
            // Dispatch on the tag field; each arm decodes its body codec
            // from the cursor. The default arm (when declared) carries
            // the runtime tag value so encode can round-trip it back
            // onto the wire.
            val body: CodecZenohOamVariant = when (((header.toInt() shr 5) and 0x03)) {
                0 -> {
                    val _arm = com.sce.generated.codec_zenoh_ext_unit.CodecZenohExtUnit.decode(cursor) ?: return null
                    CodecZenohOamVariant.CodecZenohExtUnit(_arm)
                }
                1 -> {
                    val _arm = com.sce.generated.codec_zenoh_ext_zint.CodecZenohExtZint.decode(cursor) ?: return null
                    CodecZenohOamVariant.CodecZenohExtZint(_arm)
                }
                2 -> {
                    val _arm = com.sce.generated.codec_zenoh_ext_zbuf.CodecZenohExtZbuf.decode(cursor) ?: return null
                    CodecZenohOamVariant.CodecZenohExtZbuf(_arm)
                }
                else -> {
                    val _arm = com.sce.generated.codec_zenoh_ext_unit.CodecZenohExtUnit.decode(cursor) ?: return null
                    CodecZenohOamVariant.Default(tag = ((header.toInt() shr 5) and 0x03).toUByte(), body = _arm)
                }
            }
            return CodecZenohOam(
                header = header,
                id = id,
                extensions = extensions,
                body = body
            )
        }
    }
}
