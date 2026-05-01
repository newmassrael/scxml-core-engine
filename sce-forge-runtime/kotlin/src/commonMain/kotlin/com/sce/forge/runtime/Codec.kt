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

/// Typed decode error. NeedMoreBytes is the only reachable variant
/// while every codec field is fixed-width. B1-α adds VleWidthOverflow,
/// B1-β adds UnknownVariantTag.
sealed class CodecError {
    object NeedMoreBytes : CodecError()
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
}
