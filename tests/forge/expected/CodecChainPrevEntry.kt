// SCE-MAP: codec_chain_prev_entry:19 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_chain_prev_entry

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.Decoded
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink

/// The `after_marker` group of [CodecChainPrevEntry]: the arm the value holds, each the
/// members present while the gates hold its values (docs/adr/0065 §6 R4).
sealed class CodecChainPrevEntryAfterMarker {
    /// The `after_marker` group of [CodecChainPrevEntry] where `after_marker` holds (docs/adr/0065 §6 R4): the
    /// members present then.
    data class AfterMarker(
        val long_value: UShort = 0.toUShort()
    ) : CodecChainPrevEntryAfterMarker()
    /// The `after_marker` group of [CodecChainPrevEntry] where `after_marker` does not hold (docs/adr/0065 §6 R4): the
    /// members present then.
    data class NotAfterMarker(
        val short_value: UByte = 0.toUByte()
    ) : CodecChainPrevEntryAfterMarker()
}

/// One-bit flags of [CodecChainPrevEntry] a parent may set in place of the ones a value
/// presents, such as the flag a chain whose entries are [CodecChainPrevEntry] ends them
/// on, which the chain derives from where each entry stands.
class CodecChainPrevEntryOverrides(
    /// The `Z` flag of `header`, where a parent sets it.
    val header_Z: Boolean? = null,
)

