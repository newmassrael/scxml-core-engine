// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A Kotlin machine reads a composed payload at any depth (docs/adr/0086,
// section 6), run with no script engine, as the Rust machine does
// (backends/rust/tests/tests/a_composed_payload_is_read_at_any_depth.rs). The
// committed machine under
// `src/main/kotlin/com/sce/integration/static_datamodel/static_composed_payload/`
// is generated from
// `sce-build/tests/fixtures/static_datamodel/static_composed_payload.scxml`
// (regen: `scripts/regen_static_datamodel_kotlin.sh`).
//
// A delivery whose every list is full is lifted into the machine's own types,
// and each read at depths 1 to 3 is what a host reads back. A delivery one
// past a bound at depth 2 is refused whole (section 4.2): none of the content
// runs, and `error.execution` is raised once. A value a host injects through
// the seam is held to every bound where it is made (section 5.4): within them
// it is read as any delivery, and one past a bound at any depth throws
// `BoundExceeded` from the `init` of the class that holds it, before anything
// is sent.

package com.sce.integration

import com.sce.integration.static_datamodel.static_composed_payload.StaticComposedPayloadItemRecord
import com.sce.integration.static_datamodel.static_composed_payload.StaticComposedPayloadSignalLevelEnum
import com.sce.integration.static_datamodel.static_composed_payload.StaticComposedPayloadStateMachine
import com.sce.runtime.BoundExceeded
import com.sce.runtime.EventMetadata
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertNotEquals
import org.junit.jupiter.api.Assertions.assertThrows
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

@DisplayName("AComposedPayloadIsReadAtAnyDepth — docs/adr/0086, section 6 (Kotlin AOT)")
class AComposedPayloadIsReadAtAnyDepthTest {

    /** One item of the batch: `tags` holds [tags] texts. */
    private fun item(id: Int, tags: Int): String {
        val words = (0 until tags).joinToString(",") { "\"tag$it\"" }
        return """{"id":$id,"amount":"9007199254740993","label":"abcdefgh","level":"high","due":null,"blob":"wxyz","tags":[$words]}"""
    }

    /** A delivery of `batch.sent` whose every list is full, the second item's `tags` holding [secondTags] texts. */
    private fun batch(secondTags: Int): String {
        val items = listOf(item(1, 4), item(2, secondTags), item(3, 4)).joinToString(",")
        return """{"items":[$items],"grid":[[1,2],[3,4]],"levels":["low","critical"],"maybe":[7,null],"spare":[1,2],"note":"abcd","late":[5,6]}"""
    }

