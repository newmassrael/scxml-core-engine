// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.5 + 6.3.1 donedata surfacing — Kotlin AOT local-invoke path.
//
// Closes the W3C IRP coverage gap: no public IRP test exercises
// `<donedata>` on the invoked child's top-level `<final>` combined
// with `done.invoke.<id>._event.data` readback on the parent. Mirrors
// `tests/integration/DonedataLocalInvokeTest.cpp` (C++ Interpreter
// path, commits fb8e3c79 + 00f347cb) for the Kotlin AOT code path
// through `StateMachineEngine.startInvoke`'s completion callback.
//
// Fixture: integration_resources/donedata_local_invoke/donedata_local_invoke.scxml
//
// Regeneration (after fixture or template edit):
//   scripts/regen_donedata_local_invoke_kotlin.sh

package com.sce.integration

import com.sce.integration.donedata_local_invoke.DonedataLocalInvokeState
import com.sce.integration.donedata_local_invoke.DonedataLocalInvokeStateMachine
import com.sce.w3c.W3CTestBase
import com.sce.w3c.driveOnVirtualTime
import com.sce.w3c.initializeOnVirtualTime
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

/// W3C SCXML 5.5 + 6.3.1 — donedata on local-invoke done.invoke (Kotlin AOT).
@DisplayName("DonedataLocalInvoke — W3C SCXML 5.5 + 6.3.1")
class DonedataLocalInvokeTest {

    @Test
    fun parentObservesDonedataOnDoneInvoke() {
        val sm = DonedataLocalInvokeStateMachine(W3CTestBase.createEngine())
        sm.initializeOnVirtualTime()

        // Children are synchronous (single top-level `<final>`), so
        // `initialize()` completes both invoke handshakes before it
        // returns; the drive is a no-op unless that ever changes.
        sm.driveOnVirtualTime(2000L)

        try {
            assertEquals(
                DonedataLocalInvokeState.Pass,
                sm.terminalState,
                "parent reached Fail: `_event.data.result === 42` (param branch) or " +
                    "`_event.data === 'hello_content'` (content branch) failed. An empty " +
                    "`_event.data` means `StateMachineEngine.startInvoke`'s completion " +
                    "callback dropped the child's donedata — mirror the C++ AOT " +
                    "`stashDonedataAtFinal` / `donedataAtFinal()` contract in the Kotlin " +
                    "template (`entry_exit_actions.kt.jinja2` top-level `<final>` branch) " +
                    "and thread the stashed payload into the `EventMetadata.data` field " +
                    "of the emitted `done.invoke.<id>`."
            )
        } finally {
            sm.cleanup()
        }
    }
}
