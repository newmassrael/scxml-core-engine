// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package com.sce.forge.runtime

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.fail

/**
 * One operation of a type on its operands ([b] unread for `neg`), with the value
 * each rule gives it. The values are worked from the definition (docs/adr/0063,
 * section 1): [wrap] is the exact result moved by a multiple of `2^W` into
 * `[m, M]`, [sat] is the exact result held to `[m, M]`.
 */
private data class Case<T>(val op: String, val a: T, val b: T, val wrap: T, val sat: T)

/**
 * One type's helpers under each rule. An unsigned type has no saturating
 * negation, so its saturating table has no `neg`, and a case of any other
 * operation a table lacks is a broken table, not a skip.
 */
private class Family<T>(
    val wrapping: Map<String, (T, T) -> T>,
    val saturating: Map<String, (T, T) -> T>,
)

private fun <T> check(type: String, family: Family<T>, cases: List<Case<T>>) {
    for (c in cases) {
        val wrap = family.wrapping[c.op] ?: fail("$type: no wrapping ${c.op}")
        assertEquals(c.wrap, wrap(c.a, c.b), "$type wrapping ${c.op}(${c.a}, ${c.b})")
        val sat = family.saturating[c.op]
        if (sat == null) {
            assertEquals("neg", c.op, "$type: no saturating ${c.op}")
            continue
        }
        assertEquals(c.sat, sat(c.a, c.b), "$type saturating ${c.op}(${c.a}, ${c.b})")
    }
}

// A value of a case as the type's own, refused rather than truncated when it
// is outside the type: a mistyped expectation must not wrap into a right one.
private fun exact(v: Long, lo: Long, hi: Long): Long {
    require(v in lo..hi) { "$v is outside [$lo, $hi]" }
    return v
}

private fun exact(v: ULong, hi: ULong): ULong {
    require(v <= hi) { "$v is above $hi" }
    return v
}

/**
 * The cases every signed type shares, at its own `m` and `M`: the wrap point and
 * the saturation point on each side with their neighbours inside, `MIN * -1`,
 * `-MIN` and `MIN - 1`, and the products past `2^W` but within `2^(2W)` that
 * `M * M`, `m * m` and `M * m` make.
 */
private fun <T> signedCases(m: Long, mx: Long, n: (Long) -> T): List<Case<T>> {
    fun row(op: String, a: Long, b: Long, wrap: Long, sat: Long) = Case(op, n(a), n(b), n(wrap), n(sat))
    return listOf(
        row("add", mx, 1, m, mx),
        row("add", mx - 1, 1, mx, mx),
        row("add", m, -1, mx, m),
        row("add", m + 1, -1, m, m),
        row("add", mx, mx, -2, mx),
        row("add", m, m, 0, m),
        row("add", mx, m, -1, -1),
        row("add", 0, 0, 0, 0),
        row("add", -1, 1, 0, 0),
        row("sub", m, 1, mx, m),
        row("sub", m + 1, 1, m, m),
        row("sub", mx, -1, m, mx),
        row("sub", mx - 1, -1, mx, mx),
        row("sub", 0, m, m, mx),
        row("sub", -1, m, mx, mx),
        row("sub", m, mx, 1, m),
        row("sub", mx, m, -1, mx),
        row("sub", 0, 1, -1, -1),
        row("sub", mx, mx, 0, 0),
        row("mul", m, -1, m, mx),
        row("mul", -1, m, m, mx),
        row("mul", m + 1, -1, mx, mx),
        row("mul", mx, 2, -2, mx),
        row("mul", m, 2, 0, m),
        row("mul", mx, mx, 1, mx),
        row("mul", m, m, 0, mx),
        row("mul", mx, m, m, m),
        row("mul", m, 1, m, m),
        row("mul", 1, mx, mx, mx),
        row("mul", -1, -1, 1, 1),
        row("mul", 0, m, 0, 0),
        row("neg", m, 0, m, mx),
        row("neg", m + 1, 0, mx, mx),
        row("neg", mx, 0, m + 1, m + 1),
        row("neg", 0, 0, 0, 0),
        row("neg", 1, 0, -1, -1),
        row("neg", -1, 0, 1, 1),
    )
}

