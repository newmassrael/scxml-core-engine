// SCE-MAP: codec_present_if_disjunction:15 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_present_if_disjunction

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.Decoded
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink

/// The `flags_wants_a_or_wants_b` group of [CodecPresentIfDisjunction]: the arm the value holds, each the
/// members present while the gates hold its values (docs/adr/0065 §6 R4).
sealed class CodecPresentIfDisjunctionFlagsWantsAOrWantsB {
    /// The `flags_wants_a_or_wants_b` group of [CodecPresentIfDisjunction] where `flags.wants_a` holds and `flags.wants_b` does not hold (docs/adr/0065 §6 R4): the
    /// members present then.
    data class WantsA(
        val seq: UShort = 0.toUShort()
    ) : CodecPresentIfDisjunctionFlagsWantsAOrWantsB()
    /// The `flags_wants_a_or_wants_b` group of [CodecPresentIfDisjunction] where `flags.wants_a` does not hold and `flags.wants_b` holds (docs/adr/0065 §6 R4): the
    /// members present then.
    data class WantsB(
        val seq: UShort = 0.toUShort()
    ) : CodecPresentIfDisjunctionFlagsWantsAOrWantsB()
    /// The `flags_wants_a_or_wants_b` group of [CodecPresentIfDisjunction] where `flags.wants_a` holds and `flags.wants_b` holds (docs/adr/0065 §6 R4): the
    /// members present then.
    data class WantsAWantsB(
        val seq: UShort = 0.toUShort()
    ) : CodecPresentIfDisjunctionFlagsWantsAOrWantsB()
}

