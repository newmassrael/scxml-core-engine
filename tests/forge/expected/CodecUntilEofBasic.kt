// SCE-MAP: codec_until_eof_basic:10

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_until_eof_basic

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink
import com.sce.generated.codec_repeat_elem.*

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecUntilEofBasic()` before any encode()/decode() call. Defaults
// mirror the zero-initialized shape that decode() fills in on success.
data class CodecUntilEofBasic(
    var msgs: MutableList<CodecRepeatElem> = mutableListOf()
) {
    /// RFC §5.B B1-α encode-side primary: write `self` into the
    /// caller-owned `w` sink. Returns `null` on success;
    /// `CodecError.BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable
    /// sinks (e.g. `MutableListSink`) are effectively infallible.
    fun encode(w: SceSink): CodecError? {
        // RFC §5.B B2 encode: fixed prefix appends byte-by-byte;
        // list fields iterate the host MutableList and write each
        // element's encode(w) through the same sink. Author keeps
        // count field == list length (trust contract).
        for (_e in this.msgs) {
            _e.encode(w)?.let { return it }
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
        fun decode(cursor: SceCursor): CodecUntilEofBasic? {
            // RFC §5.B B2 repeat primitive: streaming decode mixes
            // plain fixed-width reads (per-field via the present-if
            // helper's non-gated arm) with `mutableListOf<T>().also { ... }`
            // loops that iterate the imported codec's `decode()`
            // either `count_ref.toInt()` times (length-field) or
            // until cursor exhaustion (until-eof). Element bodies
            // recurse into their own codec — each may itself surface
            // null, unwinding the partial frame via `?: return null`.
            val msgs: MutableList<CodecRepeatElem> = mutableListOf<CodecRepeatElem>().apply {
                while (cursor.remaining() > 0) {
                    add(CodecRepeatElem.decode(cursor) ?: return null)
                }
            }
            return CodecUntilEofBasic(
                msgs = msgs
            )
        }
    }
}
