// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A union value is read tag first, then the record of the case its tag names,
// from the same object, and written as one object with its tag among the fields
// of its case (docs/adr/0092, sections 2 and 4).

package com.sce.runtime

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith

class UnionPayloadTest {

    /** The tag of [Shape], read as a generated reader of an enum reads it. */
    private enum class ShapeKind { DOT, CIRCLE, DISC }

    /** A union over [ShapeKind] whose `circle` and `disc` both hold one record. */
    private sealed interface Shape {
        data object Dot : Shape

        data class Circle(val radius: UShort) : Shape

        data class Disc(val radius: UShort) : Shape
    }

    private fun readShape(data: String): Shape =
        EventPayload.decode(data).field("s").union("kind", { tag ->
            when (val name = tag.name()) {
                "dot" -> ShapeKind.DOT
                "circle" -> ShapeKind.CIRCLE
                "disc" -> ShapeKind.DISC
                else -> throw tag.unknownVariant(name, "ShapeKind")
            }
        }) { kind, objectValue ->
            when (kind) {
                ShapeKind.DOT -> Shape.Dot
                ShapeKind.CIRCLE -> objectValue.record { Shape.Circle(it.uint16("radius")) }
                ShapeKind.DISC -> objectValue.record { Shape.Disc(it.uint16("radius")) }
            }
        }

    /**
     * A union's tag is read first, wherever it stands, and names the case whose
     * record is read from the same object: two cases that hold one record stay
     * apart by their tag, a key or a `null` the case does not declare is stepped
     * over, and each refusal points at the tag or at the case's field.
     */
    @Test
    fun `a union reads its tag first and then the record of its case`() {
        assertEquals(Shape.Dot, readShape("{\"s\":{\"kind\":\"dot\"}}"))
        assertEquals(Shape.Disc(2u), readShape("{\"s\":{\"radius\":2,\"kind\":\"disc\"}}"))
        assertEquals(
            Shape.Circle(2u),
            readShape("{\"s\":{\"kind\":\"circle\",\"radius\":2,\"width\":null,\"text\":\"x\"}}"),
        )
        assertEquals(Shape.Dot, readShape("{\"s\":{\"kind\":\"dot\",\"radius\":null}}"))
        for ((data, want) in listOf(
            "{\"s\":[]}" to Pair(EventPayload.RefusalCode.WRONG_KIND, "/s"),
            "{\"s\":null}" to Pair(EventPayload.RefusalCode.NULL_NOT_OPTIONAL, "/s"),
            "{\"s\":{\"radius\":2}}" to Pair(EventPayload.RefusalCode.MISSING, "/s/kind"),
            "{\"s\":{\"kind\":null,\"radius\":null}}" to Pair(EventPayload.RefusalCode.NULL_NOT_OPTIONAL, "/s/kind"),
            "{\"s\":{\"kind\":3,\"radius\":\"x\"}}" to Pair(EventPayload.RefusalCode.WRONG_KIND, "/s/kind"),
            "{\"s\":{\"kind\":\"oval\",\"radius\":2}}" to Pair(EventPayload.RefusalCode.UNKNOWN_VARIANT, "/s/kind"),
            "{\"s\":{\"kind\":\"circle\",\"width\":2}}" to Pair(EventPayload.RefusalCode.MISSING, "/s/radius"),
            "{\"s\":{\"kind\":\"disc\",\"radius\":null}}" to
                Pair(EventPayload.RefusalCode.NULL_NOT_OPTIONAL, "/s/radius"),
        )) {
            val refusal = assertFailsWith<EventPayload.Refusal>(data) { readShape(data) }
            assertEquals(want, Pair(refusal.code, refusal.pointer), data)
        }
    }

    /** A union is written as one object, its tag among the fields of its case, every key ascending by its bytes. */
    @Test
    fun `a union is written with its tag among the fields of its case`() {
        assertEquals("{\"kind\":\"none\"}", EventPayload.unionJson("kind", "none", emptyMap()).json)
        assertEquals(
            "{\"count\":2,\"kind\":\"relativeWeeks\"}",
            EventPayload.unionJson("kind", "relativeWeeks", mapOf("count" to 2)).json,
        )
        assertEquals(
            "{\"day\":3,\"kind\":\"absolute\",\"month\":3,\"year\":2027}",
            EventPayload.unionJson("kind", "absolute", mapOf("year" to 2027, "month" to 3, "day" to 3)).json,
        )
    }
}