/// The elements a value of [CodecChainPrevEntry] presents, which its encode reads
/// (docs/adr/0081 §2): any object that presents them is a source of it, which
/// [CodecChainPrevEntry.encodeSource] writes. The encode derives every other element
/// of the frame.
interface CodecChainPrevEntrySource {
    val header: UByte
    val tag: UByte?
    val after_marker: CodecChainPrevEntryAfterMarker
}

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecChainPrevEntry()` before any encode()/decode() call. Each default
// is a value of that field's own type, which decode() then fills in on
// success — the carrier's zero for a number, and for an enum the first
// variant its document declares, since a closed set does not hold a
// value it never declared.
data class CodecChainPrevEntry(
    override var header: UByte = 0.toUByte(),
    override var tag: UByte? = null,
    override var after_marker: CodecChainPrevEntryAfterMarker = CodecChainPrevEntryAfterMarker.NotAfterMarker()
) : CodecChainPrevEntrySource {
    // RFC §synth-5-B flags primitive: per-bit-range accessors over
    // the carrier field. Single-bit (width=1) reads as Boolean; multi-
    // bit (width>=2) reads as the smallest unsigned Kotlin type that
    // fits (UByte / UShort / UInt / ULong). UByte/UShort widen through
    // `.toInt()` and UInt/ULong through `.toLong()` for the bitwise
    // ops; the result narrows back via the carrier's `toU*` ctor.
    fun extId(): UByte {
        val _carrier = this.header.toInt()
        return ((_carrier shr 0) and 0x0F).toUByte()
    }

    fun setExtId(v: UByte) {
        val _carrier = this.header.toInt()
        val _shifted_mask = 0x0F shl 0
        val _val = (v.toInt() and 0x0F) shl 0
        this.header = ((_carrier and _shifted_mask.inv()) or _val).toUByte()
    }

    fun Z(): Boolean = (this.header.toInt() and 0x80) != 0

    fun setZ(v: Boolean) {
        this.header = if (v) {
            (this.header.toInt() or 0x80).toUByte()
        } else {
            (this.header.toInt() and 0x80.inv()).toUByte()
        }
    }

    /// RFC §synth-5-B encode-side primary: write `self` into the
    /// caller-owned `w` sink. Returns `null` on success;
    /// `CodecError.BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable
    /// sinks (e.g. `MutableListSink`) are effectively infallible.
    @Suppress("UNUSED_PARAMETER")
    fun encode(w: SceSink, afterMarker: UByte, tagged: UByte): CodecError? {
        // docs/adr/0081 step 4: the encode over any source, which derives
        // every element the value does not present. It is handed the flag
        // inputs the value does not say; one the value says is read off it,
        // and what a caller hands this `encode` for it is held to that
        // (docs/adr/0065 §6 R4a).
        if (afterMarker != inputAfterMarker(this)) return CodecError.PresentIfMismatch
        if (tagged != inputTagged(this)) return CodecError.PresentIfMismatch
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
    fun encodeToByteArray(afterMarker: UByte, tagged: UByte): ByteArray? {
        val _list = mutableListOf<Byte>()
        if (encode(MutableListSink(_list), afterMarker, tagged) != null) return null
        return _list.toByteArray()
    }

    companion object {
        /// Write any source of [CodecChainPrevEntry] into `w`: each element read once off
        /// `src` and written where the codec's encode plan lays it out
        /// (docs/adr/0081). A chain that holds it
        /// sets the flag its entries end on by `overrides`, in place of the value's. It refuses no value: it answers `null`, or
        /// [CodecError.BufferOverflow] from a bounded sink with no room left.
        fun encodeSource(src: CodecChainPrevEntrySource, w: SceSink, overrides: CodecChainPrevEntryOverrides? = null): CodecError? {
            val _e_header = src.header
            val _e_tag = src.tag
            val _g_after_marker = src.after_marker
            val _e_long_value = when (val _g = _g_after_marker) { is CodecChainPrevEntryAfterMarker.AfterMarker -> _g.long_value; else -> null }
            val _e_short_value = when (val _g = _g_after_marker) { is CodecChainPrevEntryAfterMarker.NotAfterMarker -> _g.short_value; else -> null }
            var _u_header = _e_header.toUInt()
            overrides?.header_Z?.let { _u_header = (_u_header and 0x80u.inv()) or (if (it) 0x80u else 0x0u) }
            val _c_header = _u_header.toUByte()
            w.writeU8(_c_header.toByte())?.let { return it }
            if (_e_short_value != null) {
                w.writeU8(_e_short_value.toByte())?.let { return it }
            }
            if (_e_long_value != null) {
                w.writeU8((_e_long_value.toInt() ushr 8 and 0xFF).toByte())?.let { return it }
                w.writeU8((_e_long_value.toInt() and 0xFF).toByte())?.let { return it }
            }
            if (_e_tag != null) {
                w.writeU8(_e_tag.toByte())?.let { return it }
            }
            return null
        }

        /// The flag input `after_marker` a source of [CodecChainPrevEntry] says, which its encode
        /// is not handed (docs/adr/0065 §6 R4a).
        fun inputAfterMarker(src: CodecChainPrevEntrySource): UByte =
            (if (src.after_marker is CodecChainPrevEntryAfterMarker.AfterMarker) 1 else 0).toUByte()

        /// The flag input `tagged` a source of [CodecChainPrevEntry] says, which its encode
        /// is not handed (docs/adr/0065 §6 R4a).
        fun inputTagged(src: CodecChainPrevEntrySource): UByte =
            (if (src.tag != null) 1 else 0).toUByte()

        /// Decode the next frame from `cursor`. On success the cursor
        /// advances past the consumed bytes and the value is `Decoded.Ok`;
        /// a refused frame is `Decoded.Refused` with the kind that refused
        /// it (docs/adr/0099, amendment of 2026-11-16): `NeedMoreBytes` when
        /// the cursor's tail is shorter than the frame (RFC §synth-5-B
        /// L494-519), which the caller resumes after appending bytes, and
        /// another kind for a frame no more bytes make whole.
        @Suppress("UNUSED_PARAMETER")
        fun decode(cursor: SceCursor, afterMarker: UByte, tagged: UByte): Decoded<CodecChainPrevEntry> {
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
            val header = run {
                val raw = cursor.peekSlice(1) ?: return Decoded.Refused(CodecError.NeedMoreBytes)
                val _v = raw[0].toUByte()
                if (!cursor.advance(1)) return Decoded.Refused(CodecError.NeedMoreBytes)
                _v
            }
            val short_value = if ((afterMarker.toInt() and 0x01) == 0) {
                val raw = cursor.peekSlice(1) ?: return Decoded.Refused(CodecError.NeedMoreBytes)
                val _v = raw[0].toUByte()
                if (!cursor.advance(1)) return Decoded.Refused(CodecError.NeedMoreBytes)
                _v
            } else {
                null
            }
            val long_value = if ((afterMarker.toInt() and 0x01) != 0) {
                val raw = cursor.peekSlice(2) ?: return Decoded.Refused(CodecError.NeedMoreBytes)
                val _v = (((raw[0].toInt() and 0xFF) shl 8) or (raw[1].toInt() and 0xFF)).toUShort()
                if (!cursor.advance(2)) return Decoded.Refused(CodecError.NeedMoreBytes)
                _v
            } else {
                null
            }
            val tag = if ((tagged.toInt() and 0x01) != 0) {
                val raw = cursor.peekSlice(1) ?: return Decoded.Refused(CodecError.NeedMoreBytes)
                val _v = raw[0].toUByte()
                if (!cursor.advance(1)) return Decoded.Refused(CodecError.NeedMoreBytes)
                _v
            } else {
                null
            }
            return Decoded.Ok(
                CodecChainPrevEntry(
                    header = header,
                    tag = tag,
                    after_marker = if (afterMarker.toUInt() != 0u) CodecChainPrevEntryAfterMarker.AfterMarker(long_value = long_value!!) else CodecChainPrevEntryAfterMarker.NotAfterMarker(short_value = short_value!!)
                )
            )
        }
    }
}
