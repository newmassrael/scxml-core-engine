// SCE-MAP: codec_zenoh_request:73

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_zenoh_request

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink
import com.sce.generated.codec_zenoh_wireexpr.*
import com.sce.generated.codec_zenoh_ext_entry.*
import com.sce.generated.codec_zenoh_msg_put.*
import com.sce.generated.codec_zenoh_msg_del.*
import com.sce.generated.codec_zenoh_query.*

// RFC §5.B variant primitive: discriminated-union body for the
// codec's tag-field suffix. Each arm wraps an imported codec's decoded
// value; the optional Default arm preserves the runtime tag value
// alongside its catch-all body. Arm body types are referenced by FQN
// (defensive — wildcard imports could otherwise surface an ambiguity if
// two imported codecs declare same-named inner classes).
sealed class CodecZenohRequestVariant {
    data class CodecZenohMsgPut(val body: com.sce.generated.codec_zenoh_msg_put.CodecZenohMsgPut) : CodecZenohRequestVariant()
    data class CodecZenohMsgDel(val body: com.sce.generated.codec_zenoh_msg_del.CodecZenohMsgDel) : CodecZenohRequestVariant()
    data class CodecZenohQuery(val body: com.sce.generated.codec_zenoh_query.CodecZenohQuery) : CodecZenohRequestVariant()
    data class Default(val tag: UByte, val body: com.sce.generated.codec_zenoh_query.CodecZenohQuery) : CodecZenohRequestVariant()
}

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecZenohRequest()` before any encode()/decode() call. Defaults
// mirror the zero-initialized shape that decode() fills in on success.
data class CodecZenohRequest(
    var header: UByte = 0x1c.toUByte(),
    var rid: ULong = 0uL,
    var keyexpr: CodecZenohWireexpr = CodecZenohWireexpr(),
    var extensions: MutableList<CodecZenohExtEntry>? = null,
    // RFC variant-default-uniformity Atomic β-kotlin: pick the declared
    // default arm (`<sce:arm default="true"/>`) instead of the first
    // alternative so a freshly-constructed envelope round-trips byte-
    // exactly through `encode() -> decode()`. Paired with the inner
    // codec's `<sce:flag value=>`-baked default fields above.
    var body: CodecZenohRequestVariant = CodecZenohRequestVariant.CodecZenohMsgPut(com.sce.generated.codec_zenoh_msg_put.CodecZenohMsgPut())
) {
    // RFC §5.B flags primitive: per-bit-range accessors over
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

    fun Z(): Boolean = (this.header.toInt() and 0x80) != 0

    fun setZ(v: Boolean) {
        this.header = if (v) {
            (this.header.toInt() or 0x80).toUByte()
        } else {
            (this.header.toInt() and 0x80.inv()).toUByte()
        }
    }

    /// RFC §5.B encode-side primary: write `self` into the
    /// caller-owned `w` sink. Returns `null` on success;
    /// `CodecError.BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable
    /// sinks (e.g. `MutableListSink`) are effectively infallible.
    fun encode(w: SceSink): CodecError? {
        // RFC §5.B Y3 atomic 2b-ii peek-byte / 2b-iv streaming-prefix:
        // streaming prefix encode. Peek-byte mode: arm body's encode
        // prepends its own header byte (which the decoder peeked); no
        // separate tag byte here. Streaming-prefix mode (own-field):
        // carrier is part of the prefix fields and emits via the same
        // per-field path.
        w.writeU8(this.header.toByte())?.let { return it }
        run {
            var _vle: ULong = (rid).toULong()
            while (_vle >= 0x80UL) {
                w.writeU8((_vle.toLong() and 0x7F or 0x80).toByte())?.let { return it }
                _vle = _vle shr 7
            }
            w.writeU8(_vle.toByte())?.let { return it }
        }
        this.keyexpr.encode(w, (((this.header.toInt() shr 5) and 0x1).toUByte()))?.let { return it }
        this.extensions?.let { _list ->
            for (_e in _list) {
                _e.encode(w)?.let { return it }
            }
        }
        // Append the active arm body's encoded bytes via the same sink.
        when (val _b = this.body) {
            is CodecZenohRequestVariant.CodecZenohMsgPut -> _b.body.encode(w)?.let { return it }
            is CodecZenohRequestVariant.CodecZenohMsgDel -> _b.body.encode(w)?.let { return it }
            is CodecZenohRequestVariant.CodecZenohQuery -> _b.body.encode(w)?.let { return it }
            is CodecZenohRequestVariant.Default -> _b.body.encode(w)?.let { return it }
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
        fun decode(cursor: SceCursor): CodecZenohRequest? {
            // RFC §5.B Y3 atomic 2b-ii peek-byte / 2b-iv streaming-
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
            val rid = cursor.readVleU64() ?: return null
            val keyexpr = CodecZenohWireexpr.decode(cursor, (((header.toInt() shr 5) and 0x1).toUByte())) ?: return null
            val extensions: MutableList<CodecZenohExtEntry>? = if ((header.toInt() and 0x80) != 0) {
            mutableListOf<CodecZenohExtEntry>().also {
                for (_i in 0 until 4) {
                    if (cursor.remaining() == 0) break
                    val _entry = CodecZenohExtEntry.decode(cursor) ?: return null
                    it.add(_entry)
                    if (!_entry.Z()) break
                }
            }
        } else {
            null
        }
            val _peekRaw = cursor.peekSlice(1) ?: return null
        val _peek: UByte = _peekRaw[0].toUByte()
            // Dispatch on the tag field; each arm decodes its body codec
            // from the cursor. The default arm (when declared) carries
            // the runtime tag value so encode can round-trip it back
            // onto the wire.
            val body: CodecZenohRequestVariant = when (((_peek.toInt() shr 0) and 0x1F)) {
                1 -> {
                    val _arm = com.sce.generated.codec_zenoh_msg_put.CodecZenohMsgPut.decode(cursor) ?: return null
                    CodecZenohRequestVariant.CodecZenohMsgPut(_arm)
                }
                2 -> {
                    val _arm = com.sce.generated.codec_zenoh_msg_del.CodecZenohMsgDel.decode(cursor) ?: return null
                    CodecZenohRequestVariant.CodecZenohMsgDel(_arm)
                }
                3 -> {
                    val _arm = com.sce.generated.codec_zenoh_query.CodecZenohQuery.decode(cursor) ?: return null
                    CodecZenohRequestVariant.CodecZenohQuery(_arm)
                }
                else -> {
                    val _arm = com.sce.generated.codec_zenoh_query.CodecZenohQuery.decode(cursor) ?: return null
                    CodecZenohRequestVariant.Default(tag = ((_peek.toInt() shr 0) and 0x1F).toUByte(), body = _arm)
                }
            }
            return CodecZenohRequest(
                header = header,
                rid = rid,
                keyexpr = keyexpr,
                extensions = extensions,
                body = body
            )
        }
    }
}
