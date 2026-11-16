// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

// sce_forge_runtime — codec cursor + typed error contract (Kotlin).
//
// Mirrors `backends/rust/forge-runtime/src/codec.rs`. RFC §synth-5-B L494-519 pins
// the per-language cursor + typed-refusal contract on decode so a truncated
// input never aborts: a decode answers a [Decoded], the value it read or the
// [CodecError] that refused the frame (docs/adr/0099, amendment of
// 2026-11-16).
//
// The cursor ships peekSlice / advance / remaining plus the readVle*
// readers. Other streaming readers (e.g. a dedicated readU8 / readTag)
// are not provided until a consumer needs them.

package com.sce.forge.runtime

/// Typed codec error: the kind of rejection a decode answers, or of
/// refusal an encode answers. The kinds a decode answers are one
/// vocabulary across every runtime (`sce-build/src/forge/codec_failure.rs`).
///
/// The variant primitive intentionally does NOT need a typed
/// UnknownVariantTag — RFC §synth-5-B requires `<sce:default>` when arms
/// don't exhaust the tag domain (build-time codec/variant-arm-unreachable
/// otherwise), so the default arm catches every unmatched tag at runtime.
///
/// The TLV chain primitive emits on Kotlin like every other backend
/// (Zenoh extension envelopes need server-class peers, not only the
/// MCU). A chain past its depth is refused as `TooManyElements`, as a
/// list past its count is.
sealed class CodecError {
    data object NeedMoreBytes : CodecError()
    /// A `vle_u<N>` field's continuation chain implies a value wider
    /// than the declared type. RFC §synth-5-B `codec/vle-width-overflow`.
    data object VleWidthOverflow : CodecError()
    /// The bytes of a `sce:type="string"` field are not well-formed UTF-8
    /// (RFC 3629): the frame is refused rather than read as a malformed text.
    data object InvalidUtf8 : CodecError()
    /// A field of a closed enum (`sce:strict-variants`, the default) carried a
    /// value no variant declares: not a value of the type, so the frame does
    /// not decode. An open set carries every value of its carrier.
    data object UndeclaredEnumValue : CodecError()
    /// A value inside its field's bound and past the capacity a deployment gave
    /// the container that stores it (docs/adr/0077 §7). A Kotlin list or array
    /// is sized to its value, so no decode of this runtime refuses with it: it
    /// is declared so that every runtime holds one vocabulary of the kinds a
    /// decode is refused with (docs/adr/0099, amendment of 2026-11-16).
    data object CapacityExceeded : CodecError()
    /// A frame whose variant tag selects an arm the document declares under a
    /// build feature the build left out (docs/adr/0072). A Kotlin codec keeps
    /// every arm, so it is declared for the one vocabulary, as
    /// [CapacityExceeded] is.
    data object VariantArmNotBuilt : CodecError()
    /// RFC §synth-5-B encode-side counterpart to NeedMoreBytes: the
    /// destination sink reported insufficient remaining capacity for
    /// the next write. Only the bounded [ByteArraySink] can raise
    /// this; the growable [MutableListSink] is effectively infallible.
    data object BufferOverflow : CodecError()
    /// A `sce:encoding="cbor"` codec (SCE_FORGE.md §4.6.1) read bytes that
    /// are not the CBOR it expects.
    data object CborMalformed : CodecError()
    /// A CBOR codec's map lacked an entry of a bare type, one that is not `optional<T>`.
    data object CborRequiredKeyMissing : CodecError()
    /// A CBOR codec's `bytes` entry declared `sce:length` held another number
    /// of bytes, on decode or on encode.
    data object CborWrongLength : CodecError()
    /// A CBOR codec skipped an unknown entry nested deeper than 16.
    data object CborTooDeep : CodecError()
    /// A CBOR codec's value exceeds its entry's width or `sce:max-size`, on
    /// decode or on encode.
    data object CborOutOfRange : CodecError()
    /// Encode-side: a field gated by a `sce:present-if` the codec cannot
    /// derive (a chain-membership test, a flag input the caller gives, a
    /// disjunction, a flag wider than one bit) was given when the predicate
    /// says it does not belong on the wire, or left out when it does.
    /// `encode()` refuses before writing the first byte, so the sink is
    /// untouched.
    data object PresentIfMismatch : CodecError()
    /// Encode-side: two things a value says of one number disagree: two byte
    /// strings that share one length field and differ in length, or a length
    /// the value states beside the content it measures that is not that
    /// content's length (docs/adr/0065 §2). No frame says both, so the encode
    /// refuses before writing the first byte and the sink is untouched.
    /// Mirrors the Rust runtime's `CodecError::InconsistentSource`.
    data object InconsistentSource : CodecError()
    /// A `sce:encoding="content-line"` codec (SCE_FORGE.md §4.6.4) read a line,
    /// a parameter or an `END:` the grammar does not admit.
    data object LineMalformed : CodecError()
    /// A content-line component lacked a property or parameter of a bare type,
    /// one that is not `optional<T>`; on encode, a parameter was given without the
    /// property it belongs to.
    data object LineRequiredMissing : CodecError()
    /// A content-line property that holds one value occurred twice, a parameter
    /// was given twice in one line, or a list passed its `sce:max-count`.
    data object LineTooMany : CodecError()
    /// A content-line value passed its `sce:max-size`.
    data object LineTooLong : CodecError()
    /// A content-line TEXT carried an escape other than `\\`, `\;`, `\,`, `\n`
    /// and `\N`.
    data object LineBadEscape : CodecError()
    /// A content-line value its entry cannot hold: a control character, invalid
    /// UTF-8, an integer or `bool` out of its type, a parameter of more values
    /// than one, a `"` in a parameter value.
    data object LineBadValue : CodecError()
    /// On decode, a byte string's or a text's length outside its bound: the
    /// declared `sce:max-size` intersected with what its length field can say
    /// after its `sce:length-arith` (docs/adr/0077 §1). Judged before the
    /// frame's own length, so such a frame is refused and never waited on.
    data object LengthOutOfRange : CodecError()
    /// On decode, a list's count past its declared `max-count`, or an entry
    /// past it in a list read to the end of the frame, refused before that
    /// entry is read (docs/adr/0077 §2).
    data object TooManyElements : CodecError()
}

