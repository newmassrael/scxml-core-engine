// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE_MESH.md §9.5, §mesh-19: an `<invoke type="sce:mesh-rpc">` reaches the
// host's Mesh router through the runtime's mesh-rpc door — Kotlin AOT path.
//
// Fixture: integration_resources/a_mesh_request_reaches_the_router/
// a_mesh_request_reaches_the_router.scxml (canonical, shared with the other
// channels).
//
// Regeneration: scripts/regen_a_mesh_request_reaches_the_router_kotlin.sh

package com.sce.integration

import com.sce.integration.a_mesh_request_reaches_the_router.AMeshRequestReachesTheRouterState
import com.sce.integration.a_mesh_request_reaches_the_router.AMeshRequestReachesTheRouterStateMachine
import com.sce.runtime.MESH_RPC_INVOKE_TYPE
import com.sce.runtime.StateMachineEngine
import com.sce.w3c.W3CTestBase
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

@DisplayName("AMeshRequestReachesTheRouter — SCE_MESH 9.5, 19")
class AMeshRequestReachesTheRouterTest {

    /** Runs the machine into `asking`, with a router recording into [requests] when given. */
    private fun started(
        requests: MutableList<StateMachineEngine.HostInvokeRequest>?,
    ): AMeshRequestReachesTheRouterStateMachine {
        val sm = AMeshRequestReachesTheRouterStateMachine(W3CTestBase.createEngine())
        if (requests != null) {
            // Keeps every request and answers none, so the test decides how
            // each one ends.
            sm.registerMeshRpcInvoker { event ->
                event.start?.let { requests += it }
                null
            }
        }
        sm.initialize()
        sm.tick()
        return sm
    }

    /** The one request the router was started with, checked against what the document wrote. */
    private fun theRequest(
        requests: List<StateMachineEngine.HostInvokeRequest>,
    ): StateMachineEngine.HostInvokeRequest {
        assertEquals(1, requests.size, "exactly one request reaches the router: $requests")
        val request = requests[0]
        assertEquals(MESH_RPC_INVOKE_TYPE, request.processorType)
        assertEquals("ask", request.invokeId)
        assertEquals("#motor", request.src)
        assertEquals(
            mapOf(
                "_mesh_event" to listOf("service.request.force"),
                "_mesh_deadline_ms" to listOf("250"),
                "speed" to listOf("3"),
            ),
            request.params,
        )
        return request
    }

    private fun assertEnded(
        sm: AMeshRequestReachesTheRouterStateMachine,
        answered: Long,
        failed: Long,
        refused: Long,
    ) {
        assertEquals(answered, sm.answered())
        assertEquals(failed, sm.failed())
        assertEquals(refused, sm.refused())
        assertEquals(AMeshRequestReachesTheRouterState.Done, sm.terminalState)
    }

    /** The router answers: done.invoke.ask ends the run. */
    @Test
    fun aMeshRequestIsAnsweredThroughTheRouter() {
        val requests = mutableListOf<StateMachineEngine.HostInvokeRequest>()
        val sm = started(requests)
        try {
            val request = theRequest(requests)
            assertTrue(sm.completeHostInvoke(MESH_RPC_INVOKE_TYPE, "ask", request.token, "\"ok\""))
            sm.tick()
            assertEnded(sm, answered = 1, failed = 0, refused = 0)
        } finally {
            sm.cleanup()
        }
    }

    /** The router fails the request: error.invoke.ask, carrying the router's data. */
    @Test
    fun aMeshRequestTheRouterFailsIsErrorInvoke() {
        val requests = mutableListOf<StateMachineEngine.HostInvokeRequest>()
        val sm = started(requests)
        try {
            val request = theRequest(requests)
            assertTrue(
                sm.failHostInvoke(MESH_RPC_INVOKE_TYPE, "ask", request.token, "\"unreachable\"", "mesh://motor"),
            )
            sm.tick()
            assertEnded(sm, answered = 0, failed = 1, refused = 0)
        } finally {
            sm.cleanup()
        }
    }

    /** With no router registered the invoke names a type nobody runs: error.execution (W3C SCXML 6.4.1). */
    @Test
    fun aMeshRequestWithNoRouterRegisteredIsErrorExecution() {
        val sm = started(null)
        try {
            assertEnded(sm, answered = 0, failed = 0, refused = 1)
        } finally {
            sm.cleanup()
        }
    }
}
