// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A computed event name is matched like any other (§scxml-3.12.1, §scxml-5.10) —
// Kotlin AOT.
//
// A `<send eventexpr>` names its event at run time, so the document cannot have
// written the name: it is delivered as the event the document's names resolve it
// to (its own, the longest token prefix of it the document writes, or its
// wildcard), and `_event.name` is the whole name. The document sends six, over
// the external and the internal queue, now and after a delay, and takes each only
// when it is told the whole name.
//
// Fixture: integration_resources/a_computed_event_name_is_matched_like_any_other/a_computed_event_name_is_matched_like_any_other.scxml
// (canonical, shared with the C++ / Rust / Go / Python / C11 channels).
//
// Regeneration (after fixture or template edit):
//   scripts/materialize.sh kotlin-integration

package com.sce.integration

import com.sce.integration.a_computed_event_name_is_matched_like_any_other.AComputedEventNameIsMatchedLikeAnyOtherState
import com.sce.integration.a_computed_event_name_is_matched_like_any_other.AComputedEventNameIsMatchedLikeAnyOtherStateMachine
import com.sce.integration.a_computed_event_name_is_matched_like_any_other.AComputedEventNameIsMatchedLikeAnyOtherTransitions
import com.sce.runtime.TraceFired
import com.sce.runtime.TraceStep
import com.sce.runtime.TraceStepKind
import com.sce.runtime.TransitionTaken
import com.sce.w3c.W3CTestBase
import com.sce.w3c.driveOnVirtualTime
import com.sce.w3c.initializeOnVirtualTime
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

/// W3C SCXML 3.12.1 + 5.10 — a computed event name is matched like any other (Kotlin AOT).
@DisplayName("AComputedEventNameIsMatchedLikeAnyOther — W3C SCXML 3.12.1 + 5.10")
class AComputedEventNameIsMatchedLikeAnyOtherTest {

    @Test
    fun aComputedNameIsMatchedByTokenAndToldWhole() {
        val sm = AComputedEventNameIsMatchedLikeAnyOtherStateMachine(W3CTestBase.createEngine())
        sm.initializeOnVirtualTime()
        sm.driveOnVirtualTime(2000L)

        assertEquals(
            AComputedEventNameIsMatchedLikeAnyOtherState.Pass,
            sm.terminalState,
            "a computed name was dropped, or matched but told shorter than the name it was " +
                "sent under (`request.new`, `other.thing`, `request.again`, `last.one`)"
        )
    }

    /**
     * The document declares its transition table, and a run that reaches `pass`
     * takes each of its six transitions once, in document order: the transition
     * observer is told exactly those rows, each by the `(from, index)` its row of
     * the table carries (docs/adr/0089).
     */
    @Test
    fun theObserverIsToldEachRowOfTheTableTheRunTakes() {
        val seen = mutableListOf<TransitionTaken<AComputedEventNameIsMatchedLikeAnyOtherState>>()
        val sm = AComputedEventNameIsMatchedLikeAnyOtherStateMachine(W3CTestBase.createEngine())
        sm.transitionObserver = { taken -> seen.add(taken) }
        sm.initializeOnVirtualTime()
        sm.driveOnVirtualTime(2000L)

        assertEquals(AComputedEventNameIsMatchedLikeAnyOtherState.Pass, sm.terminalState)
        val expected = listOf(
            AComputedEventNameIsMatchedLikeAnyOtherState.S1,
            AComputedEventNameIsMatchedLikeAnyOtherState.S2,
            AComputedEventNameIsMatchedLikeAnyOtherState.S3,
            AComputedEventNameIsMatchedLikeAnyOtherState.S4,
            AComputedEventNameIsMatchedLikeAnyOtherState.S5,
            AComputedEventNameIsMatchedLikeAnyOtherState.S6,
        ).map { state -> TransitionTaken(state, 0) }
        assertEquals(expected, seen, "the rows taken, in the order the run took them")
        val rows = AComputedEventNameIsMatchedLikeAnyOtherTransitions.all
            .map { row -> TransitionTaken(row.from, row.index) }
            .toSet()
        assertTrue(
            rows.containsAll(seen),
            "every pair the observer is told is a row of the table: $seen against $rows"
        )
        assertEquals(rows, seen.toSet(), "the run takes every row of the table")
    }