/// What a decode answers (docs/adr/0099, amendment of 2026-11-16): the value
/// it read, or the kind of rejection that refused the frame. A rejection is
/// not a value of the decoded type and no sentinel stands for it, so a caller
/// tells a frame cut short ([CodecError.NeedMoreBytes], resumed by appending
/// bytes) from one no more bytes make whole by the kind it holds. The sealed
/// result is Kotlin's own spelling of a failure the caller is expected to
/// handle, as Rust's `Result<T, CodecError>` is, and [valueOr] is its `?`.
sealed class Decoded<out T> {
    /// The value a decode read.
    data class Ok<out T>(val value: T) : Decoded<T>()

    /// The kind of rejection that refused the frame.
    data class Refused(val error: CodecError) : Decoded<Nothing>()

    /// The value, or what [onRefused] does with the refusal, which can only
    /// leave: a generated decode writes `valueOr { r -> return r }`, so a
    /// nested refusal is answered as its own, and a caller that cannot go on
    /// throws or returns.
    inline fun valueOr(onRefused: (Refused) -> Nothing): T =
        when (this) {
            is Ok -> this.value
            is Refused -> onRefused(this)
        }

    /// The value, or `null` for a refusal, for a caller that has no use for
    /// the kind.
    fun valueOrNull(): T? = (this as? Ok)?.value

    /// The kind of rejection, or `null` for a value.
    fun errorOrNull(): CodecError? = (this as? Refused)?.error
}

/// The bytes of a ZInt that carry a continuation flag: a chain of this many
/// flagged bytes ends with a ninth that holds a full 8 data bits, so no ZInt
/// is longer than nine bytes, whatever the width of the field that reads it
/// (Appendix B of the protocol-synthesis RFC).
private const val ZINT_FLAGGED_BYTES = 8

/// Read-only cursor over a borrowed input buffer. Decode bodies use
/// `peekSlice` to bounds-check + read fixed-offset bytes positionally,
/// then `advance` after the construction succeeds.
class SceCursor(private val buf: ByteArray, private var pos: Int = 0) {
    fun remaining(): Int = buf.size - pos

    /// Returns a copy of the next `n` bytes without advancing.
    /// Returns `null` when the cursor's tail is shorter than `n`.
    fun peekSlice(n: Int): ByteArray? {
        if (remaining() < n) return null
        return buf.copyOfRange(pos, pos + n)
    }

    /// Advance the cursor by `n` bytes. Returns `false` if `n` would
    /// overrun the buffer.
    fun advance(n: Int): Boolean {
        if (remaining() < n) return false
        pos += n
        return true
    }

    /// Where the cursor stands, to [reset] to — a decode that reads item by
    /// item (a CBOR map) leaves the cursor untouched when it refuses.
    fun mark(): Int = pos

    /// Return to a position [mark] answered.
    fun reset(mark: Int) {
        pos = mark
    }

