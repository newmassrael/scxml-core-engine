// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A line holding a list of values (docs/adr/0014, SCE_FORGE.md §4.6.4): how the
// runtime cuts it, the first failing part it names, and what it will not write.
// The same properties the Rust, Python and Go runtimes' own tests pin; what a
// generated codec does with them is the conformance harness's.

package com.sce.forge.runtime

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertNull

class ContentLineListTest {
    private fun values(value: String, separator: Char, maxValues: Int, maxSize: Int, text: Boolean): List<String>? {
        val bytes = "BEGIN:VEVENT\r\nV:$value\r\nEND:VEVENT\r\n".encodeToByteArray()
        val reader = assertNotNull(ContentLineReader.begin(bytes, "VEVENT"))
        val line = assertNotNull(reader.next())
        return line.readStrings(separator, maxValues, maxSize, text)
    }

    private fun refused(value: String, why: CodecError) {
        assertNull(values(value, ',', 3, 8, true), value)
        assertEquals(why, ContentLine.lastError, value)
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
    fun aListIsCutAtTheSeparatorBeforeItIsUnescaped() {
        assertEquals(listOf("a", "b", "c"), values("a,b,c", ',', 4, 8, false))
        assertEquals(listOf("a,b", "c"), values("a\\,b,c", ',', 4, 8, true))
        // Without TEXT there is no escape: a backslash is a byte and every separator cuts.
        assertEquals(listOf("a\\", "b"), values("a\\,b", ',', 4, 8, false))
        assertEquals(listOf("a;b", "c"), values("a;b,c", ',', 4, 8, true))
        assertEquals(listOf("1", "2"), values("1;2", ';', 2, 8, false))
        assertEquals(listOf("a", "bc"), values("a,b\r\n c", ',', 4, 8, false))
    }

    @Test
    fun aListIsRefusedAtItsFirstFailingPart() {
        refused("", CodecError.LineBadValue)
        refused("a,,b", CodecError.LineBadValue)
        refused(",a", CodecError.LineBadValue)
        refused("a,", CodecError.LineBadValue)
        refused("a,b,c,d", CodecError.LineTooMany)
        // A part past the bound is too many, whatever the part is.
        refused("a,b,c,", CodecError.LineTooMany)
        refused("a,b,c,xxxxxxxxxxxxxxx", CodecError.LineTooMany)
        refused("xxxxxxxxxxxxxxx,,", CodecError.LineTooLong)
        refused(",,xxxxxxxxxxxxxxx", CodecError.LineBadValue)
        refused("a,b\\x", CodecError.LineBadEscape)
        refused("a,b\\", CodecError.LineBadEscape)
    }

    @Test
    fun aListIsWrittenWithItsSeparatorAndHeldBeforeItIsWritten() {
        val text = written { w ->
            assertNull(w.property("C"))
            assertNull(w.strings(listOf("a,b", "c;d", "e\\f"), ',', true, 8, 4))
            assertNull(w.property("G"))
            assertNull(w.strings(listOf("1", "2"), ';', false, 8, 2))
        }
        assertEquals("BEGIN:VEVENT\r\nC:a\\,b,c\\;d,e\\\\f\r\nG:1;2\r\nEND:VEVENT\r\n", text)

        val out = mutableListOf<Byte>()
        val w = ContentLineWriter(MutableListSink(out), "VEVENT")
        assertNull(w.begin())
        assertNull(w.property("C"))
        val before = out.size
        assertEquals(CodecError.LineRequiredMissing, w.strings(emptyList(), ',', false, 8, 4))
        assertEquals(CodecError.LineTooMany, w.strings(listOf("a", "b", "c"), ',', false, 8, 2))
        assertEquals(CodecError.LineTooLong, w.strings(listOf("a", "abcdefghi"), ',', false, 8, 4))
        assertEquals(CodecError.LineBadValue, w.strings(listOf("a", ""), ',', false, 8, 4))
        // Not a TEXT, so a separator in a value could not be told from a cut.
        assertEquals(CodecError.LineBadValue, w.strings(listOf("a,b"), ',', false, 8, 4))
        assertEquals(CodecError.LineBadValue, w.strings(listOf("a\nb"), ',', false, 8, 4))
        assertEquals(before, out.size, "something of a refused list reached the sink")
    }

    @Test
    fun aTextListIsReadBackAsWritten() {
        for (list in listOf(listOf("a"), listOf("a,b", "c"), listOf("x".repeat(8), "é"), listOf("a;b"))) {
            val text = written { w ->
                assertNull(w.property("V"))
                assertNull(w.strings(list, ',', true, 8, 4))
            }
            val reader = assertNotNull(ContentLineReader.begin(text.encodeToByteArray(), "VEVENT"))
            val line = assertNotNull(reader.next())
            assertEquals<List<String>?>(list, line.readStrings(',', 4, 8, true))
        }
    }
}