/**
 * The cases every unsigned type shares, at its own `M`: the wrap point and the
 * saturation point at each end, and `M * M`, which is `2^(2W) - 2^(W+1) + 1` and
 * so 1 modulo `2^W`. A negation's saturating value is unread.
 */
private fun <T> unsignedCases(mx: ULong, n: (ULong) -> T): List<Case<T>> {
    fun row(op: String, a: ULong, b: ULong, wrap: ULong, sat: ULong) = Case(op, n(a), n(b), n(wrap), n(sat))
    val half = mx / 2uL + 1uL // 2^(W-1)
    return listOf(
        row("add", mx, 1uL, 0uL, mx),
        row("add", mx - 1uL, 1uL, mx, mx),
        row("add", 1uL, mx - 1uL, mx, mx),
        row("add", mx, mx, mx - 1uL, mx),
        row("add", 0uL, 0uL, 0uL, 0uL),
        row("sub", 0uL, 1uL, mx, 0uL),
        row("sub", 1uL, 1uL, 0uL, 0uL),
        row("sub", 0uL, mx, 1uL, 0uL),
        row("sub", mx - 1uL, mx, mx, 0uL),
        row("sub", mx, mx, 0uL, 0uL),
        row("sub", mx, 0uL, mx, mx),
        row("mul", mx, mx, 1uL, mx),
        row("mul", mx, 2uL, mx - 1uL, mx),
        row("mul", half, 2uL, 0uL, mx),
        row("mul", 1uL, mx, mx, mx),
        row("mul", 0uL, mx, 0uL, 0uL),
        row("neg", 0uL, 0uL, 0uL, 0uL),
        row("neg", 1uL, 0uL, mx, 0uL),
        row("neg", mx, 0uL, 1uL, 0uL),
        row("neg", half, 0uL, half, 0uL),
    )
}

private val byte: (Long) -> Byte = { exact(it, Byte.MIN_VALUE.toLong(), Byte.MAX_VALUE.toLong()).toByte() }
private val short: (Long) -> Short = { exact(it, Short.MIN_VALUE.toLong(), Short.MAX_VALUE.toLong()).toShort() }
private val int: (Long) -> Int = { exact(it, Int.MIN_VALUE.toLong(), Int.MAX_VALUE.toLong()).toInt() }
private val long: (Long) -> Long = { it }
private val uByte: (ULong) -> UByte = { exact(it, UByte.MAX_VALUE.toULong()).toUByte() }
private val uShort: (ULong) -> UShort = { exact(it, UShort.MAX_VALUE.toULong()).toUShort() }
private val uInt: (ULong) -> UInt = { exact(it, UInt.MAX_VALUE.toULong()).toUInt() }
private val uLong: (ULong) -> ULong = { it }

// Cases of one type alone, written as values of `Long` or `ULong` and taken
// into the type by `n`.
private fun <T> signedRows(n: (Long) -> T, vararg rows: Case<Long>): List<Case<T>> =
    rows.map { Case(it.op, n(it.a), n(it.b), n(it.wrap), n(it.sat)) }

private fun <T> unsignedRows(n: (ULong) -> T, vararg rows: Case<ULong>): List<Case<T>> =
    rows.map { Case(it.op, n(it.a), n(it.b), n(it.wrap), n(it.sat)) }

class TotalArithmeticTest {

