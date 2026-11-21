// SCE-MAP: codec_chain_has_tagged:11 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_chain_has_tagged

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.Decoded
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink
import com.sce.generated.codec_chain_has_tagged_entry.*

/// The `entries` chain of [CodecChainHasTagged] where it holds an entry of each kind of 0x7, 0x9, 1 to 3 entries, held
/// to that where a value of it is made (docs/adr/0065 §6 R6b, docs/adr/0081 §2):
/// its constructor is private and [of] makes a value only of entries within the
/// bound that hold those kinds, which it copies, so no list it was made of can
/// change it after.
@JvmInline
value class CodecChainHasTaggedEntriesHas7Has9Chain private constructor(val entries: List<CodecChainHasTaggedEntrySource>) : com.sce.forge.runtime.Counted {
    /// How many entries the chain holds.
    override val size: Int
        get() = entries.size

    companion object {
        /// The fewest entries a value holds.
        const val LEAST: Int = 1

        /// The most entries a value holds.
        const val MOST: Int = 3

        /// The kind of `entry`: the identifier the chain tells its entries apart by.
        private fun kind(entry: CodecChainHasTaggedEntrySource): ULong = entry.entry_type.toULong()

        /// `entries` as a value of the chain, or `null` where they lie outside its
        /// bound, lack a kind it holds or hold one it lacks.
        fun of(entries: List<CodecChainHasTaggedEntrySource>): CodecChainHasTaggedEntriesHas7Has9Chain? =
            if (entries.size in LEAST..MOST && entries.any { kind(it) == 0x7uL } && entries.any { kind(it) == 0x9uL }) CodecChainHasTaggedEntriesHas7Has9Chain(entries.toList()) else null
    }
}

/// The `entries` chain of [CodecChainHasTagged] where it holds an entry of each kind of 0x7 and no entry of a kind of 0x9, 1 to 3 entries, held
/// to that where a value of it is made (docs/adr/0065 §6 R6b, docs/adr/0081 §2):
/// its constructor is private and [of] makes a value only of entries within the
/// bound that hold those kinds, which it copies, so no list it was made of can
/// change it after.
@JvmInline
value class CodecChainHasTaggedEntriesHas7Lacks9Chain private constructor(val entries: List<CodecChainHasTaggedEntrySource>) : com.sce.forge.runtime.Counted {
    /// How many entries the chain holds.
    override val size: Int
        get() = entries.size

    companion object {
        /// The fewest entries a value holds.
        const val LEAST: Int = 1

        /// The most entries a value holds.
        const val MOST: Int = 3

        /// The kind of `entry`: the identifier the chain tells its entries apart by.
        private fun kind(entry: CodecChainHasTaggedEntrySource): ULong = entry.entry_type.toULong()

        /// `entries` as a value of the chain, or `null` where they lie outside its
        /// bound, lack a kind it holds or hold one it lacks.
        fun of(entries: List<CodecChainHasTaggedEntrySource>): CodecChainHasTaggedEntriesHas7Lacks9Chain? =
            if (entries.size in LEAST..MOST && entries.any { kind(it) == 0x7uL } && entries.none { kind(it) == 0x9uL }) CodecChainHasTaggedEntriesHas7Lacks9Chain(entries.toList()) else null
    }
}

/// The `entries` chain of [CodecChainHasTagged] where it holds an entry of each kind of 0x9 and no entry of a kind of 0x7, 1 to 3 entries, held
/// to that where a value of it is made (docs/adr/0065 §6 R6b, docs/adr/0081 §2):
/// its constructor is private and [of] makes a value only of entries within the
/// bound that hold those kinds, which it copies, so no list it was made of can
/// change it after.
@JvmInline
value class CodecChainHasTaggedEntriesHas9Lacks7Chain private constructor(val entries: List<CodecChainHasTaggedEntrySource>) : com.sce.forge.runtime.Counted {
    /// How many entries the chain holds.
    override val size: Int
        get() = entries.size

    companion object {
        /// The fewest entries a value holds.
        const val LEAST: Int = 1

        /// The most entries a value holds.
        const val MOST: Int = 3

        /// The kind of `entry`: the identifier the chain tells its entries apart by.
        private fun kind(entry: CodecChainHasTaggedEntrySource): ULong = entry.entry_type.toULong()

        /// `entries` as a value of the chain, or `null` where they lie outside its
        /// bound, lack a kind it holds or hold one it lacks.
        fun of(entries: List<CodecChainHasTaggedEntrySource>): CodecChainHasTaggedEntriesHas9Lacks7Chain? =
            if (entries.size in LEAST..MOST && entries.any { kind(it) == 0x9uL } && entries.none { kind(it) == 0x7uL }) CodecChainHasTaggedEntriesHas9Lacks7Chain(entries.toList()) else null
    }
}

