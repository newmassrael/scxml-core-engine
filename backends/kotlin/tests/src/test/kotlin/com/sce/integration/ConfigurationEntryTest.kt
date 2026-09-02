// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.11 — what `StateMachineEngine.enterAt` accepts, and what it
// refuses, on the Kotlin engine.
//
// The door exists so a host can bring a machine back where it was, in a new
// process, without replaying the entry actions the earlier run already ran.
// Refusals are the part that has to be enumerated rather than sampled:
// entering "near" the requested configuration is the one outcome this door
// must never produce, because nothing afterwards can detect it — the machine
// reports a current state, `activeConfiguration` answers, and the set behind
// those answers is one the document never describes. A gate holding only the
// accepting case would pass on an engine that accepted everything.
//
// The Kotlin sibling of `backends/rust/runtime/tests/configuration_entry.rs`,
// `tests/integration/ConfigurationEntryAotTest.cpp`,
// `backends/go/tests/configuration_entry/` and
// `backends/python/tests/configuration_entry/`, asking the same questions of
// the same rules, so a set one engine accepts is one the others accept.
//
// Two machines, because the two halves of the door are different code paths:
//
//   - `parallel_regions_take_own_transitions` has `<parallel>` regions, so its
//     configuration holds more than one leaf and the recorded current state is
//     not recoverable from the set alone.
//   - `statechart_native_action` has no regions and no script engine at all,
//     so it is the path where the §scxml-5.3 declaration hook is a no-op.
//
// This file is deliberately not a fixture stem's driver: it drives documents
// that already exist in the tree rather than adding a document of its own,
// because the claim is about a runtime door and not about a topology.

package com.sce.integration

import com.sce.integration.parallel_regions_take_own_transitions.ParallelRegionsTakeOwnTransitionsEvent
import com.sce.integration.parallel_regions_take_own_transitions.ParallelRegionsTakeOwnTransitionsState
import com.sce.integration.parallel_regions_take_own_transitions.ParallelRegionsTakeOwnTransitionsStateMachine
import com.sce.integration.statechart_native_action.StatechartNativeActionActions
import com.sce.integration.statechart_native_action.StatechartNativeActionState
import com.sce.integration.statechart_native_action.StatechartNativeActionStateMachine
import com.sce.runtime.ConfigurationRejection
import com.sce.w3c.W3CTestBase
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertNotNull
import org.junit.jupiter.api.Assertions.assertNull
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

/// W3C SCXML 3.11 — the resume seam: what a restored configuration may be.
@DisplayName("ConfigurationEntry — W3C SCXML 3.11")
class ConfigurationEntryTest {

    private fun newParallel() =
        ParallelRegionsTakeOwnTransitionsStateMachine(W3CTestBase.createEngine())

    /// A mid-run configuration of the parallel document: both regions live, the
    /// deeper one in `working` and the shallower in `within`. Written out rather
    /// than taken from a live run because every refusal below is a MUTATION of
    /// it — one change each, so a refusal names one rule.
    private fun atWork() = listOf(
        ParallelRegionsTakeOwnTransitionsState.Run,
        ParallelRegionsTakeOwnTransitionsState.Drive,
        ParallelRegionsTakeOwnTransitionsState.Running,
        ParallelRegionsTakeOwnTransitionsState.Working,
        ParallelRegionsTakeOwnTransitionsState.Budget,
        ParallelRegionsTakeOwnTransitionsState.Within,
    )

    /// The host for the linear machine. Its every effect is a `<sce:action>`, so
    /// it cannot be constructed without one — which is the point of that seam
    /// and merely plumbing here, except for the two counters, which are what
    /// says no entry or exit content ran during a resume.
    private class SilentActions : StatechartNativeActionActions {
        var idleEntries = 0
        var assemblingExits = 0

        override fun appendFragmentPayload(payload: ByteArray, offset: UInt) {}

        override fun resetSlot() {}

        override fun onIdleEntry() {
            idleEntries++
        }

        override fun onAssemblingExit() {
            assemblingExits++
        }
    }

    // The set written above is a configuration of the document, so it is
    // accepted and the machine comes back holding exactly it. This is the
    // baseline every refusal below is one mutation away from — without it, a
    // validator that refused everything would pass every other case here.
    @Test
    fun aParallelConfigurationIsAccepted() {
        val sm = newParallel()
        val configuration = atWork()

        assertEquals(
            ConfigurationRejection.NONE,
            sm.enterAt(configuration, ParallelRegionsTakeOwnTransitionsState.Working),
            "a configuration of the document was refused",
        )
        assertEquals(
            configuration.toSet(),
            sm.activeConfiguration,
            "the machine came back holding a different set from the one it was handed",
        )
        assertEquals(
            ParallelRegionsTakeOwnTransitionsState.Working,
            sm.currentState.value,
            "this engine stores the leaf it is given, so a host that journalled it gets it back",
        )
    }

