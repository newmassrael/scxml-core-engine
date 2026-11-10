// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A union in a saved state is one object, its tag under the tag's key as the
// declared name of its case, before the fields of the case's record
// (docs/adr/0092, section 6.1 item 7), and is read back by its tag first,
// wherever the tag stands, the fields beside it being the case's record's to
// read.

package com.sce.runtime

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith

class SavedUnionTest {

    private fun refusal(read: () -> Any?): String? = assertFailsWith<StateRefusal> { read() }.message

    @Test
    fun `a union is its tag before the fields of its case`() {
        val circle = SavedValues.union("kind", "circle", linkedMapOf<String, Any?>("radius" to SavedValues.of(7.toUShort())))
        assertEquals("{\"kind\":\"circle\",\"radius\":7}", Json.write(circle))
        assertEquals("{\"kind\":\"dot\"}", Json.write(SavedValues.union("kind", "dot", null)))
    }

    @Test
    fun `a saved union is read back by its tag wherever it stands`() {
        val circle = SavedValues.union("kind", "circle", linkedMapOf<String, Any?>("radius" to SavedValues.of(7.toUShort())))
        assertEquals("circle", SavedValues.unionTag(circle, "shape", "kind"))
        val tagLast = linkedMapOf<String, Any?>("radius" to Json.Number("7"), "kind" to "disc")
        assertEquals("disc", SavedValues.unionTag(tagLast, "shape", "kind"))
        assertEquals(7.toUShort(), SavedValues.uint16(SavedValues.field(tagLast, "shape", "radius"), "shape.radius"))
    }

    @Test
    fun `a saved union that is no object or has no text for its tag is refused`() {
        assertEquals("'shape' is not an object", refusal { SavedValues.unionTag(Json.Number("1"), "shape", "kind") })
        assertEquals(
            "'shape' has no field 'kind'",
            refusal { SavedValues.unionTag(linkedMapOf<String, Any?>("radius" to Json.Number("7")), "shape", "kind") },
        )
        assertEquals(
            "'shape.kind' is not a text",
            refusal { SavedValues.unionTag(linkedMapOf<String, Any?>("kind" to Json.Number("3")), "shape", "kind") },
        )
    }
}
