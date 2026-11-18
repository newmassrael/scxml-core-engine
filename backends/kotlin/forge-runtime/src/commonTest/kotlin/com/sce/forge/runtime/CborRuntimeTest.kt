// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package com.sce.forge.runtime

import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlin.test.assertTrue
import kotlin.test.fail

/**
 * The items of [Cbor] on their own, the twin of the Rust, C++ and C11
 * runtimes' `cbor_runtime_test` (docs/adr/0101, "Conformance" item 2): a
 * signed integer and a float at every head and precision boundary, written in
 * their one deterministic form and read from every well-formed encoding of
 * their value; an indefinite text, byte string and skipped value refused as
 * outside the profile; and each refusal named by its class. What a generated
 * codec does with these is [CborCodecTest]'s and the conformance harness's.
 */
class CborRuntimeTest {

    private fun wire(vararg bytes: Int): ByteArray = ByteArray(bytes.size) { bytes[it].toByte() }

    private fun written(write: (SceSink) -> CodecError?): ByteArray {
        val out = mutableListOf<Byte>()
        assertNull(write(MutableListSink(out)), "a write into a growable sink refused")
        return out.toByteArray()
    }

    private fun hex(bytes: ByteArray): String = bytes.joinToString(" ") { (it.toInt() and 0xFF).toString(16).padStart(2, '0') }

    /** [read] over [bytes] reads it whole. */
    private fun <T> readWhole(bytes: ByteArray, read: (SceCursor) -> Decoded<T>): T {
        val cursor = SceCursor(bytes)
        val value = read(cursor).valueOr { fail("${hex(bytes)} is refused as ${it.error}") }
        assertEquals(0, cursor.remaining(), "${hex(bytes)} left bytes unread")
        return value
    }

    /** An `int64`, the widest signed type. */
    private fun int64(c: SceCursor): Decoded<Long> = Cbor.readIntWithin(c, Long.MIN_VALUE, Long.MAX_VALUE)

    @Test
    fun aSignedIntegerIsWrittenInTheShortestHeadOfItsSign() {
        val cases = listOf(
            0L to wire(0x00),
            23L to wire(0x17),
            24L to wire(0x18, 0x18),
            255L to wire(0x18, 0xff),
            256L to wire(0x19, 0x01, 0x00),
            65535L to wire(0x19, 0xff, 0xff),
            65536L to wire(0x1a, 0x00, 0x01, 0x00, 0x00),
            4294967295L to wire(0x1a, 0xff, 0xff, 0xff, 0xff),
            4294967296L to wire(0x1b, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00),
            Long.MAX_VALUE to wire(0x1b, 0x7f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff),
            -1L to wire(0x20),
            -24L to wire(0x37),
            -25L to wire(0x38, 0x18),
            -256L to wire(0x38, 0xff),
            -257L to wire(0x39, 0x01, 0x00),
            -500L to wire(0x39, 0x01, 0xf3),
            -65536L to wire(0x39, 0xff, 0xff),
            -65537L to wire(0x3a, 0x00, 0x01, 0x00, 0x00),
            -4294967296L to wire(0x3a, 0xff, 0xff, 0xff, 0xff),
            -4294967297L to wire(0x3b, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00),
            Long.MIN_VALUE to wire(0x3b, 0x7f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff),
        )
        for ((value, bytes) in cases) {
            assertContentEquals(bytes, written { Cbor.writeInt(it, value) }, "$value")
            assertEquals(value, readWhole(bytes) { int64(it) }, hex(bytes))
        }
    }

    @Test
    fun aSignedIntegerIsReadFromALongerHeadAndHeldToItsType() {
        // -1 with its argument in a one-byte head, and 1 in an eight-byte one.
        assertEquals(-1L, readWhole(wire(0x38, 0x00)) { int64(it) })
        assertEquals(1L, readWhole(wire(0x1b, 0, 0, 0, 0, 0, 0, 0, 1)) { int64(it) })
        // An int8 holds -128 and 127, and neither -129 nor 128.
        val int8 = { c: SceCursor -> Cbor.readIntWithin(c, Byte.MIN_VALUE.toLong(), Byte.MAX_VALUE.toLong()) }
        assertEquals(-128L, readWhole(wire(0x38, 0x7f), int8))
        assertEquals(127L, readWhole(wire(0x18, 0x7f), int8))
        assertEquals(Decoded.Refused(CodecError.CborOutOfRange), int8(SceCursor(wire(0x38, 0x80))))
        assertEquals(Decoded.Refused(CodecError.CborOutOfRange), int8(SceCursor(wire(0x18, 0x80))))
        // An int64 holds neither -2^63 - 1 nor 2^63.
        assertEquals(
            Decoded.Refused(CodecError.CborOutOfRange),
            int64(SceCursor(wire(0x3b, 0x80, 0, 0, 0, 0, 0, 0, 0))),
        )
        assertEquals(
            Decoded.Refused(CodecError.CborOutOfRange),
            int64(SceCursor(wire(0x1b, 0x80, 0, 0, 0, 0, 0, 0, 0))),
        )
        // A text, a float and a tagged item are no integer.
        for (bytes in listOf(wire(0x61, 0x61), wire(0xf9, 0x3c, 0x00), wire(0xc2, 0x41, 0x01))) {
            assertEquals(Decoded.Refused(CodecError.CborWrongType), int64(SceCursor(bytes)), hex(bytes))
        }
    }