    // A machine with no `<parallel>` has one leaf, so its configuration is that
    // leaf and its ancestors. The round trip has to close there too, through a
    // set with a single member and a datamodel hook that does nothing.
    @Test
    fun aLinearConfigurationRoundTrips() {
        val host = SilentActions()
        val sm = StatechartNativeActionStateMachine(host)

        assertEquals(
            ConfigurationRejection.NONE,
            sm.enterAt(
                listOf(StatechartNativeActionState.Assembling),
                StatechartNativeActionState.Assembling,
            ),
            "a single-state configuration was refused",
        )
        assertEquals(
            setOf(StatechartNativeActionState.Assembling),
            sm.activeConfiguration,
        )
        assertEquals(StatechartNativeActionState.Assembling, sm.currentState.value)
        assertEquals(0, host.idleEntries, "entry content ran during a resume")
        assertEquals(0, host.assemblingExits, "exit content ran during a resume")
    }

    @Test
    fun anEmptyConfigurationIsRefused() {
        val sm = newParallel()
        assertEquals(
            ConfigurationRejection.EMPTY,
            sm.enterAt(emptyList(), ParallelRegionsTakeOwnTransitionsState.Working),
            "a machine is never in nothing",
        )
    }

    // W3C SCXML 3.11: a compound state holds exactly one active child. `working`
    // and `judging` are both children of `running`, and a run stands in one.
    @Test
    fun twoSiblingsOfOneRegionAreRefused() {
        val sm = newParallel()
        val configuration = atWork() + ParallelRegionsTakeOwnTransitionsState.Judging

        assertEquals(
            ConfigurationRejection.COMPOUND_CHILD_COUNT,
            sm.enterAt(configuration, ParallelRegionsTakeOwnTransitionsState.Working),
            "`running` was given two active children, which is a configuration the " +
                "document has no reading for",
        )
    }

    // W3C SCXML 3.11: a `<parallel>` holds EVERY region. Dropping one is the
    // shape a host produces when it journals only the region it cares about.
    @Test
    fun aParallelWithARegionMissingIsRefused() {
        val sm = newParallel()
        val configuration = listOf(
            ParallelRegionsTakeOwnTransitionsState.Run,
            ParallelRegionsTakeOwnTransitionsState.Drive,
            ParallelRegionsTakeOwnTransitionsState.Running,
            ParallelRegionsTakeOwnTransitionsState.Working,
        )

        assertEquals(
            ConfigurationRejection.PARALLEL_REGION_MISSING,
            sm.enterAt(configuration, ParallelRegionsTakeOwnTransitionsState.Working),
            "`budget` is a region of `run` and a run is always in both at once",
        )
    }

    // The set has to be ancestor-closed: a state is active only if its parent is.
    @Test
    fun aConfigurationThatSkipsAnAncestorIsRefused() {
        val sm = newParallel()
        val configuration = listOf(
            ParallelRegionsTakeOwnTransitionsState.Run,
            ParallelRegionsTakeOwnTransitionsState.Drive,
            ParallelRegionsTakeOwnTransitionsState.Working,
            ParallelRegionsTakeOwnTransitionsState.Budget,
            ParallelRegionsTakeOwnTransitionsState.Within,
        )

        assertEquals(
            ConfigurationRejection.ANCESTOR_MISSING,
            sm.enterAt(configuration, ParallelRegionsTakeOwnTransitionsState.Working),
            "`working` is a child of `running`, which the set does not hold",
        )
    }

    // Checked before the arity counts, because a duplicate would otherwise read
    // as a second child and the refusal would name the wrong rule.
    @Test
    fun aRepeatedStateIsRefused() {
        val sm = newParallel()
        val configuration = atWork() + ParallelRegionsTakeOwnTransitionsState.Working

        assertEquals(
            ConfigurationRejection.DUPLICATE,
            sm.enterAt(configuration, ParallelRegionsTakeOwnTransitionsState.Working),
        )
    }

    // W3C SCXML 3.11: a configuration closes on exactly one root. `settled` is a
    // top-level `<final>`, so a set holding both it and `run` describes two
    // machines.
    @Test
    fun twoRootsAreRefused() {
        val sm = newParallel()
        val configuration = atWork() + ParallelRegionsTakeOwnTransitionsState.Settled

        assertEquals(
            ConfigurationRejection.ROOT_COUNT,
            sm.enterAt(configuration, ParallelRegionsTakeOwnTransitionsState.Working),
        )
    }

