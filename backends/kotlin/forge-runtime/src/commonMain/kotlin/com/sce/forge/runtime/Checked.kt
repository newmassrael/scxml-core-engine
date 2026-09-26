// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package com.sce.forge.runtime

/**
 * The integer arithmetic contract's runtime half — SCE_FORGE.md Section
 * 3.4.1.
 *
 * An algorithm that declares `<sce:return may-fail="true">` returns an
 * [AlgorithmResult], and the generator lowers each of its integer `+ - * / %`
 * and unary `-` to one of the [SceChecked] helpers. A helper computes the
 * operation at the declared width or reports why it has no value there: an
 * overflow (a signed `MIN / -1` or `MIN % -1` included, which is one), or a
 * division by zero, refused before it is attempted.
 *
 * A helper reports by throwing [AlgorithmFailure], and the generated function
 * turns it into [AlgorithmResult.Failed] at its own boundary, so the throw
 * never leaves the algorithm: its caller reads a value, never an exception.
 * Checking a flag after every operation would do the same without the throw;
 * it would also make every expression a statement.
 */

/** Why a `may-fail` algorithm has no value to return. */
public enum class AlgorithmError(
    /** The failure's name in the contract — the spelling every backend shares. */
    public val contractName: String,
) {
    /** An integer result outside its declared width. */
    Overflow("overflow"),

    /** An integer `/` or `%` by zero. */
    DivideByZero("divide-by-zero"),

    /**
     * A buffer append past its declared capacity. This backend's buffers grow
     * past their capacity (SCE_FORGE.md Section 4.12), so it never reports
     * one; the case exists because the failure has one name on every backend.
     */
    CapacityExceeded("capacity-exceeded"),

    /**
     * A `<sce:require>` precondition that does not hold: an input outside the
     * algorithm's domain.
     */
    Precondition("precondition"),
}

/** What a `may-fail` algorithm returns: its value, or why it has none. */
public sealed class AlgorithmResult<out T> {
    public data class Ok<out T>(val value: T) : AlgorithmResult<T>()

    public data class Failed(val error: AlgorithmError) : AlgorithmResult<Nothing>()
}

/**
 * The internal signal from a [SceChecked] helper to the algorithm that called
 * it. Caught at that algorithm's boundary; never part of its signature.
 */
public class AlgorithmFailure(public val error: AlgorithmError) : RuntimeException(error.contractName)

/** Checked integer operations, one overload per SCE integer width. */
public object SceChecked {
    private fun fail(error: AlgorithmError): Nothing = throw AlgorithmFailure(error)

    /**
     * The value of a call to another `may-fail` algorithm, or its failure
     * passed on to the calling algorithm's own boundary (SCE_FORGE.md
     * Section 3.4.1).
     */
    public fun <T> take(result: AlgorithmResult<T>): T =
        when (result) {
            is AlgorithmResult.Ok -> result.value
            is AlgorithmResult.Failed -> fail(result.error)
        }

    /** `v` when it lies in `[lo, hi]`; an overflow otherwise. */
    private fun fit(v: Long, lo: Long, hi: Long): Long =
        if (v < lo || v > hi) fail(AlgorithmError.Overflow) else v

    private fun nonZero(b: Long) {
        if (b == 0L) fail(AlgorithmError.DivideByZero)
    }

    // Every width up to 32 bits computes exactly in `Long` and is then held
    // to its own range. Division truncates toward zero and `%` takes the
    // dividend's sign, as `Long` does (SCE_FORGE.md Section 3.4.1).
    private fun add(a: Long, b: Long, lo: Long, hi: Long): Long = fit(a + b, lo, hi)
    private fun sub(a: Long, b: Long, lo: Long, hi: Long): Long = fit(a - b, lo, hi)
    private fun mul(a: Long, b: Long, lo: Long, hi: Long): Long = fit(a * b, lo, hi)
    private fun div(a: Long, b: Long, lo: Long, hi: Long): Long {
        nonZero(b)
        return fit(a / b, lo, hi)
    }

    // A remainder whose quotient overflows has no value either: `MIN % -1`
    // fails as `MIN / -1` does, on every backend.
    private fun rem(a: Long, b: Long, lo: Long, hi: Long): Long {
        div(a, b, lo, hi)
        return a % b
    }

    private fun neg(a: Long, lo: Long, hi: Long): Long = fit(-a, lo, hi)

    private const val I8_MIN = Byte.MIN_VALUE.toLong()
    private const val I8_MAX = Byte.MAX_VALUE.toLong()
    private const val I16_MIN = Short.MIN_VALUE.toLong()
    private const val I16_MAX = Short.MAX_VALUE.toLong()
    private const val I32_MIN = Int.MIN_VALUE.toLong()
    private const val I32_MAX = Int.MAX_VALUE.toLong()
    private const val U8_MAX = 0xFFL
    private const val U16_MAX = 0xFFFFL
    private const val U32_MAX = 0xFFFF_FFFFL