    @Test
    fun anUnsignedTypeTakesNoNegativeIntegerAndNoBignum() {
        assertEquals(Decoded.Refused(CodecError.CborWrongType), Cbor.readUint(SceCursor(wire(0x20))))
        assertEquals(Decoded.Refused(CodecError.CborWrongType), Cbor.readUint(SceCursor(wire(0xc2, 0x41, 0x01))))
        assertEquals(Decoded.Refused(CodecError.CborWrongType), Cbor.readUint(SceCursor(wire(0x61, 0x61))))
    }

    @Test
    fun aFloatIsWrittenInTheShortestPrecisionThatHoldsItExactly() {
        val cases = listOf(
            0.0 to wire(0xf9, 0x00, 0x00),
            -0.0 to wire(0xf9, 0x80, 0x00),
            1.0 to wire(0xf9, 0x3c, 0x00),
            1.5 to wire(0xf9, 0x3e, 0x00),
            // The largest half, the least normal half and the least subnormal half.
            65504.0 to wire(0xf9, 0x7b, 0xff),
            0.00006103515625 to wire(0xf9, 0x04, 0x00),
            5.9604644775390625e-8 to wire(0xf9, 0x00, 0x01),
            Double.POSITIVE_INFINITY to wire(0xf9, 0x7c, 0x00),
            Double.NEGATIVE_INFINITY to wire(0xf9, 0xfc, 0x00),
            // Past a half's range, or between its values: a single holds it.
            65536.0 to wire(0xfa, 0x47, 0x80, 0x00, 0x00),
            100000.0 to wire(0xfa, 0x47, 0xc3, 0x50, 0x00),
            3.4028234663852886e38 to wire(0xfa, 0x7f, 0x7f, 0xff, 0xff),
            // Only a double holds it.
            1.1 to wire(0xfb, 0x3f, 0xf1, 0x99, 0x99, 0x99, 0x99, 0x99, 0x9a),
            1.0e300 to wire(0xfb, 0x7e, 0x37, 0xe4, 0x3c, 0x88, 0x00, 0x75, 0x9c),
        )
        for ((value, bytes) in cases) {
            assertContentEquals(bytes, written { Cbor.writeFloat(it, value) }, "$value")
            val read = readWhole(bytes) { Cbor.readFloat(it) }
            assertEquals(value.toRawBits(), read.toRawBits(), hex(bytes))
        }
        // The one NaN, whatever NaN is given.
        assertContentEquals(wire(0xf9, 0x7e, 0x00), written { Cbor.writeFloat(it, Double.NaN) })
        assertContentEquals(wire(0xf9, 0x7e, 0x00), written { Cbor.writeFloat(it, Double.fromBits(0x7ff0000000000001L)) })
        // A float32 is written as the double it is exactly: 0.1f needs a single.
        assertContentEquals(wire(0xfa, 0x3d, 0xcc, 0xcc, 0xcd), written { Cbor.writeFloat(it, 0.1f) })
    }

    @Test
    fun aFloatIsReadFromAWiderPrecisionThanItNeeds() {
        // 1.5 as a single and as a double.
        assertEquals(1.5, readWhole(wire(0xfa, 0x3f, 0xc0, 0x00, 0x00)) { Cbor.readFloat(it) })
        assertEquals(1.5, readWhole(wire(0xfb, 0x3f, 0xf8, 0, 0, 0, 0, 0, 0)) { Cbor.readFloat(it) })
        // A float32 given as the double that is exactly 0.1f, and as a half.
        assertEquals(0.1f, readWhole(wire(0xfb, 0x3f, 0xb9, 0x99, 0x99, 0xa0, 0, 0, 0)) { Cbor.readFloat32(it) })
        assertEquals(1.5f, readWhole(wire(0xf9, 0x3e, 0x00)) { Cbor.readFloat32(it) })
        // A NaN of any precision is a NaN of either type.
        assertTrue(readWhole(wire(0xf9, 0x7e, 0x00)) { Cbor.readFloat(it) }.isNaN())
        assertTrue(readWhole(wire(0xfb, 0x7f, 0xf8, 0, 0, 0, 0, 0, 0)) { Cbor.readFloat32(it) }.isNaN())
    }

