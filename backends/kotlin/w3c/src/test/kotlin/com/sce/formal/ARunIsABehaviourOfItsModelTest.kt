// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A run of the Kotlin lane, recorded as the trace `sce-codegen formal traces`
// judges a behaviour of the document's model (docs/adr/0115, D4 and D8). The
// twin of the Rust lane's `a_run_is_a_behaviour_of_its_model`
// (backends/rust/tests) and the C11 lane's `c11_a_run_of_test_*` programs.
//
// Each test runs one W3C document that `docs/formal` has a model of
// (`W3CTest388`, `W3CTest570`, `W3CTest403c` over `AppendixD.tla`), the machine
// this module's conformance class of it runs, with the engine's trace port set,
// and holds what the port was told to the recorded run `docs/formal/traces.json`
// declares of that model, byte for byte. The judge reads the recorded file, so a
// run that moves is seen here first, and a recorded run that no longer is the
// lane's is never judged in its place. A Kotlin run of a document is judged
// against the same generated model as the Rust run of it, and these documents
// leave a run's steps no choice, so the recorded runs of the lanes differ only
// in their lane, or the judge refuses one.
//
// Recording again, after a change that moves a run on purpose:
//   UPDATE_EXPECT=1 ./gradlew :sce-kotlin-w3c:test --tests 'com.sce.formal.*'
// then `sce-codegen formal traces --manifest docs/formal/traces.json`, and read
// the diff of the recorded runs.

package com.sce.formal

import com.sce.generated.test388.Test388State
import com.sce.generated.test388.Test388StateMachine
import com.sce.generated.test403c.Test403cState
import com.sce.generated.test403c.Test403cStateMachine
import com.sce.generated.test570.Test570State
import com.sce.generated.test570.Test570StateMachine
import com.sce.runtime.Event
import com.sce.runtime.RunTrace
import com.sce.runtime.State
import com.sce.runtime.StateMachineEngine
import com.sce.runtime.TraceStepKind
import com.sce.w3c.W3CTestBase
import com.sce.w3c.driveOnVirtualTime
import com.sce.w3c.initializeOnVirtualTime
import java.io.File
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

@DisplayName("A run of the Kotlin lane is a behaviour of its model (docs/adr/0115, D4 and D8)")
class ARunIsABehaviourOfItsModelTest {

    /**
     * The trace port's account of one run of [machine] to the end, and the
     * top-level final state the run ended in.
     */
    private fun <S : State, E : Event> tracedRun(machine: StateMachineEngine<S, E>): Pair<RunTrace, S?> {
        val trace = RunTrace()
        machine.tracePort = { step -> trace.push(step) }
        try {
            machine.initializeOnVirtualTime()
            assertTrue(machine.driveOnVirtualTime(), "the run reached no final state")
            return trace to machine.terminalState
        } finally {
            machine.tracePort = null
            machine.cleanup()
        }
    }

    /** The recorded run of [model]'s document by this lane, as the traces manifest names it. */
    private fun recorded(model: String): File = File("docs/formal/traces/$model.kotlin.json")

    /**
     * Hold [trace] to the recorded run of [model], or record it again when
     * `UPDATE_EXPECT` is set.
     */
    private fun holdsToTheRecordedRun(model: String, trace: RunTrace) {
        val text = trace.toJson()
        val file = recorded(model)
        if (System.getenv("UPDATE_EXPECT") != null) {
            file.writeBytes(text.encodeToByteArray())
            return
        }
        assertTrue(
            file.isFile,
            "${file.path} cannot be read: record it with `UPDATE_EXPECT=1 ./gradlew " +
                ":sce-kotlin-w3c:test --tests 'com.sce.formal.*'`"
        )
        assertEquals(
            file.readBytes().decodeToString(),
            text,
            "the run of $model is not the recorded one that `sce-codegen formal traces` judges"
        )
    }

    /**
     * The run starts, ends in its pass state, and the interpreter's exit leaves
     * that state alone: the first and the last step a recorded run must hold.
     */
    private fun startsAndStops(trace: RunTrace) {
        val steps = trace.steps
        assertEquals(TraceStepKind.START, steps.firstOrNull()?.kind, "the run begins with a start step")
        val last = steps.last()
        assertEquals(TraceStepKind.STOP, last.kind, "the run ends with a stop step")
        assertEquals("pass", last.outcome, "the stop names the run's outcome")
        assertEquals(listOf("pass"), last.exited, "the stop exits the state the run ended in alone")
    }

    @Test
    fun aRunOfTest388IsTheRecordedRunOfItsModel() {
        val (trace, ended) = tracedRun(Test388StateMachine(W3CTestBase.createEngine()))
        assertEquals(Test388State.Pass, ended)
        startsAndStops(trace)
        holdsToTheRecordedRun("W3CTest388", trace)
    }

    @Test
    fun aRunOfTest570IsTheRecordedRunOfItsModel() {
        val (trace, ended) = tracedRun(Test570StateMachine(W3CTestBase.createEngine()))
        assertEquals(Test570State.Pass, ended)
        startsAndStops(trace)
        holdsToTheRecordedRun("W3CTest570", trace)
    }

    @Test
    fun aRunOfTest403cIsTheRecordedRunOfItsModel() {
        val (trace, ended) = tracedRun(Test403cStateMachine(W3CTestBase.createEngine()))
        assertEquals(Test403cState.Pass, ended)
        startsAndStops(trace)
        holdsToTheRecordedRun("W3CTest403c", trace)
    }

    /**
     * A port set and then cleared is told nothing, and the run is the one an
     * engine that never had one makes.
     */
    @Test
    fun aClearedPortIsToldNothingAndTheRunIsTheSame() {
        var told = 0
        val machine = Test388StateMachine(W3CTestBase.createEngine())
        machine.tracePort = { told++ }
        machine.tracePort = null
        try {
            machine.initializeOnVirtualTime()
            assertTrue(machine.driveOnVirtualTime(), "the run reached no final state")
            assertEquals(Test388State.Pass, machine.terminalState)
        } finally {
            machine.cleanup()
        }
        assertEquals(0, told, "a cleared port was told $told steps")
    }
}
