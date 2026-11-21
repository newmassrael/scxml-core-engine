// SCE-MAP: codec_zenoh_open_body:42 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_zenoh_open_body

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.Decoded
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink
import kotlin.jvm.JvmInline

/// The `cookie` element of [CodecZenohOpenBody], 0 or more bytes, the protocol stating no most, held to that bound where a
/// value of it is made (docs/adr/0077 §3, docs/adr/0081 §2): its constructor is
/// private and [of] makes a value only of bytes within the bound. The value it
/// holds cannot change after, so a value holds its bound for as long as it lives.
@JvmInline
value class CodecZenohOpenBodyCookieBytes private constructor(val bytes: ByteArray) {
    companion object {
        /// The fewest bytes a value holds.
        const val LEAST: Int = 0

        /// `bytes` as a value of the bound, or `null` where it lies outside it.
        fun of(bytes: ByteArray): CodecZenohOpenBodyCookieBytes? = CodecZenohOpenBodyCookieBytes(bytes)

        /// The fewest bytes the bound holds, each zero: what a value made by
        /// default holds.
        fun fewest(): CodecZenohOpenBodyCookieBytes = CodecZenohOpenBodyCookieBytes(ByteArray(LEAST))
    }
}

/// The elements a value of [CodecZenohOpenBody] presents, which its encode reads
/// (docs/adr/0081 §2): any object that presents them is a source of it, which
/// [CodecZenohOpenBody.encodeSource] writes. The encode derives every other element
/// of the frame.
interface CodecZenohOpenBodySource {
    val lease: ULong
    val initial_sn: ULong
    val cookie: CodecZenohOpenBodyCookieBytes?
}

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecZenohOpenBody()` before any encode()/decode() call. Each default
// is a value of that field's own type, which decode() then fills in on
// success — the carrier's zero for a number, and for an enum the first
// variant its document declares, since a closed set does not hold a
// value it never declared.
data class CodecZenohOpenBody(
    override var lease: ULong = 0uL,
    override var initial_sn: ULong = 0uL,
    override var cookie: CodecZenohOpenBodyCookieBytes? = null
) : CodecZenohOpenBodySource {
    /// RFC §synth-5-B encode-side primary: write `self` into the
    /// caller-owned `w` sink. Returns `null` on success;
    /// `CodecError.BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable
    /// sinks (e.g. `MutableListSink`) are effectively infallible.
    @Suppress("UNUSED_PARAMETER")
    fun encode(w: SceSink, A: UByte): CodecError? {
        // docs/adr/0081 step 4: the encode over any source, which derives
        // every element the value does not present. It is handed the flag
        // inputs the value does not say; one the value says is read off it,
        // and what a caller hands this `encode` for it is held to that
        // (docs/adr/0065 §6 R4a).
        if (A != inputA(this)) return CodecError.PresentIfMismatch
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
    fun encodeToByteArray(A: UByte): ByteArray? {
        val _list = mutableListOf<Byte>()
        if (encode(MutableListSink(_list), A) != null) return null
        return _list.toByteArray()
    }

    companion object {
        /// Write any source of [CodecZenohOpenBody] into `w`: each element read once off
        /// `src` and written where the codec's encode plan lays it out
        /// (docs/adr/0081). It derives `cookie_len` from what each measures, and never reads it off the
        /// source. It refuses no value: it answers `null`, or
        /// [CodecError.BufferOverflow] from a bounded sink with no room left.
        fun encodeSource(src: CodecZenohOpenBodySource, w: SceSink): CodecError? {
            val _e_lease = src.lease
            val _e_initial_sn = src.initial_sn
            val _e_cookie = src.cookie
            val A = (if (_e_cookie != null) 0 else 1).toUByte()
            val _d_cookie_len = if (_e_cookie != null) _e_cookie.bytes.size.toULong() else 0.toULong()
            w.writeVleU64(_e_lease)?.let { return it }
            w.writeVleU64(_e_initial_sn)?.let { return it }
            if (A.toUInt() == 0u) {
                w.writeVleU64(_d_cookie_len)?.let { return it }
            }
            if (_e_cookie != null) {
                w.writeBytes(_e_cookie.bytes)?.let { return it }
            }
            return null
        }

        /// The flag input `A` a source of [CodecZenohOpenBody] says, which its encode
        /// is not handed (docs/adr/0065 §6 R4a).
        fun inputA(src: CodecZenohOpenBodySource): UByte =
            (if (src.cookie != null) 0 else 1).toUByte()

        /// Decode the next frame from `cursor`. On success the cursor
        /// advances past the consumed bytes and the value is `Decoded.Ok`;
        /// a refused frame is `Decoded.Refused` with the kind that refused
        /// it (docs/adr/0099, amendment of 2026-11-16): `NeedMoreBytes` when
        /// the cursor's tail is shorter than the frame (RFC
        /// §synth-5-B L494-519), which the caller resumes after appending
        /// bytes, and another kind for a frame no more bytes make whole.
        @Suppress("UNUSED_PARAMETER")
        fun decode(cursor: SceCursor, A: UByte): Decoded<CodecZenohOpenBody> {
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
            val lease = cursor.readVleU64().valueOr { _refused -> return _refused }
            val initial_sn = cursor.readVleU64().valueOr { _refused -> return _refused }
            val cookie_len: ULong? = if ((A.toInt() and 0x01) == 0) {
                val _v = cursor.readVleU64().valueOr { _refused -> return _refused }
                _v
            } else {
                null
            }
            val cookie = if ((A.toInt() and 0x01) == 0) {
                val _len = cookie_len!!
                if (_len > (Int.MAX_VALUE - 0).toULong()) return Decoded.Refused(CodecError.NeedMoreBytes)
                val _n = _len.toInt()
                val raw = cursor.peekSlice(_n) ?: return Decoded.Refused(CodecError.NeedMoreBytes)
                val _v = raw.copyOf()
                if (!cursor.advance(_n)) return Decoded.Refused(CodecError.NeedMoreBytes)
                _v
            } else {
                null
            }
            return Decoded.Ok(
                CodecZenohOpenBody(
                    lease = lease,
                    initial_sn = initial_sn,
                    cookie = cookie?.let { CodecZenohOpenBodyCookieBytes.of(it) ?: return Decoded.Refused(CodecError.LengthOutOfRange) }
                )
            )
        }
    }
}
