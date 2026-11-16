// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Accepted Subset §2.15, "Child sessions", and §scxml-6.4 (docs/adr/0098)
// on the Kotlin channel: a `datamodel="sce-static"` child declares its
// variables with the values its invoke handed, each in place of its own initial
// value, so a variable declared after a handed one reads the handed value.
//
// `worker` is handed `start = 7` and declares `copied = start` after it; it
// ends the moment `copied` is 7. A child that wrote the handed value over
// `start` after declaring both holds `copied = 0`, never ends, and leaves the
// parent in `working`.
//
// Fixture: tests/integration/a_static_child_declares_its_data_with_a_handed_value.scxml
//
// Regeneration (after fixture or template edit):
//   scripts/materialize.sh kotlin-integration

package com.sce.integration

import com.sce.integration.a_static_child_declares_its_data_with_a_handed_value.AStaticChildDeclaresItsDataWithAHandedValueState
import com.sce.integration.a_static_child_declares_its_data_with_a_handed_value.AStaticChildDeclaresItsDataWithAHandedValueStateMachine
import com.sce.runtime.ManualClock
import kotlin.test.Test
import kotlin.test.assertEquals

class AStaticChildDeclaresItsDataWithAHandedValueTest {

    @Test
    fun aVariableDeclaredAfterAHandedOneReadsTheHandedValue() {
        val sm = AStaticChildDeclaresItsDataWithAHandedValueStateMachine()
        sm.clock = ManualClock(0)
        try {
            sm.initialize()
            // Let the child start, run and end, and its parent take `done.invoke`.
            repeat(5) { sm.tick() }
            assertEquals(
                AStaticChildDeclaresItsDataWithAHandedValueState.Passed,
                sm.terminalState,
                "`worker` declares `copied = start` after `start`, which its invoke handed 7, " +
                    "so it ends; the parent stands in ${sm.snapshot.value.configuration}",
            )
        } finally {
            sm.cleanup()
        }
    }
}