/// The elements a value of [CodecPresentIfDisjunction] presents, which its encode reads
/// (docs/adr/0081 §2): any object that presents them is a source of it, which
/// [CodecPresentIfDisjunction.encodeSource] writes. The encode derives every other element
/// of the frame.
interface CodecPresentIfDisjunctionSource {
    val flags: UByte
    val flags_wants_a_or_wants_b: CodecPresentIfDisjunctionFlagsWantsAOrWantsB?
}

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecPresentIfDisjunction()` before any encode()/decode() call. Each default
// is a value of that field's own type, which decode() then fills in on
// success — the carrier's zero for a number, and for an enum the first
// variant its document declares, since a closed set does not hold a
// value it never declared.
data class CodecPresentIfDisjunction(
    override var flags: UByte = 0.toUByte(),
    override var flags_wants_a_or_wants_b: CodecPresentIfDisjunctionFlagsWantsAOrWantsB? = null
) : CodecPresentIfDisjunctionSource {
    // RFC §synth-5-B flags primitive: per-bit-range accessors over
    // the carrier field. Single-bit (width=1) reads as Boolean; multi-
    // bit (width>=2) reads as the smallest unsigned Kotlin type that
    // fits (UByte / UShort / UInt / ULong). UByte/UShort widen through
    // `.toInt()` and UInt/ULong through `.toLong()` for the bitwise
    // ops; the result narrows back via the carrier's `toU*` ctor.
    fun wantsA(): Boolean = (this.flags.toInt() and 0x01) != 0

    fun setWantsA(v: Boolean) {
        this.flags = if (v) {
            (this.flags.toInt() or 0x01).toUByte()
        } else {
            (this.flags.toInt() and 0x01.inv()).toUByte()
        }
    }

    fun wantsB(): Boolean = (this.flags.toInt() and 0x02) != 0

    fun setWantsB(v: Boolean) {
        this.flags = if (v) {
            (this.flags.toInt() or 0x02).toUByte()
        } else {
            (this.flags.toInt() and 0x02.inv()).toUByte()
        }
    }

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
        /// Write any source of [CodecPresentIfDisjunction] into `w`: each element read once off
        /// `src` and written where the codec's encode plan lays it out
        /// (docs/adr/0081). It refuses no value: it answers `null`, or
        /// [CodecError.BufferOverflow] from a bounded sink with no room left.
        fun encodeSource(src: CodecPresentIfDisjunctionSource, w: SceSink): CodecError? {
            val _e_flags = src.flags
            val _g_flags_wants_a_or_wants_b = src.flags_wants_a_or_wants_b
            val _e_seq = when (val _g = _g_flags_wants_a_or_wants_b) { is CodecPresentIfDisjunctionFlagsWantsAOrWantsB.WantsA -> _g.seq; is CodecPresentIfDisjunctionFlagsWantsAOrWantsB.WantsB -> _g.seq; is CodecPresentIfDisjunctionFlagsWantsAOrWantsB.WantsAWantsB -> _g.seq; else -> null }
            var _u_flags = _e_flags.toUInt()
            _u_flags = (_u_flags and 0x1u.inv()) or (if ((_g_flags_wants_a_or_wants_b is CodecPresentIfDisjunctionFlagsWantsAOrWantsB.WantsA || _g_flags_wants_a_or_wants_b is CodecPresentIfDisjunctionFlagsWantsAOrWantsB.WantsAWantsB)) 0x1u else 0x0u)
            _u_flags = (_u_flags and 0x2u.inv()) or (if ((_g_flags_wants_a_or_wants_b is CodecPresentIfDisjunctionFlagsWantsAOrWantsB.WantsB || _g_flags_wants_a_or_wants_b is CodecPresentIfDisjunctionFlagsWantsAOrWantsB.WantsAWantsB)) 0x2u else 0x0u)
            val _c_flags = _u_flags.toUByte()
            w.writeU8(_c_flags.toByte())?.let { return it }
            if (_e_seq != null) {
                w.writeU8((_e_seq.toInt() ushr 8 and 0xFF).toByte())?.let { return it }
                w.writeU8((_e_seq.toInt() and 0xFF).toByte())?.let { return it }
            }
            return null
        }

        /// Decode the next frame from `cursor`. On success the cursor
        /// advances past the consumed bytes and the value is `Decoded.Ok`;
        /// a refused frame is `Decoded.Refused` with the kind that refused
        /// it (docs/adr/0099, amendment of 2026-11-16): `NeedMoreBytes` when
        /// the cursor's tail is shorter than the frame (RFC §synth-5-B
        /// L494-519), which the caller resumes after appending bytes, and
        /// another kind for a frame no more bytes make whole.
        fun decode(cursor: SceCursor): Decoded<CodecPresentIfDisjunction> {
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
            val flags = run {
                val raw = cursor.peekSlice(1) ?: return Decoded.Refused(CodecError.NeedMoreBytes)
                val _v = raw[0].toUByte()
                if (!cursor.advance(1)) return Decoded.Refused(CodecError.NeedMoreBytes)
                _v
            }
            val seq = if ((flags.toInt() and 0x01) != 0 || (flags.toInt() and 0x02) != 0) {
                val raw = cursor.peekSlice(2) ?: return Decoded.Refused(CodecError.NeedMoreBytes)
                val _v = (((raw[0].toInt() and 0xFF) shl 8) or (raw[1].toInt() and 0xFF)).toUShort()
                if (!cursor.advance(2)) return Decoded.Refused(CodecError.NeedMoreBytes)
                _v
            } else {
                null
            }
            return Decoded.Ok(
                CodecPresentIfDisjunction(
                    flags = flags,
                    flags_wants_a_or_wants_b = if (((flags.toUInt() shr 0) and 0x1u) != 0x0u && ((flags.toUInt() shr 1) and 0x1u) == 0x0u) CodecPresentIfDisjunctionFlagsWantsAOrWantsB.WantsA(seq = seq!!) else if (((flags.toUInt() shr 0) and 0x1u) == 0x0u && ((flags.toUInt() shr 1) and 0x1u) != 0x0u) CodecPresentIfDisjunctionFlagsWantsAOrWantsB.WantsB(seq = seq!!) else if (((flags.toUInt() shr 0) and 0x1u) != 0x0u && ((flags.toUInt() shr 1) and 0x1u) != 0x0u) CodecPresentIfDisjunctionFlagsWantsAOrWantsB.WantsAWantsB(seq = seq!!) else null
                )
            )
        }
    }
}
