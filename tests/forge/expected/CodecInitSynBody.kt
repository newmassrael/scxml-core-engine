// SCE-MAP: codec_init_syn_body:30 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_init_syn_body

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.Decoded
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink

/// The `s` group of [CodecInitSynBody] where `S` holds (docs/adr/0065 §6 R4): the
/// members present then.
data class CodecInitSynBodyS(
    val sn_res: UByte = 0.toUByte(),
    val batch_size: UShort = 0.toUShort()
)

/// The elements a value of [CodecInitSynBody] presents, which its encode reads
/// (docs/adr/0081 §2): any object that presents them is a source of it, which
/// [CodecInitSynBody.encodeSource] writes. The encode derives every other element
/// of the frame.
interface CodecInitSynBodySource {
    val version: UByte
    val s: CodecInitSynBodyS?
}

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecInitSynBody()` before any encode()/decode() call. Each default
// is a value of that field's own type, which decode() then fills in on
// success — the carrier's zero for a number, and for an enum the first
// variant its document declares, since a closed set does not hold a
// value it never declared.
data class CodecInitSynBody(
    override var version: UByte = 0.toUByte(),
    override var s: CodecInitSynBodyS? = null
) : CodecInitSynBodySource {
    /// RFC §synth-5-B encode-side primary: write `self` into the
    /// caller-owned `w` sink. Returns `null` on success;
    /// `CodecError.BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable
    /// sinks (e.g. `MutableListSink`) are effectively infallible.
    @Suppress("UNUSED_PARAMETER")
    fun encode(w: SceSink, S: UByte): CodecError? {
        // docs/adr/0081 step 4: the encode over any source, which derives
        // every element the value does not present. It is handed the flag
        // inputs the value does not say; one the value says is read off it,
        // and what a caller hands this `encode` for it is held to that
        // (docs/adr/0065 §6 R4a).
        if (S != inputS(this)) return CodecError.PresentIfMismatch
        return encodeSource(this, w)
    }

    /// Heap-backed convenience facade. Runs `encode` over a
    /// `MutableListSink` and returns the freshly-encoded ByteArray, or
    /// `null` when `encode` refuses the value before its first byte: a
    /// gated field and the `sce:present-if` that gates it contradict each
    /// other (`CodecError.PresentIfMismatch`), or a byte string, text or
    /// list lies outside its bound (`CodecError.LengthOutOfRange`,
    /// `CodecError.TooManyElements`). Callers that need the reason, and
    /// zero-alloc hot paths, call `encode` directly against a caller-owned
    /// sink (e.g. `ByteArraySink`).
    fun encodeToByteArray(S: UByte): ByteArray? {
        val _list = mutableListOf<Byte>()
        if (encode(MutableListSink(_list), S) != null) return null
        return _list.toByteArray()
    }

    companion object {
        /// Write any source of [CodecInitSynBody] into `w`: each element read once off
        /// `src` and written where the codec's encode plan lays it out
        /// (docs/adr/0081). It refuses no value: it answers `null`, or
        /// [CodecError.BufferOverflow] from a bounded sink with no room left.
        fun encodeSource(src: CodecInitSynBodySource, w: SceSink): CodecError? {
            val _e_version = src.version
            val _g_s = src.s
            val _e_sn_res = _g_s?.sn_res
            val _e_batch_size = _g_s?.batch_size
            w.writeU8(_e_version.toByte())?.let { return it }
            if (_e_sn_res != null) {
                w.writeU8(_e_sn_res.toByte())?.let { return it }
            }
            if (_e_batch_size != null) {
                w.writeU8((_e_batch_size.toInt() ushr 8 and 0xFF).toByte())?.let { return it }
                w.writeU8((_e_batch_size.toInt() and 0xFF).toByte())?.let { return it }
            }
            return null
        }

        /// The flag input `S` a source of [CodecInitSynBody] says, which its encode
        /// is not handed (docs/adr/0065 §6 R4a).
        fun inputS(src: CodecInitSynBodySource): UByte =
            (if (src.s != null) 1 else 0).toUByte()

        /// Decode the next frame from `cursor`. On success the cursor
        /// advances past the consumed bytes and the value is `Decoded.Ok`;
        /// a refused frame is `Decoded.Refused` with the kind that refused
        /// it (docs/adr/0099, amendment of 2026-11-16): `NeedMoreBytes` when
        /// the cursor's tail is shorter than the frame (RFC §synth-5-B
        /// L494-519), which the caller resumes after appending bytes, and
        /// another kind for a frame no more bytes make whole.
        @Suppress("UNUSED_PARAMETER")
        fun decode(cursor: SceCursor, S: UByte): Decoded<CodecInitSynBody> {
            // Streaming cursor decode (SSOT selection: `needs_streaming`).
            // The positional `raw[byte_off]` path is valid only when every
            // field's absolute offset is fixed at codegen time; this branch
            // handles every codec where it is not — present-if-gated fields
            // (runtime presence), VLE / repeat / TLV-chain / embed fields
            // (runtime width), string fields (UTF-8 decode), and a fixed
            // field after a variable-length payload (offset depends on the
            // payload length). Each field reads its own bytes from the
            // cursor and advances past what it consumed. Per-field
            // `is_repeat` / `is_tlv_chain` / `is_embed` route to their
            // dedicated helpers; every other field flows through
            // `present_if_decode_stmt`.
            val version = run {
                val raw = cursor.peekSlice(1) ?: return Decoded.Refused(CodecError.NeedMoreBytes)
                val _v = raw[0].toUByte()
                if (!cursor.advance(1)) return Decoded.Refused(CodecError.NeedMoreBytes)
                _v
            }
            val sn_res = if ((S.toInt() and 0x01) != 0) {
                val raw = cursor.peekSlice(1) ?: return Decoded.Refused(CodecError.NeedMoreBytes)
                val _v = raw[0].toUByte()
                if (!cursor.advance(1)) return Decoded.Refused(CodecError.NeedMoreBytes)
                _v
            } else {
                null
            }
            val batch_size = if ((S.toInt() and 0x01) != 0) {
                val raw = cursor.peekSlice(2) ?: return Decoded.Refused(CodecError.NeedMoreBytes)
                val _v = (((raw[0].toInt() and 0xFF) shl 8) or (raw[1].toInt() and 0xFF)).toUShort()
                if (!cursor.advance(2)) return Decoded.Refused(CodecError.NeedMoreBytes)
                _v
            } else {
                null
            }
            return Decoded.Ok(
                CodecInitSynBody(
                    version = version,
                    s = if (S.toUInt() != 0u) CodecInitSynBodyS(sn_res = sn_res!!, batch_size = batch_size!!) else null
                )
            )
        }
    }
}
