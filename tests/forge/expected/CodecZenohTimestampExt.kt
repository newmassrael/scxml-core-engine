// SCE-MAP: codec_zenoh_timestamp_ext:48 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_zenoh_timestamp_ext

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.Decoded
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink
import com.sce.generated.codec_zenoh_timestamp.*

/// The elements a value of [CodecZenohTimestampExt] presents, which its encode reads
/// (docs/adr/0081 §2): any object that presents them is a source of it, which
/// [CodecZenohTimestampExt.encodeSource] writes. The encode derives every other element
/// of the frame.
interface CodecZenohTimestampExtSource {
    val ts: CodecZenohTimestampSource
}

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecZenohTimestampExt()` before any encode()/decode() call. Each default
// is a value of that field's own type, which decode() then fills in on
// success — the carrier's zero for a number, and for an enum the first
// variant its document declares, since a closed set does not hold a
// value it never declared.
data class CodecZenohTimestampExt(
    override var ts: CodecZenohTimestamp = CodecZenohTimestamp()
) : CodecZenohTimestampExtSource {
    /// RFC §synth-5-B encode-side primary: write `self` into the
    /// caller-owned `w` sink. Returns `null` on success;
    /// `CodecError.BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable
    /// sinks (e.g. `MutableListSink`) are effectively infallible.
    fun encode(w: SceSink): CodecError? {
        // docs/adr/0081 step 4: the encode over any source, which derives
        // every element the value does not present. It is handed the flag
        // inputs the value does not say; one the value says is read off it,
        // and what a caller hands this `encode` for it is held to that
        // (docs/adr/0065 §6 R4a).
        return encodeSource(this, w)
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
        /// Write any source of [CodecZenohTimestampExt] into `w`: each element read once off
        /// `src` and written where the codec's encode plan lays it out
        /// (docs/adr/0081). It derives `ext_size` from what each measures, and never reads it off the
        /// source. It refuses no value: it answers `null`, or
        /// [CodecError.BufferOverflow] from a bounded sink with no room left.
        fun encodeSource(src: CodecZenohTimestampExtSource, w: SceSink): CodecError? {
            val _e_ts = src.ts
            val _l_ts = mutableListOf<Byte>()
            CodecZenohTimestamp.encodeSource(_e_ts, MutableListSink(_l_ts))?.let { return it }
            val _b_ts = _l_ts.toByteArray()
            val _d_ext_size = _b_ts.size.toULong()
            w.writeVleU64(_d_ext_size)?.let { return it }
            w.writeBytes(_b_ts)?.let { return it }
            return null
        }

        /// Decode the next frame from `cursor`. On success the cursor
        /// advances past the consumed bytes and the value is `Decoded.Ok`;
        /// a refused frame is `Decoded.Refused` with the kind that refused
        /// it (docs/adr/0099, amendment of 2026-11-16): `NeedMoreBytes` when
        /// the cursor's tail is shorter than the frame (RFC §synth-5-B
        /// L494-519), which the caller resumes after appending bytes, and
        /// another kind for a frame no more bytes make whole.
        fun decode(cursor: SceCursor): Decoded<CodecZenohTimestampExt> {
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
            val ext_size = cursor.readVleU64().valueOr { _refused -> return _refused }
            val ts = run {
                val _len = (ext_size).toInt()
                val _raw = cursor.peekSlice(_len) ?: return Decoded.Refused(CodecError.NeedMoreBytes)
                val _inner = SceCursor(_raw)
                val _v = CodecZenohTimestamp.decode(_inner).valueOr { _refused -> return _refused }
                if (!cursor.advance(_len)) return Decoded.Refused(CodecError.NeedMoreBytes)
                _v
            }
            return Decoded.Ok(
                CodecZenohTimestampExt(
                    ts = ts
                )
            )
        }
    }
}
