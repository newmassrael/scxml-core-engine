// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE_FORGE.md §4.6.4 — a `sce:encoding="content-line"` codec, generated for
// Kotlin from tests/forge/resources/codec_content_line_event.scxml. The wire
// vectors are the numerical harness's, run from numerical_reference.json; what
// is held here is what a vector cannot say on Kotlin: that a refusal is a
// `Decoded.Refused` holding the rule that refused it, that a refused decode
// leaves the cursor where it was, and what encode answers for a value it cannot
// write.

package com.sce.forge.runtime

import com.sce.generated.codec_content_line_event.CodecContentLineEvent
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue

class ContentLineCodecTest {
    private fun value() = CodecContentLineEvent(
        uid = "evt-1",
        dtstart = "20261008T090000",
        dtstartTzid = "Asia/Seoul",
    )

    private val expected =
        "BEGIN:VEVENT\r\nUID:evt-1\r\nDTSTART;TZID=Asia/Seoul:20261008T090000\r\nEND:VEVENT\r\n"

    private fun written(v: CodecContentLineEvent): String? = v.encodeToByteArray()?.decodeToString()

    /** What encode answers for [v]: the refusal, or `null` when it is written. */
    private fun refusal(v: CodecContentLineEvent): CodecError? = v.encode(MutableListSink(mutableListOf()))

    private fun refused(text: String, why: CodecError) {
        val bytes = text.encodeToByteArray()
        val cursor = SceCursor(bytes)
        assertEquals(Decoded.Refused(why), CodecContentLineEvent.decode(cursor), text)
        assertEquals(bytes.size, cursor.remaining(), "the cursor moved: $text")
    }

    @Test
    fun aValueIsWrittenInItsOneFoldedForm() {
        assertEquals(expected, written(value()))
    }

    @Test
    fun aNewValueHoldsNothingOptional() {
        val v = CodecContentLineEvent()
        assertNull(v.summary)
        assertNull(v.organizer)
        assertNull(v.organizerCn)
        assertNull(v.sequence)
        assertNull(v.allDay)
        assertTrue(v.exdate.isEmpty())
        assertEquals("", v.uid)
    }

    @Test
    fun aDecodeStandsAfterTheComponentAndNoFurther() {
        val tail = "BEGIN:VEVENT\r\nUID:next\r\n"
        val cursor = SceCursor((expected + tail).encodeToByteArray())
        assertEquals(Decoded.Ok(value()), CodecContentLineEvent.decode(cursor))
        assertEquals(tail.length, cursor.remaining())
    }

    @Test
    fun everyRefusalNamesItsRuleAndLeavesTheCursorWhereItWas() {
        refused("", CodecError.NeedMoreBytes)
        refused("BEGIN:VEVENT\r\nUID:a\r\nEND:VEVENT\r\n", CodecError.LineRequiredMissing)
        refused(
            "BEGIN:VEVENT\r\nUID:a\r\nUID:b\r\nDTSTART;TZID=Z:1\r\nEND:VEVENT\r\n",
            CodecError.LineTooMany,
        )
        refused(
            "BEGIN:VEVENT\r\nUID:" + "x".repeat(25) + "\r\nDTSTART;TZID=Z:1\r\nEND:VEVENT\r\n",
            CodecError.LineTooLong,
        )
        refused(
            "BEGIN:VEVENT\r\nUID:a\r\nDTSTART;TZID=Z:1\r\nSUMMARY:a\\xb\r\nEND:VEVENT\r\n",
            CodecError.LineBadEscape,
        )
        refused(
            "BEGIN:VEVENT\r\nUID:a\r\nDTSTART;TZID=Z:1\r\nSEQUENCE:65536\r\nEND:VEVENT\r\n",
            CodecError.LineBadValue,
        )
        refused("BEGIN:VEVENT\r\nUID\r\nEND:VEVENT\r\n", CodecError.LineMalformed)
        refused(expected.dropLast(4), CodecError.NeedMoreBytes)
    }

    @Test
    fun aParameterWithNoValueOfItsPropertyIsRefusedOnEncode() {
        val v = value().copy(organizerCn = "Kim")
        assertEquals(CodecError.LineRequiredMissing, refusal(v))
        val ok = v.copy(organizer = "mailto:kim@example.org")
        assertTrue(written(ok)!!.contains("ORGANIZER;CN=Kim:mailto:kim@example.org\r\n"))
    }

    @Test
    fun aValueALineCouldNotCarryIsRefusedOnEncode() {
        assertEquals(CodecError.LineBadValue, refusal(value().copy(rrule = "FREQ=DAILY\r\nATTENDEE:mailto:x")))
        assertEquals(CodecError.LineBadValue, refusal(value().copy(dtstartTzid = "a\"b")))
        assertEquals(CodecError.LineTooLong, refusal(value().copy(uid = "x".repeat(25))))
        assertEquals(CodecError.LineTooMany, refusal(value().copy(exdate = mutableListOf("1", "2", "3", "4"))))
    }

    @Test
    fun whatIsWrittenIsReadBackWithEveryEntryPresent() {
        val v = value().copy(
            summary = "a;b,c\\d\ne",
            description = "é".repeat(70),
            organizer = "mailto:kim@example.org",
            organizerCn = "Kim, Lee",
            rrule = "FREQ=WEEKLY;BYDAY=MO,WE",
            exdate = mutableListOf("20261015T090000", "20261022T090000", "20261029T090000"),
            sequence = 65535.toUShort(),
            priority = Int.MIN_VALUE,
            allDay = true,
        )
        val bytes = assertNotNull(v.encodeToByteArray())
        val cursor = SceCursor(bytes)
        assertEquals(Decoded.Ok(v), CodecContentLineEvent.decode(cursor))
        assertEquals(0, cursor.remaining())
    }
}
