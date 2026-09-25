// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package com.sce.runtime

import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertFalse
import org.junit.jupiter.api.Assertions.assertNull
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test

// W3C SCXML Appendix D — what a microstep selects, exits and enters, asked of
// Microstep.kt over a document written out by hand.
//
// Each case states the answer the appendix computes, worked from the
// pseudo-code, and asks the transcription for it. The cases, the document and
// the answers are those of backends/go/runtime/microstep_test.go,
// backends/rust/runtime/tests/microstep_algorithms.rs and
// backends/python/tests/microstep/test_microstep_algorithms.py, and through them
// of the C++ engines' tests/states/EntrySetAlgorithmsTest.cpp and
// tests/states/CompletionAlgorithmsTest.cpp, so the five transcriptions answer
// one set of questions. Three answers are the reason the entry procedures exist
// and are pinned by name:
//
//   - a target SET enters every target, and a <parallel> region no target
//     descends into still gets its default;
//   - a compound state entered only as an ANCESTOR of a deeper target is not in
//     statesForDefaultEntry, so its initial transition content does not run;
//   - a history that recorded several regions restores all of them.
//
//	<scxml initial="s">
//	  <state id="s" initial="p">
//	    <parallel id="p">
//	      <state id="r1" initial="a1"> a1 a2 </state>
//	      <state id="r2" initial="b1"> b1 b2
//	        <history id="hb" type="shallow"> -> b2 </history> </state>
//	      <state id="r3"> c1 c2 </state>                    (no initial: c1)
//	    </parallel>
//	    <state id="q" initial="q1"> q1 <final id="qf"/> </state>
//	    <history id="hs" type="deep"> -> "a2 b2" </history>
//	    <state id="m" initial="m1a m2b">
//	      <parallel id="mp">
//	        <state id="m1"> m1a m1b </state>
//	        <state id="m2"> m2a m2b </state>
//	      </parallel>
//	    </state>
//	  </state>
//	  <state id="t"/>
//	</scxml>

/** Declared in document order, so an ordinal IS the document position. */
private enum class St { S, P, R1, A1, A2, R2, B1, B2, R3, C1, C2, Q, Q1, Qf, M, Mp, M1, M1a, M1b, M2, M2a, M2b, T }

private enum class Hist { Hb, Hs }

/** The events the transition table below answers; `null` is the eventless selection's "no event". */
private enum class Ev { E, F, G, H, K, R }

private typealias Target = EntryTarget<St, Hist>

// Not `to`, which the Go twin calls it: inside a class body Kotlin resolves
// `to(x)` to the standard library's infix `this.to(x)` and builds a Pair.
private fun toState(state: St): Target = StateTarget(state)

private fun toHistory(history: Hist): Target = HistoryTarget(history)

private data class Tr(val event: Ev, val targets: List<Target> = emptyList(), val internal: Boolean = false)

// The transitions, per source, in document order. Every one has content:
//
//	a1: E -> a2, F -> a2, H (targetless), K -> a2, R -> a1 (a self-transition)
//	b1: E -> b2, G -> b2, K -> b2, R -> b2
//	p:  F -> t,  G -> t,  H -> t,  K (targetless)
private val chartTransitions: Map<St, List<Tr>> = mapOf(
    St.A1 to listOf(
        Tr(Ev.E, listOf(toState(St.A2))),
        Tr(Ev.F, listOf(toState(St.A2))),
        Tr(Ev.H),
        Tr(Ev.K, listOf(toState(St.A2))),
        Tr(Ev.R, listOf(toState(St.A1))),
    ),
    St.B1 to listOf(
        Tr(Ev.E, listOf(toState(St.B2))),
        Tr(Ev.G, listOf(toState(St.B2))),
        Tr(Ev.K, listOf(toState(St.B2))),
        Tr(Ev.R, listOf(toState(St.B2))),
    ),
    St.P to listOf(
        Tr(Ev.F, listOf(toState(St.T))),
        Tr(Ev.G, listOf(toState(St.T))),
        Tr(Ev.H, listOf(toState(St.T))),
        Tr(Ev.K),
    ),
)

