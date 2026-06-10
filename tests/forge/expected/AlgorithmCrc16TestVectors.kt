// SCE Forge: Auto-generated test-vector sidecar (RFC §5.B B2)
// Companion to AlgorithmCrc16.kt — do not edit; regenerate from the source SCXML.

package com.sce.generated.algorithm_crc16

import kotlin.test.Test
import kotlin.test.assertEquals

class AlgorithmCrc16TestVectors {
    @Test
    fun testVectorAlgorithmCrc16L47() {
        val actual: UShort = algorithmCrc16(byteArrayOf(0x31.toByte(), 0x32.toByte(), 0x33.toByte(), 0x34.toByte(), 0x35.toByte(), 0x36.toByte(), 0x37.toByte(), 0x38.toByte(), 0x39.toByte()))
        val expected: UShort = 0x29b1.toUShort()
        assertEquals(
            expected, actual,
            "<sce:test-vector> at SCXML L47: algorithmCrc16(<313233343536373839>) returned $actual, expected $expected"
        )
    }
}
