// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package com.sce.forge.runtime

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

/**
 * The wire of a `vle_u<N>` field is a ZInt64 at every width, and the type
 * bounds the value: a narrow field reads every encoding of a value it holds,
 * the padded ones included, and refuses a value past its width.
 */
class VleTest {

    private fun wire(vararg bytes: Int): ByteArray = ByteArray(bytes.size) { bytes[it].toByte() }

    @Test
    fun aU16ReadsEachEncodingOfAValueItHoldsAlike() {
        val cases = listOf(
            wire(0x81, 0x80, 0x80, 0x00) to 1,
            wire(0x81, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x00) to 1,
            wire(0xFF, 0x7F) to 0x3FFF,
            wire(0xFF, 0xFF, 0x80, 0x00) to 0x3FFF,
            wire(0x80, 0x80, 0x01) to 0x4000,
            wire(0x80, 0x80, 0x81, 0x80, 0x00) to 0x4000,
            wire(0xFF, 0xFF, 0x03) to 0xFFFF,
            wire(0xFF, 0xFF, 0x83, 0x80, 0x80, 0x80, 0x80, 0x80, 0x00) to 0xFFFF,
        )
        for ((bytes, value) in cases) {
            val cursor = SceCursor(bytes)
            assertEquals(value.toUShort(), cursor.readVleU16(), bytes.joinToString())
            assertEquals(0, cursor.remaining(), "${bytes.joinToString()} left bytes unread")
        }
    }

    @Test
    fun aU32AndAU64ReadAPaddedEncodingOfAValueTheyHold() {
        val nine = wire(0x81, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x00)
        assertEquals(1u, SceCursor(nine).readVleU32())
        assertEquals(1uL, SceCursor(nine).readVleU64())
        assertEquals(
            UInt.MAX_VALUE,
            SceCursor(wire(0xFF, 0xFF, 0xFF, 0xFF, 0x8F, 0x80, 0x80, 0x80, 0x00)).readVleU32(),
        )
        assertEquals(
            ULong.MAX_VALUE,
            SceCursor(wire(0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF)).readVleU64(),
        )
    }

    @Test
    fun aNarrowFieldRefusesTheFirstValuePastItsWidth() {
        val u16 = SceCursor(wire(0x80, 0x80, 0x04))
        assertNull(u16.readVleU16())
        assertTrue(u16.lastVleOverflow, "65536 is past a u16")
        val u32 = SceCursor(wire(0x80, 0x80, 0x80, 0x80, 0x10))
        assertNull(u32.readVleU32())
        assertTrue(u32.lastVleOverflow, "2^32 is past a u32")
        // A ninth byte with its high bit set is data, not a flag asking for a tenth.
        val high = wire(0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80)
        val highU32 = SceCursor(high)
        assertNull(highU32.readVleU32())
        assertTrue(highU32.lastVleOverflow, "2^63 is past a u32")
        assertEquals(1uL shl 63, SceCursor(high).readVleU64())
    }

    @Test
    fun aChainTheBufferEndsInsideNeedsMoreBytes() {
        val short = wire(0x81, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80)
        val u16 = SceCursor(short)
        assertNull(u16.readVleU16())
        assertFalse(u16.lastVleOverflow, "a short chain is not an overflow")
        val u64 = SceCursor(short)
        assertNull(u64.readVleU64())
        assertFalse(u64.lastVleOverflow, "a short chain is not an overflow")
    }
}
