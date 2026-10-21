// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A field of `optional<T>` on the event wire (docs/adr/0055, section 4): a value
// of `T` is present, `null` is absent, a missing key of a first-revision field is
// refused, and a writer always writes the key, an absent value as `null`.

package com.sce.runtime

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertNull

class OptionalPayloadTest {

    @Test
    fun `a value of the field's type is present`() {
        val fields = EventPayload.decode("""{"due":12,"title":"soon","count":7}""")
        assertEquals(12L, fields.optional("due", fields::int64))
        assertEquals("soon", fields.optional("title", fields::string))
        assertEquals(7u, fields.optional("count", fields::uint32))
    }

    @Test
    fun `null is absent`() {
        val fields = EventPayload.decode("""{"due":null,"title":null}""")
        assertNull(fields.optional("due", fields::int64))
        assertNull(fields.optional("title", fields::string))
    }

    @Test
    fun `a delivery that lacks the key is refused`() {
        val fields = EventPayload.decode("""{"id":3,"title":"gone"}""")
        assertFailsWith<EventPayload.Refusal> { fields.optional("due", fields::int64) }
    }

    @Test
    fun `a value of another type is refused as a field of the type refuses it`() {
        val fields = EventPayload.decode("""{"due":"soon","count":-1}""")
        assertFailsWith<EventPayload.Refusal> { fields.optional("due", fields::int64) }
        assertFailsWith<EventPayload.Refusal> { fields.optional("count", fields::uint32) }
    }

    @Test
    fun `a default that is absent stands in for a field a later revision added`() {
        val fields = EventPayload.decode("""{"id":1}""", """{"due":null}""")
        assertNull(fields.optional("due", fields::int64))
    }

    @Test
    fun `an absent value is written as null and its key is kept`() {
        assertEquals(
            """{"id":1,"due":null,"title":"soon"}""",
            EventPayload.encode(mapOf("id" to 1, "due" to null, "title" to "soon")),
        )
    }
}
