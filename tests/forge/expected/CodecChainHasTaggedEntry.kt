// SCE-MAP: codec_chain_has_tagged_entry:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_chain_has_tagged_entry

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.Decoded
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink

/// One-bit flags of [CodecChainHasTaggedEntry] a parent may set in place of the ones a value
/// presents, such as the flag a chain whose entries are [CodecChainHasTaggedEntry] ends them
/// on, which the chain derives from where each entry stands.
class CodecChainHasTaggedEntryOverrides(
    /// The `more` flag of `ctl`, where a parent sets it.
    val ctl_more: Boolean? = null,
)

/// The elements a value of [CodecChainHasTaggedEntry] presents, which its encode reads
/// (docs/adr/0081 §2): any object that presents them is a source of it, which
/// [CodecChainHasTaggedEntry.encodeSource] writes. The encode derives every other element
/// of the frame.
interface CodecChainHasTaggedEntrySource {
    val entry_type: UByte
    val ctl: UByte
    val body: UByte
}

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecChainHasTaggedEntry()` before any encode()/decode() call. Each default
// is a value of that field's own type, which decode() then fills in on
// success — the carrier's zero for a number, and for an enum the first
// variant its document declares, since a closed set does not hold a
// value it never declared.
data class CodecChainHasTaggedEntry(
    override var entry_type: UByte = 0.toUByte(),
    override var ctl: UByte = 0.toUByte(),
    override var body: UByte = 0.toUByte()
) : CodecChainHasTaggedEntrySource {
    // RFC §synth-5-B flags primitive: per-bit-range accessors over
    // the carrier field. Single-bit (width=1) reads as Boolean; multi-
    // bit (width>=2) reads as the smallest unsigned Kotlin type that
    // fits (UByte / UShort / UInt / ULong). UByte/UShort widen through
    // `.toInt()` and UInt/ULong through `.toLong()` for the bitwise
    // ops; the result narrows back via the carrier's `toU*` ctor.
    fun more(): Boolean = (this.ctl.toInt() and 0x01) != 0

    fun setMore(v: Boolean) {
        this.ctl = if (v) {
            (this.ctl.toInt() or 0x01).toUByte()
        } else {
            (this.ctl.toInt() and 0x01.inv()).toUByte()
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
        /// Write any source of [CodecChainHasTaggedEntry] into `w`: each element read once off
        /// `src` and written where the codec's encode plan lays it out
        /// (docs/adr/0081). A chain that holds it
        /// sets the flag its entries end on by `overrides`, in place of the value's. It refuses no value: it answers `null`, or
        /// [CodecError.BufferOverflow] from a bounded sink with no room left.
        fun encodeSource(src: CodecChainHasTaggedEntrySource, w: SceSink, overrides: CodecChainHasTaggedEntryOverrides? = null): CodecError? {
            val _e_entry_type = src.entry_type
            val _e_ctl = src.ctl
            val _e_body = src.body
            var _u_ctl = _e_ctl.toUInt()
            overrides?.ctl_more?.let { _u_ctl = (_u_ctl and 0x1u.inv()) or (if (it) 0x1u else 0x0u) }
            val _c_ctl = _u_ctl.toUByte()
            w.writeU8(_e_entry_type.toByte())?.let { return it }
            w.writeU8(_c_ctl.toByte())?.let { return it }
            w.writeU8(_e_body.toByte())?.let { return it }
            return null
        }

        /// Decode the next frame from `cursor`. On success the cursor
        /// advances past the consumed bytes and the value is `Decoded.Ok`;
        /// a refused frame is `Decoded.Refused` with the kind that refused
        /// it (docs/adr/0099, amendment of 2026-11-16): `NeedMoreBytes` when
        /// the cursor's tail is shorter than the frame (RFC
        /// §synth-5-B L494-519), which the caller resumes after appending
        /// bytes, and another kind for a frame no more bytes make whole.
        fun decode(cursor: SceCursor): Decoded<CodecChainHasTaggedEntry> {
            val raw = cursor.peekSlice(3) ?: return Decoded.Refused(CodecError.NeedMoreBytes)
            val entry_type = raw[0].toUByte()
            val ctl = raw[1].toUByte()
            val body = raw[2].toUByte()
            if (!cursor.advance(3)) return Decoded.Refused(CodecError.NeedMoreBytes)
            // Construct in the `return` (no intermediate local) so a field
            // literally named `value` cannot shadow a result-struct local.
            return Decoded.Ok(
                CodecChainHasTaggedEntry(
                    entry_type = entry_type,
                    ctl = ctl,
                    body = body
                )
            )
        }
    }
}
