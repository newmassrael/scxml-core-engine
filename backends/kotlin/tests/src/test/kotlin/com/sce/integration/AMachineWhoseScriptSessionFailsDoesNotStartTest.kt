// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// docs/adr/0096 — Kotlin AOT path: a generated machine whose script session
// cannot be established does not start, and its host is told why.
//
// The script engine is a double over the Lua engine the committed machines are
// generated for. It fails, on request, one of the three steps that establish a
// session — creating it, loading the shared library into it, binding the
// system variables — and counts every call that runs a document's expressions,
// so "no executable content ran" is read from the engine the machine runs its
// content in.
//
// Fixture: integration_resources/a_child_may_send_many_events_in_one_tick/a_child_may_send_many_events_in_one_tick.scxml,
// reused for its shape: a parent in `s0` with `<data ticks>` that invokes an
// inline child, whose `<onentry>` sends `tick` to the parent 110 times. The
// child declares `<data>` of its own, so it has a script session of its own to
// fail. A start that ran would initialize `ticks`, enter `s0` and start the
// child, asking the engine for the child's session; a child that ran would
// send `tick`.
//
// Regeneration: scripts/regen_a_child_may_send_many_events_in_one_tick_kotlin.sh

package com.sce.integration

import com.sce.integration.a_child_may_send_many_events_in_one_tick.AChildMaySendManyEventsInOneTickEvent
import com.sce.integration.a_child_may_send_many_events_in_one_tick.AChildMaySendManyEventsInOneTickState
import com.sce.integration.a_child_may_send_many_events_in_one_tick.AChildMaySendManyEventsInOneTickStateMachine
import com.sce.runtime.ConfigurationRejection
import com.sce.runtime.EventMetadata
import com.sce.runtime.InboxInput
import com.sce.runtime.IoProcessorDescriptor
import com.sce.runtime.MeshCall
import com.sce.runtime.PayloadReading
import com.sce.runtime.RootStartRefusal
import com.sce.runtime.ScriptEngineException
import com.sce.runtime.ScriptLibraryNotLoadedException
import com.sce.runtime.ScriptSessionStage
import com.sce.runtime.ScriptSessionUnavailableException
import com.sce.runtime.ScriptSource
import com.sce.runtime.ScxmlScriptEngine
import com.sce.runtime.SetCurrentEventArgs
import com.sce.scripting.lua.LuaScriptEngine
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertFalse
import org.junit.jupiter.api.Assertions.assertInstanceOf
import org.junit.jupiter.api.Assertions.assertNotEquals
import org.junit.jupiter.api.Assertions.assertNull
import org.junit.jupiter.api.Assertions.assertThrows
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

/** Which step of establishing a session the double fails. */
private enum class FailStep {
    /** The double is the Lua engine, counted. */
    NEVER,

    /** Every session's creation fails as the engine's own failure. */
    CREATE,

    /** Every session's creation fails as a shared library that did not load. */
    LIBRARY,

    /** Every bind of the system variables fails. */
    SYSTEM_VARIABLES,

    /**
     * The first session asked for, the parent's, is created; every later one, a
     * child's, fails to be.
     */
    CREATE_AFTER_FIRST,
}

/**
 * The Lua engine, with one step of establishing a session failing on request
 * and every call that runs the document's expressions counted. The members it
 * does not name are the Lua engine's, unchanged.
 */
