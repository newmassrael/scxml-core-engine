// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE_MESH.md §9.5, §mesh-19 under `datamodel="sce-static"` (docs/adr/0005,
// decisions 5 and 7): an `<invoke type="sce:mesh-rpc">` whose peer and `<param>`s
// are typed expressions over the machine's own fields reaches the host's Mesh
// router through the runtime's mesh-rpc door — Kotlin AOT path. The Rust twin is
// `a_static_mesh_request_reaches_the_router`, and the script-model one is
// `AMeshRequestReachesTheRouterTest`.
//
// The committed machine (com/sce/integration/statechart_static_mesh_request/) is
// generated from
// sce-build/tests/fixtures/host_processor/statechart_static_mesh_request.scxml
// (regen: scripts/regen_host_processor_kotlin.sh) and constructed with NO script
// engine: its variables are fields, and a peer or a `<param>` that needed an
// engine to be read would not have one to ask.
//
// The run `bump`, `go` changes `load` and `label` before the request reads them,
// so a copy taken at start-up (3, "idle") is told from what the fields hold now
// (4, "busy"): the request carries 8 and "busy".

package com.sce.integration

import com.sce.forge.runtime.SceCursor
import com.sce.generated.envelope.Envelope
import com.sce.generated.pattern_kind.PatternKind
import com.sce.generated.payload_codec.PayloadCodec
import com.sce.generated.rpc_status.RpcStatus
import com.sce.integration.statechart_static_mesh_request.StatechartStaticMeshRequestEvent
import com.sce.integration.statechart_static_mesh_request.StatechartStaticMeshRequestState
import com.sce.integration.statechart_static_mesh_request.StatechartStaticMeshRequestStateMachine
import com.sce.mesh.Delivery
import com.sce.mesh.Endpoint
import com.sce.mesh.Environment
import com.sce.mesh.PeerConfig
import com.sce.mesh.Router
import com.sce.mesh.applyTo
import com.sce.mesh.register
import com.sce.runtime.MESH_RPC_INVOKE_TYPE
import com.sce.runtime.StateMachineEngine
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

@DisplayName("AStaticMeshRequestReachesTheRouter — SCE_MESH 9.5, 19 under sce-static")
class AStaticMeshRequestReachesTheRouterTest {

    /** A machine with no script engine, initialised and standing at `idle`. */
    private fun started(): StatechartStaticMeshRequestStateMachine {
        val sm = StatechartStaticMeshRequestStateMachine()
        sm.initialize()
        sm.tick()
        return sm
    }

    private fun drive(sm: StatechartStaticMeshRequestStateMachine, vararg events: StatechartStaticMeshRequestEvent) {
        for (event in events) {
            sm.send(event)
            sm.tick()
        }
    }