    @Test
    fun byteWrapsAndSaturates() {
        val family = Family<Byte>(
            wrapping = mapOf(
                "add" to { a: Byte, b: Byte -> SceWrapping.add(a, b) },
                "sub" to { a: Byte, b: Byte -> SceWrapping.sub(a, b) },
                "mul" to { a: Byte, b: Byte -> SceWrapping.mul(a, b) },
                "neg" to { a: Byte, _: Byte -> SceWrapping.neg(a) },
            ),
            saturating = mapOf(
                "add" to { a: Byte, b: Byte -> SceSaturating.add(a, b) },
                "sub" to { a: Byte, b: Byte -> SceSaturating.sub(a, b) },
                "mul" to { a: Byte, b: Byte -> SceSaturating.mul(a, b) },
                "neg" to { a: Byte, _: Byte -> SceSaturating.neg(a) },
            ),
        )
        check("Byte", family, signedCases(Byte.MIN_VALUE.toLong(), Byte.MAX_VALUE.toLong(), byte))
        check(
            "Byte",
            family,
            signedRows(
                byte,
                Case("mul", 17L, 15L, -1L, 127L),
                Case("mul", -17L, 15L, 1L, -128L),
                Case("mul", 16L, 16L, 0L, 127L),
                Case("add", 100L, 100L, -56L, 127L),
            ),
        )
    }

    @Test
    fun shortWrapsAndSaturates() {
        val family = Family<Short>(
            wrapping = mapOf(
                "add" to { a: Short, b: Short -> SceWrapping.add(a, b) },
                "sub" to { a: Short, b: Short -> SceWrapping.sub(a, b) },
                "mul" to { a: Short, b: Short -> SceWrapping.mul(a, b) },
                "neg" to { a: Short, _: Short -> SceWrapping.neg(a) },
            ),
            saturating = mapOf(
                "add" to { a: Short, b: Short -> SceSaturating.add(a, b) },
                "sub" to { a: Short, b: Short -> SceSaturating.sub(a, b) },
                "mul" to { a: Short, b: Short -> SceSaturating.mul(a, b) },
                "neg" to { a: Short, _: Short -> SceSaturating.neg(a) },
            ),
        )
        check("Short", family, signedCases(Short.MIN_VALUE.toLong(), Short.MAX_VALUE.toLong(), short))
        check(
            "Short",
            family,
            signedRows(
                short,
                Case("mul", 257L, 255L, -1L, 32767L),
                Case("mul", -257L, 255L, 1L, -32768L),
                Case("mul", 256L, 256L, 0L, 32767L),
            ),
        )
    }

    @Test
    fun intWrapsAndSaturates() {
        val family = Family<Int>(
            wrapping = mapOf(
                "add" to { a: Int, b: Int -> SceWrapping.add(a, b) },
                "sub" to { a: Int, b: Int -> SceWrapping.sub(a, b) },
                "mul" to { a: Int, b: Int -> SceWrapping.mul(a, b) },
                "neg" to { a: Int, _: Int -> SceWrapping.neg(a) },
            ),
            saturating = mapOf(
                "add" to { a: Int, b: Int -> SceSaturating.add(a, b) },
                "sub" to { a: Int, b: Int -> SceSaturating.sub(a, b) },
                "mul" to { a: Int, b: Int -> SceSaturating.mul(a, b) },
                "neg" to { a: Int, _: Int -> SceSaturating.neg(a) },
            ),
        )
        check("Int", family, signedCases(Int.MIN_VALUE.toLong(), Int.MAX_VALUE.toLong(), int))
        check(
            "Int",
            family,
            signedRows(
                int,
                Case("mul", 65537L, 65535L, -1L, 2147483647L),
                Case("mul", -65537L, 65535L, 1L, -2147483648L),
                Case("mul", 65536L, 65536L, 0L, 2147483647L),
            ),
        )
    }