private class SessionDouble(
    private val fail: FailStep,
    private val engine: ScxmlScriptEngine = LuaScriptEngine(),
) : ScxmlScriptEngine by engine {
    /** Every session id [createSession] was asked for, in order. */
    val asked = mutableListOf<String>()

    /** Every session id [destroySession] was asked for, in order. */
    val destroyed = mutableListOf<String>()

    /** The sessions this double created and has not destroyed. */
    val live = mutableSetOf<String>()

    /** The calls that ran a document's expressions or wrote its datamodel. */
    var ran = 0
        private set

    override fun createSession(sessionId: String) {
        asked += sessionId
        when {
            fail == FailStep.CREATE -> throw ScriptEngineException("the double creates no session")
            fail == FailStep.LIBRARY ->
                throw ScriptLibraryNotLoadedException("ECMAScript semantics", "the double loads no library")
            fail == FailStep.CREATE_AFTER_FIRST && asked.size > 1 ->
                throw ScriptEngineException("the double creates no second session")
            else -> {
                engine.createSession(sessionId)
                live += sessionId
            }
        }
    }

    override fun destroySession(sessionId: String) {
        destroyed += sessionId
        live -= sessionId
        engine.destroySession(sessionId)
    }

    override fun setupSystemVariables(
        sessionId: String,
        machineName: String,
        ioProcessors: List<IoProcessorDescriptor>,
    ) {
        if (fail == FailStep.SYSTEM_VARIABLES) throw ScriptEngineException("the double binds no system variable")
        engine.setupSystemVariables(sessionId, machineName, ioProcessors)
    }

    override fun evaluateExpr(sessionId: String, expr: String): Any? {
        ran++
        return engine.evaluateExpr(sessionId, expr)
    }

    override fun evaluateExpr(sessionId: String, expr: ScriptSource): Any? {
        ran++
        return engine.evaluateExpr(sessionId, expr)
    }

    override fun executeScript(sessionId: String, script: String) {
        ran++
        engine.executeScript(sessionId, script)
    }

    override fun executeScript(sessionId: String, script: ScriptSource) {
        ran++
        engine.executeScript(sessionId, script)
    }

    override fun evaluateCondition(sessionId: String, expr: ScriptSource): Boolean {
        ran++
        return engine.evaluateCondition(sessionId, expr)
    }

    override fun assign(sessionId: String, location: ScriptSource, expr: ScriptSource) {
        ran++
        engine.assign(sessionId, location, expr)
    }

    override fun setVariable(sessionId: String, name: String, value: Any?) {
        ran++
        engine.setVariable(sessionId, name, value)
    }

    override fun setCurrentEvent(sessionId: String, args: SetCurrentEventArgs): PayloadReading {
        ran++
        return engine.setCurrentEvent(sessionId, args)
    }
}

/** The fixture's machine over a double that fails as [fail] says, and the double, to read afterwards. */
private fun overDouble(fail: FailStep): Pair<AChildMaySendManyEventsInOneTickStateMachine, SessionDouble> {
    val double = SessionDouble(fail)
    return AChildMaySendManyEventsInOneTickStateMachine(double) to double
}

/**
 * A refused start left nothing behind: no state entered, no final state
 * reached, no content run, no session kept and no session id held for one; and
 * only the machine's own session was asked for, since entering `s0` would have
 * started its child and asked the engine for the child's.
 */
private fun assertNothingStarted(
    sm: AChildMaySendManyEventsInOneTickStateMachine,
    double: SessionDouble,
    fail: FailStep,
) {
    assertTrue(sm.activeConfiguration.isEmpty(), "$fail: a refused start enters no state, and ${sm.activeConfiguration} is active")
    assertNull(sm.terminalState, "$fail: a refused start reaches no final state")
    assertFalse(sm.isInFinalState, "$fail: a refused start has not ended")
    assertEquals(0, double.ran, "$fail: a refused start evaluates nothing (no <data>, no <onentry>, no cond)")
    assertTrue(double.live.isEmpty(), "$fail: a refused start keeps no session, and ${double.live} are live")
    assertEquals("", sm.sessionId, "$fail: a refused start holds no id for a session it does not have")
    assertTrue(double.asked.isNotEmpty(), "$fail: the start asked for its session")
    assertTrue(
        double.asked.none { it.contains(".") },
        "$fail: a refused start enters no state, and a child session was asked for: ${double.asked}",
    )
}

@DisplayName("A machine whose script session fails does not start (docs/adr/0096)")
class AMachineWhoseScriptSessionFailsDoesNotStartTest {