    @Test
    fun aFloatThatNoSingleIsIsNoFloat32AndAnIntegerIsNoFloat() {
        // 0.1 as a double is between two singles.
        assertEquals(
            Decoded.Refused(CodecError.CborOutOfRange),
            Cbor.readFloat32(SceCursor(wire(0xfb, 0x3f, 0xb9, 0x99, 0x99, 0x99, 0x99, 0x99, 0x9a))),
        )
        assertEquals(Decoded.Refused(CodecError.CborWrongType), Cbor.readFloat(SceCursor(wire(0x01))))
        assertEquals(Decoded.Refused(CodecError.CborWrongType), Cbor.readFloat(SceCursor(wire(0xf5))))
        // `null` where a float stands.
        assertEquals(Decoded.Refused(CodecError.CborWrongType), Cbor.readFloat(SceCursor(wire(0xf6))))
    }

    @Test
    fun aTruthValueIsNeitherNullNorAFloatAndASimpleValueBelow32HasOneForm() {
        assertEquals(Decoded.Ok(false), Cbor.readBool(SceCursor(wire(0xf4))))
        assertEquals(Decoded.Ok(true), Cbor.readBool(SceCursor(wire(0xf5))))
        // `null`, a half whose bits are 20, and the simple value 32.
        for (bytes in listOf(wire(0xf6), wire(0xf9, 0x00, 0x14), wire(0xf8, 0x20))) {
            assertEquals(Decoded.Refused(CodecError.CborWrongType), Cbor.readBool(SceCursor(bytes)), hex(bytes))
        }
        // `false` in the two-byte form, and a break code standing alone: not well-formed.
        for (bytes in listOf(wire(0xf8, 0x14), wire(0xff))) {
            assertEquals(Decoded.Refused(CodecError.CborMalformed), Cbor.readBool(SceCursor(bytes)), hex(bytes))
        }
    }

    @Test
    fun anIndefiniteLengthIsOutsideTheProfileAtAnyDepth() {
        val malformed = Decoded.Refused(CodecError.CborMalformed)
        // A text in chunks, a byte string in chunks.
        assertEquals(malformed, Cbor.readText(SceCursor(wire(0x7f, 0x61, 0x61, 0x61, 0x62, 0xff)), null))
        assertEquals(malformed, Cbor.readBytes(SceCursor(wire(0x5f, 0x41, 0x01, 0xff)), null))
        // An indefinite array skipped, and one inside a map skipped.
        assertEquals(malformed, Cbor.skip(SceCursor(wire(0x9f, 0x01, 0x02, 0xff))))
        assertEquals(malformed, Cbor.skip(SceCursor(wire(0xa1, 0x01, 0x7f, 0x61, 0x61, 0xff))))
        // Reserved additional information.
        assertEquals(malformed, Cbor.readUint(SceCursor(wire(0x1c))))
    }

    @Test
    fun aUnionIsAMapOfOneEntryKeyedByItsVariantName() {
        val cases = listOf("card", "cash", "voucher")
        // `cash`, a case that holds no record.
        val cash = wire(0xa1, 0x64, 0x63, 0x61, 0x73, 0x68, 0xa0)
        assertContentEquals(cash, written { Cbor.writeVariant(it, "cash") ?: Cbor.writeMapHead(it, 0UL) })
        val cursor = SceCursor(cash)
        assertEquals(Decoded.Ok(1), Cbor.readVariant(cursor, cases))
        assertEquals(Decoded.Ok(0UL), Cbor.readMapLen(cursor))
        assertEquals(0, cursor.remaining())
    }

    @Test
    fun aUnionMapIsJudgedForOneEntryThenForItsKey() {
        val cases = listOf("card", "cash", "voucher")
        fun refused(kind: CodecError, bytes: ByteArray) =
            assertEquals(Decoded.Refused(kind), Cbor.readVariant(SceCursor(bytes), cases), hex(bytes))
        // No entry, and two entries refused before the key is read: no key follows.
        refused(CodecError.CborUnionEntries, wire(0xa0))
        refused(CodecError.CborUnionEntries, wire(0xa2))
        // A name no case declares.
        refused(CodecError.CborUnknownVariant, wire(0xa1, 0x64, 0x63, 0x6f, 0x69, 0x6e, 0xa0))
        // A key that is not a text, and an item that is no map.
        refused(CodecError.CborWrongType, wire(0xa1, 0x01, 0xa0))
        refused(CodecError.CborWrongType, wire(0x80))
        // An indefinite union map.
        refused(CodecError.CborMalformed, wire(0xbf, 0x64, 0x63, 0x61, 0x73, 0x68, 0xa0, 0xff))
    }
}