    /** [body] run over a machine that took one delivery of `batch.sent` carrying [data]. */
    private fun delivered(data: String, body: (StaticComposedPayloadStateMachine) -> Unit) {
        val sm = StaticComposedPayloadStateMachine()
        sm.initialize()
        try {
            sm.sendEventByName("batch.sent", EventMetadata(data = data))
            sm.tick()
            body(sm)
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun aFullDeliveryIsReadAtEveryDepth() {
        delivered(batch(4)) { sm ->
            assertEquals(0u, sm.refusals, "a delivery of its type is no error")
            // Depth 1: the length of a list of the payload.
            assertEquals(3u, sm.itemCount)
            // Depth 2: a loop over each item's list, and a field of an element
            // read through its index.
            assertEquals(12u, sm.tagCount)
            assertEquals(12u, sm.wordCount)
            assertEquals(6u, sm.idSum)
            // A list of lists, walked at both depths and read by two indices.
            assertEquals(10u, sm.cellSum)
            assertEquals(3.toUByte(), sm.cell)
            // Depth 3: the length of the first item's `tags`.
            assertEquals(4u, sm.firstTags)
            // An element that may be absent, present.
            assertEquals(7.toUShort(), sm.firstMaybe)
            // A list taken whole into a list of a larger bound.
            assertEquals(listOf(5.toUByte(), 6.toUByte()), sm.late)
        }
    }

    /** An item a host builds, its `tags` holding [tags] texts. */
    private fun hostItem(id: UInt, tags: Int): StaticComposedPayloadItemRecord =
        StaticComposedPayloadItemRecord(
            id = id,
            amount = 2L,
            label = "abc",
            level = StaticComposedPayloadSignalLevelEnum.entries.first(),
            due = null,
            blob = "wx".toByteArray(),
            tags = List(tags) { "t$it" },
        )

    /** [body] run over a machine whose host injected `batch.sent` with [late] and [items] through its seam. */
    private fun injected(
        late: List<UByte>,
        items: List<StaticComposedPayloadItemRecord>,
        body: (StaticComposedPayloadStateMachine) -> Unit,
    ) {
        val sm = StaticComposedPayloadStateMachine()
        sm.initialize()
        try {
            sm.raiseBatchSent(
                items,
                listOf(listOf<UByte>(1u, 2u)),
                listOf(StaticComposedPayloadSignalLevelEnum.entries.first()),
                listOf<UShort?>(9u, null),
                null,
                "ab",
                late,
            )
            sm.tick()
            body(sm)
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun aValueAHostInjectsWithinItsBoundsIsReadAsAnyDelivery() {
        injected(listOf<UByte>(5u, 6u), listOf(hostItem(4u, 2))) { sm ->
            assertEquals(0u, sm.refusals)
            assertEquals(1u, sm.itemCount)
            assertEquals(2u, sm.tagCount)
            assertEquals(4u, sm.idSum)
            assertEquals(3u, sm.cellSum)
            assertEquals(9.toUShort(), sm.firstMaybe)
            assertEquals(listOf(5.toUByte(), 6.toUByte()), sm.late)
        }
    }

    @Test
    fun aListAHostInjectsOnePastItsBoundIsRefusedWhereItIsMade() {
        // `late` is a list<uint8>[2]: three elements are no value of its type,
        // and the seam's payload refuses them before anything is sent.
        val refusal = assertThrows(BoundExceeded::class.java) {
            injected(listOf<UByte>(5u, 6u, 7u), listOf(hostItem(4u, 2))) { }
        }
        assertEquals("'late' holds 3 elements, past the 2 its type declares", refusal.message)
    }

    @Test
    fun aRecordAHostBuildsOnePastAnInnerBoundIsRefusedWhereItIsMade() {
        // `tags` is a list<string[8]>[4] inside each item: an item of five is
        // refused where the host builds it, and a tag of nine bytes too.
        val many = assertThrows(BoundExceeded::class.java) { hostItem(1u, 5) }
        assertEquals("'tags' holds 5 elements, past the 4 its type declares", many.message)
        val long = assertThrows(BoundExceeded::class.java) {
            hostItem(1u, 1).copy(tags = listOf("ninebytes"))
        }
        assertEquals("'tags/0' is 9 bytes, past the 8 its type declares", long.message)
    }

    @Test
    fun aListPastItsBoundAtDepthTwoIsRefusedWhole() {
        delivered(batch(5)) { sm ->
            assertEquals(1u, sm.refusals, "error.execution reached the document once")
            assertEquals(0u, sm.itemCount, "none of the content ran")
            assertEquals(emptyList<UByte>(), sm.late, "nothing of the delivery was kept")
        }
    }

    /**
     * Every bound of the schema is held where the delivery is read (docs/adr/0086,
     * section 4.4): a text and a byte string inside an element, the top-level text,
     * and the count of the top-level list, each one past its bound in a delivery
     * otherwise full, are refused whole.
     */
    @Test
    fun everyBoundOfTheSchemaIsHeldWhereTheDeliveryIsRead() {
        val full = batch(4)
        val cases = listOf(
            full.replaceFirst("\"label\":\"abcdefgh\"", "\"label\":\"abcdefghi\""),
            full.replaceFirst("\"blob\":\"wxyz\"", "\"blob\":\"vwxyz\""),
            full.replaceFirst("\"note\":\"abcd\"", "\"note\":\"abcde\""),
            full.replaceFirst("\"items\":[", "\"items\":[${item(0, 1)},"),
        )
        for (delivery in cases) {
            assertNotEquals(full, delivery, "the case changes the delivery")
            delivered(delivery) { sm ->
                assertEquals(1u, sm.refusals, "error.execution reached the document once: $delivery")
                assertEquals(0u, sm.itemCount, "none of the content ran: $delivery")
                assertEquals(emptyList<UByte>(), sm.late, "nothing of the delivery was kept: $delivery")
            }
        }
    }
}