    @Test
    fun `a start over the double that fails nothing runs the fixture`() {
        // The control: the double fails nothing, so the machine is the fixture's,
        // and its child's ticks arrive. Without it the cases below could pass
        // over a double that breaks every machine.
        val (sm, double) = overDouble(FailStep.NEVER)
        try {
            sm.initialize()
            sm.send(AChildMaySendManyEventsInOneTickEvent.Finish)
            sm.tick()

            assertEquals(AChildMaySendManyEventsInOneTickState.Done, sm.terminalState, "`finish` carries the run to `done`")
            assertEquals(110L, sm.ticks(), "every tick the child sent arrived")
            assertTrue(double.ran > 0, "the machine ran its content through the double")
            assertEquals(2, double.asked.size, "the parent's session and its child's were asked for: ${double.asked}")
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun `a start whose session cannot be established is refused with the step`() {
        for ((fail, stage) in listOf(
            FailStep.CREATE to ScriptSessionStage.CREATE,
            FailStep.LIBRARY to ScriptSessionStage.LIBRARY,
            FailStep.SYSTEM_VARIABLES to ScriptSessionStage.SYSTEM_VARIABLES,
        )) {
            val (sm, double) = overDouble(fail)
            val refused = assertThrows(ScriptSessionUnavailableException::class.java) { sm.initialize() }
            assertEquals(stage, refused.failure.stage, "$fail: the refusal names the step that failed")
            assertEquals(
                "the script session could not be established at step '${stage.stepName}': ${refused.failure.message}",
                refused.message,
                "$fail: the refusal is the sentence every backend writes",
            )
            assertNothingStarted(sm, double, fail)
        }
    }

    @Test
    fun `a refused start is refused again rather than run without a session`() {
        // A session that failed is not marked established: the next start asks
        // for it again and is refused again, rather than finding it "done" and
        // running the machine over no session.
        for (fail in listOf(FailStep.CREATE, FailStep.LIBRARY, FailStep.SYSTEM_VARIABLES)) {
            val (sm, double) = overDouble(fail)
            assertThrows(ScriptSessionUnavailableException::class.java) { sm.initialize() }
            assertThrows(ScriptSessionUnavailableException::class.java) { sm.initialize() }
            assertEquals(2, double.asked.size, "$fail: each start asks for the session once: ${double.asked}")
            assertNothingStarted(sm, double, fail)
        }
    }

    @Test
    fun `a session whose system variables cannot be bound is released`() {
        val (sm, double) = overDouble(FailStep.SYSTEM_VARIABLES)
        assertThrows(ScriptSessionUnavailableException::class.java) { sm.initialize() }
        assertEquals(1, double.asked.size, "the machine asked for its own session once: ${double.asked}")
        assertEquals(
            double.asked,
            double.destroyed,
            "the session the failed bind was made in is destroyed",
        )
    }

    @Test
    fun `a refused machine takes no event`() {
        val (sm, double) = overDouble(FailStep.CREATE)
        assertThrows(ScriptSessionUnavailableException::class.java) { sm.initialize() }
        // The machine was not started, so the event is held for a start that has
        // not happened, and nothing reads it.
        sm.send(AChildMaySendManyEventsInOneTickEvent.Finish)
        assertEquals(AChildMaySendManyEventsInOneTickState.S0, sm.currentState.value, "a refused machine stays where it was built")
        assertNothingStarted(sm, double, FailStep.CREATE)
    }

    @Test
    fun `the coroutine start throws to its caller and launches nothing`() {
        // `start` establishes the session on the caller's thread, before the
        // coroutine of the run is launched, so the refusal reaches the caller
        // rather than ending a coroutine it holds no handle on.
        val (sm, double) = overDouble(FailStep.CREATE)
        val job = Job()
        try {
            val refused = assertThrows(ScriptSessionUnavailableException::class.java) { sm.start(CoroutineScope(job)) }
            assertEquals(ScriptSessionStage.CREATE, refused.failure.stage)
            assertTrue(job.children.none(), "a refused start launches no coroutine")
            assertNothingStarted(sm, double, FailStep.CREATE)
        } finally {
            job.cancel()
        }
    }

    @Test
    fun `the checked root start is refused for the session`() {
        val (sm, double) = overDouble(FailStep.LIBRARY)
        val refusal = sm.initializeAsRoot()
        val forSession = assertInstanceOf(RootStartRefusal.ScriptSession::class.java, refusal)
        assertNotEquals(RootStartRefusal.NeedsParent, refusal, "a refusal for the session is not one for a parent")
        assertEquals(ScriptSessionStage.LIBRARY, forSession.failure.stage)
        assertEquals(forSession.failure.toString(), forSession.reason, "the refusal's words are the failure's sentence")
        assertNothingStarted(sm, double, FailStep.LIBRARY)

        val job = Job()
        try {
            val coroutineRefusal = sm.startAsRoot(CoroutineScope(job))
            assertInstanceOf(RootStartRefusal.ScriptSession::class.java, coroutineRefusal)
            assertTrue(job.children.none(), "a refused root start launches no coroutine")
            assertNothingStarted(sm, double, FailStep.LIBRARY)
        } finally {
            job.cancel()
        }
    }

    @Test
    fun `a resumed start whose session cannot be established is refused`() {
        val (sm, double) = overDouble(FailStep.CREATE)
        val verdict = sm.enterAt(
            listOf(AChildMaySendManyEventsInOneTickState.S0),
            AChildMaySendManyEventsInOneTickState.S0,
        )
        assertEquals(ConfigurationRejection.SCRIPT_SESSION_UNAVAILABLE, verdict.rejection, "enterAt answered $verdict")
        assertFalse(verdict.accepted)
        assertEquals(ScriptSessionStage.CREATE, verdict.scriptSessionFailure?.stage, "the verdict carries the failure of the create step")
        assertNothingStarted(sm, double, FailStep.CREATE)
    }

    @Test
    fun `a child whose session fails leaves its parent with error execution and no child`() {
        val (sm, double) = overDouble(FailStep.CREATE_AFTER_FIRST)
        try {
            sm.initialize()

            assertEquals(2, double.asked.size, "the parent asked for its own session and the child's: ${double.asked}")
            val child = double.asked[1]
            assertEquals("${sm.sessionId}.s0.1.chatty", child, "the second session asked for is the child's")
            assertFalse(child in double.live, "the child whose session failed keeps none")
            // The tree holds no session under the child's id: a post naming it is
            // refused, as one naming no session of the tree is.
            assertFalse(
                sm.inbox.post(InboxInput.MeshDelivery(MeshCall.Raise("tick", EventMetadata.EMPTY), sessionId = child)),
                "the child whose start was refused is not kept as a session of the tree",
            )
            // The invoke ends without a child, with error.execution, which no
            // transition of `s0` takes.
            assertEquals(
                AChildMaySendManyEventsInOneTickEvent.Error.Execution,
                sm.lastUnhandledError(),
                "the refused child is reported as error.execution",
            )
            assertEquals(1, sm.unhandledErrorEvents(), "one error.execution for the one refused child")
            // No child ran, so no `tick` and no `done.invoke` ever arrived.
            assertEquals(0L, sm.ticks(), "no child ran, so no tick arrived")
            assertEquals(0, sm.discardedExternalEvents(), "no done.invoke was raised for the refused child")

            // The parent itself runs on.
            assertEquals(AChildMaySendManyEventsInOneTickState.S0, sm.currentState.value, "the parent stays in `s0`")
            sm.send(AChildMaySendManyEventsInOneTickEvent.Finish)
            sm.tick()
            assertEquals(AChildMaySendManyEventsInOneTickState.Done, sm.terminalState, "`finish` carries the parent to `done`")
        } finally {
            sm.cleanup()
        }
    }
}
