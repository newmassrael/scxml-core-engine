// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Accepted Subset §2.15, "Child sessions" (docs/adr/0005, decision 6).
//
// A child that declares `<sce:action>`s takes the host that performs them when
// it is built, because its first `<onentry>` can already perform an act: a host
// installed afterwards would arrive one act too late. So the host has to exist
// when the invocation starts, and the parent obtains it from its own host, which
// answers one for the child each time the invocation starts.
//
// `static_child_host.scxml` invokes `worker`, which announces itself on entry
// (`started`) and reports its steps when it ends (`finished`). The parent declares
// no act: its host is there for the child alone. Every case is read from the
// hosts, which record what they were asked.

package com.sce.integration

import com.sce.integration.static_child_host.RecordingStaticChildHostActions
import com.sce.integration.static_child_host.RecordingStaticChildHostSceSynthInvokeWorkerActions
import com.sce.integration.static_child_host.StaticChildHostEvent
import com.sce.integration.static_child_host.StaticChildHostStateMachine
import com.sce.runtime.ManualClock
import com.sce.runtime.SavedState
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotSame

class AChildIsGivenItsHostByItsParentTest {

    /** The wall-clock moment a machine is saved at: 2023-11-14T22:13:20Z. */
    private val savedAtMs = 1_700_000_000_000L

    private val started = RecordingStaticChildHostSceSynthInvokeWorkerActions.Call.Started

    private fun finished(steps: UInt) =
        RecordingStaticChildHostSceSynthInvokeWorkerActions.Call.Finished(steps)

    private val asked = RecordingStaticChildHostActions.Call.ActionsForWorker

    /**
     * The hosts of one machine: the parent's, which answers a recording host of
     * its own for each child it is asked for, and those children's, in the order
     * they were asked for.
     */
    private class Hosts {
        val children = mutableListOf<RecordingStaticChildHostSceSynthInvokeWorkerActions>()
        val parent = RecordingStaticChildHostActions(
            actionsForWorkerSource = {
                RecordingStaticChildHostSceSynthInvokeWorkerActions().also { children += it }
            },
        )
    }

    private fun machine(hosts: Hosts): StaticChildHostStateMachine {
        val sm = StaticChildHostStateMachine(hosts.parent)
        sm.clock = ManualClock(0)
        return sm
    }

    private fun started(hosts: Hosts): StaticChildHostStateMachine {
        val sm = machine(hosts)
        sm.initialize()
        repeat(3) { sm.tick() }
        return sm
    }

    /** Send [event] to the parent, which forwards it to its child, and let the child run. */
    private fun send(sm: StaticChildHostStateMachine, event: StaticChildHostEvent) {
        sm.send(event)
        repeat(3) { sm.tick() }
    }

    @Test
    fun theChildPerformsItsFirstActThroughTheHostItsParentAnswered() {
        val hosts = Hosts()
        val sm = started(hosts)
        try {
            assertEquals(listOf(asked), hosts.parent.calls, "the parent's host was asked once")
            assertEquals(1, hosts.children.size)
            // The act of its first `<onentry>` is already performed: the host was
            // there when the child was built, not installed after.
            assertEquals(listOf(started), hosts.children[0].calls)
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun theChildReportsWhatItDidThroughTheSameHost() {
        val hosts = Hosts()
        val sm = started(hosts)
        try {
            send(sm, StaticChildHostEvent.A)
            send(sm, StaticChildHostEvent.B)
            assertEquals(1u, sm.completed, "the child ended and the parent counted it")
            assertEquals(listOf(started, finished(2u)), hosts.children[0].calls)
            assertEquals(listOf(asked), hosts.parent.calls, "nothing asked the parent's host again")
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun aStateInvokedAgainIsGivenAHostOfItsOwn() {
        val hosts = Hosts()
        val sm = started(hosts)
        try {
            send(sm, StaticChildHostEvent.A)
            send(sm, StaticChildHostEvent.B)
            send(sm, StaticChildHostEvent.Again)
            send(sm, StaticChildHostEvent.Back)

            assertEquals(listOf(asked, asked), hosts.parent.calls, "asked once per start")
            assertEquals(2, hosts.children.size)
            assertNotSame(hosts.children[0], hosts.children[1])
            // The first child's run is its own, and the second starts from nothing.
            assertEquals(listOf(started, finished(2u)), hosts.children[0].calls)
            assertEquals(listOf(started), hosts.children[1].calls)
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun aRestoredMachineStartsItsChildAgainWithAHostFromItsOwnParent() {
        val before = Hosts()
        val first = started(before)
        val saved = try {
            send(first, StaticChildHostEvent.A)
            first.save(savedAtMs)
        } finally {
            first.cleanup()
        }
        assertEquals(listOf("worker"), saved.invokes, "the child was running")

        // A restore is another machine with another host: the child is started
        // over, and what is asked of the host is asked of that one.
        val after = Hosts()
        val sm = machine(after)
        try {
            sm.restore(SavedState.fromJson(saved.toJson()), savedAtMs)
            repeat(3) { sm.tick() }
            assertEquals(listOf(asked), after.parent.calls)
            assertEquals(listOf(started), after.children.single().calls)

            send(sm, StaticChildHostEvent.A)
            send(sm, StaticChildHostEvent.B)
            assertEquals(1u, sm.completed)
            assertEquals(listOf(started, finished(2u)), after.children.single().calls)
            // The machine that was saved is not asked for anything more.
            assertEquals(listOf(started), before.children.single().calls)
            assertEquals(listOf(asked), before.parent.calls)
        } finally {
            sm.cleanup()
        }
    }
}