private val chartParent: Map<St, St> = mapOf(
    St.P to St.S, St.Q to St.S, St.M to St.S,
    St.R1 to St.P, St.R2 to St.P, St.R3 to St.P,
    St.A1 to St.R1, St.A2 to St.R1,
    St.B1 to St.R2, St.B2 to St.R2,
    St.C1 to St.R3, St.C2 to St.R3,
    St.Q1 to St.Q, St.Qf to St.Q,
    St.Mp to St.M,
    St.M1 to St.Mp, St.M2 to St.Mp,
    St.M1a to St.M1, St.M1b to St.M1,
    St.M2a to St.M2, St.M2b to St.M2,
)

private val chartChildren: Map<St, List<St>> = mapOf(
    St.S to listOf(St.P, St.Q, St.M),
    St.P to listOf(St.R1, St.R2, St.R3),
    St.R1 to listOf(St.A1, St.A2),
    St.R2 to listOf(St.B1, St.B2),
    St.R3 to listOf(St.C1, St.C2),
    St.Q to listOf(St.Q1, St.Qf),
    St.M to listOf(St.Mp),
    St.Mp to listOf(St.M1, St.M2),
    St.M1 to listOf(St.M1a, St.M1b),
    St.M2 to listOf(St.M2a, St.M2b),
)

private val chartInitial: Map<St, List<Target>> = mapOf(
    St.S to listOf(toState(St.P)),
    St.R1 to listOf(toState(St.A1)),
    St.R2 to listOf(toState(St.B1)),
    St.R3 to listOf(toState(St.C1)),
    St.Q to listOf(toState(St.Q1)),
    St.M to listOf(toState(St.M1a), toState(St.M2b)),
    St.M1 to listOf(toState(St.M1a)),
    St.M2 to listOf(toState(St.M2a)),
)

private val initialConfiguration = listOf(St.S, St.P, St.R1, St.A1, St.R2, St.B1, St.R3, St.C1)

/** The document above, a configuration, and a record of what the microstep asked the machine to do. */
private class Chart(configuration: List<St> = emptyList()) : Run<St, Hist, Ev> {
    val recorded = mutableMapOf<Hist, List<St>>()
    var configuration: MutableList<St> = configuration.toMutableList()
    val log = mutableListOf<String>()
    val exitsSaw = mutableListOf<List<St>>()

    // Document

    override fun parentOf(state: St): St? = chartParent[state]

    override fun isCompound(state: St): Boolean =
        state in setOf(St.S, St.R1, St.R2, St.R3, St.Q, St.M, St.M1, St.M2)

    override fun isParallel(state: St): Boolean = state == St.P || state == St.Mp

    override fun isFinal(state: St): Boolean = state == St.Qf

    override fun childStates(state: St): List<St> = chartChildren[state] ?: emptyList()

    override fun initialTargets(state: St): List<Target> = chartInitial[state] ?: emptyList()

    override fun historyParent(history: Hist): St = if (history == Hist.Hb) St.R2 else St.S

    override fun historyValue(history: Hist): List<St>? = recorded[history]

    override fun historyDefaultTargets(history: Hist): List<Target> =
        if (history == Hist.Hb) listOf(toState(St.B2)) else listOf(toState(St.A2), toState(St.B2))

    override fun documentOrder(state: St): Int = state.ordinal

    // Run

    override fun configuration(): List<St> = configuration.toList()

    override fun firstEnabledTransition(state: St, event: Ev?): EnabledTransition<St, Hist>? {
        val index = chartTransitions[state]?.indexOfFirst { it.event == event } ?: return null
        return if (index < 0) null else enabled(state, index)
    }

    override fun exitState(state: St, configurationBeforeExit: List<St>) {
        exitsSaw.add(configurationBeforeExit.toList())
        configuration.remove(state)
        log.add("exit $state")
    }

