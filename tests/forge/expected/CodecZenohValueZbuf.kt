// SCE-MAP: codec_zenoh_value_zbuf:27 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_zenoh_value_zbuf

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.Decoded
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink
import com.sce.generated.codec_zenoh_value_slice.*
import kotlin.jvm.JvmInline

/// The `slices` element of [CodecZenohValueZbuf], any number of entries, the protocol stating no most, held to that bound where a
/// value of it is made (docs/adr/0065 §6 R2, docs/adr/0081 §2): its constructor is
/// private and [of] makes a value only of entries within the bound, which it
/// copies, so no list it was made of can grow it past its bound after.
@JvmInline
value class CodecZenohValueZbufSlicesList private constructor(val entries: List<CodecZenohValueSliceSource>) : com.sce.forge.runtime.Counted {
    /// How many entries the list holds, the count a frame writes of it.
    override val size: Int
        get() = entries.size

    companion object {
        /// `entries` as a value of the bound, or `null` where they lie outside it.
        fun of(entries: List<CodecZenohValueSliceSource>): CodecZenohValueZbufSlicesList? = CodecZenohValueZbufSlicesList(entries.toList())

        /// No entry: what a value made by default holds.
        fun fewest(): CodecZenohValueZbufSlicesList = CodecZenohValueZbufSlicesList(emptyList())
    }
}

/// The `value` element of [CodecZenohValueZbuf], 0 to 4294967295 bytes, held to that bound where a
/// value of it is made (docs/adr/0077 §3, docs/adr/0081 §2): its constructor is
/// private and [of] makes a value only of bytes within the bound. The value it
/// holds cannot change after, so a value holds its bound for as long as it lives.
@JvmInline
value class CodecZenohValueZbufValueBytes private constructor(val bytes: ByteArray) {
    companion object {
        /// The fewest bytes a value holds.
        const val LEAST: Int = 0

        /// The most bytes a value holds.
        const val MOST: Long = 4294967295L

        /// `bytes` as a value of the bound, or `null` where it lies outside it.
        fun of(bytes: ByteArray): CodecZenohValueZbufValueBytes? =
            if (bytes.size <= MOST) CodecZenohValueZbufValueBytes(bytes) else null

        /// The fewest bytes the bound holds, each zero: what a value made by
        /// default holds.
        fun fewest(): CodecZenohValueZbufValueBytes = CodecZenohValueZbufValueBytes(ByteArray(LEAST))
    }
}

/// The `after_shm` group of [CodecZenohValueZbuf]: the arm the value holds, each the
/// members present while the gates hold its values (docs/adr/0065 §6 R4).
sealed class CodecZenohValueZbufAfterShm {
    /// The `after_shm` group of [CodecZenohValueZbuf] where `after_shm` holds (docs/adr/0065 §6 R4): the
    /// members present then.
    data class AfterShm(
        val encoding: ULong = 0uL,
        val slices: CodecZenohValueZbufSlicesList = CodecZenohValueZbufSlicesList.fewest(),
        val value_len: ULong = 0uL
    ) : CodecZenohValueZbufAfterShm()
    /// The `after_shm` group of [CodecZenohValueZbuf] where `after_shm` does not hold (docs/adr/0065 §6 R4): the
    /// members present then.
    data class NotAfterShm(
        val value: CodecZenohValueZbufValueBytes = CodecZenohValueZbufValueBytes.fewest()
    ) : CodecZenohValueZbufAfterShm()
}

