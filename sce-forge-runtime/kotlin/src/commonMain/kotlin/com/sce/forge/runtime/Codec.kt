// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

// sce_forge_runtime — codec cursor + typed error contract (Kotlin).
//
// Mirrors `sce-forge-runtime/rust/src/codec.rs`. RFC §5.B L494-519 pins
// the per-language cursor + null-on-truncation contract on decode so a
// truncated input never aborts.
//
// B1-prep ships peekSlice / advance / remaining. Streaming readers
// (readU8, readVle*, readTag) land in B1-α/β with their first consumer.

package com.sce.forge.runtime

/// Typed decode error. The B1-β variant primitive intentionally does
/// NOT need a typed UnknownVariantTag — RFC §5.B requires
/// `<sce:default>` when arms don't exhaust the tag domain (build-time
/// codec/variant-arm-unreachable otherwise), so the default arm
/// catches every unmatched tag at runtime.
///
/// The B3-α TLV chain primitive emits on Kotlin after the B5-ε
/// closures (it was originally MCU-only as a conservative scope choice;
/// Zenoh extension envelopes need server-class peers too). On reject-
/// policy overflow Kotlin collapses the failure to the truncation
/// sentinel `null` returned from `decode()` — same convention as
/// VleWidthOverflow (see also the matching Cpp runtime). The typed
/// TlvChainOverflow enum variant lives only on Rust / C11 / Go /
/// Python runtimes that construct it at the call site.
sealed class CodecError {
    object NeedMoreBytes : CodecError()
    /// A `vle_u<N>` field's continuation chain implies a value wider
    /// than the declared type. RFC §5.B `codec/vle-width-overflow`.
    object VleWidthOverflow : CodecError()
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
    /// (RFC §5.B Appendix B).
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