    /// The next byte, advancing past it; `null` at the end of the input.
    fun readByte(): Int? {
        if (remaining() < 1) return null
        return buf[pos++].toInt() and 0xFF
    }

    /// Read a `vle_u<N>` field: one ZInt, the canonical Zenoh
    /// variable-length integer (RFC §synth-5-B Appendix B), read as
    /// `zenoh` and `zenoh-pico` read every one of them, an unsigned 64-bit
    /// value of 1-9 wire bytes LSB-first, and then the value held to the
    /// field's width. The wire of every width is the same ZInt64 and the
    /// type bounds the value, not the encoding: a narrow field accepts every
    /// encoding of a value it holds, the padded ones included, and refuses a
    /// value past its width, never a longer encoding.
    ///
    /// A buffer that ends inside the chain is refused as
    /// [CodecError.NeedMoreBytes], a value past the width as
    /// [CodecError.VleWidthOverflow].
    fun readVleU16(): Decoded<UShort> =
        when (val read = readVleInner(16)) {
            is Decoded.Ok -> Decoded.Ok(read.value.toUShort())
            is Decoded.Refused -> read
        }

    fun readVleU32(): Decoded<UInt> =
        when (val read = readVleInner(32)) {
            is Decoded.Ok -> Decoded.Ok(read.value.toUInt())
            is Decoded.Refused -> read
        }

    fun readVleU64(): Decoded<ULong> = readVleInner(64)

    /// One ZInt: each of the first eight bytes carries 7 data bits in its
    /// low 7 and a continuation flag in bit 7; a ninth carries a full 8 data
    /// bits and has no flag, so bit 63 rides in its high bit and no chain is
    /// longer. `null` when the buffer ends inside the chain.
    private fun readZint(): ULong? {
        var value: ULong = 0uL
        for (group in 0 until ZINT_FLAGGED_BYTES) {
            if (remaining() < 1) return null
            val b = buf[pos].toInt() and 0xFF
            pos += 1
            value = value or ((b and 0x7F).toULong() shl (7 * group))
            if ((b and 0x80) == 0) {
                return value
            }
        }
        if (remaining() < 1) return null
        val last = buf[pos].toInt() and 0xFF
        pos += 1
        return value or (last.toULong() shl (7 * ZINT_FLAGGED_BYTES))
    }

    /// One ZInt whatever the width, then the value held to `maxBits`: past
    /// `2^maxBits - 1` it is refused as [CodecError.VleWidthOverflow].
    private fun readVleInner(maxBits: Int): Decoded<ULong> {
        val value = readZint() ?: return Decoded.Refused(CodecError.NeedMoreBytes)
        if (maxBits < 64 && (value shr maxBits) != 0uL) {
            return Decoded.Refused(CodecError.VleWidthOverflow)
        }
        return Decoded.Ok(value)
    }
}

// ── Write-side sink ──────────────────────────────────────────────

/// Write-side cursor for codec emit. Interface that generated `encode`
/// bodies accept by reference.
///
/// Return contract: `null` on success; populated [CodecError] on
/// failure — an encode has no value to answer, so its answer is the
/// refusal or nothing, where a decode answers a [Decoded]. Generated code
/// propagates via `?.let { return it }`.
///
/// `SceSink` is an interface rather than abstract class so Kotlin
/// callers can implement it on platform-native buffer wrappers
/// (Android ByteBuffer, JVM OutputStream, …) without inheritance.
interface SceSink {
    /// Append `len` bytes starting at `off` from `bytes` to the
    /// underlying storage. Concrete sinks return
    /// [CodecError.BufferOverflow] when the destination has
    /// insufficient remaining capacity; growable sinks return `null`.
    fun writeBytes(bytes: ByteArray, off: Int = 0, len: Int = bytes.size): CodecError?

    /// Bytes written by this sink instance since construction. Used by
    /// codec emit for offset-aware writes (DMA-aligned field padding,
    /// length-prefix back-patching).
    fun position(): Int

    // Default per-width helpers — concrete sinks MAY override for
    // efficiency. All forward to `writeBytes`.
    fun writeU8(v: Byte): CodecError? = writeBytes(byteArrayOf(v), 0, 1)

    fun writeU16Le(v: UShort): CodecError? = writeBytes(byteArrayOf(
        v.toByte(), (v.toInt() ushr 8).toByte()
    ), 0, 2)

    fun writeU16Be(v: UShort): CodecError? = writeBytes(byteArrayOf(
        (v.toInt() ushr 8).toByte(), v.toByte()
    ), 0, 2)

