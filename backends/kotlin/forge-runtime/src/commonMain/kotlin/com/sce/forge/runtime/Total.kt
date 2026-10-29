// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package com.sce.forge.runtime

/**
 * The two total rules of an integer operation (docs/adr/0063). An author who
 * names one asks for the exact result `e` of `add`, `sub`, `mul` or `neg`
 * brought into the operands' type `T`, of width `W` and range `[m, M]`:
 *
 * - [SceWrapping]: the `r` in `[m, M]` with `r = e (mod 2^W)`, the
 *   two's-complement representative for a signed `T`;
 * - [SceSaturating]: `max(m, min(M, e))`.
 *
 * Neither has a failure, so neither throws [AlgorithmFailure], and the generator
 * calls them in any body, `may-fail` or not, beside the [SceChecked] helpers
 * whose overloads they mirror.
 */

/**
 * Wrapping integer operations, one overload per SCE integer width.
 *
 * `Int`, `Long`, `UInt` and `ULong` arithmetic wraps at its own width already
 * (the Java Language Specification, sections 15.17.1 and 15.18.2, which this
 * language's integers follow). `Byte`, `Short`, `UByte` and `UShort` arithmetic
 * is made in `Int` or `UInt`, and the result is truncated back: a conversion to
 * a narrower integer type keeps the low bits, which is the reduction modulo
 * `2^W` onto the type's two's-complement range.
 */
public object SceWrapping {
    public fun add(a: Byte, b: Byte): Byte = (a + b).toByte()
    public fun sub(a: Byte, b: Byte): Byte = (a - b).toByte()
    public fun mul(a: Byte, b: Byte): Byte = (a * b).toByte()
    public fun neg(a: Byte): Byte = (-a).toByte()

    public fun add(a: Short, b: Short): Short = (a + b).toShort()
    public fun sub(a: Short, b: Short): Short = (a - b).toShort()
    public fun mul(a: Short, b: Short): Short = (a * b).toShort()
    public fun neg(a: Short): Short = (-a).toShort()

    public fun add(a: Int, b: Int): Int = a + b
    public fun sub(a: Int, b: Int): Int = a - b
    public fun mul(a: Int, b: Int): Int = a * b
    public fun neg(a: Int): Int = -a

    public fun add(a: Long, b: Long): Long = a + b
    public fun sub(a: Long, b: Long): Long = a - b
    public fun mul(a: Long, b: Long): Long = a * b
    public fun neg(a: Long): Long = -a

    // The unsigned types have no unary minus: a negation is `0 - a`, which
    // wraps as every other difference does.
    public fun add(a: UByte, b: UByte): UByte = (a + b).toUByte()
    public fun sub(a: UByte, b: UByte): UByte = (a - b).toUByte()
    public fun mul(a: UByte, b: UByte): UByte = (a * b).toUByte()
    public fun neg(a: UByte): UByte = (0u - a).toUByte()

    public fun add(a: UShort, b: UShort): UShort = (a + b).toUShort()
    public fun sub(a: UShort, b: UShort): UShort = (a - b).toUShort()
    public fun mul(a: UShort, b: UShort): UShort = (a * b).toUShort()
    public fun neg(a: UShort): UShort = (0u - a).toUShort()

    public fun add(a: UInt, b: UInt): UInt = a + b
    public fun sub(a: UInt, b: UInt): UInt = a - b
    public fun mul(a: UInt, b: UInt): UInt = a * b
    public fun neg(a: UInt): UInt = 0u - a

    public fun add(a: ULong, b: ULong): ULong = a + b
    public fun sub(a: ULong, b: ULong): ULong = a - b
    public fun mul(a: ULong, b: ULong): ULong = a * b
    public fun neg(a: ULong): ULong = 0uL - a
}

/**
 * Saturating integer operations, one overload per SCE integer width; `neg` for
 * the signed widths only, as a saturating negation of an unsigned value is 0
 * whatever the value, and the generator refuses it where it is written
 * (docs/adr/0063, section 2).
 */
public object SceSaturating {
    // A signed width up to 32 bits computes its exact result in `Long`, where
    // none of these operations can overflow, and holds it to its own range.
    private fun byteOf(e: Long): Byte = e.coerceIn(Byte.MIN_VALUE.toLong(), Byte.MAX_VALUE.toLong()).toByte()
    private fun shortOf(e: Long): Short = e.coerceIn(Short.MIN_VALUE.toLong(), Short.MAX_VALUE.toLong()).toShort()
    private fun intOf(e: Long): Int = e.coerceIn(Int.MIN_VALUE.toLong(), Int.MAX_VALUE.toLong()).toInt()

