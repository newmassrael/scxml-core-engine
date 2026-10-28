// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// docs/adr/0055, section 6 and step 7 — a record's field a revision widened from
// `T` to `optional<T>` (`sce:optional-since`). A state saved at a revision before
// the widening holds the field as a value of `T`, so a `null` in it is no absence:
// no build wrote it, and the restore refuses it, naming the field and the
// revision. Saved at or after the widening, `null` is absent as it always is.
//
// The runtime's `SavedState.upgrade` is driven with the shape a generated machine
// holds for a record whose third revision widened two fields: `level`, a field of
// the first revision, and `urgent`, which the second revision added.

package com.sce.integration

import com.sce.runtime.AddedField
import com.sce.runtime.Json
import com.sce.runtime.RecordHolder
import com.sce.runtime.RecordLineage
import com.sce.runtime.SavedShape
import com.sce.runtime.SavedState
import com.sce.runtime.StateRefusal
import com.sce.runtime.WidenedField
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertNull
import kotlin.test.assertTrue

class ANullSavedBeforeAFieldWasWidenedIsRefusedTest {

    private val firstShape = "a".repeat(64)
    private val revisedShape = "b".repeat(64)
    private val second = "2".repeat(64)
    private val third = "3".repeat(64)

    private val shape = SavedShape(
        first = firstShape,
        revised = revisedShape,
        format2First = firstShape,
        format2Revised = revisedShape,
        records = listOf(
            RecordLineage(
                event = "reading.taken",
                revision = 3,
                revisions = listOf(second, third),
                added = listOf(AddedField("urgent", 2, "true")),
                widened = listOf(
                    WidenedField("level", 1, 3),
                    WidenedField("urgent", 2, 3),
                ),
                holders = listOf(
                    RecordHolder("last", false),
                    RecordHolder("seen", true),
                ),
            ),
        ),
    )

    /** A saved state holding `last` and `seen`, with the `revisions` member [revisions] (empty for none). */
    private fun state(digest: String, last: String, seen: String, revisions: String = ""): SavedState =
        SavedState.fromJson(
            """{"format":1,"shape":"$digest","configuration":["s"],"current":"s","variables":{"last":$last,"seen":$seen},"history":{},"pending":[],"invokes":[],"hostinvokes":[],"hostinvoketoken":"0","sendseq":"0","external":[]$revisions}""",
        )

    private fun named(revision: Int, fields: String): String =
        ""","revisions":{"reading.taken":{"revision":$revision,"fields":"$fields"}}"""

    private fun refusal(saved: SavedState): String =
        assertFailsWith<StateRefusal> { SavedState.upgrade(saved, shape) }.message ?: ""

    @Test
    fun aNullInAFieldOfTheFirstRevisionIsRefusedForARecordSavedBeforeTheWidening() {
        assertEquals(
            "'last.level' is null, which revision 1 of 'reading.taken' holds as a value: revision 3 widened it",
            refusal(state(firstShape, """{"level":null}""", "[]")),
        )
        assertTrue(
            refusal(state(firstShape, """{"level":4}""", """[{"level":1},{"level":null}]"""))
                .contains("'seen[1].level' is null"),
        )
    }

    @Test
    fun aNullInAFieldTheSecondRevisionAddedIsRefusedForARecordSavedAtTheSecond() {
        assertEquals(
            "'last.urgent' is null, which revision 2 of 'reading.taken' holds as a value: revision 3 widened it",
            refusal(state(revisedShape, """{"level":4,"urgent":null}""", "[]", named(2, second))),
        )
    }

    @Test
    fun aRecordSavedAtTheFirstRevisionHoldingTheFieldTheSecondAddedIsRefusedForThat() {
        assertTrue(
            refusal(state(firstShape, """{"level":4,"urgent":null}""", "[]"))
                .contains("'last' holds 'urgent'"),
        )
    }

    @Test
    fun aNullIsAbsentFromTheRevisionThatWidenedTheField() {
        val saved = state(
            revisedShape,
            """{"level":null,"urgent":null}""",
            """[{"level":null,"urgent":false}]""",
            named(3, third),
        )
        val upgraded = SavedState.upgrade(saved, shape)
        assertEquals(Json.write(saved.variables["last"]), Json.write(upgraded.variables["last"]))
        assertEquals(Json.write(saved.variables["seen"]), Json.write(upgraded.variables["seen"]))
    }

    @Test
    fun aRecordSavedBeforeTheWideningHoldingEachFieldPresentRestoresAsItWas() {
        val upgraded = SavedState.upgrade(
            state(firstShape, """{"level":4}""", """[{"level":1}]"""),
            shape,
        )
        assertEquals("""{"level":4,"urgent":true}""", Json.write(upgraded.variables["last"]))
        assertEquals("""[{"level":1,"urgent":true}]""", Json.write(upgraded.variables["seen"]))
    }

    @Test
    fun whatIsNoRecordWhereOneIsHeldIsLeftToTheVariablesOwnRead() {
        val upgraded = SavedState.upgrade(state(firstShape, "null", "[{}]"), shape)
        assertNull(upgraded.variables["last"])
    }
}
