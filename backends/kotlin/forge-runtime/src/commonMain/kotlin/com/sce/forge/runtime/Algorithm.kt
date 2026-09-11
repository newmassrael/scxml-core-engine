// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

package com.sce.forge.runtime

/**
 * Byte-buffer-build support for `sce:kind="algorithm"`. See SCE_FORGE.md
 * Section 4.12.
 */

/**
 * Growable output buffer for a `bytes`-returning algorithm, backed by a
 * primitive [ByteArray].
 *
 * This is the Kotlin half of the byte-buffer-build primitive set: a bounded
 * growable buffer seeded empty, filled forward-only through [add] / [addAll],
 * and converted once at the return through [toByteArray]. It exists because
 * the obvious stdlib spelling does not fit the contract. `MutableList<Byte>`
 * stores each byte as a boxed reference in an `Object[]`, so a buffer of N
 * bytes costs N pointers plus a per-read unboxing, and the final conversion
 * walks the list. Nothing in the SCXML asked for a collection — the author
 * declared a capacity and a byte count.
 *
 * `initialCapacity` is the author's `capacity` attribute, so a buffer whose
 * contents stay within the declared bound allocates exactly once. Exceeding it
 * is not an error on this backend: the buffer grows, which is the behaviour
 * SCE_FORGE.md Section 4.12's backend table documents for Kotlin (the bounded
 * backends, Rust and C11, are the ones that surface overflow).
 */
public class SceByteBuf(initialCapacity: Int = 0) {
    private var buf: ByteArray = ByteArray(if (initialCapacity > 0) initialCapacity else 0)
    private var len: Int = 0

    /** Bytes appended so far. */
    public val size: Int get() = len

    /** Append one byte. */
    public fun add(b: Byte) {
        reserve(len + 1)
        buf[len] = b
        len += 1
    }

    /** Append every byte of [bytes], in order. */
    public fun addAll(bytes: ByteArray) {
        if (bytes.isEmpty()) return
        reserve(len + bytes.size)
        bytes.copyInto(buf, len)
        len += bytes.size
    }

    /** The appended bytes, as an independent array. */
    public fun toByteArray(): ByteArray = buf.copyOf(len)

    /**
     * Grow the backing array to hold at least [needed] bytes. Doubling keeps
     * append amortized O(1); the `needed` seed covers a buffer declared with
     * capacity 0, where doubling alone would never leave zero.
     */
    private fun reserve(needed: Int) {
        if (needed <= buf.size) return
        var next = if (buf.size == 0) needed else buf.size
        while (next < needed) next *= 2
        buf = buf.copyOf(next)
    }
}
