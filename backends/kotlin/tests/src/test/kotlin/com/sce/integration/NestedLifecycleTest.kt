// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A statechart declared as a transition function answers as its machine does
// (docs/SCE_TRANSITION_FUNCTION.md) — Kotlin AOT, the nested lifecycle.
//
// The document declares `sce:transition-function="emit"` and reaches every rule
// the function is computed by: transitions held by an ancestor, a compound
// state entered by its `initial` attribute, by an `<initial>` element and by its
// first child, an internal and an external transition, a prefix and a wildcard
// descriptor, a targetless transition and a top-level final. This drives the
// machine to every state by the function's shortest path, sends every event
// there, and holds what the engine does to what the function says, pair by
// pair.
//
// Fixture: integration_resources/nested_lifecycle/nested_lifecycle.scxml
// (canonical, shared with the C++ / Rust / Go / Python / C11 channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_nested_lifecycle_kotlin.sh

package com.sce.integration

import com.sce.integration.nested_lifecycle.NestedLifecycleStateMachine
import com.sce.integration.nested_lifecycle_transition_function.ALL_EVENTS
import com.sce.integration.nested_lifecycle_transition_function.ALL_STATES
import com.sce.integration.nested_lifecycle_transition_function.INITIAL
import com.sce.integration.nested_lifecycle_transition_function.configuration
import com.sce.integration.nested_lifecycle_transition_function.isFinal
import com.sce.integration.nested_lifecycle_transition_function.transition
import com.sce.integration.nested_lifecycle_transition_function.Event as FunctionEvent
import com.sce.integration.nested_lifecycle_transition_function.State as FunctionState
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

@DisplayName("NestedLifecycle — the machine and its transition function agree")
class NestedLifecycleTest {

    @Test
    fun theMachineAndItsTransitionFunctionAgreeOnEveryPair() {
        lateinit var machine: NestedLifecycleStateMachine
        val probe = EngineProbe(
            start = {
                machine = NestedLifecycleStateMachine()
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