/// The `entries` chain of [CodecChainHasTagged] where it holds no entry of a kind of 0x7, 0x9, 1 to 3 entries, held
/// to that where a value of it is made (docs/adr/0065 §6 R6b, docs/adr/0081 §2):
/// its constructor is private and [of] makes a value only of entries within the
/// bound that hold those kinds, which it copies, so no list it was made of can
/// change it after.
@JvmInline
value class CodecChainHasTaggedEntriesLacks7Lacks9Chain private constructor(val entries: List<CodecChainHasTaggedEntrySource>) : com.sce.forge.runtime.Counted {
    /// How many entries the chain holds.
    override val size: Int
        get() = entries.size

    companion object {
        /// The fewest entries a value holds.
        const val LEAST: Int = 1

        /// The most entries a value holds.
        const val MOST: Int = 3

        /// The kind of `entry`: the identifier the chain tells its entries apart by.
        private fun kind(entry: CodecChainHasTaggedEntrySource): ULong = entry.entry_type.toULong()

        /// `entries` as a value of the chain, or `null` where they lie outside its
        /// bound, lack a kind it holds or hold one it lacks.
        fun of(entries: List<CodecChainHasTaggedEntrySource>): CodecChainHasTaggedEntriesLacks7Lacks9Chain? =
            if (entries.size in LEAST..MOST && entries.none { kind(it) == 0x7uL } && entries.none { kind(it) == 0x9uL }) CodecChainHasTaggedEntriesLacks7Lacks9Chain(entries.toList()) else null

        /// The fewest entries the bound holds, each made by default, which hold no kind the chain
        /// lacks: what a value made by default holds.
        fun fewest(): CodecChainHasTaggedEntriesLacks7Lacks9Chain =
            checkNotNull(of(List(LEAST) { CodecChainHasTaggedEntry() })) { "a value-made entry is of no kind the chain lacks" }
    }
}