    /**
     * An observer set and then cleared is told nothing, and the run still
     * reaches `pass`, as one that never had an observer does.
     */
    @Test
    fun aClearedObserverIsToldNothingAndTheRunIsTheSame() {
        val seen = mutableListOf<TransitionTaken<AComputedEventNameIsMatchedLikeAnyOtherState>>()
        val sm = AComputedEventNameIsMatchedLikeAnyOtherStateMachine(W3CTestBase.createEngine())
        sm.transitionObserver = { taken -> seen.add(taken) }
        sm.transitionObserver = null
        sm.initializeOnVirtualTime()
        sm.driveOnVirtualTime(2000L)

        assertEquals(AComputedEventNameIsMatchedLikeAnyOtherState.Pass, sm.terminalState)
        assertTrue(seen.isEmpty(), "a cleared observer was told $seen")
    }

    /**
     * The trace port (docs/adr/0115, D4 and D8) names each event step's event by
     * the name `_event.name` holds while it is taken, which the run trace format
     * defines the field as: the whole name it was sent under, and not the event
     * of the document it matched (`request`, or the wildcard). The run is a
     * start, six event steps, each off the queue its event came by and taking
     * one transition from s1 to s6 in turn, and a stop that exits `pass` and
     * names it the outcome; the transitions the steps took are the ones the
     * transition observer is told. The C11 lane's `c11_integration_trace_port`
     * and the Rust lane's `an_event_step_names_the_event_by_the_name_it_was_sent_under`
     * hold the same of this document.
     */
    @Test
    fun aTracedRunNamesEachEventByTheNameItWasSentUnder() {
        val told = mutableListOf<TraceStep>()
        val observed = mutableListOf<TransitionTaken<AComputedEventNameIsMatchedLikeAnyOtherState>>()
        val sm = AComputedEventNameIsMatchedLikeAnyOtherStateMachine(W3CTestBase.createEngine())
        sm.tracePort = { step -> told.add(step) }
        sm.transitionObserver = { taken -> observed.add(taken) }
        sm.initializeOnVirtualTime()
        sm.driveOnVirtualTime(2000L)

        assertEquals(AComputedEventNameIsMatchedLikeAnyOtherState.Pass, sm.terminalState)
        val events = listOf(
            Triple(TraceStepKind.EXTERNAL, "request.new", "s1" to "s2"),
            Triple(TraceStepKind.EXTERNAL, "other.thing", "s2" to "s3"),
            Triple(TraceStepKind.INTERNAL, "request.again", "s3" to "s4"),
            Triple(TraceStepKind.INTERNAL, "last.one", "s4" to "s5"),
            Triple(TraceStepKind.INTERNAL, "request.third", "s5" to "s6"),
            Triple(TraceStepKind.EXTERNAL, "last.two", "s6" to "pass"),
        )
        val expected = buildList {
            add(TraceStep(TraceStepKind.START, entered = listOf("s1"), configuration = listOf("s1")))
            for ((kind, event, move) in events) {
                val (source, target) = move
                add(
                    TraceStep(
                        kind,
                        event = event,
                        fired = listOf(TraceFired(source, 0)),
                        exited = listOf(source),
                        entered = listOf(target),
                        configuration = listOf(target),
                    )
                )
            }
            add(TraceStep(TraceStepKind.STOP, exited = listOf("pass"), outcome = "pass"))
        }
        assertEquals(expected, told, "the steps the trace port was told, in the order the run took them")
        assertEquals(
            observed.map { taken -> TraceFired(sm.nameOfState(taken.source), taken.index) },
            told.flatMap { step -> step.fired },
            "the transitions the steps took are the ones the observer is told"
        )
    }
}
