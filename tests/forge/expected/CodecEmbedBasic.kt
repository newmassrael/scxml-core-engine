// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_embed_basic

import com.sce.forge.runtime.SceCursor
import com.sce.generated.codec_zenoh_locator.*

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecEmbedBasic()` before any encode()/decode() call. Defaults
// mirror the zero-initialized shape that decode() fills in on success.
data class CodecEmbedBasic(
    var tag: UByte = 0.toUByte(),
    var locator: CodecZenohLocator = byteArrayOf()
) {
    fun encode(): ByteArray {
        // RFC §5.B B2 encode: fixed prefix appends byte-by-byte;
        // repeat fields iterate the host MutableList and splice each
        // element's encode().toList() into the parent buffer. Author
        // keeps count field == list length (trust contract).
        val r = mutableListOf<Byte>()
        r.add(this.tag.toByte())
        r.addAll(this.locator.encode().toList())
        return r.toByteArray()
    }

    companion object {
        /// Decode the next frame from `cursor`. On success the cursor
        /// advances past the consumed bytes; returns `null` when the
        /// cursor's tail is shorter than the declared minimum frame
        /// (RFC §5.B L494-519).
        fun decode(cursor: SceCursor): CodecEmbedBasic? {
            // RFC §5.B B2 repeat primitive: streaming decode mixes
            // plain fixed-width reads (per-field via the present-if
            // helper's non-gated arm) with `mutableListOf<T>().also { ... }`
            // loops that iterate the imported codec's `decode()`
            // either `count_ref.toInt()` times (length-field) or
            // until cursor exhaustion (until-eof). Element bodies
            // recurse into their own codec — each may itself surface
            // null, unwinding the partial frame via `?: return null`.
            val tag = run {
                val raw = cursor.peekSlice(1) ?: return null
                val _v = raw[0].toUByte()
                if (!cursor.advance(1)) return null
                _v
            }
            val locator = CodecZenohLocator.decode(cursor) ?: return null
            return CodecEmbedBasic(
                tag = tag,
                locator = locator
            )
        }
    }
}
