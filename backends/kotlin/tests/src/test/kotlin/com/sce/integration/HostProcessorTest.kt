// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.2.5 — Kotlin compile+run gate for a `<send type>` the HOST serves.
//
// The clause makes the Event I/O Processor identifier extensible, so the set is
// open by design. SCE implemented two of them and refused everything else with
// `error.execution`; nothing let a platform widen the set. Rust, C++, C11, Go
// and Python grew a registry first, and this backend refused the declaration by
// name until it grew one of its own — the refusal being honest is exactly what
// made the gap a coverage debt rather than a silent drop. With this one the
// roster is complete and the refusal itself is retired.
//
// The committed machine is generated from
// `sce-build/tests/fixtures/host_processor/statechart_host_processor.scxml`
// WITH the declaration (regen: `scripts/regen_host_processor_kotlin.sh`), the
// same document the other five channels drive.
//
// The pair at the top is the whole contract:
//
//   * a registered handler receives the send and its reply arrives as an
//     event — the feature working;
//   * the same machine with nothing registered raises `error.execution` — a
//     wiring mistake staying visible instead of reading as success.
//
// A gate holding only the first would pass on an engine that dispatched to
// nothing and called it delivered, which is the silence being repaid.

package com.sce.integration

import com.sce.integration.statechart_host_processor.StatechartHostProcessorStateMachine
import com.sce.runtime.MESH_PROCESSOR_TYPE
import com.sce.runtime.StateMachineEngine
import com.sce.runtime.isReservedType
import com.sce.w3c.W3CTestBase
import java.io.File
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.boolean
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertFalse
import org.junit.jupiter.api.Assertions.assertNotNull
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

/// W3C SCXML 6.2.5 — a `<send type>` the host serves.
@DisplayName("HostProcessor — W3C SCXML 6.2.5")
class HostProcessorTest {

    // The type the fixture was compiled for.
    // `scripts/regen_host_processor_kotlin.sh` passes this same string to
    // `--host-processor`; a test registering a different one would measure
    // nothing and pass, so the `refused` counter is asserted rather than the
    // registration trusted.
    private val declaredType = "x-sce-host"

    // The fixture counts every outcome with `<assign>`, so this is an
    // ECMAScript-datamodel machine. Not started here: a handler has to be
    // registered BEFORE `initialize()`, because the send this measures is in
    // the initial state's `<onentry>`.
    private fun machine(): StatechartHostProcessorStateMachine =
        StatechartHostProcessorStateMachine(W3CTestBase.createEngine())

    /// The fixture's `<assign>`s are the only witness: every outcome leaves the
    /// machine in the same single state, so the configuration cannot tell them
    /// apart.
    private fun counter(sm: StatechartHostProcessorStateMachine, name: String): Long {
        val value = when (name) {
            "served" -> sm.served()
            "refused" -> sm.refused()
            "plain" -> sm.plain()
            "paramErrors" -> sm.paramErrors()
            else -> error("the fixture declares no counter `$name`")
        }
        assertNotNull(value, "the fixture declares `$name` and the machine could not read it")
        return value!!
    }

