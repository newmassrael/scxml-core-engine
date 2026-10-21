// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// docs/adr/0045: a child session's id is `<parent>.<invokeid>`, derived from the
// tree alone, so a saved tree is one text whatever backend wrote it. The id was
// given to the child when it started, after the parent had built it and handed it
// its arguments; a child that opened its script session in between kept the id its
// process drew for that session, and the tree no longer derived it. The id is now
// the child's from where it is built (`adoptChild`), and a start refuses a child
// that was not given it there.

package com.sce.integration

import com.sce.runtime.EnabledTransition
import com.sce.runtime.Event
import com.sce.runtime.HistoryId
import com.sce.runtime.State
import com.sce.runtime.StateMachineEngine
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertThrows
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

private object IdOnlyState : State

private sealed interface IdOnlyEvent : Event

/**
 * The smallest machine that can be a parent and a child: one state, no
 * transition, and doors onto what a generated machine does with a child's id.
 */
private class IdMachine : StateMachineEngine<IdOnlyState, IdOnlyEvent>() {
    override val initialState: IdOnlyState = IdOnlyState

    override fun firstEnabledTransition(state: IdOnlyState, event: IdOnlyEvent?): EnabledTransition<IdOnlyState, HistoryId>? = null

    override fun onEntry(state: IdOnlyState, isDefaultEntry: Boolean) {}

    override fun onExit(state: IdOnlyState) {}

    override fun executeTransitionContent(source: IdOnlyState, transitionIndex: Int) {}

    /** Open this session's script session, as a generated `ensureScriptEngine` does, and name it. */
    fun openScriptSession(): String = allocateScriptSession()

    /** The id this session's script session was opened under, if one was. */
    val openedSession: String?
        get() = scriptSessionId

    /** Give [child] its id for the start [generatedInvokeId], as generated code does where it builds one. */
    fun adopt(child: IdMachine, generatedInvokeId: String): IdMachine = adoptChild(child, generatedInvokeId)

    /** Start [child] as the `<invoke>` `worker` under [generatedInvokeId]. */
    fun startWorker(child: IdMachine, generatedInvokeId: String) =
        startInvoke("worker", child, autoforward = false, doneEvent = null, generatedInvokeId = generatedInvokeId)
}

@DisplayName("A child takes its id where it is built (docs/adr/0045)")
class AChildTakesItsIdWhereItIsBuiltTest {

    @Test
    fun `a child that opens its script session before its start opens it under the derived id`() {
        val parent = IdMachine()
        val parentId = parent.openScriptSession()
        val child = parent.adopt(IdMachine(), "s0.1.worker")
        // What a child does to take an argument before it starts: its script
        // session is named by the id the tree derives, not by one its process drew.
        assertEquals("$parentId.s0.1.worker", child.openScriptSession())
    }

    @Test
    fun `a child whose script session is already open is refused its id`() {
        val parent = IdMachine()
        parent.openScriptSession()
        val child = IdMachine()
        child.openScriptSession()
        assertThrows(IllegalStateException::class.java) { parent.adopt(child, "s0.1.worker") }
    }

    @Test
    fun `a start refuses a child that was not given its id where it was built`() {
        val parent = IdMachine()
        parent.openScriptSession()
        parent.initialize()
        assertThrows(IllegalStateException::class.java) { parent.startWorker(IdMachine(), "s0.1.worker") }
        // Given the id of another start, it is refused too: the id is the start's.
        val other = parent.adopt(IdMachine(), "s0.2.worker")
        assertThrows(IllegalStateException::class.java) { parent.startWorker(other, "s0.1.worker") }
    }

    @Test
    fun `a child given its id where it was built starts and keeps it`() {
        val parent = IdMachine()
        val parentId = parent.openScriptSession()
        parent.initialize()
        val child = parent.adopt(IdMachine(), "s0.1.worker")
        parent.startWorker(child, "s0.1.worker")
        assertEquals("$parentId.s0.1.worker", child.openScriptSession())
        assertEquals("$parentId.s0.1.worker", child.openedSession)
    }
}
