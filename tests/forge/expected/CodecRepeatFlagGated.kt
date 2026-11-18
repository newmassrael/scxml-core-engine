// SCE-MAP: codec_repeat_flag_gated:14 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_repeat_flag_gated

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.Decoded
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink
import com.sce.generated.codec_chain_flag_entry.*
import kotlin.jvm.JvmInline

/// The `entries` element of [CodecRepeatFlagGated], 0 to 4 entries, held to that bound where a
/// value of it is made (docs/adr/0065 §6 R2, docs/adr/0081 §2): its constructor is
/// private and [of] makes a value only of entries within the bound, which it
/// copies, so no list it was made of can grow it past its bound after.
@JvmInline
value class CodecRepeatFlagGatedEntriesList private constructor(val entries: List<CodecChainFlagEntrySource>) : com.sce.forge.runtime.Counted {
    /// How many entries the list holds, the count a frame writes of it.
    override val size: Int
        get() = entries.size

    companion object {
        /// The most entries a value holds.
        const val MOST: Int = 4

        /// `entries` as a value of the bound, or `null` where they lie outside it.
        fun of(entries: List<CodecChainFlagEntrySource>): CodecRepeatFlagGatedEntriesList? =
            if (entries.size <= MOST) CodecRepeatFlagGatedEntriesList(entries.toList()) else null

        /// No entry: what a value made by default holds.
        fun fewest(): CodecRepeatFlagGatedEntriesList = CodecRepeatFlagGatedEntriesList(emptyList())
    }
}

/// The `entries` element of [CodecRepeatFlagGated], its entries held to the law they say
/// their inputs by (docs/adr/0065 §6 R8, docs/adr/0081 §2): its constructor is
/// private and [of] makes a value only of a list whose entries follow the law
/// from the start the value gives it, so the encode, which writes the flags the
/// list hands its entries from the start, refuses nothing of it.
class CodecRepeatFlagGatedEntriesLawful private constructor(
    /// The list, within its bound.
    val list: CodecRepeatFlagGatedEntriesList,
    /// The value of the input `wide` the list hands every entry: where its
    /// law starts.
    val startWide: Boolean,
) : com.sce.forge.runtime.Counted {
    /// The entries the list holds.
    val entries: List<CodecChainFlagEntrySource>
        get() = list.entries

    /// How many entries the list holds, the count a frame writes of it.
    override val size: Int
        get() = list.size

    override fun equals(other: Any?): Boolean =
        other is CodecRepeatFlagGatedEntriesLawful && other.list == list && other.startWide == startWide

    override fun hashCode(): Int = 31 * (list.hashCode()) + startWide.hashCode()

    override fun toString(): String = "CodecRepeatFlagGatedEntriesLawful(list=$list, startWide=$startWide)"

    companion object {
        /// `list` as a value of the law from the start given, or `null` where an
        /// entry says a value other than the one its place asks of it.
        fun of(list: CodecRepeatFlagGatedEntriesList, startWide: Boolean): CodecRepeatFlagGatedEntriesLawful? {
            for (entry in list.entries) {
                if ((CodecChainFlagEntry.inputWide(entry).toUInt() != 0u) != startWide) return null
            }
            return CodecRepeatFlagGatedEntriesLawful(list, startWide)
        }

        /// The fewest entries of the bound, each made by default, from the start
        /// the first of them says: what a value made by default holds.
        fun fewest(): CodecRepeatFlagGatedEntriesLawful {
            val list = CodecRepeatFlagGatedEntriesList.fewest()
            val first = list.entries.firstOrNull()
            return checkNotNull(of(list, (first?.let { CodecChainFlagEntry.inputWide(it) }?.toUInt() ?: 0u) != 0u)) { "the entries made by default follow the law" }
        }
    }
}

