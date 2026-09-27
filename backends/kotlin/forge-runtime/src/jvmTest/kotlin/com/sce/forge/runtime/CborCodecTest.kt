// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE_FORGE.md §4.6.1 — a `sce:encoding="cbor"` codec, generated for Kotlin
// from tests/forge/resources/codec_cbor_map.scxml and held to the SAME bytes
// the Rust suite holds its codec to (backends/rust/forge-conformance/tests/
// forge_cbor_codec.rs): the shortest heads, the entries present in ascending
// key order, keys read in any order, an unknown key skipped whole, and every
// refusal — each leaving the cursor where it was.

package com.sce.forge.runtime

import com.sce.generated.codec_cbor_map.CodecCborMap
import com.sce.generated.envelope.Envelope
import com.sce.generated.pattern_kind.PatternKind
import com.sce.generated.payload_codec.PayloadCodec
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertNull

class CborCodecTest {
    private val id = ByteArray(16) { it.toByte() }

    private fun value() = CodecCborMap(
        source = "node-a",
        id = id,
        pattern = 3.toUByte(),
        payload = byteArrayOf(0xde.toByte(), 0xad.toByte()),
        deadline = null,
        urgent = true,
    )

    private fun bytes(vararg b: Int) = ByteArray(b.size) { b[it].toByte() }

    /** `{0: h'000102…0f', 1: "node-a", 3: 3, 5: h'dead', 7: true}`. */
    private fun expected(): ByteArray =
        bytes(0xa5, 0x00, 0x50) + id + bytes(0x01, 0x66) + "node-a".encodeToByteArray() +
            bytes(0x03, 0x03, 0x05, 0x42, 0xde, 0xad, 0x07, 0xf5)

    private fun assertSame(want: CodecCborMap, got: CodecCborMap?) {
        assertNotNull(got)
        assertEquals(want.source, got.source)
        assertContentEquals(want.id, got.id)
        assertEquals(want.pattern, got.pattern)
        assertContentEquals(want.payload, got.payload)
        assertEquals(want.deadline, got.deadline)
        assertEquals(want.urgent, got.urgent)
    }

    @Test
    fun aValueIsWrittenInItsOneDeterministicEncoding() {
        assertContentEquals(expected(), value().encodeToByteArray())
    }

    @Test
    fun anOptionalEntryIsWrittenOnlyWhenPresentAndInKeyOrder() {
        val v = value().copy(deadline = 1UL shl 32)
        val want = expected()
        want[0] = 0xa6.toByte()
        assertContentEquals(
            want + bytes(0x0c, 0x1b, 0, 0, 0, 1, 0, 0, 0, 0),
            v.encodeToByteArray(),
        )
    }

    @Test
    fun whatWasWrittenIsReadBack() {
        val cursor = SceCursor(expected())
        assertSame(value(), CodecCborMap.decode(cursor))
        assertEquals(0, cursor.remaining())
    }

    @Test
    fun keysAreReadInAnyOrderAndAnUnknownOneIsSkippedWhole() {
        // {7: true, 20: [1, {2: h'00'}], 5: h'dead', 3: 3, 1: "node-a", 0: id}
        val input = bytes(0xa6, 0x07, 0xf5, 0x14, 0x82, 0x01, 0xa1, 0x02, 0x41, 0x00) +
            bytes(0x05, 0x42, 0xde, 0xad, 0x03, 0x03, 0x01, 0x66) + "node-a".encodeToByteArray() +
            bytes(0x00, 0x50) + id
        assertSame(value(), CodecCborMap.decode(SceCursor(input)))
    }

    @Test
    fun aHeadLongerThanItNeedsIsStillRead() {
        val input = expected().toMutableList()
        val at = (0 until input.size - 1).first { input[it] == 0x03.toByte() && input[it + 1] == 0x03.toByte() }
        input[at + 1] = 0x18.toByte()
        input.add(at + 2, 0x03.toByte())
        assertSame(value(), CodecCborMap.decode(SceCursor(input.toByteArray())))
    }