/// The `entries_has_0x7_has_0x9_header_wide` group of [CodecChainHasTagged]: the arm the value holds, each the
/// members present while the gates hold its values (docs/adr/0065 §6 R4).
sealed class CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide {
    /// The `entries_has_0x7_has_0x9_header_wide` group of [CodecChainHasTagged] where `entries.has(0x7)` holds and `entries.has(0x9)` holds and `header.wide` holds (docs/adr/0065 §6 R4): the
    /// members present then.
    data class Has0x7Has0x9Wide(
        val entries: CodecChainHasTaggedEntriesHas7Has9Chain,
        val priority: UByte = 0.toUByte()
    ) : CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide()
    /// The `entries_has_0x7_has_0x9_header_wide` group of [CodecChainHasTagged] where `entries.has(0x7)` holds and `entries.has(0x9)` holds and `header.wide` does not hold (docs/adr/0065 §6 R4): the
    /// members present then.
    data class Has0x7Has0x9(
        val entries: CodecChainHasTaggedEntriesHas7Has9Chain,
        val priority: UByte = 0.toUByte()
    ) : CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide()
    /// The `entries_has_0x7_has_0x9_header_wide` group of [CodecChainHasTagged] where `entries.has(0x7)` holds and `entries.has(0x9)` does not hold and `header.wide` holds (docs/adr/0065 §6 R4): the
    /// members present then.
    data class Has0x7Wide(
        val entries: CodecChainHasTaggedEntriesHas7Lacks9Chain,
        val priority: UByte = 0.toUByte(),
        val checksum: UShort = 0.toUShort()
    ) : CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide()
    /// The `entries_has_0x7_has_0x9_header_wide` group of [CodecChainHasTagged] where `entries.has(0x7)` does not hold and `entries.has(0x9)` holds and `header.wide` holds (docs/adr/0065 §6 R4): the
    /// members present then.
    data class Has0x9Wide(
        val entries: CodecChainHasTaggedEntriesHas9Lacks7Chain,
        val priority: UByte = 0.toUByte()
    ) : CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide()
    /// The `entries_has_0x7_has_0x9_header_wide` group of [CodecChainHasTagged] where `entries.has(0x7)` holds and `entries.has(0x9)` does not hold and `header.wide` does not hold (docs/adr/0065 §6 R4): the
    /// members present then.
    data class Has0x7(
        val entries: CodecChainHasTaggedEntriesHas7Lacks9Chain,
        val priority: UByte = 0.toUByte(),
        val checksum: UShort = 0.toUShort()
    ) : CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide()
    /// The `entries_has_0x7_has_0x9_header_wide` group of [CodecChainHasTagged] where `entries.has(0x7)` does not hold and `entries.has(0x9)` holds and `header.wide` does not hold (docs/adr/0065 §6 R4): the
    /// members present then.
    data class Has0x9(
        val entries: CodecChainHasTaggedEntriesHas9Lacks7Chain
    ) : CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide()
    /// The `entries_has_0x7_has_0x9_header_wide` group of [CodecChainHasTagged] where `entries.has(0x7)` does not hold and `entries.has(0x9)` does not hold and `header.wide` holds (docs/adr/0065 §6 R4): the
    /// members present then.
    data class Wide(
        val entries: CodecChainHasTaggedEntriesLacks7Lacks9Chain = CodecChainHasTaggedEntriesLacks7Lacks9Chain.fewest(),
        val priority: UByte = 0.toUByte(),
        val checksum: UShort = 0.toUShort()
    ) : CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide()
    /// The `entries_has_0x7_has_0x9_header_wide` group of [CodecChainHasTagged] where `entries.has(0x7)` does not hold and `entries.has(0x9)` does not hold and `header.wide` does not hold (docs/adr/0065 §6 R4): the
    /// members present then.
    data class Neither(
        val entries: CodecChainHasTaggedEntriesLacks7Lacks9Chain = CodecChainHasTaggedEntriesLacks7Lacks9Chain.fewest(),
        val checksum: UShort = 0.toUShort()
    ) : CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide()
}

