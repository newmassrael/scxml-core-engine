// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.12.2: the processor MUST signal its own failures by raising
// `error.*` events into the internal queue, and the same paragraph says they
// "are ignored if no transition is found that matches them". Being ignored is
// the clause. Being unable to say it happened is not. Kotlin AOT path.
//
// `DiscardedEventIsObservableTest` asks this for the EXTERNAL queue and stops
// at its edge on the stated ground that an unmatched internal event is the
// document's own business with both ends inside the document. That is exactly
// right for an author's `<raise>` and exactly wrong for an error event, whose
// sender is the ENGINE. The host never wrote the document, cannot see the
// failure in the configuration, and is the only party able to act on it.
//
// Four outcomes the fixture separates, all four leaving the configuration on
// the same state:
//
//   poke              handled, no error            control: proves a run fired
//   whisper           author's <raise>, unmatched  NOT counted
//   boom in idle      error, unmatched             COUNTED — the silent failure
//   boom in guarded   error, HANDLED               not counted
//
// Fixture: integration_resources/unhandled_error_is_observable/unhandled_error_is_observable.scxml
//
// Regeneration (after fixture or template edit):
//   scripts/regen_unhandled_error_is_observable_kotlin.sh

package com.sce.integration

import com.sce.integration.unhandled_error_is_observable.UnhandledErrorIsObservableEvent
import com.sce.integration.unhandled_error_is_observable.UnhandledErrorIsObservableState
import com.sce.integration.unhandled_error_is_observable.UnhandledErrorIsObservableStateMachine
import com.sce.w3c.W3CTestBase
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertNull
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

/// W3C SCXML 3.12.2 — an error nobody answered is something the host can see.
@DisplayName("UnhandledErrorIsObservable — W3C SCXML 3.12.2")
class UnhandledErrorIsObservableTest {

    private fun started(): UnhandledErrorIsObservableStateMachine {
        // The fixture counts what ran with `<assign>`, so this is an
        // ECMAScript-datamodel machine.
        val sm = UnhandledErrorIsObservableStateMachine(W3CTestBase.createEngine())
        sm.initialize()
        return sm
    }

    private fun deliver(sm: UnhandledErrorIsObservableStateMachine, event: UnhandledErrorIsObservableEvent) {
        sm.send(event)
        sm.tick()
    }

    /// The axis: an error the engine raised that no active state answers.
    @Test
    fun anErrorNoTransitionAnsweredIsCounted() {
        val sm = started()
        assertEquals(0, sm.unhandledErrorEvents(), "no error has gone unhandled before the first event")

        deliver(sm, UnhandledErrorIsObservableEvent.Boom)

        assertEquals(
            1L,
            sm.booms(),
            "`boom`'s transition did not run, so nothing below is measuring an error " +
                "raised inside a transition that fired"
        )
        assertEquals(
            1,
            sm.unhandledErrorEvents(),
            "`boom`'s second <assign> has W3C 5.3's invalid empty location, so the engine " +
                "raised error.execution — and `idle` declares no transition for it. The host " +
                "driving this machine has no other way to learn its <assign> failed"
        )
        assertEquals(
            UnhandledErrorIsObservableState.Idle,
            sm.currentState.value,
            "the error must not move the machine on its own"
        )
    }

    /// The other half: an error the DOCUMENT answered must not be counted.
    @Test
    fun anErrorTheDocumentHandledIsNotCounted() {
        val sm = started()

        deliver(sm, UnhandledErrorIsObservableEvent.Go)
        assertEquals(
            UnhandledErrorIsObservableState.Guarded,
            sm.currentState.value,
            "`go` should have moved the machine to the state that answers errors"
        )

        deliver(sm, UnhandledErrorIsObservableEvent.Boom)

        assertEquals(
            1L,
            sm.caught(),
            "`guarded`'s error.execution transition did not run, so this test is not " +
                "measuring a HANDLED error"
        )
        assertEquals(
            0,
            sm.unhandledErrorEvents(),
            "the same <assign> failed in `guarded`, where the document does declare a " +
                "transition for error.execution. The document dealt with it, and its handling " +
                "is already visible in the configuration — counting it would report the " +
                "author's own error handling as a silent failure"
        )
        assertNull(sm.lastUnhandledError(), "nothing went unhandled, so there is no last one to name")
    }