    @Test
    fun aCurrentStateOutsideTheConfigurationIsRefused() {
        val sm = newParallel()
        assertEquals(
            ConfigurationRejection.CURRENT_NOT_ACTIVE,
            sm.enterAt(atWork(), ParallelRegionsTakeOwnTransitionsState.Judging),
            "the current state is the one the machine is standing in, so it is in the " +
                "set by definition",
        )
    }

    // W3C SCXML 3.11 makes the current state the ATOMIC state the engine
    // descended to. A compound one is the shape a host produces when it
    // journals the ancestor rather than the leaf.
    @Test
    fun aNonAtomicCurrentStateIsRefused() {
        val sm = newParallel()
        assertEquals(
            ConfigurationRejection.CURRENT_NOT_ATOMIC,
            sm.enterAt(atWork(), ParallelRegionsTakeOwnTransitionsState.Running),
        )
    }

    // The claim that makes every refusal above safe to act on: validation runs
    // BEFORE any mutation, so a host that gets a rejection still holds the
    // machine it had. Without this the door could half-enter, and a host reading
    // a rejection would be told nothing happened while the engine had moved.
    @Test
    fun aRefusedEntryLeavesTheEngineUntouched() {
        val sm = newParallel()
        val before = sm.currentState.value

        assertEquals(
            ConfigurationRejection.EMPTY,
            sm.enterAt(emptyList(), ParallelRegionsTakeOwnTransitionsState.Working),
        )

        assertEquals(before, sm.currentState.value, "a refused entry moved the current state")
        assertTrue(sm.activeConfiguration.isEmpty(), "a refused entry wrote an active set")
    }

    // W3C SCXML 3.3: every state this document declares reads back from its own
    // name.
    //
    // A host can only record a configuration as TEXT — the generated state
    // objects are a build artefact of one process, and the process that resumes
    // is a different one. This walks the sealed interface's own members rather
    // than a list spelled here, so a document that grows a state grows this
    // check with it.
    @Test
    fun everyStateReadsBackFromItsOwnName() {
        val sm = newParallel()
        val states = listOf(
            ParallelRegionsTakeOwnTransitionsState.Budget,
            ParallelRegionsTakeOwnTransitionsState.Drive,
            ParallelRegionsTakeOwnTransitionsState.Judging,
            ParallelRegionsTakeOwnTransitionsState.Run,
            ParallelRegionsTakeOwnTransitionsState.Running,
            ParallelRegionsTakeOwnTransitionsState.Settled,
            ParallelRegionsTakeOwnTransitionsState.Within,
            ParallelRegionsTakeOwnTransitionsState.Working,
        )

        for (state in states) {
            val name = sm.nameOfState(state)
            assertTrue(name.isNotEmpty(), "a state of this document publishes no name")
            val back = sm.stateOfName(name)
            assertNotNull(back, "'$name' is a name this machine publishes and it could not read it back")
            assertEquals(state, back, "'$name' read back as a different state")
        }

        assertNull(
            sm.stateOfName("a-state-this-document-does-not-declare"),
            "a name the document does not carry was answered with a state rather than " +
                "refused; a name guessed at is how a restore reaches a configuration " +
                "nobody recorded",
        )
    }

    // A configuration that crossed a process: journalled as names, read back
    // through the generated reverse table, and handed to the door. This is the
    // whole point of the pair — the two halves in one call chain rather than
    // each proved alone.
    @Test
    fun aConfigurationJournalledAsNamesIsAcceptedBack() {
        val writer = newParallel()
        writer.initialize()
        writer.send(ParallelRegionsTakeOwnTransitionsEvent.E)
        writer.tick()

        val journal = writer.activeConfiguration.map { writer.nameOfState(it) }
        val currentName = writer.nameOfState(writer.currentState.value)

        val reader = newParallel()
        val configuration = journal.map { name ->
            val state = reader.stateOfName(name)
            assertNotNull(state, "the journal names '$name' and the reader could not read it back")
            state!!
        }
        val current = reader.stateOfName(currentName)
        assertNotNull(current)

        assertEquals(
            ConfigurationRejection.NONE,
            reader.enterAt(configuration, current!!),
            "a configuration a run actually reached was refused on the way back",
        )
        assertEquals(configuration.toSet(), reader.activeConfiguration)
        assertEquals(current, reader.currentState.value)
    }
}
