// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.1.2: "If no transition matches in any state, the event is
// discarded" — and the host that fed it in can find out. Kotlin AOT path.
//
// Three outcomes leave the configuration identical, so no accessor that
// existed before this fixture separates them:
//
//   poke    self transition       handled (exits and re-enters `idle`)
//   nudge   targetless internal   handled (actions only, no exit/entry)
//   settle  no matching           DISCARDED — the host's event went nowhere
//
// The C++ Interpreter answers all three (`processEvent`'s `TransitionResult`
// and `getStatistics().failedTransitions`); the generated engines computed the
// same fact and dropped it. Kotlin dropped it in a way its own type system had
// already named: `TransitionResult.Ignored` reached the engine and was matched
// with `-> {}`.
//
// ⚠ This engine has TWO external-event entry points — `send` + `tick` in sync
// mode, and the coroutine mode's channel — so the count is recorded at both.
// The sync path is what this driver exercises.
//
// Fixture: integration_resources/discarded_event_is_observable/discarded_event_is_observable.scxml
//
// Regeneration (after fixture or template edit):
//   scripts/regen_discarded_event_is_observable_kotlin.sh

package com.sce.integration

import com.sce.integration.discarded_event_is_observable.DiscardedEventIsObservableEvent
import com.sce.integration.discarded_event_is_observable.DiscardedEventIsObservableState
import com.sce.integration.discarded_event_is_observable.DiscardedEventIsObservableStateMachine
import com.sce.w3c.W3CTestBase
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertNull
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

/// W3C SCXML 3.1.2 — a discarded event is something the host can see.
@DisplayName("DiscardedEventIsObservable — W3C SCXML 3.1.2")
class DiscardedEventIsObservableTest {

    private fun started(): DiscardedEventIsObservableStateMachine {
        // The fixture counts handled events with `<assign>`, so this is an
        // ECMAScript-datamodel machine.
        val sm = DiscardedEventIsObservableStateMachine(W3CTestBase.createEngine())
        sm.initialize()
        return sm
    }

    private fun deliver(sm: DiscardedEventIsObservableStateMachine, event: DiscardedEventIsObservableEvent) {
        sm.send(event)
        sm.tick()
    }

    @Test
    fun anEventNoActiveStateAnsweredIsCounted() {
        val sm = started()
        assertEquals(0, sm.discardedExternalEvents(), "nothing has been discarded before the first event")

        // `settle` is declared in `busy`, so it is in the machine's vocabulary
        // and the host can name it — it just matches nothing in `idle`.
        deliver(sm, DiscardedEventIsObservableEvent.Settle)

        assertEquals(
            1,
            sm.discardedExternalEvents(),
            "`settle` came off the external queue in `idle`, where no transition matches it. " +
                "W3C SCXML 3.1.2 discards it; the host that queued it has no other way to " +
                "learn its event went nowhere"
        )
        assertEquals(
            DiscardedEventIsObservableState.Idle,
            sm.currentState.value,
            "a discarded event must not move the machine"
        )
    }

    @Test
    fun aHandledEventIsNotCounted() {
        val sm = started()

        deliver(sm, DiscardedEventIsObservableEvent.Poke)
        assertEquals(
            1L,
            sm.pokes(),
            "`poke`'s self transition did not run, so nothing below is measuring a handled event"
        )
        assertEquals(
            0,
            sm.discardedExternalEvents(),
            "`poke` matched a self transition — handled, and the configuration is unchanged " +
                "only because the transition returns to its own source"
        )

        deliver(sm, DiscardedEventIsObservableEvent.Nudge)
        assertEquals(1L, sm.nudges(), "`nudge`'s targetless transition did not run")
        assertEquals(
            0,
            sm.discardedExternalEvents(),
            "`nudge` matched a targetless internal transition: its actions ran and no state " +
                "was exited or entered, which is why the count cannot be keyed off whether " +
                "the configuration changed"
        )
    }

    @Test
    fun theDiscardIsNotDerivableFromAnyOtherAccessor() {
        val sm = started()

        deliver(sm, DiscardedEventIsObservableEvent.Poke)
        val handledState = sm.currentState.value
        val handledFinal = sm.isInFinalState

        deliver(sm, DiscardedEventIsObservableEvent.Settle)

        assertEquals(
            handledState,
            sm.currentState.value,
            "this fixture exists because a handled event and a discarded one are " +
                "indistinguishable through the accessors a host had; if they ever differ, " +
                "the fixture stopped measuring what it claims"
        )
        assertEquals(handledFinal, sm.isInFinalState)
        assertEquals(
            1,
            sm.discardedExternalEvents(),
            "the two are indistinguishable through every other accessor, so the count is the " +
                "only thing that separates them"
        )
    }

    @Test
    fun theEngineNamesTheEventItDiscarded() {
        val sm = started()
        assertNull(sm.lastDiscardedEvent(), "nothing has been discarded yet")

        deliver(sm, DiscardedEventIsObservableEvent.Settle)

        assertEquals(
            DiscardedEventIsObservableEvent.Settle,
            sm.lastDiscardedEvent(),
            "the engine counted a discard but cannot say which event it was"
        )
    }

    @Test
    fun anEventTheMachineHasMovedPastIsCounted() {
        val sm = started()
        deliver(sm, DiscardedEventIsObservableEvent.Go)
        assertEquals(
            DiscardedEventIsObservableState.Busy,
            sm.currentState.value,
            "`go` should have moved the machine out of `idle`"
        )

        deliver(sm, DiscardedEventIsObservableEvent.Poke)

        assertEquals(
            1,
            sm.discardedExternalEvents(),
            "the machine left `idle`, so `poke` no longer matches — the host that kept " +
                "sending it is exactly who the count is for"
        )
        assertEquals(DiscardedEventIsObservableEvent.Poke, sm.lastDiscardedEvent())
    }
}