/// The elements a value of [CodecZenohValueZbuf] presents, which its encode reads
/// (docs/adr/0081 §2): any object that presents them is a source of it, which
/// [CodecZenohValueZbuf.encodeSource] writes. The encode derives every other element
/// of the frame.
interface CodecZenohValueZbufSource {
    val after_shm: CodecZenohValueZbufAfterShm
}

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecZenohValueZbuf()` before any encode()/decode() call. Each default
// is a value of that field's own type, which decode() then fills in on
// success — the carrier's zero for a number, and for an enum the first
// variant its document declares, since a closed set does not hold a
// value it never declared.
data class CodecZenohValueZbuf(
    override var after_shm: CodecZenohValueZbufAfterShm = CodecZenohValueZbufAfterShm.AfterShm()
) : CodecZenohValueZbufSource {
    /// RFC §synth-5-B encode-side primary: write `self` into the
    /// caller-owned `w` sink. Returns `null` on success;
    /// `CodecError.BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable
    /// sinks (e.g. `MutableListSink`) are effectively infallible.
    @Suppress("UNUSED_PARAMETER")
    fun encode(w: SceSink, afterShm: UByte): CodecError? {
        // docs/adr/0081 step 4: the encode over any source, which derives
        // every element the value does not present. It is handed the flag
        // inputs the value does not say; one the value says is read off it,
        // and what a caller hands this `encode` for it is held to that
        // (docs/adr/0065 §6 R4a).
        if (afterShm != inputAfterShm(this)) return CodecError.PresentIfMismatch
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
    fun encodeToByteArray(afterShm: UByte): ByteArray? {
        val _list = mutableListOf<Byte>()
        if (encode(MutableListSink(_list), afterShm) != null) return null
        return _list.toByteArray()
    }

    companion object {
        /// Write any source of [CodecZenohValueZbuf] into `w`: each element read once off
        /// `src` and written where the codec's encode plan lays it out
        /// (docs/adr/0081). It derives `slice_count` from what each measures, and never reads it off the
        /// source. It derives `value_len` where what it measures is there, and reads what
        /// the source states where none is. It refuses no value: it answers `null`, or
        /// [CodecError.BufferOverflow] from a bounded sink with no room left.
        fun encodeSource(src: CodecZenohValueZbufSource, w: SceSink): CodecError? {
            val _g_after_shm = src.after_shm
            val _e_encoding = when (val _g = _g_after_shm) { is CodecZenohValueZbufAfterShm.AfterShm -> _g.encoding; else -> null }
            val _e_slices = when (val _g = _g_after_shm) { is CodecZenohValueZbufAfterShm.AfterShm -> _g.slices; else -> null }
            val _e_value_len = when (val _g = _g_after_shm) { is CodecZenohValueZbufAfterShm.AfterShm -> _g.value_len; else -> null }
            val _e_value = when (val _g = _g_after_shm) { is CodecZenohValueZbufAfterShm.NotAfterShm -> _g.value; else -> null }
            val afterShm = (if (_g_after_shm is CodecZenohValueZbufAfterShm.AfterShm) 1 else 0).toUByte()
            val _d_value_len = if (_e_value != null) _e_value.bytes.size.toULong() else (_e_value_len ?: 0.toULong())
            val _d_slice_count = if (_e_slices != null) _e_slices.entries.size.toULong() else 0.toULong()
            w.writeVleU64(_d_value_len)?.let { return it }
            if (_e_value != null) {
                w.writeBytes(_e_value.bytes)?.let { return it }
            }
            if (_e_encoding != null) {
                w.writeVleU64(_e_encoding)?.let { return it }
            }
            if (afterShm.toUInt() != 0u) {
                w.writeVleU64(_d_slice_count)?.let { return it }
            }
            if (_e_slices != null) {
                for (_entry in _e_slices.entries) {
                    CodecZenohValueSlice.encodeSource(_entry, w)?.let { return it }
                }
            }
            return null
        }

        /// The flag input `after_shm` a source of [CodecZenohValueZbuf] says, which its encode
        /// is not handed (docs/adr/0065 §6 R4a).
        fun inputAfterShm(src: CodecZenohValueZbufSource): UByte =
            (if (src.after_shm is CodecZenohValueZbufAfterShm.AfterShm) 1 else 0).toUByte()

        /// Decode the next frame from `cursor`. On success the cursor
        /// advances past the consumed bytes and the value is `Decoded.Ok`;
        /// a refused frame is `Decoded.Refused` with the kind that refused
        /// it (docs/adr/0099, amendment of 2026-11-16): `NeedMoreBytes` when
        /// the cursor's tail is shorter than the frame (RFC §synth-5-B
        /// L494-519), which the caller resumes after appending bytes, and
        /// another kind for a frame no more bytes make whole.
        @Suppress("UNUSED_PARAMETER")
        fun decode(cursor: SceCursor, afterShm: UByte): Decoded<CodecZenohValueZbuf> {
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
            val value_len = cursor.readVleU64().valueOr { _refused -> return _refused }
            val value = if ((afterShm.toInt() and 0x01) == 0) {
                val _len = value_len
                if (_len > 4294967295uL) {
                    return Decoded.Refused(CodecError.LengthOutOfRange)
                }
                if (_len > (Int.MAX_VALUE - 0).toULong()) return Decoded.Refused(CodecError.NeedMoreBytes)
                val _n = _len.toInt()
                val raw = cursor.peekSlice(_n) ?: return Decoded.Refused(CodecError.NeedMoreBytes)
                val _v = raw.copyOf()
                if (!cursor.advance(_n)) return Decoded.Refused(CodecError.NeedMoreBytes)
                _v
            } else {
                null
            }
            val encoding: ULong? = if ((afterShm.toInt() and 0x01) != 0) {
                val _v = cursor.readVleU64().valueOr { _refused -> return _refused }
                _v
            } else {
                null
            }
            val slice_count: ULong? = if ((afterShm.toInt() and 0x01) != 0) {
                val _v = cursor.readVleU64().valueOr { _refused -> return _refused }
                _v
            } else {
                null
            }
            val slices: MutableList<CodecZenohValueSlice>? = if ((afterShm.toInt() and 0x01) != 0) {
                val _n = slice_count!!
                mutableListOf<CodecZenohValueSlice>().apply {
                    repeat(_n.toInt()) {
                        add(CodecZenohValueSlice.decode(cursor).valueOr { _refused -> return _refused })
                    }
                }
            } else null
            return Decoded.Ok(
                CodecZenohValueZbuf(
                    after_shm = if (afterShm.toUInt() != 0u) CodecZenohValueZbufAfterShm.AfterShm(encoding = encoding!!, slices = CodecZenohValueZbufSlicesList.of(slices!!) ?: return Decoded.Refused(CodecError.TooManyElements), value_len = value_len) else CodecZenohValueZbufAfterShm.NotAfterShm(value = CodecZenohValueZbufValueBytes.of(value!!) ?: return Decoded.Refused(CodecError.LengthOutOfRange))
                )
            )
        }
    }
}