    // At 64 bits the saturating helpers test their operands against the bounds,
    // so the cases also hold a product just inside the range beside one just past
    // it: 3037000499^2 is below 2^63 - 1 and 3037000500^2 above it.
    @Test
    fun longWrapsAndSaturates() {
        val family = Family<Long>(
            wrapping = mapOf(
                "add" to { a: Long, b: Long -> SceWrapping.add(a, b) },
                "sub" to { a: Long, b: Long -> SceWrapping.sub(a, b) },
                "mul" to { a: Long, b: Long -> SceWrapping.mul(a, b) },
                "neg" to { a: Long, _: Long -> SceWrapping.neg(a) },
            ),
            saturating = mapOf(
                "add" to { a: Long, b: Long -> SceSaturating.add(a, b) },
                "sub" to { a: Long, b: Long -> SceSaturating.sub(a, b) },
                "mul" to { a: Long, b: Long -> SceSaturating.mul(a, b) },
                "neg" to { a: Long, _: Long -> SceSaturating.neg(a) },
            ),
        )
        check("Long", family, signedCases(Long.MIN_VALUE, Long.MAX_VALUE, long))
        check(
            "Long",
            family,
            signedRows(
                long,
                Case("mul", 4294967297L, 4294967295L, -1L, Long.MAX_VALUE),
                Case("mul", -4294967297L, 4294967295L, 1L, Long.MIN_VALUE),
                Case("mul", 4294967296L, 4294967296L, 0L, Long.MAX_VALUE),
                Case("mul", 3037000499L, 3037000499L, 9223372030926249001L, 9223372030926249001L),
                Case("mul", 3037000500L, 3037000500L, -9223372036709301616L, Long.MAX_VALUE),
                Case("mul", -3037000500L, 3037000500L, 9223372036709301616L, Long.MIN_VALUE),
                Case("mul", 3037000500L, -3037000500L, 9223372036709301616L, Long.MIN_VALUE),
                Case("mul", -3037000500L, -3037000500L, -9223372036709301616L, Long.MAX_VALUE),
            ),
        )
    }

    @Test
    fun uByteWrapsAndSaturates() {
        val family = Family<UByte>(
            wrapping = mapOf(
                "add" to { a: UByte, b: UByte -> SceWrapping.add(a, b) },
                "sub" to { a: UByte, b: UByte -> SceWrapping.sub(a, b) },
                "mul" to { a: UByte, b: UByte -> SceWrapping.mul(a, b) },
                "neg" to { a: UByte, _: UByte -> SceWrapping.neg(a) },
            ),
            saturating = mapOf(
                "add" to { a: UByte, b: UByte -> SceSaturating.add(a, b) },
                "sub" to { a: UByte, b: UByte -> SceSaturating.sub(a, b) },
                "mul" to { a: UByte, b: UByte -> SceSaturating.mul(a, b) },
            ),
        )
        check("UByte", family, unsignedCases(UByte.MAX_VALUE.toULong(), uByte))
        check(
            "UByte",
            family,
            unsignedRows(
                uByte,
                Case("mul", 16uL, 16uL, 0uL, 255uL),
                Case("mul", 17uL, 15uL, 255uL, 255uL),
                Case("mul", 17uL, 17uL, 33uL, 255uL),
            ),
        )
    }

    @Test
    fun uShortWrapsAndSaturates() {
        val family = Family<UShort>(
            wrapping = mapOf(
                "add" to { a: UShort, b: UShort -> SceWrapping.add(a, b) },
                "sub" to { a: UShort, b: UShort -> SceWrapping.sub(a, b) },
                "mul" to { a: UShort, b: UShort -> SceWrapping.mul(a, b) },
                "neg" to { a: UShort, _: UShort -> SceWrapping.neg(a) },
            ),
            saturating = mapOf(
                "add" to { a: UShort, b: UShort -> SceSaturating.add(a, b) },
                "sub" to { a: UShort, b: UShort -> SceSaturating.sub(a, b) },
                "mul" to { a: UShort, b: UShort -> SceSaturating.mul(a, b) },
            ),
        )
        check("UShort", family, unsignedCases(UShort.MAX_VALUE.toULong(), uShort))
        check(
            "UShort",
            family,
            unsignedRows(
                uShort,
                Case("mul", 256uL, 256uL, 0uL, 65535uL),
                Case("mul", 257uL, 255uL, 65535uL, 65535uL),
                Case("mul", 257uL, 257uL, 513uL, 65535uL),
            ),
        )
    }

