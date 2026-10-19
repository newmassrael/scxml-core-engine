// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A statechart declared as a transition function answers as its machine does
// (docs/SCE_TRANSITION_FUNCTION.md) — Kotlin AOT, the flat lifecycle.
//
// The document declares `sce:transition-function="emit"`, so its machine is
// generated with a pure function of (state, event) beside it, in the package
// `com.sce.integration.flat_lifecycle_transition_function`. The function was
// computed at build time by the selection rule the engine follows at run time;
// this drives the machine to every state by the function's shortest path, sends
// every event there, and holds what the engine does to what the function says,
// pair by pair.
//
// Fixture: integration_resources/flat_lifecycle/flat_lifecycle.scxml
// (canonical, shared with the C++ / Rust / Go / Python / C11 channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_flat_lifecycle_kotlin.sh

package com.sce.integration

import com.sce.integration.flat_lifecycle.FlatLifecycleStateMachine
import com.sce.integration.flat_lifecycle_transition_function.ALL_EVENTS
import com.sce.integration.flat_lifecycle_transition_function.ALL_STATES
import com.sce.integration.flat_lifecycle_transition_function.INITIAL
import com.sce.integration.flat_lifecycle_transition_function.configuration
import com.sce.integration.flat_lifecycle_transition_function.isFinal
import com.sce.integration.flat_lifecycle_transition_function.transition
import com.sce.integration.flat_lifecycle_transition_function.Event as FunctionEvent
import com.sce.integration.flat_lifecycle_transition_function.State as FunctionState
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

@DisplayName("FlatLifecycle — the machine and its transition function agree")
class FlatLifecycleTest {

    @Test
    fun theMachineAndItsTransitionFunctionAgreeOnEveryPair() {
        lateinit var machine: FlatLifecycleStateMachine
        val probe = EngineProbe(
            start = {
                machine = FlatLifecycleStateMachine()
                machine.initialize()
            },
            send = { name ->
                machine.sendEventByName(name)
                machine.tick()
            },
            active = { machine.activeConfiguration.map { machine.nameOfState(it) }.toSet() },
            terminal = { machine.terminalState?.let { machine.nameOfState(it) } },
            discarded = { machine.discardedExternalEvents() },
        )
        val function = TransitionFunctionView(
            states = ALL_STATES,
            events = ALL_EVENTS,
            initial = INITIAL,
            transition = ::transition,
            isFinal = ::isFinal,
            configuration = ::configuration,
            stateName = FunctionState::scxmlName,
            eventName = FunctionEvent::scxmlName,
        )
        assertEquals(
            ALL_STATES.size * ALL_EVENTS.size,
            expectEveryPairAgrees(function, probe),
            "the drive did not check every pair",
        )
    }
}