    /** A router recording into [requests]: it keeps every request and answers none, so the test decides how each one ends. */
    private fun recording(
        sm: StatechartStaticMeshRequestStateMachine,
        requests: MutableList<StateMachineEngine.HostInvokeRequest>,
    ) {
        sm.registerMeshRpcInvoker { event ->
            event.start?.let { requests += it }
            null
        }
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
                "force" to listOf("8"),
                "speed" to listOf("busy"),
            ),
            request.params,
        )
        // The author's pairs alone, typed: `force` was computed, `speed` is a
        // string, and the envelope fields are not payload.
        assertEquals("""{"force":8,"speed":"busy"}""", request.eventData)
        return request
    }

    private fun assertEnded(
        sm: StatechartStaticMeshRequestStateMachine,
        answered: UInt,
        failed: UInt,
        refused: UInt,
    ) {
        assertEquals(answered, sm.answered)
        assertEquals(failed, sm.failed)
        assertEquals(refused, sm.refused)
        assertEquals(StatechartStaticMeshRequestState.Done, sm.terminalState)
    }

    /** The router answers: done.invoke.ask ends the run. */
    @Test
    fun aStaticMeshRequestIsAnsweredThroughTheRouter() {
        val requests = mutableListOf<StateMachineEngine.HostInvokeRequest>()
        val sm = started()
        try {
            recording(sm, requests)
            drive(sm, StatechartStaticMeshRequestEvent.Bump, StatechartStaticMeshRequestEvent.Go)
            val request = theRequest(requests)
            assertTrue(sm.completeHostInvoke(MESH_RPC_INVOKE_TYPE, "ask", request.token, "\"ok\""))
            sm.tick()
            assertEnded(sm, answered = 1u, failed = 0u, refused = 0u)
        } finally {
            sm.cleanup()
        }
    }

    /** The router fails the request: error.invoke.ask, carrying the router's data. */
    @Test
    fun aStaticMeshRequestTheRouterFailsIsErrorInvoke() {
        val requests = mutableListOf<StateMachineEngine.HostInvokeRequest>()
        val sm = started()
        try {
            recording(sm, requests)
            drive(sm, StatechartStaticMeshRequestEvent.Bump, StatechartStaticMeshRequestEvent.Go)
            val request = theRequest(requests)
            assertTrue(
                sm.failHostInvoke(MESH_RPC_INVOKE_TYPE, "ask", request.token, "\"unreachable\"", "mesh://motor"),
            )
            sm.tick()
            assertEnded(sm, answered = 0u, failed = 1u, refused = 0u)
        } finally {
            sm.cleanup()
        }
    }

    /** The control: nothing has written a field, so the request carries what `<data expr>` gave them and not the values `bump` would have set. */
    @Test
    fun theRequestCarriesTheFieldsAsTheyStandAndNotACopyFromStartUp() {
        val requests = mutableListOf<StateMachineEngine.HostInvokeRequest>()
        val sm = started()
        try {
            recording(sm, requests)
            drive(sm, StatechartStaticMeshRequestEvent.Go)
            assertEquals(1, requests.size)
            assertEquals("""{"force":6,"speed":"idle"}""", requests[0].eventData)
        } finally {
            sm.cleanup()
        }
    }

    /**
     * The peer is the string the machine holds when the invocation starts: `ghost`
     * changes it, so the router is handed that name as the request's `src`, and the
     * host's router is the one that looks it up among its bindings.
     */
    @Test
    fun thePeerIsTheStringTheMachineHoldsWhenTheRequestStarts() {
        val requests = mutableListOf<StateMachineEngine.HostInvokeRequest>()
        val sm = started()
        try {
            recording(sm, requests)
            drive(sm, StatechartStaticMeshRequestEvent.Ghost, StatechartStaticMeshRequestEvent.Go)
            assertEquals(1, requests.size)
            assertEquals("#ghost", requests[0].src)
        } finally {
            sm.cleanup()
        }
    }

    /** With no router registered the invoke names a type nobody runs: error.execution (W3C SCXML 6.4.1). */
    @Test
    fun aStaticMeshRequestWithNoRouterRegisteredIsErrorExecution() {
        val sm = started()
        try {
            drive(sm, StatechartStaticMeshRequestEvent.Bump, StatechartStaticMeshRequestEvent.Go)
            assertEnded(sm, answered = 0u, failed = 0u, refused = 1u)
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

    /** The requester's endpoint, bound to `motor`, sending into [sent]. */
    private fun endpoint(sent: MutableList<Pair<String, ByteArray>>): Endpoint {
        val router = Router("brake", 8u, 50)
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
                replyEvents = emptyList(),
            ),
        )
        return Endpoint(router, { peer, bytes -> sent += peer to bytes; null }, Fixed())
    }

    /** The request reaches `motor` as an `RpcRequest`, and motor's `Ok` reply ends the invoke with `done.invoke.ask`. */
    @Test
    fun aReplyThroughTheHostCoreAnswersTheDocument() {
        val sent = mutableListOf<Pair<String, ByteArray>>()
        val endpoint = endpoint(sent)
        val sm = started()
        try {
            register(sm, endpoint)
            drive(sm, StatechartStaticMeshRequestEvent.Bump, StatechartStaticMeshRequestEvent.Go)
            val (peer, bytes) = sent.single()
            assertEquals("motor", peer)
            val request = Envelope.decode(SceCursor(bytes)) ?: error("not an envelope")
            assertEquals(PatternKind.RPC_REQUEST, request.pattern)
            assertEquals("service.request.force", request.event_type)
            assertEquals("""{"force":8,"speed":"busy"}""", request.data.decodeToString())
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
            assertEnded(sm, answered = 1u, failed = 0u, refused = 0u)
        } finally {
            sm.cleanup()
        }
    }

    /**
     * The peer the field names is looked up among the bindings the host core has:
     * `ghost` makes it a name it has none for, which cannot reach the wire, so the
     * invocation is refused and the document sees error.execution (SCE_MESH.md
     * §mesh-9.5's pre-envelope tier, as the generated C++ router answers it).
     */
    @Test
    fun aPeerWithNoBindingThroughTheHostCoreIsErrorExecution() {
        val sent = mutableListOf<Pair<String, ByteArray>>()
        val sm = started()
        try {
            register(sm, endpoint(sent))
            drive(sm, StatechartStaticMeshRequestEvent.Ghost, StatechartStaticMeshRequestEvent.Go)
            assertTrue(sent.isEmpty(), "nothing reached the wire")
            assertEnded(sm, answered = 0u, failed = 0u, refused = 1u)
        } finally {
            sm.cleanup()
        }
    }
}
