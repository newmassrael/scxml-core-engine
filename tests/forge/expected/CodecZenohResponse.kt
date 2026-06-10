// SCE-MAP: codec_zenoh_response:75

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_zenoh_response

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink
import com.sce.generated.codec_zenoh_ext_entry.*
import com.sce.generated.codec_zenoh_reply.*
import com.sce.generated.codec_zenoh_err.*

// RFC §5.B variant primitive: discriminated-union body for the
// codec's tag-field suffix. Each arm wraps an imported codec's decoded
// value; the optional Default arm preserves the runtime tag value
// alongside its catch-all body. Arm body types are referenced by FQN
// (defensive — wildcard imports could otherwise surface an ambiguity if
// two imported codecs declare same-named inner classes).
sealed class CodecZenohResponseVariant {
    data class CodecZenohReply(val body: com.sce.generated.codec_zenoh_reply.CodecZenohReply) : CodecZenohResponseVariant()
    data class CodecZenohErr(val body: com.sce.generated.codec_zenoh_err.CodecZenohErr) : CodecZenohResponseVariant()
    data class Default(val tag: UByte, val body: com.sce.generated.codec_zenoh_reply.CodecZenohReply) : CodecZenohResponseVariant()
}

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecZenohResponse()` before any encode()/decode() call. Defaults
// mirror the zero-initialized shape that decode() fills in on success.
data class CodecZenohResponse(
    var header: UByte = 0x1b.toUByte(),
    var request_id: ULong = 0uL,
    var key_id: UInt = 0u,
    var suffix_len: ULong? = null,
    var suffix: String? = null,
    var extensions: MutableList<CodecZenohExtEntry>? = null,
    // RFC variant-default-uniformity Atomic β-kotlin: pick the declared
    // default arm (`<sce:arm default="true"/>`) instead of the first
    // alternative so a freshly-constructed envelope round-trips byte-
    // exactly through `encode() -> decode()`. Paired with the inner
    // codec's `<sce:flag value=>`-baked default fields above.
    var body: CodecZenohResponseVariant = CodecZenohResponseVariant.CodecZenohReply(com.sce.generated.codec_zenoh_reply.CodecZenohReply())
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
            var _vle: ULong = (request_id).toULong()
            while (_vle >= 0x80UL) {
                w.writeU8((_vle.toLong() and 0x7F or 0x80).toByte())?.let { return it }
                _vle = _vle shr 7
            }
            w.writeU8(_vle.toByte())?.let { return it }
        }
        run {
            var _vle: ULong = (key_id).toULong()
            while (_vle >= 0x80UL) {
                w.writeU8((_vle.toLong() and 0x7F or 0x80).toByte())?.let { return it }
                _vle = _vle shr 7
            }
            w.writeU8(_vle.toByte())?.let { return it }
        }
        this.suffix_len?.let { _v ->
        run {
            var _vle: ULong = (_v).toULong()
            while (_vle >= 0x80UL) {
                w.writeU8((_vle.toLong() and 0x7F or 0x80).toByte())?.let { return it }
                _vle = _vle shr 7
            }
            w.writeU8(_vle.toByte())?.let { return it }
        }
        }
        this.suffix?.let { _v ->
            w.writeBytes(_v.toByteArray(Charsets.UTF_8))?.let { return it }
        }
        this.extensions?.let { _list ->
            for (_e in _list) {
                _e.encode(w)?.let { return it }
            }
        }
        // Append the active arm body's encoded bytes via the same sink.
        when (val _b = this.body) {
            is CodecZenohResponseVariant.CodecZenohReply -> _b.body.encode(w)?.let { return it }
            is CodecZenohResponseVariant.CodecZenohErr -> _b.body.encode(w)?.let { return it }
            is CodecZenohResponseVariant.Default -> _b.body.encode(w)?.let { return it }
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
        fun decode(cursor: SceCursor): CodecZenohResponse? {
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
            val request_id = cursor.readVleU64() ?: return null
            val key_id = cursor.readVleU32() ?: return null
            val suffix_len: ULong? = if ((header.toInt() and 0x20) != 0) {
                val _v = cursor.readVleU64() ?: return null
                _v
            } else {
                null
            }
            val suffix = if ((header.toInt() and 0x20) != 0) {
                val _n = suffix_len!!.toInt()
                val raw = cursor.peekSlice(_n) ?: return null
                val _v = try {
                    java.nio.charset.StandardCharsets.UTF_8.newDecoder()
                        .decode(java.nio.ByteBuffer.wrap(raw)).toString()
                } catch (_: java.nio.charset.CharacterCodingException) { return null }
                if (!cursor.advance(_n)) return null
                _v
            } else {
                null
            }
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
            val body: CodecZenohResponseVariant = when (((_peek.toInt() shr 0) and 0x1F)) {
                4 -> {
                    val _arm = com.sce.generated.codec_zenoh_reply.CodecZenohReply.decode(cursor) ?: return null
                    CodecZenohResponseVariant.CodecZenohReply(_arm)
                }
                5 -> {
                    val _arm = com.sce.generated.codec_zenoh_err.CodecZenohErr.decode(cursor) ?: return null
                    CodecZenohResponseVariant.CodecZenohErr(_arm)
                }
                else -> {
                    val _arm = com.sce.generated.codec_zenoh_reply.CodecZenohReply.decode(cursor) ?: return null
                    CodecZenohResponseVariant.Default(tag = ((_peek.toInt() shr 0) and 0x1F).toUByte(), body = _arm)
                }
            }
            return CodecZenohResponse(
                header = header,
                request_id = request_id,
                key_id = key_id,
                suffix_len = suffix_len,
                suffix = suffix,
                extensions = extensions,
                body = body
            )
        }
    }
}
