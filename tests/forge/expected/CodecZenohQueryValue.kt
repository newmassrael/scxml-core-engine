// SCE-MAP: codec_zenoh_query_value:24 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_zenoh_query_value

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.Decoded
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink
import com.sce.generated.codec_zenoh_value_entry.*
import kotlin.jvm.JvmInline

/// The `exts` element of [CodecZenohQueryValue], 1 or more entries, the protocol stating no most, held to that bound where a
/// value of it is made (docs/adr/0065 §6 R2, docs/adr/0081 §2): its constructor is
/// private and [of] makes a value only of entries within the bound, which it
/// copies, so no list it was made of can grow it past its bound after.
@JvmInline
value class CodecZenohQueryValueExtsList private constructor(val entries: List<CodecZenohValueEntrySource>) : com.sce.forge.runtime.Counted {
    /// How many entries the list holds, the count a frame writes of it.
    override val size: Int
        get() = entries.size

    companion object {
        /// The fewest entries a value holds.
        const val LEAST: Int = 1

        /// `entries` as a value of the bound, or `null` where they lie outside it.
        fun of(entries: List<CodecZenohValueEntrySource>): CodecZenohQueryValueExtsList? =
            if (entries.size >= LEAST) CodecZenohQueryValueExtsList(entries.toList()) else null

        /// The fewest entries the bound holds, each made by default: what a value made by default holds.
        fun fewest(): CodecZenohQueryValueExtsList = CodecZenohQueryValueExtsList(List(LEAST) { CodecZenohValueEntry() })
    }
}

/// The `exts` element of [CodecZenohQueryValue], its entries held to the law they say
/// their inputs by (docs/adr/0065 §6 R8, docs/adr/0081 §2): its constructor is
/// private and [of] makes a value only of a list whose entries follow the law
/// from the start the value gives it, so the encode, which writes the flags the
/// list hands its entries from the start, refuses nothing of it.
class CodecZenohQueryValueExtsLawful private constructor(
    /// The list, within its bound.
    val list: CodecZenohQueryValueExtsList,
) : com.sce.forge.runtime.Counted {
    /// The entries the list holds.
    val entries: List<CodecZenohValueEntrySource>
        get() = list.entries

    /// How many entries the list holds, the count a frame writes of it.
    override val size: Int
        get() = list.size

    override fun equals(other: Any?): Boolean =
        other is CodecZenohQueryValueExtsLawful && other.list == list

    override fun hashCode(): Int = list.hashCode()

    override fun toString(): String = "CodecZenohQueryValueExtsLawful(list=$list)"

    companion object {
        /// The kind of `entry`: the identifier the chain tells its entries apart by.
        private fun kind(entry: CodecZenohValueEntrySource): ULong = ((entry.header.toULong() shr 0) and 0xFuL)

        /// `list` as a value of the law from the start given, or `null` where an
        /// entry says a value other than the one its place asks of it.
        fun of(list: CodecZenohQueryValueExtsList): CodecZenohQueryValueExtsLawful? {
            var afterAfterShm = false
            for (entry in list.entries) {
                CodecZenohValueEntry.inputAfterShm(entry)?.let { if ((it.toUInt() != 0u) != afterAfterShm) return null }
                afterAfterShm = kind(entry) == 0x4uL
            }
            return CodecZenohQueryValueExtsLawful(list)
        }

        /// The fewest entries of the bound, each made by default, from the start
        /// the first of them says: what a value made by default holds.
        fun fewest(): CodecZenohQueryValueExtsLawful {
            val list = CodecZenohQueryValueExtsList.fewest()
            return checkNotNull(of(list)) { "the entries made by default follow the law" }
        }
    }
}

/// The elements a value of [CodecZenohQueryValue] presents, which its encode reads
/// (docs/adr/0081 §2): any object that presents them is a source of it, which
/// [CodecZenohQueryValue.encodeSource] writes. The encode derives every other element
/// of the frame.
interface CodecZenohQueryValueSource {
    val exts: CodecZenohQueryValueExtsLawful
}

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecZenohQueryValue()` before any encode()/decode() call. Each default
// is a value of that field's own type, which decode() then fills in on
// success — the carrier's zero for a number, and for an enum the first
// variant its document declares, since a closed set does not hold a
// value it never declared.
data class CodecZenohQueryValue(
    override var exts: CodecZenohQueryValueExtsLawful = CodecZenohQueryValueExtsLawful.fewest()
) : CodecZenohQueryValueSource {
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
        /// Write any source of [CodecZenohQueryValue] into `w`: each element read once off
        /// `src` and written where the codec's encode plan lays it out
        /// (docs/adr/0081). It refuses no value: it answers `null`, or
        /// [CodecError.BufferOverflow] from a bounded sink with no room left.
        fun encodeSource(src: CodecZenohQueryValueSource, w: SceSink): CodecError? {
            val _e_exts = src.exts
            var _left_exts = _e_exts.entries.size
            for (_entry in _e_exts.entries) {
                _left_exts -= 1
                CodecZenohValueEntry.encodeSource(_entry, w, CodecZenohValueEntryOverrides(header_Z = _left_exts != 0))?.let { return it }
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
        fun decode(cursor: SceCursor): Decoded<CodecZenohQueryValue> {
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
            val exts: MutableList<CodecZenohValueEntry> = mutableListOf<CodecZenohValueEntry>().also {
                var _prev_exts_after_shm: UByte = 0.toUByte()
                var _more = false
                while (true) {
                    val _entry = CodecZenohValueEntry.decode(cursor, _prev_exts_after_shm).valueOr { _refused -> return _refused }
                    _more = _entry.Z()
                    _prev_exts_after_shm = if ((_entry.extId()).toULong() == 4UL) 1.toUByte() else 0.toUByte()
                    it.add(_entry)
                    if (!_more) break
                }
                if (_more && cursor.remaining() == 0) { return Decoded.Refused(CodecError.NeedMoreBytes) }
            }
            return Decoded.Ok(
                CodecZenohQueryValue(
                    exts = CodecZenohQueryValueExtsLawful.of(CodecZenohQueryValueExtsList.of(exts) ?: return Decoded.Refused(CodecError.TooManyElements)) ?: return Decoded.Refused(CodecError.PresentIfMismatch)
                )
            )
        }
    }
}