    override fun executeTransitionContent(transition: EnabledTransition<St, Hist>) {
        log.add("content ${transition.source}#${transition.transitionIndex}")
    }

    override fun enterState(state: St, isDefaultEntry: Boolean) {
        configuration.add(state)
        log.add("enter $state" + if (isDefaultEntry) " (default)" else "")
    }

    override fun executeHistoryDefaultContent(history: Hist) {
        log.add("history default $history")
    }
}

private fun atInitialConfiguration() = Chart(initialConfiguration)

private fun entering(source: St, vararg targets: Target) = EntryTransition(source, targets.toList())

private fun enteringInternal(source: St, vararg targets: Target) =
    EntryTransition(source, targets.toList(), isInternal = true)

private fun fromDocument(vararg targets: Target) = EntryTransition<St, Hist>(null, targets.toList())

private fun enabled(source: St, index: Int): EnabledTransition<St, Hist> {
    val t = chartTransitions.getValue(source)[index]
    return EnabledTransition(source, t.targets, index, hasActions = true, isInternal = t.internal)
}

private fun selected(transitions: List<EnabledTransition<St, Hist>>): List<Pair<St, Int>> =
    transitions.map { it.source to it.transitionIndex }

class MicrostepTest {

    // ════════════════════════════════════════════════════════════════════════
    // computeEntrySet
    // ════════════════════════════════════════════════════════════════════════

    @Test
    fun theInitialConfigurationEntersEveryRegionByDefault() {
        val entry = Microstep.computeEntrySet(Chart(), listOf(fromDocument(toState(St.S))))
        assertEquals(initialConfiguration, entry.statesToEnter)
        // A <parallel> is not compound, so it has no initial transition to run.
        assertEquals(listOf(St.S, St.R1, St.R2, St.R3), entry.statesForDefaultEntry.sorted())
        assertTrue(entry.defaultHistoryContent.isEmpty(), "no history was taken, but content is owed")
    }

    @Test
    fun aTargetSetEntersEveryTargetAndDefaultsTheRegionNoTargetReaches() {
        val entry = Microstep.computeEntrySet(Chart(), listOf(entering(St.T, toState(St.A2), toState(St.B2))))
        assertEquals(listOf(St.S, St.P, St.R1, St.A2, St.R2, St.B2, St.R3, St.C1), entry.statesToEnter)
        // s, r1 and r2 are entered as ANCESTORS of a named target, so their
        // initial transitions do not run; only r3, reached by nobody, defaults.
        assertEquals(listOf(St.R3), entry.statesForDefaultEntry)
        assertFalse(entry.isDefaultEntry(St.S))
        assertTrue(entry.isDefaultEntry(St.R3))
    }

    @Test
    fun anUnrecordedShallowHistoryTakesItsDefaultAndOwesItsContentToItsParent() {
        val entry = Microstep.computeEntrySet(Chart(), listOf(entering(St.A1, toHistory(Hist.Hb))))
        // The domain is s: the history stands for b2, and the least compound
        // ancestor of a1 and b2 walks past the <parallel>.
        assertEquals(listOf(St.P, St.R1, St.A1, St.R2, St.B2, St.R3, St.C1), entry.statesToEnter)
        assertEquals(listOf(St.R1, St.R3), entry.statesForDefaultEntry.sorted())
        assertEquals(Hist.Hb, entry.defaultHistoryContentOf(St.R2), "r2 must owe hb's default content")
        assertNull(entry.defaultHistoryContentOf(St.P), "p owes no history content")
    }

    @Test
    fun aRecordedShallowHistoryRestoresWhatItRecordedAndOwesNoContent() {
        val chart = Chart()
        chart.recorded[Hist.Hb] = listOf(St.B1)
        val entry = Microstep.computeEntrySet(chart, listOf(entering(St.A1, toHistory(Hist.Hb))))
        assertEquals(listOf(St.P, St.R1, St.A1, St.R2, St.B1, St.R3, St.C1), entry.statesToEnter)
        assertTrue(entry.defaultHistoryContent.isEmpty(), "a recorded history owes no default content")
    }