    /// The boundary: an author's own unmatched `<raise>` is not an error.
    @Test
    fun anAuthorsUnmatchedRaiseIsNotAnUnhandledError() {
        val sm = started()

        deliver(sm, UnhandledErrorIsObservableEvent.Whisper)

        assertEquals(
            0,
            sm.unhandledErrorEvents(),
            "`whisper` raises `unheard` and `retry.error.execution`, neither of which any state " +
                "answers. Both are discarded exactly as an unmatched error is, and neither is " +
                "one: the author wrote the raises and the absent handlers. " +
                "`retry.error.execution` is the sharper half — it CONTAINS `error.` without " +
                "starting with it, and W3C 3.12.2 reserves the prefix, not the substring"
        )
        assertEquals(
            1L,
            sm.heards(),
            "`whisper`'s third raise, `heard`, does match — and the transition it matches did " +
                "not run. The count above is a byproduct of the internal drain, never its job: " +
                "an implementation that only selects transitions for error events stops running " +
                "the document for everything else"
        )
        assertEquals(
            0,
            sm.discardedExternalEvents(),
            "`whisper` itself was handled, so the external-queue count stays put — the " +
                "internal events it raised are not on that queue at all"
        )
    }

    /// Every pre-existing accessor answers the same for both runs.
    @Test
    fun theUnhandledErrorIsNotDerivableFromAnyOtherAccessor() {
        val sm = started()

        deliver(sm, UnhandledErrorIsObservableEvent.Poke)
        val clean = listOf<Any?>(
            sm.currentState.value,
            sm.isInFinalState,
            sm.discardedExternalEvents(),
            sm.lastDiscardedEvent()
        )

        deliver(sm, UnhandledErrorIsObservableEvent.Boom)
        val failed = listOf<Any?>(
            sm.currentState.value,
            sm.isInFinalState,
            sm.discardedExternalEvents(),
            sm.lastDiscardedEvent()
        )

        assertEquals(
            clean,
            failed,
            "this fixture exists because these two are indistinguishable through every " +
                "accessor a host had — including layer three's discard count, which never " +
                "sees the internal queue. If they ever differ, the fixture stopped measuring " +
                "what it claims"
        )
        assertEquals(
            1,
            sm.unhandledErrorEvents(),
            "the two are indistinguishable through every other accessor, so this count is " +
                "the only thing that separates a silent failure from a clean run"
        )
    }

    /// A count says something failed; a repair needs the class of error.
    @Test
    fun theEngineNamesTheErrorItDropped() {
        val sm = started()
        assertNull(sm.lastUnhandledError(), "nothing has gone unhandled yet")

        deliver(sm, UnhandledErrorIsObservableEvent.Boom)

        assertEquals(
            UnhandledErrorIsObservableEvent.Error.Execution,
            sm.lastUnhandledError(),
            "`error.execution` is the document's own executable content failing; " +
                "`error.communication` would be a <send> that could not reach its target. " +
                "Two different repairs, and a bare count separates neither"
        )
    }

    /// The supervisor's failure mode: every round fails identically.
    @Test
    fun aMachineFailingEveryRoundIsCountedEveryRound() {
        val sm = started()

        for (round in 1..3) {
            deliver(sm, UnhandledErrorIsObservableEvent.Boom)
            assertEquals(
                round,
                sm.unhandledErrorEvents(),
                "round $round did not add to the count; a supervisor polling this number is " +
                    "exactly who learns the loop is not making progress"
            )
            assertEquals(
                UnhandledErrorIsObservableState.Idle,
                sm.currentState.value,
                "the machine looks identical on every round, which is the problem"
            )
        }
        assertEquals(3L, sm.booms(), "all three rounds should have run their transition")
    }
}
