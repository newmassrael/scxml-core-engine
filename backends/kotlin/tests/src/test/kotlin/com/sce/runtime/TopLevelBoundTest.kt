// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A payload's own text and byte-string fields are held to their bound where the
// delivery is read, as a text and a byte string are at any depth
// (docs/adr/0086, section 4.4): `Fields.string(name, bound)` and
// `Fields.bytes(name, bound)` are the readers a generated lift of a schema of
// entries calls, and each refuses one past the bound with the code, pointer and
// bound of the value reader's own refusal.

package com.sce.runtime

import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith

class TopLevelBoundTest {

    @Test
    fun aTopLevelTextIsHeldToItsBoundWhereItIsRead() {
        val fields = EventPayload.decode("{\"note\": \"abcde\", \"wide\": \"éé\"}")
        val refusal = assertFailsWith<EventPayload.Refusal> { fields.string("note", 4) }
        assertEquals("'note' is 5 bytes, past the 4 its type declares", refusal.message)
        assertEquals(EventPayload.RefusalCode.TOO_LONG, refusal.code)
        assertEquals("/note", refusal.pointer)
        assertEquals(4L, refusal.bound)
        assertEquals("abcde", fields.string("note", 5))
        // The bound counts UTF-8 bytes: two characters, four bytes.
        assertEquals(
            EventPayload.RefusalCode.TOO_LONG,
            assertFailsWith<EventPayload.Refusal> { fields.string("wide", 3) }.code,
        )
        assertEquals("éé", fields.string("wide", 4))
    }

    @Test
    fun aTopLevelByteStringIsHeldToItsBoundWhereItIsRead() {
        val fields = EventPayload.decode("{\"blob\": \"abcde\"}")
        val refusal = assertFailsWith<EventPayload.Refusal> { fields.bytes("blob", 4) }
        assertEquals("'blob' is 5 bytes, past the 4 its type declares", refusal.message)
        assertEquals(EventPayload.RefusalCode.TOO_LONG, refusal.code)
        assertEquals("/blob", refusal.pointer)
        assertEquals(4L, refusal.bound)
        assertContentEquals("abcde".encodeToByteArray(), fields.bytes("blob", 5))
    }
}