    @Test
    fun aDeepHistoryThatRecordedEveryRegionRestoresAllOfThem() {
        val chart = Chart()
        chart.recorded[Hist.Hs] = listOf(St.A2, St.B1, St.C2)
        val entry = Microstep.computeEntrySet(chart, listOf(entering(St.T, toHistory(Hist.Hs))))
        assertEquals(listOf(St.S, St.P, St.R1, St.A2, St.R2, St.B1, St.R3, St.C2), entry.statesToEnter)
        // Nothing is entered by default: every compound state on the way is an
        // ancestor of a recorded state.
        assertEquals(emptyList<St>(), entry.statesForDefaultEntry)
        assertTrue(entry.defaultHistoryContent.isEmpty(), "a recorded history owes no default content")
    }

    @Test
    fun anUnrecordedDeepHistoryWithAMultiStateDefaultEntersTheWholeSet() {
        val entry = Microstep.computeEntrySet(Chart(), listOf(entering(St.T, toHistory(Hist.Hs))))
        assertEquals(listOf(St.S, St.P, St.R1, St.A2, St.R2, St.B2, St.R3, St.C1), entry.statesToEnter)
        assertEquals(listOf(St.R3), entry.statesForDefaultEntry)
        assertEquals(Hist.Hs, entry.defaultHistoryContentOf(St.S), "s must owe hs's default content")
    }

    @Test
    fun anInternalTransitionToADescendantLeavesItsSourceOutOfTheSet() {
        val entry = Microstep.computeEntrySet(Chart(), listOf(enteringInternal(St.S, toState(St.Q1))))
        assertEquals(listOf(St.Q, St.Q1), entry.statesToEnter)
        assertEquals(emptyList<St>(), entry.statesForDefaultEntry)
    }

    @Test
    fun anExternalTransitionToADescendantReentersItsSource() {
        val entry = Microstep.computeEntrySet(Chart(), listOf(entering(St.S, toState(St.Q1))))
        assertEquals(listOf(St.S, St.Q, St.Q1), entry.statesToEnter)
        assertEquals(emptyList<St>(), entry.statesForDefaultEntry)
    }

    @Test
    fun anExternalTransitionOnARegionRootReentersEverySiblingRegion() {
        val entry = Microstep.computeEntrySet(Chart(), listOf(entering(St.R1, toState(St.A2))))
        // A <parallel> is never a domain, so the domain is s, and the sibling
        // regions are exited and entered again at their defaults.
        assertEquals(listOf(St.P, St.R1, St.A2, St.R2, St.B1, St.R3, St.C1), entry.statesToEnter)
        assertEquals(listOf(St.R2, St.R3), entry.statesForDefaultEntry.sorted())
    }

    @Test
    fun anInternalTransitionOnARegionRootStaysInsideItsRegion() {
        val entry = Microstep.computeEntrySet(Chart(), listOf(enteringInternal(St.R1, toState(St.A2))))
        assertEquals(listOf(St.A2), entry.statesToEnter)
    }

    @Test
    fun transitionsOfOneMicrostepEnterInDocumentOrder() {
        // Selected second-region first, as a set is free to be; entry order is
        // document order all the same.
        val entry = Microstep.computeEntrySet(
            Chart(),
            listOf(entering(St.B1, toState(St.B2)), entering(St.A1, toState(St.A2))),
        )
        assertEquals(listOf(St.A2, St.B2), entry.statesToEnter)
    }

    @Test
    fun aDeepMultiTargetInitialDefaultsOnlyTheStateThatNamesIt() {
        val entry = Microstep.computeEntrySet(Chart(), listOf(entering(St.T, toState(St.M))))
        assertEquals(listOf(St.S, St.M, St.Mp, St.M1, St.M1a, St.M2, St.M2b), entry.statesToEnter)
        // m is the target and defaults; m1 and m2 are ancestors of its initial
        // targets, and s an ancestor of the target itself.
        assertEquals(listOf(St.M), entry.statesForDefaultEntry)
    }

