// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_decl_final

import com.sce.forge.runtime.SceCursor

// RFC §5.B B5-α empty body — Kotlin's `data class` requires at least
// one primary-ctor parameter, so empty-body codecs (e.g. Zenoh
// KeepAlive) emit as a plain class with a no-arg constructor.
class CodecDeclFinal {
    fun encode(): ByteArray {
        // RFC §5.B B5-α empty body — zero-byte payload.
        return ByteArray(0)
    }

    companion object {
        /// Decode the next frame from `cursor`. On success the cursor
        /// advances past the consumed bytes; returns `null` when the
        /// cursor's tail is shorter than the declared minimum frame
        /// (RFC §5.B L494-519).
        fun decode(cursor: SceCursor): CodecDeclFinal? {
            // RFC §5.B B5-α empty body — zero-byte payload, no cursor work.
            return CodecDeclFinal()
        }
    }
}