/// The elements a value of [CodecRepeatFlagGated] presents, which its encode reads
/// (docs/adr/0081 §2): any object that presents them is a source of it, which
/// [CodecRepeatFlagGated.encodeSource] writes. The encode derives every other element
/// of the frame.
interface CodecRepeatFlagGatedSource {
    val head: UByte
    val entries: CodecRepeatFlagGatedEntriesLawful?
}

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecRepeatFlagGated()` before any encode()/decode() call. Each default
// is a value of that field's own type, which decode() then fills in on
// success — the carrier's zero for a number, and for an enum the first
// variant its document declares, since a closed set does not hold a
// value it never declared.
data class CodecRepeatFlagGated(
    override var head: UByte = 0.toUByte(),
    override var entries: CodecRepeatFlagGatedEntriesLawful? = null
) : CodecRepeatFlagGatedSource {
    // RFC §synth-5-B flags primitive: per-bit-range accessors over
    // the carrier field. Single-bit (width=1) reads as Boolean; multi-
    // bit (width>=2) reads as the smallest unsigned Kotlin type that
    // fits (UByte / UShort / UInt / ULong). UByte/UShort widen through
    // `.toInt()` and UInt/ULong through `.toLong()` for the bitwise
    // ops; the result narrows back via the carrier's `toU*` ctor.
    fun W(): Boolean = (this.head.toInt() and 0x01) != 0

    fun setW(v: Boolean) {
        this.head = if (v) {
            (this.head.toInt() or 0x01).toUByte()
        } else {
            (this.head.toInt() and 0x01.inv()).toUByte()
        }
    }

    fun L(): Boolean = (this.head.toInt() and 0x02) != 0

    fun setL(v: Boolean) {
        this.head = if (v) {
            (this.head.toInt() or 0x02).toUByte()
        } else {
            (this.head.toInt() and 0x02.inv()).toUByte()
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
        /// Write any source of [CodecRepeatFlagGated] into `w`: each element read once off
        /// `src` and written where the codec's encode plan lays it out
        /// (docs/adr/0081). It derives `count` from what each measures, and never reads it off the
        /// source. It refuses no value: it answers `null`, or
        /// [CodecError.BufferOverflow] from a bounded sink with no room left.
        fun encodeSource(src: CodecRepeatFlagGatedSource, w: SceSink): CodecError? {
            val _e_head = src.head
            val _e_entries = src.entries
            val _d_count = if (_e_entries != null) _e_entries.entries.size.toUByte() else 0.toUByte()
            var _u_head = _e_head.toUInt()
            if (_e_entries != null) {
                _u_head = (_u_head and 0x1u.inv()) or (((if (_e_entries.startWide) 1 else 0)).toUInt() shl 0)
            }
            _u_head = (_u_head and 0x2u.inv()) or (if (_e_entries != null) 0x2u else 0x0u)
            val _c_head = _u_head.toUByte()
            w.writeU8(_c_head.toByte())?.let { return it }
            if (((_c_head.toUInt() shr 1) and 0x1u) != 0x0u) {
                w.writeU8(_d_count.toByte())?.let { return it }
            }
            if (_e_entries != null) {
                for (_entry in _e_entries.entries) {
                    CodecChainFlagEntry.encodeSource(_entry, w)?.let { return it }
                }
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
        fun decode(cursor: SceCursor): Decoded<CodecRepeatFlagGated> {
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
            val head = run {
                val raw = cursor.peekSlice(1) ?: return Decoded.Refused(CodecError.NeedMoreBytes)
                val _v = raw[0].toUByte()
                if (!cursor.advance(1)) return Decoded.Refused(CodecError.NeedMoreBytes)
                _v
            }
            val count = if ((head.toInt() and 0x02) != 0) {
                val raw = cursor.peekSlice(1) ?: return Decoded.Refused(CodecError.NeedMoreBytes)
                val _v = raw[0].toUByte()
                if (!cursor.advance(1)) return Decoded.Refused(CodecError.NeedMoreBytes)
                _v
            } else {
                null
            }
            val entries: MutableList<CodecChainFlagEntry>? = if ((head.toInt() and 0x02) != 0) {
                val _n = count!!
                if (_n > 4u) {
                    return Decoded.Refused(CodecError.TooManyElements)
                }
                mutableListOf<CodecChainFlagEntry>().apply {
                    repeat(_n.toInt()) {
                        add(CodecChainFlagEntry.decode(cursor, (((head.toInt() shr 0) and 0x1).toUByte())).valueOr { _refused -> return _refused })
                    }
                }
            } else null
            return Decoded.Ok(
                CodecRepeatFlagGated(
                    head = head,
                    entries = entries?.let { CodecRepeatFlagGatedEntriesLawful.of(CodecRepeatFlagGatedEntriesList.of(it) ?: return Decoded.Refused(CodecError.TooManyElements), startWide = ((head.toUInt() shr 0) and 0x1u) != 0x0u) ?: return Decoded.Refused(CodecError.PresentIfMismatch) }
                )
            )
        }
    }
}