    @Test
    fun aTargetlessTransitionEntersNothing() {
        val entry = Microstep.computeEntrySet(Chart(), listOf(entering(St.A1)))
        assertEquals(emptyList<St>(), entry.statesToEnter)
        assertEquals(emptyList<St>(), entry.statesForDefaultEntry)
    }

    @Test
    fun theDomainIsAskedOfTheStatesAHistoryStandsFor() {
        val chart = Chart()
        // Unrecorded, hs stands for "a2 b2", whose least compound ancestor with
        // t is the document itself.
        val unrecorded = Microstep.effectiveTargetStates(chart, listOf(toHistory(Hist.Hs)))
        assertEquals(listOf(St.A2, St.B2), unrecorded)
        assertNull(
            Microstep.transitionDomain(chart, entering(St.T), unrecorded),
            "the domain of t -> hs is the <scxml> element",
        )

        // Recorded inside q, an internal transition on q keeps q as its domain.
        chart.recorded[Hist.Hs] = listOf(St.Q1)
        val recorded = Microstep.effectiveTargetStates(chart, listOf(toHistory(Hist.Hs)))
        assertEquals(listOf(St.Q1), recorded)
        assertEquals(St.Q, Microstep.transitionDomain(chart, enteringInternal(St.Q), recorded))
    }

    // ════════════════════════════════════════════════════════════════════════
    // isDescendant
    // ════════════════════════════════════════════════════════════════════════

    @Test
    fun aChildAndAGrandchildAreDescendants() {
        val chart = Chart()
        for ((state, ancestor) in listOf(St.A1 to St.R1, St.A1 to St.P, St.M2b to St.S)) {
            assertTrue(Microstep.isDescendant(chart, state, ancestor), "$state lies below $ancestor")
        }
    }

    @Test
    fun aStateIsNotItsOwnDescendant() {
        // Appendix D's isDescendant is strict.
        assertFalse(Microstep.isDescendant(Chart(), St.R1, St.R1))
    }

    @Test
    fun statesOnUnrelatedBranchesAreNotDescendants() {
        val chart = Chart()
        for ((state, ancestor) in listOf(St.A1 to St.R2, St.Q1 to St.P, St.T to St.S)) {
            assertFalse(Microstep.isDescendant(chart, state, ancestor), "$state does not lie below $ancestor")
        }
    }

    // ════════════════════════════════════════════════════════════════════════
    // Domains and exit sets
    // ════════════════════════════════════════════════════════════════════════

    @Test
    fun aSelfTransitionExitsOnlyItsSource() {
        val chart = atInitialConfiguration()
        // §scxml-D-findLCCA chooses among the PROPER ancestors, so the domain of
        // a1 -> a1 is r1, and the exit set is a1 alone. A source that were its
        // own domain would exit nothing, and the self-transition would not leave
        // and re-enter its state at all.
        assertEquals(listOf(St.A1), Microstep.computeExitSet(chart, enabled(St.A1, 4), chart.configuration))
    }

    @Test
    fun anExternalTransitionLeavingAParallelExitsEveryRegion() {
        val chart = atInitialConfiguration()
        // p -> t: the domain is the <scxml> element, so the whole configuration
        // goes — the sibling regions of p included, which no walk up from the
        // source alone could name.
        val exitSet = Microstep.computeExitSet(chart, enabled(St.P, 0), chart.configuration)
        assertEquals(chart.configuration.sorted(), exitSet.sorted())
    }

