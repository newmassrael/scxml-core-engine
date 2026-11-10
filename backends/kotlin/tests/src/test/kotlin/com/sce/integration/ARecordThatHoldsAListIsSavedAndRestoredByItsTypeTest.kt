// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Accepted Subset §2.15, "Saving and restoring", for a variable of a record that
// holds a list of records (docs/adr/0087, "Landing order" item 8).
//
// `static_lift_list.scxml`: `text.given` lifts the text its payload carries into
// `answer`, an `optional<record:Plan>` of `lift_answer_list.scxml` whose `items` is a
// list of one to three records of `lift_item.scxml`. The scenario replayed by
// StaticScenarioTest reads what a saved state writes of it; this holds what a restore
// reads back: the record by its type at every depth, a list short of its lower count
// refused by name before the record is built, and a state saved with the record
// absent restored absent.

package com.sce.integration

import com.sce.integration.static_lift_list.StaticLiftListItemRecord
import com.sce.integration.static_lift_list.StaticLiftListPlanRecord
import com.sce.integration.static_lift_list.StaticLiftListStateMachine
import com.sce.runtime.EventMetadata
import com.sce.runtime.Json
import com.sce.runtime.SavedState
import com.sce.runtime.StateRefusal
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertNotEquals
import kotlin.test.assertNull
import kotlin.test.assertTrue

class ARecordThatHoldsAListIsSavedAndRestoredByItsTypeTest {

    /** The text of a plan of two items, which the machine lifts whole. */
    private val week =
        "{\"title\":\"Week\",\"items\":[{\"title\":\"Plan\",\"at\":9},{\"title\":\"Ship\",\"at\":17}],\"priority\":1}"

    /** What [week] is as the machine's record. */
    private val weekRecord = StaticLiftListPlanRecord(
        title = "Week",
        items = listOf(StaticLiftListItemRecord(title = "Plan", at = 9u), StaticLiftListItemRecord(title = "Ship", at = 17u)),
        priority = 1u,
    )

    /** [week]'s items as a saved state writes them. */
    private val weekItems = "\"items\":[{\"title\":\"Plan\",\"at\":9},{\"title\":\"Ship\",\"at\":17}]"

    /** A running machine that has lifted [text] once. */
    private fun lifted(text: String): StaticLiftListStateMachine {
        val sm = StaticLiftListStateMachine()
        sm.initialize()
        sm.sendEventByName("text.given", EventMetadata(data = "{\"text\":${Json.quote(text)}}"))
        sm.tick()
        return sm
    }

    @Test
    fun aRestoredRecordIsTheRecordThatWasSaved() {
        val first = lifted(week)
        val saved = try {
            assertEquals(weekRecord, first.snapshot.value.data.answer)
            assertNull(first.snapshot.value.data.why)
            first.save()
        } finally {
            first.cleanup()
        }
        val text = saved.toJson()
        assertTrue(text.contains(weekItems), text)
        val second = StaticLiftListStateMachine()
        try {
            second.restore(SavedState.fromJson(text))
            // What was restored is saved as it was read, at every depth of the record.
            assertEquals(text, second.save().toJson())
        } finally {
            second.cleanup()
        }
    }

    @Test
    fun aSavedListShortOfItsLowerCountIsRefusedByName() {
        val first = lifted(week)
        val whole = try {
            first.save().toJson()
        } finally {
            first.cleanup()
        }
        val empty = whole.replace(weekItems, "\"items\":[]")
        assertNotEquals(whole, empty, "the record's list is written")
        val second = StaticLiftListStateMachine()
        try {
            val refusal = assertFailsWith<StateRefusal> { second.restore(SavedState.fromJson(empty)) }
            val message = refusal.message ?: ""
            assertTrue(
                message.contains("'answer.items' holds 0 elements, fewer than the 1 its type declares"),
                message,
            )
            // A refused restore leaves the machine as it was: never started.
            assertFailsWith<StateRefusal> { second.save() }
        } finally {
            second.cleanup()
        }
    }

    @Test
    fun aRecordSavedAbsentIsRestoredAbsent() {
        val first = StaticLiftListStateMachine()
        val text = try {
            first.initialize()
            first.save().toJson()
        } finally {
            first.cleanup()
        }
        assertTrue(text.contains("\"answer\":null"), text)
        val second = StaticLiftListStateMachine()
        try {
            second.restore(SavedState.fromJson(text))
            assertEquals(text, second.save().toJson())
        } finally {
            second.cleanup()
        }
    }
}
