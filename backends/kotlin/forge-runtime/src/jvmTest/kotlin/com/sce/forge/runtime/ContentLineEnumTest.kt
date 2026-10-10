// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// An enum read and written by the text of its variants (docs/adr/0015,
// SCE_FORGE.md §4.6.4): how the runtime matches a text, what it refuses, and what
// it writes. The same properties the Rust, Python and Go runtimes' own tests pin;
// what a generated codec does with them is the conformance harness's.

package com.sce.forge.runtime

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertNull

class ContentLineEnumTest {
    /** A closed vocabulary the way a generated codec holds it: the carriers neither start at zero nor run on. */
    private val status = EnumTexts(
        listOf(
            EnumText("NEEDS-ACTION", 1UL),
            EnumText("IN-PROCESS", 2UL),
            EnumText("COMPLETED", 4UL),
            EnumText("cancelled", 8UL),
        ),
    )

    private fun enumValue(value: String): ULong? {
        val bytes = "BEGIN:VEVENT\r\nS:$value\r\nEND:VEVENT\r\n".encodeToByteArray()
        val reader = assertNotNull(ContentLineReader.begin(bytes, "VEVENT"))
        return assertNotNull(reader.next()).readEnum(status)
    }

    private fun enumParam(line: String): ULong? {
        val bytes = "BEGIN:VEVENT\r\n$line\r\nEND:VEVENT\r\n".encodeToByteArray()
        val reader = assertNotNull(ContentLineReader.begin(bytes, "VEVENT"))
        val property = assertNotNull(reader.next())
        assertEquals(true, property.nextParam())
        return property.readParamEnum(status)
    }

    private fun refusedValue(value: String) {
        assertNull(enumValue(value), value)
        assertEquals(CodecError.LineBadValue, ContentLine.lastError, value)
    }

    private fun written(write: (ContentLineWriter) -> Unit): String {
        val out = mutableListOf<Byte>()
        val w = ContentLineWriter(MutableListSink(out), "VEVENT")
        assertNull(w.begin())
        write(w)
        assertNull(w.finish())
        return out.toByteArray().decodeToString()
    }

    @Test
    fun aTextIsMatchedAsciiCaseInsensitively() {
        assertEquals(1UL, enumValue("NEEDS-ACTION"))
        assertEquals(1UL, enumValue("needs-action"))
        assertEquals(2UL, enumValue("In-Process"))
        assertEquals(4UL, enumValue("completed"))
        // A variant with no text of its own is read by its declared name, in any case.
        assertEquals(8UL, enumValue("CANCELLED"))
        assertEquals(8UL, enumValue("Cancelled"))
    }

    @Test
    fun aValueNoVariantNamesIsABadValue() {
        for (value in listOf(
            "",
            "DONE",
            "NEEDS",
            "NEEDS-ACTION-",
            "NEEDS-ACTION-NEEDS-ACTION-NEEDS-ACTION",
            " IN-PROCESS",
            "IN-PROCESS ",
            // The declared name of a variant that has a text is not its text.
            "needsAction",
            // A value is no TEXT, so a backslash in it is not an escape.
            "IN\\-PROCESS",
            "IN-\u0007PROCESS",
            // Only the 26 letters fold: U+017F and U+0131 are not S and I.
            "IN-PROCEſſ",
            "ıN-PROCESS",
        )) {
            refusedValue(value)
        }
    }

    @Test
    fun aTextCutByAFoldIsReadWhole() {
        val bytes = "BEGIN:VEVENT\r\nS:IN-PRO\r\n CESS\r\nEND:VEVENT\r\n".encodeToByteArray()
        val reader = assertNotNull(ContentLineReader.begin(bytes, "VEVENT"))
        assertEquals(2UL, assertNotNull(reader.next()).readEnum(status))
    }

    @Test
    fun aParameterThatIsAnEnumIsMatchedQuotedOrNot() {
        assertEquals(4UL, enumParam("S;ROLE=completed:x"))
        assertEquals(1UL, enumParam("S;ROLE=\"Needs-Action\":x"))
        for (bad in listOf("S;ROLE=:x", "S;ROLE=BOSS:x", "S;ROLE=\"\":x", "S;ROLE=COMPLETED,COMPLETED:x")) {
            assertNull(enumParam(bad), bad)
            assertEquals(CodecError.LineBadValue, ContentLine.lastError, bad)
        }
        // The value is scanned whole before it is judged, so a quote that never closes
        // is malformed even though the text before it names no variant.
        assertNull(enumParam("S;ROLE=\"ZZZ:x"))
        assertEquals(CodecError.LineMalformed, ContentLine.lastError)
    }

    @Test
    fun anEnumIsWrittenAsTheTextItsEnumDeclares() {
        val text = written { w ->
            assertNull(w.property("S"))
            assertNull(w.enumParam("ROLE", status, 1UL))
            assertNull(w.enumParam("X-Q", status, 8UL))
            assertNull(w.enumValue(status, 2UL))
            assertNull(w.property("T"))
            assertNull(w.enumValue(status, 8UL))
        }
        assertEquals(
            "BEGIN:VEVENT\r\nS;ROLE=NEEDS-ACTION;X-Q=cancelled:IN-PROCESS\r\nT:cancelled\r\nEND:VEVENT\r\n",
            text,
        )
    }

    @Test
    fun aCarrierNoVariantDeclaresHasNoTextAndIsNotWritten() {
        val out = mutableListOf<Byte>()
        val w = ContentLineWriter(MutableListSink(out), "VEVENT")
        assertNull(w.begin())
        assertNull(w.property("S"))
        val before = out.size
        assertEquals(CodecError.LineBadValue, w.enumValue(status, 9UL))
        assertEquals(CodecError.LineBadValue, w.enumParam("ROLE", status, 9UL))
        assertEquals(before, out.size)
    }
}