    @Test
    fun anInternalTransitionFromARegionRootExitsOnlyInsideTheRegion() {
        val chart = atInitialConfiguration()
        val internal = EnabledTransition(St.R1, listOf(toState(St.A2)), 0, hasActions = false, isInternal = true)
        val external = internal.copy(isInternal = false)
        assertEquals(listOf(St.A1), Microstep.computeExitSet(chart, internal, chart.configuration))
        // Written external, the same transition's domain is s — a <parallel> is
        // never a domain — so every region goes with it.
        assertEquals(
            listOf(St.P, St.R1, St.A1, St.R2, St.B1, St.R3, St.C1),
            Microstep.computeExitSet(chart, external, chart.configuration).sorted(),
        )
    }

    @Test
    fun aTargetlessTransitionExitsNothing() {
        val chart = atInitialConfiguration()
        assertEquals(emptyList<St>(), Microstep.computeExitSet(chart, enabled(St.A1, 2), chart.configuration))
    }

    @Test
    fun theStatesToExitComeOutInReverseDocumentOrder() {
        val chart = atInitialConfiguration()
        assertEquals(
            listOf(St.C1, St.R3, St.B1, St.R2, St.A1, St.R1, St.P, St.S),
            Microstep.computeStatesToExit(chart, listOf(enabled(St.P, 0)), chart.configuration),
        )
    }

    // ════════════════════════════════════════════════════════════════════════
    // selectTransitions / removeConflictingTransitions
    // ════════════════════════════════════════════════════════════════════════

    @Test
    fun everyRegionTakesItsOwnTransition() {
        // §scxml-3.4: two regions' transitions have domains in disjoint
        // subtrees, so their exit sets are disjoint and both survive.
        val chart = atInitialConfiguration()
        assertEquals(listOf(St.A1 to 0, St.B1 to 0), selected(Microstep.selectTransitions(chart, Ev.E)))
    }

    @Test
    fun aTransitionSelectedFirstPreemptsALaterOneThatIsNotItsDescendant() {
        // a1 selects a1 -> a2; b1 walks up to p -> t, which exits a1 too, and p
        // does not descend from a1: it is preempted.
        val chart = atInitialConfiguration()
        assertEquals(listOf(St.A1 to 1), selected(Microstep.selectTransitions(chart, Ev.F)))
    }

    @Test
    fun aDescendantSourcePreemptsAnAncestorSelectedBeforeIt() {
        // a1 walks up to p -> t first; b1 then selects b1 -> b2, which conflicts
        // with it and descends from p, so it wins. c1 reaches p -> t again — one
        // transition, already in the set.
        val chart = atInitialConfiguration()
        assertEquals(listOf(St.B1 to 1), selected(Microstep.selectTransitions(chart, Ev.G)))
    }

    @Test
    fun aTargetlessTransitionIsNeverPreempted() {
        // W3C test 403c: an empty exit set conflicts with nothing, so a1's
        // targetless transition survives the p -> t that exits a1 itself.
        val chart = atInitialConfiguration()
        assertEquals(listOf(St.A1 to 2, St.P to 2), selected(Microstep.selectTransitions(chart, Ev.H)))
    }

    @Test
    fun aTransitionReachedFromTwoRegionsIsSelectedOnce() {
        // In the initial configuration only c1 walks up to p's targetless K — a1
        // and b1 answer K themselves. Move the first two regions to a2 and b2,
        // which answer nothing, and all three atomic states reach it: it is
        // still one element of the set.
        val chart = atInitialConfiguration()
        assertEquals(
            listOf(St.A1 to 3, St.B1 to 2, St.P to 3),
            selected(Microstep.selectTransitions(chart, Ev.K)),
        )
        chart.configuration = mutableListOf(St.S, St.P, St.R1, St.A2, St.R2, St.B2, St.R3, St.C1)
        assertEquals(listOf(St.P to 3), selected(Microstep.selectTransitions(chart, Ev.K)))
    }

    @Test
    fun anEventlessSelectionWithNothingEnabledSelectsNothing() {
        assertEquals(emptyList<Pair<St, Int>>(), selected(Microstep.selectTransitions(atInitialConfiguration(), null)))
    }

    // ════════════════════════════════════════════════════════════════════════
    // The microstep
    // ════════════════════════════════════════════════════════════════════════

