// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A Kotlin machine sends a composed value by its name (docs/adr/0086, section
// 5.2), run with no script engine, as the Rust machine does
// (backends/rust/tests/tests/a_composed_value_is_sent_by_its_name.rs). The
// committed machine under `src/main/kotlin/com/sce/integration/static_composed_send/`
// is generated from
// `sce-build/tests/fixtures/static_datamodel/static_composed_send.scxml`
// (regen: `scripts/regen_static_datamodel_kotlin.sh`).
//
// The first delivery of `batch.sent` is forwarded to the machine itself: four
// lists and an optional list named as the parts of the payload they are, and a
// list variable the machine took the payload's `late` into. The delivery that
// comes back is lifted as any delivery is (section 4), so it is refused whole
// if anything written is outside its type; it is not, and every value read
// from it at depths 1 to 3 is what the first delivery held.

package com.sce.integration

import com.sce.integration.static_composed_send.StaticComposedSendStateMachine
import com.sce.runtime.EventMetadata
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

@DisplayName("AComposedValueIsSentByItsName — docs/adr/0086, section 5.2 (Kotlin AOT)")
class AComposedValueIsSentByItsNameTest {

    /** One item of the batch: `tags` holds [tags] texts. */
    private fun item(id: Int, tags: Int): String {
        val words = (0 until tags).joinToString(",") { "\"tag$it\"" }
        return """{"id":$id,"amount":"9007199254740993","label":"abcdefgh","level":"high","due":null,"blob":"wxyz","tags":[$words]}"""
    }

    /** A delivery of `batch.sent` whose every list is full, an element of `maybe` absent and `spare` present. */
    private fun batch(): String {
        val items = listOf(item(1, 4), item(2, 3), item(3, 4)).joinToString(",")
        return """{"items":[$items],"grid":[[1,2],[3,4]],"levels":["low","critical"],"maybe":[7,null],"spare":[1,2],"note":"abcd","late":[5,6]}"""
    }

    @Test
    fun aDeliveryForwardedByItsNamesComesBackWhole() {
        val sm = StaticComposedSendStateMachine()
        sm.initialize()
        try {
            sm.sendEventByName("batch.sent", EventMetadata(data = batch()))
            sm.tick()
            assertEquals(
                1u,
                sm.refusals,
                "the delivery the machine sent itself fits its schema, and the one refusal is the list sent short of its field's lower count",
            )
            // A list variable sent to a field that states a lower count (docs/adr/0086, section
            // 15.6): sent empty, refused whole and never delivered; sent with its one element,
            // delivered.
            assertEquals(1u, sm.countedDeliveries)
            assertEquals(7u, sm.countedSum)
            assertEquals(2u, sm.deliveries, "the forwarded delivery came back on the internal queue")
            // Depth 1: the lengths of a list of records and a list of enum values.
            assertEquals(3u, sm.itemCount)
            assertEquals(2u, sm.levelCount)
            // Depth 2: each item's list, and a field of an element read by its index.
            assertEquals(11u, sm.tagCount)
            assertEquals(6u, sm.idSum)
            // A list of lists, walked at both depths.
            assertEquals(10u, sm.cellSum)
            // An element that may be absent, present.
            assertEquals(7.toUShort(), sm.firstMaybe)
            // The list variable, sent by its name.
            assertEquals(11u, sm.lateSum)
            assertEquals(listOf(5.toUByte(), 6.toUByte()), sm.late)
        } finally {
            sm.cleanup()
        }
    }
}
