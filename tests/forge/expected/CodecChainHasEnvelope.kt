// SCE-MAP: codec_chain_has_envelope:23 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_chain_has_envelope

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.Decoded
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink
import com.sce.generated.codec_zenoh_ext_entry.*
import com.sce.generated.codec_chain_has_slice.*
import kotlin.jvm.JvmInline

/// The `slices` element of [CodecChainHasEnvelope], 0 to 4 entries, held to that bound where a
/// value of it is made (docs/adr/0065 §6 R2, docs/adr/0081 §2): its constructor is
/// private and [of] makes a value only of entries within the bound, which it
/// copies, so no list it was made of can grow it past its bound after.
@JvmInline
value class CodecChainHasEnvelopeSlicesList private constructor(val entries: List<CodecChainHasSliceSource>) : com.sce.forge.runtime.Counted {
    /// How many entries the list holds, the count a frame writes of it.
    override val size: Int
        get() = entries.size

    companion object {
        /// The most entries a value holds.
        const val MOST: Int = 4

        /// `entries` as a value of the bound, or `null` where they lie outside it.
        fun of(entries: List<CodecChainHasSliceSource>): CodecChainHasEnvelopeSlicesList? =
            if (entries.size <= MOST) CodecChainHasEnvelopeSlicesList(entries.toList()) else null

        /// No entry: what a value made by default holds.
        fun fewest(): CodecChainHasEnvelopeSlicesList = CodecChainHasEnvelopeSlicesList(emptyList())
    }
}

/// The `payload` element of [CodecChainHasEnvelope], 0 to 256 bytes, held to that bound where a
/// value of it is made (docs/adr/0077 §3, docs/adr/0081 §2): its constructor is
/// private and [of] makes a value only of bytes within the bound. The value it
/// holds cannot change after, so a value holds its bound for as long as it lives.
@JvmInline
value class CodecChainHasEnvelopePayloadBytes private constructor(val bytes: ByteArray) {
    companion object {
        /// The fewest bytes a value holds.
        const val LEAST: Int = 0

        /// The most bytes a value holds.
        const val MOST: Int = 256

        /// `bytes` as a value of the bound, or `null` where it lies outside it.
        fun of(bytes: ByteArray): CodecChainHasEnvelopePayloadBytes? =
            if (bytes.size <= MOST) CodecChainHasEnvelopePayloadBytes(bytes) else null

        /// The fewest bytes the bound holds, each zero: what a value made by
        /// default holds.
        fun fewest(): CodecChainHasEnvelopePayloadBytes = CodecChainHasEnvelopePayloadBytes(ByteArray(LEAST))
    }
}

/// The `extensions` chain of [CodecChainHasEnvelope] where it holds an entry of each kind of 0x2, 1 to 4 entries, held
/// to that where a value of it is made (docs/adr/0065 §6 R6b, docs/adr/0081 §2):
/// its constructor is private and [of] makes a value only of entries within the
/// bound that hold those kinds, which it copies, so no list it was made of can
/// change it after.
@JvmInline
value class CodecChainHasEnvelopeExtensionsHas2Chain private constructor(val entries: List<CodecZenohExtEntrySource>) : com.sce.forge.runtime.Counted {
    /// How many entries the chain holds.
    override val size: Int
        get() = entries.size

    companion object {
        /// The fewest entries a value holds.
        const val LEAST: Int = 1

        /// The most entries a value holds.
        const val MOST: Int = 4

        /// The kind of `entry`: the identifier the chain tells its entries apart by.
        private fun kind(entry: CodecZenohExtEntrySource): ULong = ((entry.header.toULong() shr 0) and 0xFuL)

        /// `entries` as a value of the chain, or `null` where they lie outside its
        /// bound, lack a kind it holds or hold one it lacks.
        fun of(entries: List<CodecZenohExtEntrySource>): CodecChainHasEnvelopeExtensionsHas2Chain? =
            if (entries.size in LEAST..MOST && entries.any { kind(it) == 0x2uL }) CodecChainHasEnvelopeExtensionsHas2Chain(entries.toList()) else null
    }
}

/// The `extensions` chain of [CodecChainHasEnvelope] where it holds no entry of a kind of 0x2, 1 to 4 entries, held
/// to that where a value of it is made (docs/adr/0065 §6 R6b, docs/adr/0081 §2):
/// its constructor is private and [of] makes a value only of entries within the
/// bound that hold those kinds, which it copies, so no list it was made of can
/// change it after.
@JvmInline
value class CodecChainHasEnvelopeExtensionsLacks2Chain private constructor(val entries: List<CodecZenohExtEntrySource>) : com.sce.forge.runtime.Counted {
    /// How many entries the chain holds.
    override val size: Int
        get() = entries.size

    companion object {
        /// The fewest entries a value holds.
        const val LEAST: Int = 1

        /// The most entries a value holds.
        const val MOST: Int = 4

        /// The kind of `entry`: the identifier the chain tells its entries apart by.
        private fun kind(entry: CodecZenohExtEntrySource): ULong = ((entry.header.toULong() shr 0) and 0xFuL)

        /// `entries` as a value of the chain, or `null` where they lie outside its
        /// bound, lack a kind it holds or hold one it lacks.
        fun of(entries: List<CodecZenohExtEntrySource>): CodecChainHasEnvelopeExtensionsLacks2Chain? =
            if (entries.size in LEAST..MOST && entries.none { kind(it) == 0x2uL }) CodecChainHasEnvelopeExtensionsLacks2Chain(entries.toList()) else null

        /// The fewest entries the bound holds, each made by default, which hold no kind the chain
        /// lacks: what a value made by default holds.
        fun fewest(): CodecChainHasEnvelopeExtensionsLacks2Chain =
            checkNotNull(of(List(LEAST) { CodecZenohExtEntry() })) { "a value-made entry is of no kind the chain lacks" }
    }
}