    @Test
    fun aRegisteredHandlerReceivesTheSendAndItsReplyArrives() {
        val sm = machine()
        val seen = mutableListOf<StateMachineEngine.HostSendRequest>()
        sm.registerEventProcessor(declaredType) { request ->
            seen.add(request)
            // The request/reply shape: the reply becomes an event the document
            // was already waiting for, which is what lets a state DECLARE an
            // act instead of a host-side table performing it.
            listOf(StateMachineEngine.HostSendResponse("turn.done"))
        }
        sm.initialize()

        try {
            assertEquals(1L, counter(sm, "served"), "the handler's reply never reached the document")
            assertEquals(0L, counter(sm, "refused"), "a served send also raised error.execution")
            // The false-positive guard: an ordinary `<send>` in the same block
            // must still deliver. Without it a change that broke every send
            // while leaving the host branch intact would read as a pass.
            assertEquals(1L, counter(sm, "plain"), "an ordinary <send> in the same block stopped delivering")

            // W3C SCXML 5.7.1: the second send's unreadable <param> raised, and
            // the message still went — which the second request confirms.
            assertEquals(1L, counter(sm, "paramErrors"), "the <param> that cannot be read was not reported")
            assertEquals(2, seen.size, "the handler ran ${seen.size} times; the served send and the one from `pairs` each run it once")
            val request = seen[0]
            // W3C SCXML 5.10: the event data is what a local delivery of this
            // send would carry in `_event.data`, computed by the engine, not
            // rebuilt by the host from `params`. One key, so its spelling is exact.
            assertEquals("""{"within":"2500"}""", request.eventData)
            val pairs = seen[1]
            assertEquals("watch.pairs", pairs.eventName)
            // The namelist pair reaches the host as a param, the literal one
            // too, and the unreadable one is absent rather than carried empty.
            assertEquals(listOf("pairs"), pairs.params["mode"], "the namelist pair: ${pairs.params}")
            assertEquals(listOf("here"), pairs.params["kept"], "the literal <param>: ${pairs.params}")
            assertTrue("broken" !in pairs.params, "a <param> that could not be read reached the host: ${pairs.params}")
            // Parsed rather than compared as text: with more than one key,
            // member order is each backend's own.
            assertEquals(
                mapOf("kept" to "here", "mode" to "pairs"),
                com.sce.runtime.Json.parse(pairs.eventData),
                "the event data: ${pairs.eventData}"
            )
            assertEquals(declaredType, request.processorType)
            assertEquals("watch.turn", request.eventName)
            // The payload the author wrote has to survive the crossing, or the
            // document can name an act but not parameterise it — which is most
            // of the reason to move an act into the document at all.
            assertEquals(
                listOf("2500"),
                request.params["within"],
                "the <param> did not reach the handler: ${request.params}"
            )
            // W3C SCXML 6.2.4: correlating a reply, or honouring a `<cancel>`,
            // needs the send id — auto-generated here because the fixture
            // declares none.
            assertTrue(request.sendId.isNotEmpty(), "the request carried no send id")
        } finally {
            sm.cleanup()
        }
    }

    /// The other half, and the one that keeps the repair honest: the build
    /// declared the type so codegen emitted a dispatch, but nothing was
    /// registered, so nobody performed the act.
    @Test
    fun aDeclaredTypeWithNoHandlerStillRaisesErrorExecution() {
        val sm = machine()
        sm.initialize()
        try {
            assertEquals(1L, counter(sm, "refused"), "an unregistered processor was silently treated as served")
            assertEquals(0L, counter(sm, "served"))
        } finally {
            sm.cleanup()
        }
    }

    /// A handler may perform work and have nothing to say. That is not an
    /// error, and reporting it as one would cost every fire-and-forget act a
    /// spurious `error.execution`.
    @Test
    fun aHandlerThatAnswersNothingIsNotAnError() {
        val sm = machine()
        var ran = false
        sm.registerEventProcessor(declaredType) {
            ran = true
            emptyList()
        }
        sm.initialize()
        try {
            assertTrue(ran, "the handler never ran")
            assertEquals(0L, counter(sm, "refused"), "a silent handler was reported as an unsupported processor")
            assertEquals(0L, counter(sm, "served"), "no reply was sent, so no reply event should have arrived")
        } finally {
            sm.cleanup()
        }
    }

    /// The registry is keyed. A lookup falling back to "any handler" would
    /// deliver a document's acts to a processor it never named.
    @Test
    fun aHandlerRegisteredForAnotherTypeDoesNotServeThisOne() {
        val sm = machine()
        sm.registerEventProcessor("x-some-other-host") {
            listOf(StateMachineEngine.HostSendResponse("turn.done"))
        }
        sm.initialize()
        try {
            assertEquals(0L, counter(sm, "served"), "a handler for a different type answered this send")
            assertEquals(1L, counter(sm, "refused"))
        } finally {
            sm.cleanup()
        }
    }

    /// A reply may name an event this machine does not declare — a host serving
    /// several documents, or one that has moved on since. That is dropped,
    /// exactly as any undeclared event reaching the queue is, and it is not an
    /// error.
    @Test
    fun aReplyNamingAnUndeclaredEventIsDropped() {
        val sm = machine()
        sm.registerEventProcessor(declaredType) {
            listOf(StateMachineEngine.HostSendResponse("turn.never.declared"))
        }
        sm.initialize()
        try {
            assertEquals(0L, counter(sm, "served"), "an undeclared reply name reached a transition")
            assertEquals(0L, counter(sm, "refused"), "a dropped reply was reported as a refusal")
            assertEquals(1L, counter(sm, "plain"), "the machine stopped running after an unknown reply name")
        } finally {
            sm.cleanup()
        }
    }

