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

/**
 * Growable output buffer for a `list<T>`-returning algorithm, one class for
 * every fixed-width element type.
 *
 * Every admitted element fits one 64-bit slot without loss. A signed or
 * unsigned integer is stored as its bits (`ULong.toLong()` reinterprets, a
 * narrower value widens). A `float32` or `float64` is stored as the raw bits
 * of a `Double`, and `Float` to `Double` is exact. A `bool` is stored as
 * 0 or 1. The return converts once to the primitive array the signature
 * declares, which is the same one copy [SceByteBuf.toByteArray] makes.
 *
 * ⚠ Why one class rather than one per width. Kotlin has no generics over
 * primitives, so a per-width design is seven near-identical classes
 * (`LongArray`, `IntArray`, ... `BooleanArray`) kept in step by hand, which
 * is how the stdlib does it. One slot width costs memory: eight bytes per
 * element where an `int8` list would need one. It buys a single
 * implementation of growth and conversion. The buffers this kind builds are
 * bounded by an author-declared capacity, so the memory trade is bounded
 * too. `MutableList<Long>` is rejected for the reason [SceByteBuf] gives.
 *
 * `initialCapacity` is the author's `capacity` attribute. Exceeding it grows
 * the buffer, as SCE_FORGE.md Section 4.12's backend table documents for
 * Kotlin.
 */
public class SceListBuf(initialCapacity: Int = 0) {
    private var slots: LongArray = LongArray(if (initialCapacity > 0) initialCapacity else 0)
    private var len: Int = 0

    /** Elements appended so far. */
    public val size: Int get() = len

    public fun add(v: Long): Unit = push(v)
    public fun add(v: Int): Unit = push(v.toLong())
    public fun add(v: Short): Unit = push(v.toLong())
    public fun add(v: Byte): Unit = push(v.toLong())
    public fun add(v: ULong): Unit = push(v.toLong())
    public fun add(v: UInt): Unit = push(v.toLong())
    public fun add(v: UShort): Unit = push(v.toLong())
    public fun add(v: UByte): Unit = push(v.toLong())
    public fun add(v: Double): Unit = push(v.toRawBits())
    public fun add(v: Float): Unit = push(v.toDouble().toRawBits())
    public fun add(v: Boolean): Unit = push(if (v) 1L else 0L)

    public fun toLongArray(): LongArray = slots.copyOf(len)
    public fun toIntArray(): IntArray = IntArray(len) { slots[it].toInt() }
    public fun toShortArray(): ShortArray = ShortArray(len) { slots[it].toShort() }
    public fun toByteArray(): ByteArray = ByteArray(len) { slots[it].toByte() }
    public fun toDoubleArray(): DoubleArray = DoubleArray(len) { Double.fromBits(slots[it]) }
    public fun toFloatArray(): FloatArray = FloatArray(len) { Double.fromBits(slots[it]).toFloat() }
    public fun toBooleanArray(): BooleanArray = BooleanArray(len) { slots[it] != 0L }

    // Unsigned returns are views over the signed array of the same width: the
    // bits are already the unsigned value, so no second copy is made.
    @OptIn(ExperimentalUnsignedTypes::class)
    public fun toULongArray(): ULongArray = toLongArray().asULongArray()

    @OptIn(ExperimentalUnsignedTypes::class)
    public fun toUIntArray(): UIntArray = toIntArray().asUIntArray()

    @OptIn(ExperimentalUnsignedTypes::class)
    public fun toUShortArray(): UShortArray = toShortArray().asUShortArray()

    @OptIn(ExperimentalUnsignedTypes::class)
    public fun toUByteArray(): UByteArray = toByteArray().asUByteArray()

    private fun push(bits: Long) {
        if (len == slots.size) {
            val next = if (slots.isEmpty()) 1 else slots.size * 2
            slots = slots.copyOf(next)
        }
        slots[len] = bits
        len += 1
    }
}