/// The `extensions_has_0x2` group of [CodecChainHasEnvelope]: the arm the value holds, each the
/// members present while the gates hold its values (docs/adr/0065 §6 R4).
sealed class CodecChainHasEnvelopeExtensionsHas0x2 {
    /// The `extensions_has_0x2` group of [CodecChainHasEnvelope] where `extensions.has(0x2)` holds (docs/adr/0065 §6 R4): the
    /// members present then.
    data class Has0x2(
        val extensions: CodecChainHasEnvelopeExtensionsHas2Chain,
        val slices: CodecChainHasEnvelopeSlicesList = CodecChainHasEnvelopeSlicesList.fewest()
    ) : CodecChainHasEnvelopeExtensionsHas0x2()
    /// The `extensions_has_0x2` group of [CodecChainHasEnvelope] where `extensions.has(0x2)` does not hold (docs/adr/0065 §6 R4): the
    /// members present then.
    data class NotHas0x2(
        val extensions: CodecChainHasEnvelopeExtensionsLacks2Chain? = null,
        val payload: CodecChainHasEnvelopePayloadBytes = CodecChainHasEnvelopePayloadBytes.fewest()
    ) : CodecChainHasEnvelopeExtensionsHas0x2()
}

/// The elements a value of [CodecChainHasEnvelope] presents, which its encode reads
/// (docs/adr/0081 §2): any object that presents them is a source of it, which
/// [CodecChainHasEnvelope.encodeSource] writes. The encode derives every other element
/// of the frame.
interface CodecChainHasEnvelopeSource {
    val header: UByte
    val extensions_has_0x2: CodecChainHasEnvelopeExtensionsHas0x2
}

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecChainHasEnvelope()` before any encode()/decode() call. Each default
// is a value of that field's own type, which decode() then fills in on
// success — the carrier's zero for a number, and for an enum the first
// variant its document declares, since a closed set does not hold a
// value it never declared.
data class CodecChainHasEnvelope(
    override var header: UByte = 0.toUByte(),
    override var extensions_has_0x2: CodecChainHasEnvelopeExtensionsHas0x2 = CodecChainHasEnvelopeExtensionsHas0x2.NotHas0x2()
) : CodecChainHasEnvelopeSource {
    // RFC §synth-5-B flags primitive: per-bit-range accessors over
    // the carrier field. Single-bit (width=1) reads as Boolean; multi-
    // bit (width>=2) reads as the smallest unsigned Kotlin type that
    // fits (UByte / UShort / UInt / ULong). UByte/UShort widen through
    // `.toInt()` and UInt/ULong through `.toLong()` for the bitwise
    // ops; the result narrows back via the carrier's `toU*` ctor.
    fun kind(): UByte {
        val _carrier = this.header.toInt()
        return ((_carrier shr 0) and 0x1F).toUByte()
    }

    fun setKind(v: UByte) {
        val _carrier = this.header.toInt()
        val _shifted_mask = 0x1F shl 0
        val _val = (v.toInt() and 0x1F) shl 0
        this.header = ((_carrier and _shifted_mask.inv()) or _val).toUByte()
    }

    fun E(): Boolean = (this.header.toInt() and 0x80) != 0

    fun setE(v: Boolean) {
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
        /// Write any source of [CodecChainHasEnvelope] into `w`: each element read once off
        /// `src` and written where the codec's encode plan lays it out
        /// (docs/adr/0081). It derives `payload_len`, `slice_count` from what each measures, and never reads it off the
        /// source. It refuses no value: it answers `null`, or
        /// [CodecError.BufferOverflow] from a bounded sink with no room left.
        fun encodeSource(src: CodecChainHasEnvelopeSource, w: SceSink): CodecError? {
            val _e_header = src.header
            val _g_extensions_has_0x2 = src.extensions_has_0x2
            val _e_extensions = when (val _g = _g_extensions_has_0x2) { is CodecChainHasEnvelopeExtensionsHas0x2.Has0x2 -> _g.extensions.entries; is CodecChainHasEnvelopeExtensionsHas0x2.NotHas0x2 -> _g.extensions?.entries }
            val _e_slices = when (val _g = _g_extensions_has_0x2) { is CodecChainHasEnvelopeExtensionsHas0x2.Has0x2 -> _g.slices; else -> null }
            val _e_payload = when (val _g = _g_extensions_has_0x2) { is CodecChainHasEnvelopeExtensionsHas0x2.NotHas0x2 -> _g.payload; else -> null }
            val _d_payload_len = if (_e_payload != null) _e_payload.bytes.size.toULong() else 0.toULong()
            val _d_slice_count = if (_e_slices != null) _e_slices.entries.size.toUInt() else 0.toUInt()
            var _u_header = _e_header.toUInt()
            _u_header = (_u_header and 0x80u.inv()) or (if (_e_extensions != null) 0x80u else 0x0u)
            val _c_header = _u_header.toUByte()
            w.writeU8(_c_header.toByte())?.let { return it }
            if (_e_extensions != null) {
                var _left_extensions = _e_extensions.size
                for (_entry in _e_extensions) {
                    _left_extensions -= 1
                    CodecZenohExtEntry.encodeSource(_entry, w, CodecZenohExtEntryOverrides(header_Z = _left_extensions != 0))?.let { return it }
                }
            }
            if (!(_g_extensions_has_0x2 is CodecChainHasEnvelopeExtensionsHas0x2.Has0x2)) {
                w.writeVleU64(_d_payload_len)?.let { return it }
            }
            if (_e_payload != null) {
                w.writeBytes(_e_payload.bytes)?.let { return it }
            }
            if (_g_extensions_has_0x2 is CodecChainHasEnvelopeExtensionsHas0x2.Has0x2) {
                w.writeVleU32(_d_slice_count)?.let { return it }
            }
            if (_e_slices != null) {
                for (_entry in _e_slices.entries) {
                    CodecChainHasSlice.encodeSource(_entry, w)?.let { return it }
                }
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
        fun decode(cursor: SceCursor): Decoded<CodecChainHasEnvelope> {
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
            val extensions: MutableList<CodecZenohExtEntry>? = if ((header.toInt() and 0x80) != 0) {
            mutableListOf<CodecZenohExtEntry>().also {
                var _more = false
                for (_i in 0 until 4) {
                    val _entry = CodecZenohExtEntry.decode(cursor).valueOr { _refused -> return _refused }
                    _more = _entry.Z()
                    it.add(_entry)
                    if (!_more) break
                }
                if (_more && cursor.remaining() == 0) { return Decoded.Refused(CodecError.NeedMoreBytes) }
                if (_more) { return Decoded.Refused(CodecError.TooManyElements) }
            }
        } else {
            null
        }
            val _has_extensions_2: Boolean = extensions?.any { _e -> (_e.extId()).toULong() == 2UL } ?: false
            val payload_len: ULong? = if (!_has_extensions_2) {
                val _v = cursor.readVleU64().valueOr { _refused -> return _refused }
                _v
            } else {
                null
            }
            val payload = if (!_has_extensions_2) {
                val _len = payload_len!!
                if (_len > 256uL) {
                    return Decoded.Refused(CodecError.LengthOutOfRange)
                }
                val _n = _len.toInt()
                val raw = cursor.peekSlice(_n) ?: return Decoded.Refused(CodecError.NeedMoreBytes)
                val _v = raw.copyOf()
                if (!cursor.advance(_n)) return Decoded.Refused(CodecError.NeedMoreBytes)
                _v
            } else {
                null
            }
            val slice_count: UInt? = if (_has_extensions_2) {
                val _v = cursor.readVleU32().valueOr { _refused -> return _refused }
                _v
            } else {
                null
            }
            val slices: MutableList<CodecChainHasSlice>? = if (_has_extensions_2) {
                val _n = slice_count!!
                if (_n > 4u) {
                    return Decoded.Refused(CodecError.TooManyElements)
                }
                mutableListOf<CodecChainHasSlice>().apply {
                    repeat(_n.toInt()) {
                        add(CodecChainHasSlice.decode(cursor).valueOr { _refused -> return _refused })
                    }
                }
            } else null
            return Decoded.Ok(
                CodecChainHasEnvelope(
                    header = header,
                    extensions_has_0x2 = if (_has_extensions_2) CodecChainHasEnvelopeExtensionsHas0x2.Has0x2(extensions = CodecChainHasEnvelopeExtensionsHas2Chain.of(extensions) ?: return Decoded.Refused(CodecError.PresentIfMismatch), slices = CodecChainHasEnvelopeSlicesList.of(slices!!) ?: return Decoded.Refused(CodecError.TooManyElements)) else CodecChainHasEnvelopeExtensionsHas0x2.NotHas0x2(extensions = extensions?.let { CodecChainHasEnvelopeExtensionsLacks2Chain.of(it) ?: return Decoded.Refused(CodecError.PresentIfMismatch) }, payload = CodecChainHasEnvelopePayloadBytes.of(payload!!) ?: return Decoded.Refused(CodecError.LengthOutOfRange))
                )
            )
        }
    }
}