    // A product of two UInt values past 2^63 is the case computing in Long would
    // get wrong: (2^32 - 1)^2 saturates to M, not to 0.
    @Test
    fun uIntWrapsAndSaturates() {
        val family = Family<UInt>(
            wrapping = mapOf(
                "add" to { a: UInt, b: UInt -> SceWrapping.add(a, b) },
                "sub" to { a: UInt, b: UInt -> SceWrapping.sub(a, b) },
                "mul" to { a: UInt, b: UInt -> SceWrapping.mul(a, b) },
                "neg" to { a: UInt, _: UInt -> SceWrapping.neg(a) },
            ),
            saturating = mapOf(
                "add" to { a: UInt, b: UInt -> SceSaturating.add(a, b) },
                "sub" to { a: UInt, b: UInt -> SceSaturating.sub(a, b) },
                "mul" to { a: UInt, b: UInt -> SceSaturating.mul(a, b) },
            ),
        )
        check("UInt", family, unsignedCases(UInt.MAX_VALUE.toULong(), uInt))
        check(
            "UInt",
            family,
            unsignedRows(
                uInt,
                Case("mul", 65536uL, 65536uL, 0uL, 4294967295uL),
                Case("mul", 65537uL, 65535uL, 4294967295uL, 4294967295uL),
                Case("mul", 65537uL, 65537uL, 131073uL, 4294967295uL),
            ),
        )
    }

    @Test
    fun uLongWrapsAndSaturates() {
        val family = Family<ULong>(
            wrapping = mapOf(
                "add" to { a: ULong, b: ULong -> SceWrapping.add(a, b) },
                "sub" to { a: ULong, b: ULong -> SceWrapping.sub(a, b) },
                "mul" to { a: ULong, b: ULong -> SceWrapping.mul(a, b) },
                "neg" to { a: ULong, _: ULong -> SceWrapping.neg(a) },
            ),
            saturating = mapOf(
                "add" to { a: ULong, b: ULong -> SceSaturating.add(a, b) },
                "sub" to { a: ULong, b: ULong -> SceSaturating.sub(a, b) },
                "mul" to { a: ULong, b: ULong -> SceSaturating.mul(a, b) },
            ),
        )
        check("ULong", family, unsignedCases(ULong.MAX_VALUE, uLong))
        check(
            "ULong",
            family,
            unsignedRows(
                uLong,
                Case("mul", 4294967296uL, 4294967296uL, 0uL, ULong.MAX_VALUE),
                Case("mul", 4294967297uL, 4294967295uL, ULong.MAX_VALUE, ULong.MAX_VALUE),
                Case("mul", 4294967297uL, 4294967297uL, 8589934593uL, ULong.MAX_VALUE),
            ),
        )
    }

    // The checked 64-bit helpers read the same bound tests as the saturating
    // ones: a result just past either bound fails, one at the bound passes.
    @Test
    fun checked64BitBoundsMatchTheSaturatingOnes() {
        fun overflows(run: () -> Unit) =
            assertEquals(AlgorithmError.Overflow, assertFailsWith<AlgorithmFailure> { run() }.error)
        overflows { SceChecked.sub(Long.MIN_VALUE, 1L) }
        overflows { SceChecked.add(Long.MIN_VALUE, -1L) }
        overflows { SceChecked.sub(Long.MAX_VALUE, -1L) }
        overflows { SceChecked.mul(Long.MIN_VALUE, -1L) }
        overflows { SceChecked.mul(-1L, Long.MIN_VALUE) }
        overflows { SceChecked.sub(0L, Long.MIN_VALUE) }
        overflows { SceChecked.add(ULong.MAX_VALUE, 1uL) }
        overflows { SceChecked.sub(0uL, 1uL) }
        overflows { SceChecked.mul(4294967296uL, 4294967296uL) }
        assertEquals(Long.MAX_VALUE, SceChecked.mul(Long.MIN_VALUE + 1L, -1L))
        assertEquals(Long.MIN_VALUE, SceChecked.mul(Long.MIN_VALUE, 1L))
        assertEquals(Long.MAX_VALUE, SceChecked.sub(-1L, Long.MIN_VALUE))
        assertEquals(Long.MIN_VALUE, SceChecked.add(Long.MIN_VALUE, 0L))
        assertEquals(ULong.MAX_VALUE, SceChecked.mul(4294967297uL, 4294967295uL))
        assertEquals(ULong.MAX_VALUE, SceChecked.add(ULong.MAX_VALUE - 1uL, 1uL))
    }
}