    public fun add(a: Byte, b: Byte): Byte = byteOf(a.toLong() + b.toLong())
    public fun sub(a: Byte, b: Byte): Byte = byteOf(a.toLong() - b.toLong())
    public fun mul(a: Byte, b: Byte): Byte = byteOf(a.toLong() * b.toLong())
    public fun neg(a: Byte): Byte = byteOf(-a.toLong())

    public fun add(a: Short, b: Short): Short = shortOf(a.toLong() + b.toLong())
    public fun sub(a: Short, b: Short): Short = shortOf(a.toLong() - b.toLong())
    public fun mul(a: Short, b: Short): Short = shortOf(a.toLong() * b.toLong())
    public fun neg(a: Short): Short = shortOf(-a.toLong())

    public fun add(a: Int, b: Int): Int = intOf(a.toLong() + b.toLong())
    public fun sub(a: Int, b: Int): Int = intOf(a.toLong() - b.toLong())
    public fun mul(a: Int, b: Int): Int = intOf(a.toLong() * b.toLong())
    public fun neg(a: Int): Int = intOf(-a.toLong())

    // An unsigned width up to 32 bits computes a sum or a product exactly in
    // `ULong`: neither is negative, and the product of two 32-bit values is
    // below 2^64, where the same product in `Long` would not be. A difference
    // lies below the range exactly when the subtrahend is the larger.
    private fun uByteOf(e: ULong): UByte = e.coerceAtMost(UByte.MAX_VALUE.toULong()).toUByte()
    private fun uShortOf(e: ULong): UShort = e.coerceAtMost(UShort.MAX_VALUE.toULong()).toUShort()
    private fun uIntOf(e: ULong): UInt = e.coerceAtMost(UInt.MAX_VALUE.toULong()).toUInt()

    public fun add(a: UByte, b: UByte): UByte = uByteOf(a.toULong() + b.toULong())
    public fun sub(a: UByte, b: UByte): UByte = if (b > a) UByte.MIN_VALUE else (a - b).toUByte()
    public fun mul(a: UByte, b: UByte): UByte = uByteOf(a.toULong() * b.toULong())

    public fun add(a: UShort, b: UShort): UShort = uShortOf(a.toULong() + b.toULong())
    public fun sub(a: UShort, b: UShort): UShort = if (b > a) UShort.MIN_VALUE else (a - b).toUShort()
    public fun mul(a: UShort, b: UShort): UShort = uShortOf(a.toULong() * b.toULong())

    public fun add(a: UInt, b: UInt): UInt = uIntOf(a.toULong() + b.toULong())
    public fun sub(a: UInt, b: UInt): UInt = if (b > a) UInt.MIN_VALUE else a - b
    public fun mul(a: UInt, b: UInt): UInt = uIntOf(a.toULong() * b.toULong())

    // 64 bits have no wider type, so each operation asks [Range64] on which
    // side of the range its exact result lies, and answers that bound, or its
    // wrapped result when the exact one lies inside, where the two are equal.
    private fun clamp(side: RangeSide, wrapped: Long): Long =
        when (side) {
            RangeSide.Below -> Long.MIN_VALUE
            RangeSide.Inside -> wrapped
            RangeSide.Above -> Long.MAX_VALUE
        }

    private fun clamp(side: RangeSide, wrapped: ULong): ULong =
        when (side) {
            RangeSide.Below -> ULong.MIN_VALUE
            RangeSide.Inside -> wrapped
            RangeSide.Above -> ULong.MAX_VALUE
        }

    public fun add(a: Long, b: Long): Long = clamp(Range64.add(a, b), a + b)
    public fun sub(a: Long, b: Long): Long = clamp(Range64.sub(a, b), a - b)
    public fun mul(a: Long, b: Long): Long = clamp(Range64.mul(a, b), a * b)

    // The one negation past the range is `-MIN`, which is `M + 1`.
    public fun neg(a: Long): Long = if (a == Long.MIN_VALUE) Long.MAX_VALUE else -a

    public fun add(a: ULong, b: ULong): ULong = clamp(Range64.add(a, b), a + b)
    public fun sub(a: ULong, b: ULong): ULong = clamp(Range64.sub(a, b), a - b)
    public fun mul(a: ULong, b: ULong): ULong = clamp(Range64.mul(a, b), a * b)
}
