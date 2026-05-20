// SCE-MAP: codec_variant_dispatch:8

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_variant_dispatch

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink
import com.sce.generated.codec_variant_session_open.*
import com.sce.generated.codec_variant_session_close.*

// RFC §5.B variant primitive (B1-β): discriminated-union body for the
// codec's tag-field suffix. Each arm wraps an imported codec's decoded
// value; the optional Default arm preserves the runtime tag value
// alongside its catch-all body. Arm body types are referenced by FQN
// (defensive — wildcard imports could otherwise surface an ambiguity if
// two imported codecs declare same-named inner classes).
sealed class CodecVariantDispatchVariant {
    data class CodecVariantSessionOpen(val body: com.sce.generated.codec_variant_session_open.CodecVariantSessionOpen) : CodecVariantDispatchVariant()
    data class CodecVariantSessionClose(val body: com.sce.generated.codec_variant_session_close.CodecVariantSessionClose) : CodecVariantDispatchVariant()
    data class Default(val tag: UByte, val body: com.sce.generated.codec_variant_session_close.CodecVariantSessionClose) : CodecVariantDispatchVariant()
}

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecVariantDispatch()` before any encode()/decode() call. Defaults
// mirror the zero-initialized shape that decode() fills in on success.
data class CodecVariantDispatch(
    var msg_id: UByte = 0.toUByte(),
    // RFC variant-default-uniformity Atomic β-kotlin: pick the declared
    // default arm (`<sce:arm default="true"/>`) instead of the first
    // alternative so a freshly-constructed envelope round-trips byte-
    // exactly through `encode() -> decode()`. Paired with the inner
    // codec's `<sce:flag value=>`-baked default fields above.
    var body: CodecVariantDispatchVariant = CodecVariantDispatchVariant.CodecVariantSessionClose(com.sce.generated.codec_variant_session_close.CodecVariantSessionClose())
) {
    /// RFC §5.B B1-α encode-side primary: write `self` into the
    /// caller-owned `w` sink. Returns `null` on success;
    /// `CodecError.BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable
    /// sinks (e.g. `MutableListSink`) are effectively infallible.
    fun encode(w: SceSink): CodecError? {
        // Encode fixed prefix (tag field bytes are part of the prefix).
        // The tag value is read from the struct field, NOT derived from
        // the body discriminant — keeping author-set tag / body in sync
        // is the caller's responsibility (v1 keeps the layout simple).
        w.writeU8(msg_id.toByte())?.let { return it }
        // Append the active arm body's encoded bytes via the same sink.
        when (val _b = this.body) {
            is CodecVariantDispatchVariant.CodecVariantSessionOpen -> _b.body.encode(w)?.let { return it }
            is CodecVariantDispatchVariant.CodecVariantSessionClose -> _b.body.encode(w)?.let { return it }
            is CodecVariantDispatchVariant.Default -> _b.body.encode(w)?.let { return it }
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
        fun decode(cursor: SceCursor): CodecVariantDispatch? {
            // Decode fixed prefix (RFC §5.B variant B1-β: fields before tag suffix).
            val raw = cursor.peekSlice(1) ?: return null
            val msg_id = raw[0].toUByte()
            if (!cursor.advance(1)) return null
            // Dispatch on the tag field; each arm decodes its body codec
            // from the cursor. The default arm (when declared) carries
            // the runtime tag value so encode can round-trip it back
            // onto the wire.
            val body: CodecVariantDispatchVariant = when (msg_id.toInt()) {
                1 -> {
                    val _arm = com.sce.generated.codec_variant_session_open.CodecVariantSessionOpen.decode(cursor) ?: return null
                    CodecVariantDispatchVariant.CodecVariantSessionOpen(_arm)
                }
                2 -> {
                    val _arm = com.sce.generated.codec_variant_session_close.CodecVariantSessionClose.decode(cursor) ?: return null
                    CodecVariantDispatchVariant.CodecVariantSessionClose(_arm)
                }
                else -> {
                    val _arm = com.sce.generated.codec_variant_session_close.CodecVariantSessionClose.decode(cursor) ?: return null
                    CodecVariantDispatchVariant.Default(tag = msg_id, body = _arm)
                }
            }
            return CodecVariantDispatch(
                msg_id = msg_id,
                body = body
            )
        }
    }
}