    public fun add(a: Byte, b: Byte): Byte = add(a.toLong(), b.toLong(), I8_MIN, I8_MAX).toByte()
    public fun sub(a: Byte, b: Byte): Byte = sub(a.toLong(), b.toLong(), I8_MIN, I8_MAX).toByte()
    public fun mul(a: Byte, b: Byte): Byte = mul(a.toLong(), b.toLong(), I8_MIN, I8_MAX).toByte()
    public fun div(a: Byte, b: Byte): Byte = div(a.toLong(), b.toLong(), I8_MIN, I8_MAX).toByte()
    public fun rem(a: Byte, b: Byte): Byte = rem(a.toLong(), b.toLong(), I8_MIN, I8_MAX).toByte()
    public fun neg(a: Byte): Byte = neg(a.toLong(), I8_MIN, I8_MAX).toByte()

    public fun add(a: Short, b: Short): Short = add(a.toLong(), b.toLong(), I16_MIN, I16_MAX).toShort()
    public fun sub(a: Short, b: Short): Short = sub(a.toLong(), b.toLong(), I16_MIN, I16_MAX).toShort()
    public fun mul(a: Short, b: Short): Short = mul(a.toLong(), b.toLong(), I16_MIN, I16_MAX).toShort()
    public fun div(a: Short, b: Short): Short = div(a.toLong(), b.toLong(), I16_MIN, I16_MAX).toShort()
    public fun rem(a: Short, b: Short): Short = rem(a.toLong(), b.toLong(), I16_MIN, I16_MAX).toShort()
    public fun neg(a: Short): Short = neg(a.toLong(), I16_MIN, I16_MAX).toShort()

    public fun add(a: Int, b: Int): Int = add(a.toLong(), b.toLong(), I32_MIN, I32_MAX).toInt()
    public fun sub(a: Int, b: Int): Int = sub(a.toLong(), b.toLong(), I32_MIN, I32_MAX).toInt()
    public fun mul(a: Int, b: Int): Int = mul(a.toLong(), b.toLong(), I32_MIN, I32_MAX).toInt()
    public fun div(a: Int, b: Int): Int = div(a.toLong(), b.toLong(), I32_MIN, I32_MAX).toInt()
    public fun rem(a: Int, b: Int): Int = rem(a.toLong(), b.toLong(), I32_MIN, I32_MAX).toInt()
    public fun neg(a: Int): Int = neg(a.toLong(), I32_MIN, I32_MAX).toInt()

    public fun add(a: UByte, b: UByte): UByte = add(a.toLong(), b.toLong(), 0L, U8_MAX).toUByte()
    public fun sub(a: UByte, b: UByte): UByte = sub(a.toLong(), b.toLong(), 0L, U8_MAX).toUByte()
    public fun mul(a: UByte, b: UByte): UByte = mul(a.toLong(), b.toLong(), 0L, U8_MAX).toUByte()
    public fun div(a: UByte, b: UByte): UByte = div(a.toLong(), b.toLong(), 0L, U8_MAX).toUByte()
    public fun rem(a: UByte, b: UByte): UByte = rem(a.toLong(), b.toLong(), 0L, U8_MAX).toUByte()
    public fun neg(a: UByte): UByte = neg(a.toLong(), 0L, U8_MAX).toUByte()

    public fun add(a: UShort, b: UShort): UShort = add(a.toLong(), b.toLong(), 0L, U16_MAX).toUShort()
    public fun sub(a: UShort, b: UShort): UShort = sub(a.toLong(), b.toLong(), 0L, U16_MAX).toUShort()
    public fun mul(a: UShort, b: UShort): UShort = mul(a.toLong(), b.toLong(), 0L, U16_MAX).toUShort()
    public fun div(a: UShort, b: UShort): UShort = div(a.toLong(), b.toLong(), 0L, U16_MAX).toUShort()
    public fun rem(a: UShort, b: UShort): UShort = rem(a.toLong(), b.toLong(), 0L, U16_MAX).toUShort()
    public fun neg(a: UShort): UShort = neg(a.toLong(), 0L, U16_MAX).toUShort()

    public fun add(a: UInt, b: UInt): UInt = add(a.toLong(), b.toLong(), 0L, U32_MAX).toUInt()
    public fun sub(a: UInt, b: UInt): UInt = sub(a.toLong(), b.toLong(), 0L, U32_MAX).toUInt()
    public fun mul(a: UInt, b: UInt): UInt = mul(a.toLong(), b.toLong(), 0L, U32_MAX).toUInt()
    public fun div(a: UInt, b: UInt): UInt = div(a.toLong(), b.toLong(), 0L, U32_MAX).toUInt()
    public fun rem(a: UInt, b: UInt): UInt = rem(a.toLong(), b.toLong(), 0L, U32_MAX).toUInt()
    public fun neg(a: UInt): UInt = neg(a.toLong(), 0L, U32_MAX).toUInt()