    fun writeU32Le(v: UInt): CodecError? = writeBytes(byteArrayOf(
        v.toByte(),
        (v.toInt() ushr 8).toByte(),
        (v.toInt() ushr 16).toByte(),
        (v.toInt() ushr 24).toByte(),
    ), 0, 4)

    fun writeU32Be(v: UInt): CodecError? = writeBytes(byteArrayOf(
        (v.toInt() ushr 24).toByte(),
        (v.toInt() ushr 16).toByte(),
        (v.toInt() ushr 8).toByte(),
        v.toByte(),
    ), 0, 4)

    fun writeU64Le(v: ULong): CodecError? = writeBytes(byteArrayOf(
        v.toByte(),
        (v.toLong() ushr 8).toByte(),
        (v.toLong() ushr 16).toByte(),
        (v.toLong() ushr 24).toByte(),
        (v.toLong() ushr 32).toByte(),
        (v.toLong() ushr 40).toByte(),
        (v.toLong() ushr 48).toByte(),
        (v.toLong() ushr 56).toByte(),
    ), 0, 8)

    fun writeU64Be(v: ULong): CodecError? = writeBytes(byteArrayOf(
        (v.toLong() ushr 56).toByte(),
        (v.toLong() ushr 48).toByte(),
        (v.toLong() ushr 40).toByte(),
        (v.toLong() ushr 32).toByte(),
        (v.toLong() ushr 24).toByte(),
        (v.toLong() ushr 16).toByte(),
        (v.toLong() ushr 8).toByte(),
        v.toByte(),
    ), 0, 8)

    /// Append `value` as a base-128 VLE of `maxBits` payload width. The
    /// write-side counterpart of `SceCursor.readVleInner`: leading bytes
    /// carry 7 data bits + a continuation flag; the final byte (after at
    /// most VLE_LEN-1 continuation bytes) carries a full 8 data bits with
    /// no flag, so a u64 caps at 9 bytes — canonical Zenoh ZInt (RFC
    /// §synth-5-B Appendix B). VLE_LEN = ceil((maxBits-1)/7).
    fun writeVleInner(value: ULong, maxBits: Int): CodecError? {
        val contMax = (maxBits - 1 + 6) / 7 - 1
        var v = value
        var n = 0
        while (v >= 0x80UL && n < contMax) {
            writeU8((v.toLong() and 0x7F or 0x80).toByte())?.let { return it }
            v = v shr 7
            n++
        }
        return writeU8(v.toByte())
    }

    fun writeVleU16(v: UShort): CodecError? = writeVleInner(v.toULong(), 16)
    fun writeVleU32(v: UInt): CodecError? = writeVleInner(v.toULong(), 32)
    fun writeVleU64(v: ULong): CodecError? = writeVleInner(v, 64)
}

/// Growable sink backed by a caller-owned `MutableList<Byte>`. Infallible
/// (never returns [CodecError.BufferOverflow]). Natural sink for std
/// JVM consumers and the engine behind `encodeToByteArray()`.
class MutableListSink(private val dst: MutableList<Byte>) : SceSink {
    private val startLen: Int = dst.size

    override fun writeBytes(bytes: ByteArray, off: Int, len: Int): CodecError? {
        if (len == 0) return null
        val end = off + len
        for (i in off until end) {
            dst.add(bytes[i])
        }
        return null
    }

    override fun writeU8(v: Byte): CodecError? {
        dst.add(v)
        return null
    }

    override fun position(): Int = dst.size - startLen
}

/// Bounded sink backed by a caller-owned `ByteArray` + position
/// cursor. Raises [CodecError.BufferOverflow] when a write would
/// exceed the array's capacity. Natural sink for fixed-frame /
/// DMA-aligned call sites.
class ByteArraySink(private val buf: ByteArray, private var pos: Int = 0) : SceSink {
    private val startPos: Int = pos

    override fun writeBytes(bytes: ByteArray, off: Int, len: Int): CodecError? {
        if (len == 0) return null
        if (buf.size - pos < len) return CodecError.BufferOverflow
        for (i in 0 until len) {
            buf[pos + i] = bytes[off + i]
        }
        pos += len
        return null
    }

    override fun writeU8(v: Byte): CodecError? {
        if (pos >= buf.size) return CodecError.BufferOverflow
        buf[pos] = v
        pos += 1
        return null
    }

    override fun position(): Int = pos - startPos

    /// Remaining capacity from current position to end of buffer.
    fun remaining(): Int = buf.size - pos

    /// Return the byte prefix containing the bytes written by this
    /// sink (excluding any prefix that was present at construction).
    fun written(): ByteArray = buf.copyOfRange(startPos, pos)
}
