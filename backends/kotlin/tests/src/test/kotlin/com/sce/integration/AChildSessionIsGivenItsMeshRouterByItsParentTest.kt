// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE-VERIFIES: mesh-21.3 mesh-21.4 mesh-21.5
//
// SCE_MESH.md, Child Sessions in Mesh (docs/adr/0043) — Kotlin half of the Rust
// suite's `a_child_session_is_given_its_mesh_router_by_its_parent.rs`.
//
// A child session that talks to a Mesh peer is given a router of its own by the
// machine that starts it, asked of the factory the host registered, and never the
// machine's own. `static_child_mesh.scxml` starts `list`, which greets `#server` as
// it is entered and ends when the greeting is refused; the machine counts that end
// in `refusals`. Its hybrid `peer` starts `static_mesh_relay`, whose child `inner`
// answers `inner.ping`.

package com.sce.integration

import com.sce.integration.static_child_mesh.StaticChildMeshStateMachine
import com.sce.runtime.ChildMeshRouter
import com.sce.runtime.ManualClock
import com.sce.runtime.MeshCall
import com.sce.runtime.StateMachineEngine
import com.sce.runtime.meshEventMetadata
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class AChildSessionIsGivenItsMeshRouterByItsParentTest {

    private fun started(register: (StaticChildMeshStateMachine) -> Unit): Pair<StaticChildMeshStateMachine, MutableList<String>> {
        val sm = StaticChildMeshStateMachine()
        sm.clock = ManualClock(0)
        // The machine's own router, which no child's send may reach.
        val parentSends = mutableListOf<String>()
        sm.registerMeshRouter { request ->
            parentSends += request.eventName
            emptyList()
        }
        register(sm)
        sm.initialize()
        settle(sm)
        return sm to parentSends
    }

    /** The rounds a child needs to take an event and its end to reach the parent. */
    private fun settle(sm: StaticChildMeshStateMachine) = repeat(5) { sm.tick() }

    /** A child's router that writes down what it is told and holds what a test hands it. */
    private class Recorder(
        val name: String,
        val log: MutableList<String>,
        val inbox: MutableList<MeshCall>,
    ) : ChildMeshRouter {
        override fun send(request: StateMachineEngine.HostSendRequest): List<StateMachineEngine.HostSendResponse> {
            log += "$name sent ${request.eventName}"
            return emptyList()
        }

        override fun invoke(event: StateMachineEngine.HostInvokeEvent): StateMachineEngine.HostInvokeResponse? = null

        override fun takeCalls(): List<MeshCall> = inbox.toList().also { inbox.clear() }

        override fun detach() {
            log += "$name detached"
        }

        override fun childRouter(invokeId: String, document: String): ChildMeshRouter {
            log += "$name asked for $invokeId of '$document'"
            return Recorder("$name/$invokeId", log, inbox)
        }
    }

    @Test
    fun aChildGivenNoRouterHasItsPeerSendRefusedAndTheMachinesRouterIsNotAsked() {
        val (sm, parentSends) = started { }
        try {
            assertEquals(1u, sm.refusals, "the child's greeting found no router and ended the child")
            assertTrue(parentSends.isEmpty(), "the child's send fell back to the machine's router: $parentSends")
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun aFactoryThatAnswersNoneLeavesTheChildWithNoRouter() {
        val (sm, parentSends) = started { it.registerChildMeshRouters { _, _ -> null } }
        try {
            assertEquals(1u, sm.refusals)
            assertTrue(parentSends.isEmpty())
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun eachStartAsksTheFactoryAndEachEndDetachesWhatItAnswered() {
        val log = mutableListOf<String>()
        val inbox = mutableListOf<MeshCall>()
        val (sm, parentSends) =
            started {
                it.registerChildMeshRouters { invokeId, document ->
                    log += "asked for $invokeId of '$document'"
                    Recorder(invokeId, log, inbox)
                }
            }
        try {
            // Asked before the child was entered, since the greeting went through it.
            assertEquals(listOf("asked for list of ''", "list sent list.hello"), log)
            assertEquals(0u, sm.refusals)

            // A peer's event reaches the child when the machine next drives it.
            inbox += MeshCall.Raise("list.ping", meshEventMetadata("", "server", ""))
            settle(sm)
            assertEquals("list sent list.pong", log.last())

            // The hybrid candidate is asked for by its stem, and its child of its router.
            log.clear()
            sm.sendEventByName("open")
            settle(sm)
            assertEquals(listOf("asked for peer of 'static_mesh_relay'", "peer asked for inner of ''"), log)

            // Leaving the invoking states ends every session, each detached once,
            // a grandchild before its parent.
            log.clear()
            sm.sendEventByName("again")
            settle(sm)
            assertEquals(listOf("peer/inner detached", "peer detached", "list detached"), log)

            // Entered anew, the invoke asks again.
            log.clear()
            sm.sendEventByName("back")
            settle(sm)
            assertEquals(listOf("asked for list of ''", "list sent list.hello"), log)
            assertTrue(parentSends.isEmpty())
        } finally {
            sm.cleanup()
        }
    }
}
