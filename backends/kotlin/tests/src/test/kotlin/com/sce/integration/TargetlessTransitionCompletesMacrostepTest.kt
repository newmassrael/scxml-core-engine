// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML Appendix D's main event loop returns to
// `selectEventlessTransitions()` after every microstep, and drains the internal
// queue in the same inner loop. It never asks whether the microstep it just
// took moved the machine — it cannot, because W3C SCXML 3.13 defines a
// transition with no `target` as one that exits and enters nothing and runs its
// content in place. Kotlin AOT path.
//
// Measured 2026-08-20, the two C++ engines end the macrostep at such a
// transition: whatever its content enabled is never walked, and the host is
// handed a configuration the clause says is not stable. This channel is the
// side of that comparison that was already right, and it is here so the
// contract is stated for every backend rather than only for the ones that
// broke it.
//
// `eventless_macrostep_is_bounded` owns how FAR a chain may run; this one owns
// whether the chain is entered at all.
//
// Fixture: integration_resources/targetless_transition_completes_macrostep/targetless_transition_completes_macrostep.scxml
//
// Regeneration (after fixture or template edit):
//   scripts/regen_targetless_transition_completes_macrostep_kotlin.sh

package com.sce.integration

import com.sce.integration.targetless_transition_completes_macrostep.TargetlessTransitionCompletesMacrostepEvent
import com.sce.integration.targetless_transition_completes_macrostep.TargetlessTransitionCompletesMacrostepState
import com.sce.integration.targetless_transition_completes_macrostep.TargetlessTransitionCompletesMacrostepStateMachine
import com.sce.w3c.W3CTestBase
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

/// W3C SCXML 3.13 — a transition with no target ends a microstep, not the
/// macrostep.
@DisplayName("TargetlessTransitionCompletesMacrostep — W3C SCXML 3.13")
class TargetlessTransitionCompletesMacrostepTest {

    private fun started(): TargetlessTransitionCompletesMacrostepStateMachine {
        // The fixture counts what the macrostep reached with `<assign>`, so
        // this is an ECMAScript-datamodel machine.
        val sm = TargetlessTransitionCompletesMacrostepStateMachine(W3CTestBase.createEngine())
        sm.initialize()
        return sm
    }

    private fun deliver(
        sm: TargetlessTransitionCompletesMacrostepStateMachine,
        event: TargetlessTransitionCompletesMacrostepEvent
    ) {
        sm.send(event)
        sm.tick()
    }

    /// The axis: a transition that moves nothing still ends a microstep, so the
    /// macrostep continues into whatever its content enabled.
    ///
    /// `chained == 1, polished == 0` is the signature of an engine that resumes
    /// the chain only after a transition that MOVED the machine; `chained == 0`
    /// is the signature of one that never entered the chain at all.
    @Test
    fun aTargetlessTransitionDoesNotEndTheMacrostep() {
        val sm = started()

        deliver(sm, TargetlessTransitionCompletesMacrostepEvent.Arm)

        assertEquals(
            1L,
            sm.armed(),
            "the targetless transition ran its content — without this the rest measures a lost " +
                "event rather than a stopped macrostep"
        )
        assertEquals(
            1L,
            sm.chained(),
            "and the eventless transition that content enabled was taken in the SAME macrostep, " +
                "which is the whole of what Appendix D's inner loop promises a host"
        )
        assertEquals(
            1L,
            sm.polished(),
            "including the chain's last link, which is targetless itself: an engine that walks " +
                "the chain only while the machine keeps moving stops exactly here"
        )
        assertTrue(
            sm.currentState.value == TargetlessTransitionCompletesMacrostepState.Settled,
            "the host must be handed the stable configuration, not the one the machine was " +
                "passing through"
        )
    }

    /// The other side of the same inner loop: what a targetless transition
    /// raises is answered before the host gets control back.
    @Test
    fun aRaiseFromATargetlessTransitionIsAnsweredInTheSameMacrostep() {
        val sm = started()

        deliver(sm, TargetlessTransitionCompletesMacrostepEvent.Ping)

        assertEquals(
            1L,
            sm.answered(),
            "the internal event the targetless transition raised was dequeued and matched inside " +
                "this macrostep"
        )
        assertTrue(
            sm.currentState.value == TargetlessTransitionCompletesMacrostepState.Idle,
            "neither transition moves the machine, which is the point: the macrostep has to " +
                "continue anyway"
        )
    }

    /// The control, and the reason a zero above means anything: a targetless
    /// transition that enables nothing leaves the machine exactly where it was,
    /// and having run is still observable.
    @Test
    fun aTargetlessTransitionThatEnablesNothingChangesNothingElse() {
        val sm = started()

        deliver(sm, TargetlessTransitionCompletesMacrostepEvent.Quiet)

        assertEquals(1L, sm.quiet(), "the transition fired")
        assertEquals(
            0L,
            sm.chained(),
            "and nothing else did: the eventless transition's guard is still closed, so an engine " +
                "that walked the chain here would be firing a transition the document did not enable"
        )
        assertEquals(0L, sm.polished())
        assertEquals(0L, sm.answered())
        assertTrue(sm.currentState.value == TargetlessTransitionCompletesMacrostepState.Idle)
    }

    /// The other microstep that ends where it began: a transition whose target
    /// is its own source.
    ///
    /// It is not targetless — W3C SCXML 3.13 gives it an exit and an entry —
    /// but a macrostep loop that continues only while the configuration keeps
    /// changing drops it for the same reason and, in the C++ AOT engine, in the
    /// same line of code.
    @Test
    fun anEventlessSelfTransitionExitsAndReEnters() {
        val sm = started()

        deliver(sm, TargetlessTransitionCompletesMacrostepEvent.Recycle)

        assertEquals(
            2L,
            sm.entries(),
            "the state is entered once by `recycle` and once more by the eventless self " +
                "transition its entry enabled — a self transition exits and re-enters, so " +
                "<onentry> runs again"
        )
        assertTrue(
            sm.currentState.value == TargetlessTransitionCompletesMacrostepState.Recycled,
            "and the guard closes behind it, so the machine rests here rather than spinning"
        )
    }

    /// A macrostep, not a one-shot: the second targetless transition is
    /// followed the same way the first was.
    @Test
    fun theSecondTargetlessTransitionIsFollowedToo() {
        val sm = started()

        deliver(sm, TargetlessTransitionCompletesMacrostepEvent.Quiet)
        deliver(sm, TargetlessTransitionCompletesMacrostepEvent.Ping)
        assertEquals(1L, sm.answered(), "precondition: this test is about the SECOND raise")

        deliver(sm, TargetlessTransitionCompletesMacrostepEvent.Ping)

        assertEquals(
            2L,
            sm.answered(),
            "the raise in the third macrostep was answered like the one in the second — the inner " +
                "loop belongs to every macrostep, not to the first"
        )
        assertEquals(1L, sm.quiet())
        assertTrue(sm.currentState.value == TargetlessTransitionCompletesMacrostepState.Idle)
    }
}