    /// The query the generated send site uses to tell "ran and said nothing"
    /// from "was never wired up". Both give the same answer from the dispatch,
    /// and only the second is an error, so the distinction cannot come from the
    /// return value alone.
    @Test
    fun theRegistryReportsWhatItHolds() {
        val sm = machine()
        assertFalse(sm.hasEventProcessor(declaredType), "an unregistered type reads as present")
        sm.registerEventProcessor(declaredType) { emptyList() }
        assertTrue(sm.hasEventProcessor(declaredType), "the registered type reads as absent")
        assertFalse(sm.hasEventProcessor("x-never-registered"), "an unregistered type reads as present")
    }

    /// Registering a type twice replaces. Appending would leave dispatch
    /// depending on registration order, and a host re-registering means to
    /// change what serves the act — not to add a second server whose turn may
    /// never come.
    @Test
    fun registeringATypeTwiceReplaces() {
        val sm = machine()
        var supersededRan = false
        var currentRan = false
        sm.registerEventProcessor(declaredType) {
            supersededRan = true
            emptyList()
        }
        sm.registerEventProcessor(declaredType) {
            currentRan = true
            listOf(StateMachineEngine.HostSendResponse("turn.done"))
        }
        sm.initialize()
        try {
            assertFalse(supersededRan, "the superseded handler still served the act")
            assertTrue(currentRan, "the current handler never ran")
            assertEquals(1L, counter(sm, "served"))
        } finally {
            sm.cleanup()
        }
    }

    /// sce-build/tests/fixtures/host_processor/reserved_type_cases.json: the
    /// table the build's declaration check and every runtime's registration
    /// read. Each case is asked of both registrations, not only of the
    /// predicate, so a registration that stopped consulting it fails here.
    @Test
    fun registrationRefusesTheReservedTypesTheSharedTableNames() {
        // Found by walking up rather than by a fixed depth, because Gradle's
        // working directory is the project's and that is a build detail.
        val root = generateSequence(File(System.getProperty("user.dir")).absoluteFile) { it.parentFile }
            .firstOrNull { File(it, "sce-build").isDirectory }
            ?: error("no ancestor of ${System.getProperty("user.dir")} holds sce-build/")
        val cases = Json.parseToJsonElement(
            File(root, "sce-build/tests/fixtures/host_processor/reserved_type_cases.json").readText()
        ).jsonObject.getValue("cases").jsonArray
        // A floor: an empty table would pass every assertion below.
        assertTrue(cases.size >= 10, "the table lost cases: ${cases.size}")
        for (case in cases) {
            val name = case.jsonObject.getValue("type").jsonPrimitive.content
            val reserved = case.jsonObject.getValue("reserved").jsonPrimitive.boolean
            assertEquals(reserved, isReservedType(name), "\"$name\"")
            val registrations = listOf<Pair<String, (StatechartHostProcessorStateMachine) -> Unit>>(
                "registerEventProcessor" to { sm -> sm.registerEventProcessor(name) { emptyList() } },
                "registerInvoker" to { sm -> sm.registerInvoker(name) { null } },
            )
            for ((call, register) in registrations) {
                val sm = machine()
                try {
                    val refused = runCatching { register(sm) }.exceptionOrNull() is IllegalArgumentException
                    assertEquals(reserved, refused, "$call(\"$name\") refused=$refused")
                } finally {
                    sm.cleanup()
                }
            }
        }
    }

    /// The router's door serves the type the general one refuses.
    @Test
    fun aMeshRouterIsRegisteredThroughItsOwnDoor() {
        val sm = machine()
        try {
            assertFalse(sm.hasEventProcessor(MESH_PROCESSOR_TYPE))
            sm.registerMeshRouter { emptyList() }
            assertTrue(sm.hasEventProcessor(MESH_PROCESSOR_TYPE))
        } finally {
            sm.cleanup()
        }
    }
}
