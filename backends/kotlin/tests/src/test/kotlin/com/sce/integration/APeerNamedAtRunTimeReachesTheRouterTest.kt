// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE_MESH.md §mesh-19: a `targetexpr` that evaluates to a Mesh peer reaches
// the host's Mesh router — Kotlin AOT path.
//
// Fixture: integration_resources/a_peer_named_at_run_time_reaches_the_router/
// a_peer_named_at_run_time_reaches_the_router.scxml (canonical, shared with the
// other channels).
//
// Regeneration: scripts/regen_a_peer_named_at_run_time_reaches_the_router_kotlin.sh

package com.sce.integration

import com.sce.integration.a_peer_named_at_run_time_reaches_the_router.APeerNamedAtRunTimeReachesTheRouterState
import com.sce.integration.a_peer_named_at_run_time_reaches_the_router.APeerNamedAtRunTimeReachesTheRouterStateMachine
import com.sce.runtime.MESH_PROCESSOR_TYPE
import com.sce.runtime.StateMachineEngine
import com.sce.w3c.W3CTestBase
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

@DisplayName("APeerNamedAtRunTimeReachesTheRouter — SCE_MESH 19")
class APeerNamedAtRunTimeReachesTheRouterTest {

    /// With a router registered, the peer send is the router's — processor
    /// `sce:mesh`, the evaluated target, the event — and the non-peer target is
    /// still this session's.
    @Test
    fun aPeerNamedAtRunTimeIsSentToTheRouter() {
        val routed = mutableListOf<StateMachineEngine.HostSendRequest>()
        val sm = APeerNamedAtRunTimeReachesTheRouterStateMachine(W3CTestBase.createEngine())
        try {
            sm.registerMeshRouter { request ->
                routed += request
                emptyList()
            }
            sm.initialize()
            sm.tick()

            assertEquals(1, routed.size, "exactly the peer send reaches the router: $routed")
            assertEquals(MESH_PROCESSOR_TYPE, routed[0].processorType)
            assertEquals("#hmi", routed[0].target)
            assertEquals("ping", routed[0].eventName)
            assertEquals(0L, sm.refused())
            assertEquals(1L, sm.looped())
            assertEquals(APeerNamedAtRunTimeReachesTheRouterState.Done, sm.terminalState)
        } finally {
            sm.cleanup()
        }
    }

    /// With no router registered, the peer send is a send to a host processor
    /// nobody serves: one error.execution, and the non-peer target is
    /// unaffected.
    @Test
    fun aPeerWithNoRouterRegisteredIsErrorExecution() {
        val sm = APeerNamedAtRunTimeReachesTheRouterStateMachine(W3CTestBase.createEngine())
        try {
            sm.initialize()
            sm.tick()

            assertEquals(1L, sm.refused())
            assertEquals(1L, sm.looped())
            assertEquals(APeerNamedAtRunTimeReachesTheRouterState.Done, sm.terminalState)
        } finally {
            sm.cleanup()
        }
    }
}