    @Test
    fun aMicrostepExitsThenRunsContentInSelectionOrderThenEnters() {
        val chart = atInitialConfiguration()
        Microstep.microstep(chart, Microstep.selectTransitions(chart, Ev.K))
        // §scxml-D-executeTransitionContent runs content in SELECTION order: p's
        // K was reached last, from c1, though p comes first in document order.
        assertEquals(
            listOf("exit B1", "exit A1", "content A1#3", "content B1#2", "content P#3", "enter A2", "enter B2"),
            chart.log,
        )
    }

    @Test
    fun everyExitIsHandedTheConfigurationBeforeTheFirstExit() {
        val chart = atInitialConfiguration()
        val before = chart.configuration.toList()
        // K exits two states, b1 then a1: the second exit must still be handed
        // the configuration from before the first — a microstep with one exit
        // could not tell that from the configuration as it stands at each exit.
        Microstep.microstep(chart, Microstep.selectTransitions(chart, Ev.K))
        assertEquals(listOf(before, before), chart.exitsSaw)
    }

    @Test
    fun aSelfTransitionLeavesAndReentersItsState() {
        val chart = atInitialConfiguration()
        val taken = Microstep.selectTransitions(chart, Ev.R)
        assertEquals(listOf(St.A1 to 4, St.B1 to 3), selected(taken))
        Microstep.microstep(chart, taken)
        assertEquals(
            listOf("exit B1", "exit A1", "content A1#4", "content B1#3", "enter A1", "enter B2"),
            chart.log,
        )
    }

    @Test
    fun aHistoryDefaultContentRunsAfterItsParentIsEntered() {
        val chart = atInitialConfiguration()
        val toHb = EnabledTransition(St.A1, listOf(toHistory(Hist.Hb)), 0, hasActions = false, isInternal = false)
        Microstep.microstep(chart, listOf(toHb))
        // The domain is s, so all of p leaves and comes back; r2 is an ancestor
        // of the default target b2, not entered by default, and the history's
        // default content runs once r2 has been entered. The transition itself
        // has no content, so the machine is not asked to run any.
        assertEquals(
            listOf(
                "exit C1",
                "exit R3",
                "exit B1",
                "exit R2",
                "exit A1",
                "exit R1",
                "exit P",
                "enter P",
                "enter R1 (default)",
                "enter A1",
                "enter R2",
                "history default Hb",
                "enter B2",
                "enter R3 (default)",
                "enter C1",
            ),
            chart.log,
        )
    }

    // ════════════════════════════════════════════════════════════════════════
    // isInFinalState
    // ════════════════════════════════════════════════════════════════════════
    //
    //	run (parallel)
    //	  form > filling, formDone (final)
    //	  checks (parallel)
    //	    left  > leftPending,  leftDone (final)
    //	    right > rightPending, rightDone (final)
    //
    // checks is a <parallel> among the regions of a <parallel>: it has no
    // <final> child of its own and is final only because both of its regions
    // are.

    @Test
    fun aCompoundStateIsFinalWhenAFinalChildIsActive() {
        assertTrue(
            Microstep.isInFinalState(
                Completion, Cs.Form,
                listOf(Cs.Run, Cs.Form, Cs.FormDone, Cs.Checks, Cs.Left, Cs.LeftPending, Cs.Right, Cs.RightPending),
            ),
            "form is in a final state while formDone is active",
        )
        assertFalse(
            Microstep.isInFinalState(
                Completion, Cs.Form,
                listOf(Cs.Run, Cs.Form, Cs.Filling, Cs.Checks, Cs.Left, Cs.LeftPending, Cs.Right, Cs.RightPending),
            ),
            "form is not in a final state while filling is active",
        )
    }

