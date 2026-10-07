// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Accepted Subset §2.13 and §2.15, "Child sessions" (docs/adr/0005,
// decision 6): the candidate of a hybrid `<invoke>`.
//
// A candidate is a document of its own with acts of its own, so the host its
// parent's host answers is the candidate's and not the invoke's: one question
// per candidate the invoke may start. `static_child_host_hybrid.scxml` invokes
// `work` with `srcexpr="pick"`; `pick` names `static_hosted_first` while the
// machine starts and `static_hosted_second` after `again` and `back`. Each
// candidate performs one act of its own on entry, so what each host was asked to
// do shows which candidate was built, and with which host.

package com.sce.integration

import com.sce.integration.static_child_host_hybrid.RecordingStaticChildHostHybridActions
import com.sce.integration.static_child_host_hybrid.RecordingStaticHostedFirstActions
import com.sce.integration.static_child_host_hybrid.RecordingStaticHostedSecondActions
import com.sce.integration.static_child_host_hybrid.StaticChildHostHybridEvent
import com.sce.integration.static_child_host_hybrid.StaticChildHostHybridStateMachine
import com.sce.runtime.ManualClock
import kotlin.test.Test
import kotlin.test.assertEquals

class AHybridCandidateIsGivenItsHostByItsParentTest {

    private val askedForFirst = RecordingStaticChildHostHybridActions.Call.ActionsForWorkStaticHostedFirst
    private val askedForSecond = RecordingStaticChildHostHybridActions.Call.ActionsForWorkStaticHostedSecond

    /**
     * The hosts of one machine: the parent's, which answers a recording host
     * for each candidate it is asked for — each kept, in the order asked — and
     * answers nothing for a candidate the invoke did not start.
     */
    private class Hosts {
        val first = mutableListOf<RecordingStaticHostedFirstActions>()
        val second = mutableListOf<RecordingStaticHostedSecondActions>()
        val parent = RecordingStaticChildHostHybridActions(
            actionsForWorkStaticHostedFirstSource = {
                RecordingStaticHostedFirstActions().also { first += it }
            },
            actionsForWorkStaticHostedSecondSource = {
                RecordingStaticHostedSecondActions().also { second += it }
            },
        )
    }

    private fun started(hosts: Hosts): StaticChildHostHybridStateMachine {
        val sm = StaticChildHostHybridStateMachine(hosts.parent)
        sm.clock = ManualClock(0)
        sm.initialize()
        repeat(3) { sm.tick() }
        return sm
    }

    private fun send(sm: StaticChildHostHybridStateMachine, event: StaticChildHostHybridEvent) {
        sm.send(event)
        repeat(3) { sm.tick() }
    }

    @Test
    fun theCandidateTheValueNamesIsBuiltWithTheHostTheParentAnsweredForIt() {
        val hosts = Hosts()
        val sm = started(hosts)
        try {
            assertEquals(listOf(askedForFirst), hosts.parent.calls)
            assertEquals(listOf(RecordingStaticHostedFirstActions.Call.FirstRan), hosts.first.single().calls)
            assertEquals(emptyList(), hosts.second, "the other candidate was not built")
            assertEquals(1u, sm.completed, "it ran and ended")
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun aLaterStartIsAskedAboutTheCandidateItNamesThen() {
        val hosts = Hosts()
        val sm = started(hosts)
        try {
            send(sm, StaticChildHostHybridEvent.Again)
            send(sm, StaticChildHostHybridEvent.Back)

            assertEquals(listOf(askedForFirst, askedForSecond), hosts.parent.calls)
            assertEquals(listOf(RecordingStaticHostedSecondActions.Call.SecondRan), hosts.second.single().calls)
            // The first candidate's run is its own and was not repeated.
            assertEquals(listOf(RecordingStaticHostedFirstActions.Call.FirstRan), hosts.first.single().calls)
            assertEquals(2u, sm.completed)
        } finally {
            sm.cleanup()
        }
    }
}
