// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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

import com.sce.forge.runtime.SceCursor
import com.sce.generated.envelope.Envelope
import com.sce.generated.pattern_kind.PatternKind
import com.sce.generated.payload_codec.PayloadCodec
import com.sce.generated.rpc_status.RpcStatus
import com.sce.integration.a_mesh_request_reaches_the_router.AMeshRequestReachesTheRouterState
import com.sce.integration.a_mesh_request_reaches_the_router.AMeshRequestReachesTheRouterStateMachine
import com.sce.mesh.Delivery
import com.sce.mesh.Endpoint
import com.sce.mesh.Environment
import com.sce.mesh.PeerConfig
import com.sce.mesh.Router
import com.sce.mesh.applyTo
import com.sce.mesh.register
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
                "force" to listOf("3"),
                "speed" to listOf("3"),
            ),
            request.params,
        )
        // The author's pairs alone, typed: `force` was computed, `speed` was
        // written as a string, and the envelope fields are not payload.
        assertEquals("""{"force":3,"speed":"3"}""", request.eventData)
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

    // ── The same document through the Kotlin host core itself (sce-kotlin-mesh) ──

    /** A clock that stands still and ids that count. */
    private class Fixed : Environment {
        private var ids = 0

        override fun nowMs(): Long = 0

        override fun nowUnixMs(): Long = 1_000_000

        override fun envelopeId(): ByteArray {
            ids += 1
            return ByteArray(16) { ids.toByte() }
        }

        override fun jitterDraw(): Long = 0
    }

    /** The requester's endpoint, bound to `motor` when [bound], sending into [sent]. */
    private fun endpoint(bound: Boolean, sent: MutableList<Pair<String, ByteArray>>): Endpoint {
        val router = Router("brake", 8u, 50)
        if (bound) {
            router.addPeer(
                "motor",
                PeerConfig(
                    transport = "wss",
                    buffer = null,
                    retry = null,
                    stampSequence = false,
                    delivery = Delivery(dedup = true, ordered = false),
                    responders = listOf("motor"),
                    deadlineMs = null,
                ),
            )
        }
        return Endpoint(router, { peer, bytes -> sent += peer to bytes; null }, Fixed())
    }

    /** Runs the machine into `asking` with [endpoint] serving its Mesh traffic. */
    private fun startedThrough(endpoint: Endpoint): AMeshRequestReachesTheRouterStateMachine {
        val sm = AMeshRequestReachesTheRouterStateMachine(W3CTestBase.createEngine())
        register(sm, endpoint)
        sm.initialize()
        sm.tick()
        return sm
    }

    /** The request reaches `motor` as an `RpcRequest`, and motor's `Ok` reply ends the invoke with `done.invoke.ask`. */
    @Test
    fun aReplyThroughTheHostCoreAnswersTheDocument() {
        val sent = mutableListOf<Pair<String, ByteArray>>()
        val endpoint = endpoint(true, sent)
        val sm = startedThrough(endpoint)
        try {
            val (peer, bytes) = sent.single()
            assertEquals("motor", peer)
            val request = Envelope.decode(SceCursor(bytes)) ?: error("not an envelope")
            assertEquals(PatternKind.RPC_REQUEST, request.pattern)
            assertEquals("service.request.force", request.event_type)
            assertEquals("""{"force":3,"speed":"3"}""", request.data.decodeToString())
            // The document's own deadline, on the wall clock.
            assertEquals(1_000_250uL, request.deadline_unix_ms)

            val reply = Envelope(
                id = ByteArray(16) { 0xAA.toByte() },
                source = "motor",
                event_type = "service.response.force",
                pattern = PatternKind.RPC_REPLY,
                datacontenttype = PayloadCodec.JSON,
                data = "\"ok\"".encodeToByteArray(),
                invoke_id = request.invoke_id,
                rpc_status = RpcStatus.OK,
            ).encodeToByteArray() ?: error("reply did not encode")
            endpoint.receive("motor", reply)
            applyTo(sm, endpoint.takeCalls())
            sm.tick()
            assertEnded(sm, answered = 1, failed = 0, refused = 0)
        } finally {
            sm.cleanup()
        }
    }

    /**
     * A target the host core has no binding for cannot reach the wire: the
     * invocation is refused, and the document sees error.execution (SCE_MESH.md
     * §mesh-9.5's pre-envelope tier).
     */
    @Test
    fun anUnboundTargetThroughTheHostCoreIsErrorExecution() {
        val sent = mutableListOf<Pair<String, ByteArray>>()
        val sm = startedThrough(endpoint(false, sent))
        try {
            assertTrue(sent.isEmpty(), "nothing reached the wire")
            assertEnded(sm, answered = 0, failed = 0, refused = 1)
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