    @Test
    fun aNestedParallelIsFinalOnlyWhenEveryRegionIs() {
        assertTrue(
            Microstep.isInFinalState(
                Completion, Cs.Checks,
                listOf(Cs.Checks, Cs.Left, Cs.LeftDone, Cs.Right, Cs.RightDone),
            ),
            "checks is final once both of its regions are",
        )
        assertFalse(
            Microstep.isInFinalState(
                Completion, Cs.Checks,
                listOf(Cs.Checks, Cs.Left, Cs.LeftDone, Cs.Right, Cs.RightPending),
            ),
            "one region still pending leaves the <parallel> short of final",
        )
    }

    @Test
    fun aParallelRegionOfAParallelCounts() {
        assertTrue(
            Microstep.isInFinalState(
                Completion, Cs.Run,
                listOf(Cs.Run, Cs.Form, Cs.FormDone, Cs.Checks, Cs.Left, Cs.LeftDone, Cs.Right, Cs.RightDone),
            ),
            "checks has no <final> child of its own; it is final because both of its regions are",
        )
        assertFalse(
            Microstep.isInFinalState(
                Completion, Cs.Run,
                listOf(Cs.Run, Cs.Form, Cs.FormDone, Cs.Checks, Cs.Left, Cs.LeftDone, Cs.Right, Cs.RightPending),
            ),
            "run is not final while one region of checks is pending",
        )
    }

    @Test
    fun neitherAnAtomicStateNorAFinalElementIsInAFinalState() {
        val configuration =
            listOf(Cs.Run, Cs.Form, Cs.FormDone, Cs.Checks, Cs.Left, Cs.LeftPending, Cs.Right, Cs.RightPending)
        assertFalse(
            Microstep.isInFinalState(Completion, Cs.LeftPending, configuration),
            "an atomic state has no <final> child to be active",
        )
        assertFalse(
            Microstep.isInFinalState(Completion, Cs.FormDone, configuration),
            "being a <final> element is not being IN a final state — the predicate asks about children",
        )
    }
}

private enum class Cs { Run, Form, Filling, FormDone, Checks, Left, LeftPending, LeftDone, Right, RightPending, RightDone }

private val completionParent: Map<Cs, Cs> = mapOf(
    Cs.Form to Cs.Run, Cs.Checks to Cs.Run,
    Cs.Filling to Cs.Form, Cs.FormDone to Cs.Form,
    Cs.Left to Cs.Checks, Cs.Right to Cs.Checks,
    Cs.LeftPending to Cs.Left, Cs.LeftDone to Cs.Left,
    Cs.RightPending to Cs.Right, Cs.RightDone to Cs.Right,
)

private val completionChildren: Map<Cs, List<Cs>> = mapOf(
    Cs.Run to listOf(Cs.Form, Cs.Checks),
    Cs.Form to listOf(Cs.Filling, Cs.FormDone),
    Cs.Checks to listOf(Cs.Left, Cs.Right),
    Cs.Left to listOf(Cs.LeftPending, Cs.LeftDone),
    Cs.Right to listOf(Cs.RightPending, Cs.RightDone),
)

/** The completion document above; it declares no `<history>`. */
private object Completion : Document<Cs, HistoryId> {
    override fun parentOf(state: Cs): Cs? = completionParent[state]

    override fun isCompound(state: Cs): Boolean = state == Cs.Form || state == Cs.Left || state == Cs.Right

    override fun isParallel(state: Cs): Boolean = state == Cs.Run || state == Cs.Checks

    override fun isFinal(state: Cs): Boolean = state == Cs.FormDone || state == Cs.LeftDone || state == Cs.RightDone

    override fun childStates(state: Cs): List<Cs> = completionChildren[state] ?: emptyList()

    override fun initialTargets(state: Cs): List<EntryTarget<Cs, HistoryId>> = emptyList()

    override fun historyParent(history: HistoryId): Cs =
        error("the completion document declares no <history>; asked for $history")

    override fun historyValue(history: HistoryId): List<Cs>? = null

    override fun historyDefaultTargets(history: HistoryId): List<EntryTarget<Cs, HistoryId>> = emptyList()

    override fun documentOrder(state: Cs): Int = state.ordinal
}