    // 64 bits have no wider type to compute in, so each operation detects its
    // own overflow from the wrapped result.
    public fun add(a: Long, b: Long): Long {
        val r = a + b
        if (((a xor r) and (b xor r)) < 0L) fail(AlgorithmError.Overflow)
        return r
    }

    public fun sub(a: Long, b: Long): Long {
        val r = a - b
        if (((a xor b) and (a xor r)) < 0L) fail(AlgorithmError.Overflow)
        return r
    }

    public fun mul(a: Long, b: Long): Long {
        if (a == 0L || b == 0L) return 0L
        if ((a == -1L && b == Long.MIN_VALUE) || (b == -1L && a == Long.MIN_VALUE)) {
            fail(AlgorithmError.Overflow)
        }
        val r = a * b
        if (r / b != a) fail(AlgorithmError.Overflow)
        return r
    }

    public fun div(a: Long, b: Long): Long {
        nonZero(b)
        if (a == Long.MIN_VALUE && b == -1L) fail(AlgorithmError.Overflow)
        return a / b
    }

    public fun rem(a: Long, b: Long): Long {
        div(a, b)
        return a % b
    }

    public fun neg(a: Long): Long {
        if (a == Long.MIN_VALUE) fail(AlgorithmError.Overflow)
        return -a
    }

    public fun add(a: ULong, b: ULong): ULong {
        val r = a + b
        if (r < a) fail(AlgorithmError.Overflow)
        return r
    }

    public fun sub(a: ULong, b: ULong): ULong {
        if (b > a) fail(AlgorithmError.Overflow)
        return a - b
    }

    public fun mul(a: ULong, b: ULong): ULong {
        if (a == 0uL || b == 0uL) return 0uL
        val r = a * b
        if (r / b != a) fail(AlgorithmError.Overflow)
        return r
    }

    public fun div(a: ULong, b: ULong): ULong {
        if (b == 0uL) fail(AlgorithmError.DivideByZero)
        return a / b
    }

    public fun rem(a: ULong, b: ULong): ULong {
        if (b == 0uL) fail(AlgorithmError.DivideByZero)
        return a % b
    }

    public fun neg(a: ULong): ULong {
        if (a != 0uL) fail(AlgorithmError.Overflow)
        return a
    }

    // A value stored where a narrower integer type is declared is the same
    // value, or an overflow when the type cannot hold it — never a wrapped
    // one. The value arrives as `Long` (from a signed type) or `ULong` (from
    // an unsigned one), either of which holds it exactly, so one overload per
    // target and signedness serves every source width. A pair whose every
    // value fits (`Long` from signed, `ULong` from unsigned) has none: the
    // generator emits no check there.
    private fun fitUnsigned(v: ULong, hi: Long): Long =
        if (v > hi.toULong()) fail(AlgorithmError.Overflow) else v.toLong()

    public fun narrowToByte(v: Long): Byte = fit(v, I8_MIN, I8_MAX).toByte()
    public fun narrowToByte(v: ULong): Byte = fitUnsigned(v, I8_MAX).toByte()
    public fun narrowToShort(v: Long): Short = fit(v, I16_MIN, I16_MAX).toShort()
    public fun narrowToShort(v: ULong): Short = fitUnsigned(v, I16_MAX).toShort()
    public fun narrowToInt(v: Long): Int = fit(v, I32_MIN, I32_MAX).toInt()
    public fun narrowToInt(v: ULong): Int = fitUnsigned(v, I32_MAX).toInt()
    public fun narrowToLong(v: ULong): Long = fitUnsigned(v, Long.MAX_VALUE)
    public fun narrowToUByte(v: Long): UByte = fit(v, 0L, U8_MAX).toUByte()
    public fun narrowToUByte(v: ULong): UByte = fitUnsigned(v, U8_MAX).toUByte()
    public fun narrowToUShort(v: Long): UShort = fit(v, 0L, U16_MAX).toUShort()
    public fun narrowToUShort(v: ULong): UShort = fitUnsigned(v, U16_MAX).toUShort()
    public fun narrowToUInt(v: Long): UInt = fit(v, 0L, U32_MAX).toUInt()
    public fun narrowToUInt(v: ULong): UInt = fitUnsigned(v, U32_MAX).toUInt()
    public fun narrowToULong(v: Long): ULong =
        if (v < 0L) fail(AlgorithmError.Overflow) else v.toULong()
}
