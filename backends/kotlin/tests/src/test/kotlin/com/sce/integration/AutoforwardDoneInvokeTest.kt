// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4 autoforward carries `done.invoke.<id>` — Kotlin AOT path.
//
// Appendix D's `mainEventLoop` forwards every event it dequeues from the
// external queue to each `autoforward` child without testing the event's
// name; the sole exclusion is the cancel event, expressed as control flow.
// §6.4.2 places `done.invoke.<id>` on the external queue of the invoking
// session, so a sibling child that is still running must receive it.
//
// Fixture: integration_resources/autoforward_done_invoke/autoforward_done_invoke.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_autoforward_done_invoke_kotlin.sh

package com.sce.integration

import com.sce.integration.autoforward_done_invoke.AutoforwardDoneInvokeState
import com.sce.integration.autoforward_done_invoke.AutoforwardDoneInvokeStateMachine
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

/// W3C SCXML 6.4 — a sibling's `done.invoke` reaches the autoforward child (Kotlin AOT).
@DisplayName("AutoforwardDoneInvoke — W3C SCXML 6.4")
class AutoforwardDoneInvokeTest {

    @Test
    fun doneInvokeFromASiblingReachesTheAutoforwardChild() {
        val sm = AutoforwardDoneInvokeStateMachine()
        sm.initialize()

        if (!sm.isInFinalState) {
            val deadline = System.currentTimeMillis() + 2000L
            while (!sm.isInFinalState && System.currentTimeMillis() < deadline) {
                Thread.sleep(10)
                sm.tick()
            }
        }

        assertEquals(
            AutoforwardDoneInvokeState.Pass,
            sm.currentState.value,
            "the watcher saw only `probe`: `done.invoke.inv_short` was withheld from a live " +
                "`autoforward` child. W3C Appendix D `mainEventLoop` forwards every event " +
                "dequeued from the external queue and excludes only the cancel event, and " +
                "§6.4.2 places `done.invoke.<id>` on that queue — so no name-based " +
                "platform-event filter belongs on the forwarding path."
        )
    }
}
