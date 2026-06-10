// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

// sce_forge_runtime — codec cursor + typed error contract (Kotlin).
//
// Mirrors `sce-forge-runtime/rust/src/codec.rs`. RFC §synth-5-B L494-519 pins
// the per-language cursor + null-on-truncation contract on decode so a
// truncated input never aborts.
//
// The cursor ships peekSlice / advance / remaining plus the readVle*
// readers. Other streaming readers (e.g. a dedicated readU8 / readTag)
// are not provided until a consumer needs them.

package com.sce.forge.runtime

/// Typed decode error. The variant primitive intentionally does
/// NOT need a typed UnknownVariantTag — RFC §synth-5-B requires
/// `<sce:default>` when arms don't exhaust the tag domain (build-time
/// codec/variant-arm-unreachable otherwise), so the default arm
/// catches every unmatched tag at runtime.
///
/// The TLV chain primitive emits on Kotlin like every other backend
/// (Zenoh extension envelopes need server-class peers, not only the
/// MCU). On reject-
/// policy overflow Kotlin collapses the failure to the truncation
/// sentinel `null` returned from `decode()` — same convention as
/// VleWidthOverflow (see also the matching Cpp runtime). The typed
/// TlvChainOverflow enum variant lives only on Rust / C11 / Go /
/// Python runtimes that construct it at the call site.
sealed class CodecError {
    object NeedMoreBytes : CodecError()
    /// A `vle_u<N>` field's continuation chain implies a value wider
    /// than the declared type. RFC §synth-5-B `codec/vle-width-overflow`.
    object VleWidthOverflow : CodecError()
    /// RFC §synth-5-B encode-side counterpart to NeedMoreBytes: the
    /// destination sink reported insufficient remaining capacity for
    /// the next write. Only the bounded [ByteArraySink] can raise
    /// this; the growable [MutableListSink] is effectively infallible.
    object BufferOverflow : CodecError()
}

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

    /// Read a base-128 variable-length encoded unsigned value of up to
    /// `maxBits` payload width. LSB-first byte order; bit 7 is the
    /// continuation flag. Mirrors the Zenoh ZInt wire format
    /// (RFC §synth-5-B Appendix B).
    ///
    /// Returns the decoded value as `Long` (Kotlin lacks unsigned long
    /// in the common API surface — caller casts to ULong as needed).
    /// On `null` return, [lastVleOverflow] distinguishes truncation
    /// (false) from width overflow (true).
    fun readVleU16(): UShort? = readVleInner(16)?.toUShort()
    fun readVleU32(): UInt? = readVleInner(32)?.toUInt()
    fun readVleU64(): ULong? = readVleInner(64)?.toULong()

    var lastVleOverflow: Boolean = false
        private set

    private fun readVleInner(maxBits: Int): Long? {
        lastVleOverflow = false
        val maxBytes = (maxBits + 6) / 7
        var value: Long = 0
        var shift = 0
        for (i in 0 until maxBytes) {
            if (remaining() < 1) return null
            val b = buf[pos].toInt() and 0xFF
            pos += 1
            val payload = (b and 0x7F).toLong()
            if (shift + 7 > maxBits) {
                val allowed = maxBits - shift
                val maxPayload = (1L shl allowed) - 1L
                if (payload > maxPayload) {
                    lastVleOverflow = true
                    return null
                }
            }
            value = value or (payload shl shift)
            if ((b and 0x80) == 0) {
                return value
            }
            shift += 7
        }
        lastVleOverflow = true
        return null
    }
}

// ── Write-side sink ──────────────────────────────────────────────

/// Write-side cursor for codec emit. Interface that generated `encode`
/// bodies accept by reference.
///
/// Return contract: `null` on success; populated [CodecError] on
/// failure (mirrors the decode-side `T?` convention — decode returns
/// Optional<Value>, encode returns Optional<Error>). Generated code
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
