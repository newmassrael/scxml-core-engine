// SCE-MAP: codec_chain_flag_carrier:13 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_chain_flag_carrier

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.Decoded
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink
import com.sce.generated.codec_chain_flag_entry.*
import kotlin.jvm.JvmInline

/// The `entries` element of [CodecChainFlagCarrier], 1 to 4 entries, held to that bound where a
/// value of it is made (docs/adr/0065 §6 R2, docs/adr/0081 §2): its constructor is
/// private and [of] makes a value only of entries within the bound, which it
/// copies, so no list it was made of can grow it past its bound after.
@JvmInline
value class CodecChainFlagCarrierEntriesList private constructor(val entries: List<CodecChainFlagEntrySource>) : com.sce.forge.runtime.Counted {
    /// How many entries the list holds, the count a frame writes of it.
    override val size: Int
        get() = entries.size

    companion object {
        /// The fewest entries a value holds.
        const val LEAST: Int = 1

        /// The most entries a value holds.
        const val MOST: Int = 4

        /// `entries` as a value of the bound, or `null` where they lie outside it.
        fun of(entries: List<CodecChainFlagEntrySource>): CodecChainFlagCarrierEntriesList? =
            if (entries.size in LEAST..MOST) CodecChainFlagCarrierEntriesList(entries.toList()) else null

        /// The fewest entries the bound holds, each made by default: what a value made by default holds.
        fun fewest(): CodecChainFlagCarrierEntriesList = CodecChainFlagCarrierEntriesList(List(LEAST) { CodecChainFlagEntry() })
    }
}

/// The `entries` element of [CodecChainFlagCarrier], its entries held to the law they say
/// their inputs by (docs/adr/0065 §6 R8, docs/adr/0081 §2): its constructor is
/// private and [of] makes a value only of a list whose entries follow the law
/// from the start the value gives it, so the encode, which writes the flags the
/// list hands its entries from the start, refuses nothing of it.
class CodecChainFlagCarrierEntriesLawful private constructor(
    /// The list, within its bound.
    val list: CodecChainFlagCarrierEntriesList,
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
        other is CodecChainFlagCarrierEntriesLawful && other.list == list && other.startWide == startWide

    override fun hashCode(): Int = 31 * (list.hashCode()) + startWide.hashCode()

    override fun toString(): String = "CodecChainFlagCarrierEntriesLawful(list=$list, startWide=$startWide)"

    companion object {
        /// `list` as a value of the law from the start given, or `null` where an
        /// entry says a value other than the one its place asks of it.
        fun of(list: CodecChainFlagCarrierEntriesList, startWide: Boolean): CodecChainFlagCarrierEntriesLawful? {
            for (entry in list.entries) {
                if ((CodecChainFlagEntry.inputWide(entry).toUInt() != 0u) != startWide) return null
            }
            return CodecChainFlagCarrierEntriesLawful(list, startWide)
        }

        /// The fewest entries of the bound, each made by default, from the start
        /// the first of them says: what a value made by default holds.
        fun fewest(): CodecChainFlagCarrierEntriesLawful {
            val list = CodecChainFlagCarrierEntriesList.fewest()
            val first = list.entries.firstOrNull()
            return checkNotNull(of(list, (first?.let { CodecChainFlagEntry.inputWide(it) }?.toUInt() ?: 0u) != 0u)) { "the entries made by default follow the law" }
        }
    }
}

/// The elements a value of [CodecChainFlagCarrier] presents, which its encode reads
/// (docs/adr/0081 §2): any object that presents them is a source of it, which
/// [CodecChainFlagCarrier.encodeSource] writes. The encode derives every other element
/// of the frame.
interface CodecChainFlagCarrierSource {
    val head: UByte
    val entries: CodecChainFlagCarrierEntriesLawful
}

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecChainFlagCarrier()` before any encode()/decode() call. Each default
// is a value of that field's own type, which decode() then fills in on
// success — the carrier's zero for a number, and for an enum the first
// variant its document declares, since a closed set does not hold a
// value it never declared.
data class CodecChainFlagCarrier(
    override var head: UByte = 0.toUByte(),
    override var entries: CodecChainFlagCarrierEntriesLawful = CodecChainFlagCarrierEntriesLawful.fewest()
) : CodecChainFlagCarrierSource {
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
        /// Write any source of [CodecChainFlagCarrier] into `w`: each element read once off
        /// `src` and written where the codec's encode plan lays it out
        /// (docs/adr/0081). It refuses no value: it answers `null`, or
        /// [CodecError.BufferOverflow] from a bounded sink with no room left.
        fun encodeSource(src: CodecChainFlagCarrierSource, w: SceSink): CodecError? {
            val _e_head = src.head
            val _e_entries = src.entries
            var _u_head = _e_head.toUInt()
            _u_head = (_u_head and 0x1u.inv()) or (((if (_e_entries.startWide) 1 else 0)).toUInt() shl 0)
            val _c_head = _u_head.toUByte()
            w.writeU8(_c_head.toByte())?.let { return it }
            var _left_entries = _e_entries.entries.size
            for (_entry in _e_entries.entries) {
                _left_entries -= 1
                CodecChainFlagEntry.encodeSource(_entry, w, CodecChainFlagEntryOverrides(header_Z = _left_entries != 0))?.let { return it }
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
        fun decode(cursor: SceCursor): Decoded<CodecChainFlagCarrier> {
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
            val entries: MutableList<CodecChainFlagEntry> = mutableListOf<CodecChainFlagEntry>().also {
                var _more = false
                for (_i in 0 until 4) {
                    val _entry = CodecChainFlagEntry.decode(cursor, (((head.toInt() shr 0) and 0x1).toUByte())).valueOr { _refused -> return _refused }
                    _more = _entry.Z()
                    it.add(_entry)
                    if (!_more) break
                }
                if (_more && cursor.remaining() == 0) { return Decoded.Refused(CodecError.NeedMoreBytes) }
                if (_more) { return Decoded.Refused(CodecError.TooManyElements) }
            }
            return Decoded.Ok(
                CodecChainFlagCarrier(
                    head = head,
                    entries = CodecChainFlagCarrierEntriesLawful.of(CodecChainFlagCarrierEntriesList.of(entries) ?: return Decoded.Refused(CodecError.TooManyElements), startWide = ((head.toUInt() shr 0) and 0x1u) != 0x0u) ?: return Decoded.Refused(CodecError.PresentIfMismatch)
                )
            )
        }
    }
}