/// The elements a value of [CodecChainHasTagged] presents, which its encode reads
/// (docs/adr/0081 §2): any object that presents them is a source of it, which
/// [CodecChainHasTagged.encodeSource] writes. The encode derives every other element
/// of the frame.
interface CodecChainHasTaggedSource {
    val header: UByte
    val entries_has_0x7_has_0x9_header_wide: CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide
}

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecChainHasTagged()` before any encode()/decode() call. Each default
// is a value of that field's own type, which decode() then fills in on
// success — the carrier's zero for a number, and for an enum the first
// variant its document declares, since a closed set does not hold a
// value it never declared.
data class CodecChainHasTagged(
    override var header: UByte = 0.toUByte(),
    override var entries_has_0x7_has_0x9_header_wide: CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide = CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide.Neither()
) : CodecChainHasTaggedSource {
    // RFC §synth-5-B flags primitive: per-bit-range accessors over
    // the carrier field. Single-bit (width=1) reads as Boolean; multi-
    // bit (width>=2) reads as the smallest unsigned Kotlin type that
    // fits (UByte / UShort / UInt / ULong). UByte/UShort widen through
    // `.toInt()` and UInt/ULong through `.toLong()` for the bitwise
    // ops; the result narrows back via the carrier's `toU*` ctor.
    fun wide(): Boolean = (this.header.toInt() and 0x01) != 0

    fun setWide(v: Boolean) {
        this.header = if (v) {
            (this.header.toInt() or 0x01).toUByte()
        } else {
            (this.header.toInt() and 0x01.inv()).toUByte()
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
        /// Write any source of [CodecChainHasTagged] into `w`: each element read once off
        /// `src` and written where the codec's encode plan lays it out
        /// (docs/adr/0081). It refuses no value: it answers `null`, or
        /// [CodecError.BufferOverflow] from a bounded sink with no room left.
        fun encodeSource(src: CodecChainHasTaggedSource, w: SceSink): CodecError? {
            val _e_header = src.header
            val _g_entries_has_0x7_has_0x9_header_wide = src.entries_has_0x7_has_0x9_header_wide
            val _e_entries = when (val _g = _g_entries_has_0x7_has_0x9_header_wide) { is CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide.Has0x7Has0x9Wide -> _g.entries.entries; is CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide.Has0x7Has0x9 -> _g.entries.entries; is CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide.Has0x7Wide -> _g.entries.entries; is CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide.Has0x9Wide -> _g.entries.entries; is CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide.Has0x7 -> _g.entries.entries; is CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide.Has0x9 -> _g.entries.entries; is CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide.Wide -> _g.entries.entries; is CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide.Neither -> _g.entries.entries }
            val _e_priority = when (val _g = _g_entries_has_0x7_has_0x9_header_wide) { is CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide.Has0x7Has0x9Wide -> _g.priority; is CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide.Has0x7Has0x9 -> _g.priority; is CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide.Has0x7Wide -> _g.priority; is CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide.Has0x9Wide -> _g.priority; is CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide.Has0x7 -> _g.priority; is CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide.Wide -> _g.priority; else -> null }
            val _e_checksum = when (val _g = _g_entries_has_0x7_has_0x9_header_wide) { is CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide.Has0x7Wide -> _g.checksum; is CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide.Has0x7 -> _g.checksum; is CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide.Wide -> _g.checksum; is CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide.Neither -> _g.checksum; else -> null }
            var _u_header = _e_header.toUInt()
            _u_header = (_u_header and 0x1u.inv()) or (if ((_g_entries_has_0x7_has_0x9_header_wide is CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide.Has0x7Has0x9Wide || _g_entries_has_0x7_has_0x9_header_wide is CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide.Has0x7Wide || _g_entries_has_0x7_has_0x9_header_wide is CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide.Has0x9Wide || _g_entries_has_0x7_has_0x9_header_wide is CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide.Wide)) 0x1u else 0x0u)
            val _c_header = _u_header.toUByte()
            w.writeU8(_c_header.toByte())?.let { return it }
            var _left_entries = _e_entries.size
            for (_entry in _e_entries) {
                _left_entries -= 1
                CodecChainHasTaggedEntry.encodeSource(_entry, w, CodecChainHasTaggedEntryOverrides(ctl_more = _left_entries != 0))?.let { return it }
            }
            if (_e_priority != null) {
                w.writeU8(_e_priority.toByte())?.let { return it }
            }
            if (_e_checksum != null) {
                w.writeU8((_e_checksum.toInt() ushr 8 and 0xFF).toByte())?.let { return it }
                w.writeU8((_e_checksum.toInt() and 0xFF).toByte())?.let { return it }
            }
            return null
        }

        /// Decode the next frame from `cursor`. On success the cursor
        /// advances past the consumed bytes and the value is `Decoded.Ok`;
        /// a refused frame is `Decoded.Refused` with the kind that refused
        /// it (docs/adr/0099, amendment of 2026-11-16): `NeedMoreBytes` when
        /// the cursor's tail is shorter than the frame (RFC
        /// §synth-5-B L494-519), which the caller resumes after appending
        /// bytes, and another kind for a frame no more bytes make whole.
        fun decode(cursor: SceCursor): Decoded<CodecChainHasTagged> {
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
            val entries: MutableList<CodecChainHasTaggedEntry> = mutableListOf<CodecChainHasTaggedEntry>().also {
                var _more = false
                for (_i in 0 until 3) {
                    val _entry = CodecChainHasTaggedEntry.decode(cursor).valueOr { _refused -> return _refused }
                    _more = _entry.more()
                    it.add(_entry)
                    if (!_more) break
                }
                if (_more && cursor.remaining() == 0) { return Decoded.Refused(CodecError.NeedMoreBytes) }
                if (_more) { return Decoded.Refused(CodecError.TooManyElements) }
            }
            val _has_entries_7: Boolean = entries.any { _e -> (_e.entry_type).toULong() == 7UL }
            val _has_entries_9: Boolean = entries.any { _e -> (_e.entry_type).toULong() == 9UL }
            val priority = if (_has_entries_7 || (header.toInt() and 0x01) != 0) {
                val raw = cursor.peekSlice(1) ?: return Decoded.Refused(CodecError.NeedMoreBytes)
                val _v = raw[0].toUByte()
                if (!cursor.advance(1)) return Decoded.Refused(CodecError.NeedMoreBytes)
                _v
            } else {
                null
            }
            val checksum = if (!_has_entries_9) {
                val raw = cursor.peekSlice(2) ?: return Decoded.Refused(CodecError.NeedMoreBytes)
                val _v = (((raw[0].toInt() and 0xFF) shl 8) or (raw[1].toInt() and 0xFF)).toUShort()
                if (!cursor.advance(2)) return Decoded.Refused(CodecError.NeedMoreBytes)
                _v
            } else {
                null
            }
            return Decoded.Ok(
                CodecChainHasTagged(
                    header = header,
                    entries_has_0x7_has_0x9_header_wide = if (_has_entries_7 && _has_entries_9 && ((header.toUInt() shr 0) and 0x1u) != 0x0u) CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide.Has0x7Has0x9Wide(entries = CodecChainHasTaggedEntriesHas7Has9Chain.of(entries) ?: return Decoded.Refused(CodecError.PresentIfMismatch), priority = priority!!) else if (_has_entries_7 && _has_entries_9 && ((header.toUInt() shr 0) and 0x1u) == 0x0u) CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide.Has0x7Has0x9(entries = CodecChainHasTaggedEntriesHas7Has9Chain.of(entries) ?: return Decoded.Refused(CodecError.PresentIfMismatch), priority = priority!!) else if (_has_entries_7 && !_has_entries_9 && ((header.toUInt() shr 0) and 0x1u) != 0x0u) CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide.Has0x7Wide(entries = CodecChainHasTaggedEntriesHas7Lacks9Chain.of(entries) ?: return Decoded.Refused(CodecError.PresentIfMismatch), priority = priority!!, checksum = checksum!!) else if (!_has_entries_7 && _has_entries_9 && ((header.toUInt() shr 0) and 0x1u) != 0x0u) CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide.Has0x9Wide(entries = CodecChainHasTaggedEntriesHas9Lacks7Chain.of(entries) ?: return Decoded.Refused(CodecError.PresentIfMismatch), priority = priority!!) else if (_has_entries_7 && !_has_entries_9 && ((header.toUInt() shr 0) and 0x1u) == 0x0u) CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide.Has0x7(entries = CodecChainHasTaggedEntriesHas7Lacks9Chain.of(entries) ?: return Decoded.Refused(CodecError.PresentIfMismatch), priority = priority!!, checksum = checksum!!) else if (!_has_entries_7 && _has_entries_9 && ((header.toUInt() shr 0) and 0x1u) == 0x0u) CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide.Has0x9(entries = CodecChainHasTaggedEntriesHas9Lacks7Chain.of(entries) ?: return Decoded.Refused(CodecError.PresentIfMismatch)) else if (!_has_entries_7 && !_has_entries_9 && ((header.toUInt() shr 0) and 0x1u) != 0x0u) CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide.Wide(entries = CodecChainHasTaggedEntriesLacks7Lacks9Chain.of(entries) ?: return Decoded.Refused(CodecError.PresentIfMismatch), priority = priority!!, checksum = checksum!!) else CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide.Neither(entries = CodecChainHasTaggedEntriesLacks7Lacks9Chain.of(entries) ?: return Decoded.Refused(CodecError.PresentIfMismatch), checksum = checksum!!)
                )
            )
        }
    }
}
