// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
// Fixture: sce-kotlin-tests/src/test/resources/fixtures/donedata_local_invoke.scxml
//
// Regeneration (after fixture or template edit):
//   TMP=$(mktemp -d)
//   target/release/sce-codegen generate \
//       sce-kotlin-tests/src/test/resources/fixtures/donedata_local_invoke.scxml \
//       -l kotlin -o "$TMP/"
//   # SCE_MESH.md §9.6.6 rule 1: parser emits synth siblings as
//   # `<parent>__sce_synth_invoke__<id>.scxml` (previously `_child<N>.scxml`).
//   for child in "$TMP"/donedata_local_invoke__sce_synth_invoke__*.scxml; do
//       target/release/sce-codegen generate "$child" \
//           --as-child --parent-stem donedata_local_invoke \
//           -l kotlin -o "$TMP/"
//   done
//   cp "$TMP"/*.kt \
//       sce-kotlin-tests/src/main/kotlin/com/sce/generated/donedata_local_invoke/
//   rm -rf "$TMP"

package com.sce.integration

import com.sce.generated.donedata_local_invoke.DonedataLocalInvokeState
import com.sce.generated.donedata_local_invoke.DonedataLocalInvokeStateMachine
import com.sce.w3c.W3CTestBase
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

/// W3C SCXML 5.5 + 6.3.1 — donedata on local-invoke done.invoke (Kotlin AOT).
@DisplayName("DonedataLocalInvoke — W3C SCXML 5.5 + 6.3.1")
class DonedataLocalInvokeTest {

    @Test
    fun parentObservesDonedataOnDoneInvoke() {
        val sm = DonedataLocalInvokeStateMachine(W3CTestBase.createEngine())
        sm.initialize()

        // Children are synchronous (single top-level `<final>`), so
        // `initialize()` completes both invoke handshakes before it
        // returns. Fall through to `tick()` polling only defensively in
        // case the runtime model later adds async microstep scheduling.
        if (!sm.isInFinalState) {
            val deadline = System.currentTimeMillis() + 2000L
            while (!sm.isInFinalState && System.currentTimeMillis() < deadline) {
                Thread.sleep(10)
                sm.tick()
            }
        }

        try {
            assertEquals(
                DonedataLocalInvokeState.Pass,
                sm.currentState.value,
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
