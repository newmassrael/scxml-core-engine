// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML Appendix D exitStates: a state leaves the configuration AFTER its
// own `<onexit>` has run — Kotlin AOT path.
//
// The procedure is onexit, then cancelInvoke, then configuration.delete(s),
// for each state in exitOrder. Measured 2026-09-26, this channel removed the
// state from the configuration before calling the generated `onExit`, which
// itself cancelled the state's invocations before its `<onexit>` content, so
// `In(s)` inside s's own handler answered false. The handlers record what they
// saw and the document turns those records into the final it reaches.
//
// Fixture: integration_resources/onexit_runs_before_the_state_leaves/onexit_runs_before_the_state_leaves.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python channels).
//
// Regeneration: scripts/regen_onexit_runs_before_the_state_leaves_kotlin.sh

package com.sce.integration

import com.sce.integration.onexit_runs_before_the_state_leaves.OnexitRunsBeforeTheStateLeavesEvent
import com.sce.integration.onexit_runs_before_the_state_leaves.OnexitRunsBeforeTheStateLeavesState
import com.sce.integration.onexit_runs_before_the_state_leaves.OnexitRunsBeforeTheStateLeavesStateMachine
import com.sce.w3c.W3CTestBase
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

@DisplayName("OnexitRunsBeforeTheStateLeaves — W3C SCXML Appendix D exitStates")
class OnexitRunsBeforeTheStateLeavesTest {

    @Test
    fun aStateIsStillActiveWhileItsOwnOnexitRuns() {
        // The handlers record with `<assign>`, so this is an ECMAScript-datamodel
        // machine.
        val sm = OnexitRunsBeforeTheStateLeavesStateMachine(W3CTestBase.createEngine())
        sm.initialize()
        assertEquals(
            OnexitRunsBeforeTheStateLeavesState.Inner,
            sm.currentState.value,
            "the run has to start inside `inner`",
        )

        sm.send(OnexitRunsBeforeTheStateLeavesEvent.Leave)
        sm.tick()

        // The document checks its clauses in document order and lands each in a
        // `<final>` of its own, so the state reported here names which one broke;
        // the records (W3C SCXML 5.3 readers) say what the handler actually saw.
        val records = "selfInInner=${sm.selfInInner()} parentInInner=${sm.parentInInner()} " +
            "selfInOuter=${sm.selfInOuter()} childInOuter=${sm.childInOuter()} exits=${sm.exits()}; " +
            "wanted 1 / 1 / 1 / 0 / 2"
        assertEquals(
            OnexitRunsBeforeTheStateLeavesState.Settled,
            sm.terminalState,
            "($records) `leave` did not carry the machine to `settled`: `failExits` is a handler that did " +
                "not run, `failSelfInInner` / `failSelfInOuter` a state already out of the " +
                "configuration during its own `<onexit>`, `failParentInInner` the parent " +
                "leaving before its child's `<onexit>`, `failChildInOuter` the child still " +
                "active during its parent's `<onexit>`",
        )
    }
}