    @Test
    fun everyRefusalLeavesTheCursorWhereItWas() {
        fun refused(input: ByteArray) {
            val cursor = SceCursor(input)
            assertNull(CodecCborMap.decode(cursor), input.joinToString(" ") { "%02x".format(it) })
            assertEquals(input.size, cursor.remaining(), "the cursor moved")
        }
        val e = expected()
        val pair = (0 until e.size - 1).first { e[it] == 0x03.toByte() && e[it + 1] == 0x03.toByte() }

        // A required key absent.
        refused(bytes(0xa4) + e.copyOfRange(1, pair) + e.copyOfRange(pair + 2, e.size))
        // The id one byte short of its exact length.
        refused(bytes(0xa5, 0x00, 0x4f) + id.copyOfRange(0, 15) + e.copyOfRange(19, e.size))
        // A key given twice.
        refused(bytes(0xa6) + e.copyOfRange(1, e.size) + bytes(0x07, 0xf4))
        // `pattern` holding 300, past a UByte.
        refused(e.copyOfRange(0, pair + 1) + bytes(0x19, 0x01, 0x2c) + e.copyOfRange(pair + 2, e.size))
        // A text where the map expected a truth value.
        refused(e.copyOfRange(0, e.size - 1) + bytes(0x61, 'x'.code))
        // An indefinite-length map.
        refused(bytes(0xbf, 0xff))
    }

    /**
     * The C++ codec's `GoldenBytesForFixedEnvelope`
     * (tests/mesh/MeshEnvelopeCodecTest.cpp), read and written by the codec
     * generated from `sce:std/mesh/envelope.scxml` — the vector the Rust
     * suite holds its envelope to as well.
     */
    @Test
    fun theMeshEnvelopeWritesTheBytesTheCppCodecWrites() {
        val sampleId = bytes(
            0x01, 0x82, 0xb1, 0x4d, 0xa3, 0x5c, 0x70, 0x12, 0xb4, 0xde, 0xf0, 0x42, 0x9a, 0x88, 0x77, 0x66,
        )
        val envelope = Envelope(
            id = sampleId,
            source = "ecu",
            event_type = "evt",
            pattern = PatternKind.FIRE_FORGET,
            datacontenttype = PayloadCodec.JSON,
            data = bytes(0xAA, 0xBB),
        )
        val golden = bytes(0xA6, 0x00, 0x50) + sampleId +
            bytes(0x01, 0x63, 'e'.code, 'c'.code, 'u'.code) +
            bytes(0x02, 0x63, 'e'.code, 'v'.code, 't'.code) +
            bytes(0x03, 0x01, 0x04, 0x01, 0x05, 0x42, 0xAA, 0xBB)

        assertContentEquals(golden, envelope.encodeToByteArray())
        val decoded = Envelope.decode(SceCursor(golden))
        assertNotNull(decoded)
        assertContentEquals(sampleId, decoded.id)
        assertEquals("ecu", decoded.source)
        assertEquals("evt", decoded.event_type)
        assertEquals(PatternKind.FIRE_FORGET, decoded.pattern)
        assertEquals(PayloadCodec.JSON, decoded.datacontenttype)
        assertContentEquals(bytes(0xAA, 0xBB), decoded.data)
        assertNull(decoded.deadline_unix_ms)

        // 11 is in the reserved Stream range, not a value of the closed set.
        val reserved = golden.copyOf()
        val at = (0 until reserved.size - 1).first { reserved[it] == 0x03.toByte() && reserved[it + 1] == 0x01.toByte() }
        reserved[at + 1] = 0x0b
        assertNull(Envelope.decode(SceCursor(reserved)))
    }

    @Test
    fun aValueThatBreaksItsBoundIsNotWritten() {
        val sink = MutableListSink(mutableListOf())
        assertEquals(CodecError.CborWrongLength, value().copy(id = id.copyOfRange(0, 15)).encode(sink))
        assertEquals(
            CodecError.CborOutOfRange,
            value().copy(source = "x".repeat(65)).encode(MutableListSink(mutableListOf())),
        )
    }
}
