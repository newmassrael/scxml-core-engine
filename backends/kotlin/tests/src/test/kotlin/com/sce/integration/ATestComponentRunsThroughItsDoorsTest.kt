// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Test components generated in Kotlin (docs/SCE_TEST_PROFILE.md §12.3, §12.6
// slice 6), run end to end by the profile's runtime, `TestCaseRun`. The
// machines are the statecharts sce-codegen writes from the fixtures under
// `sce-build/tests/fixtures/test_profile/runtime/` into
// `src/main/kotlin/com/sce/generated/<component>/`, the package a
// configuration's unit names them by, and the records and templates they name
// into `src/main/kotlin/com/sce/generated/` too (regen:
// `scripts/regen_static_datamodel_kotlin.sh`), driven only through the doors of
// §12.3; the configuration, the factory of machines and the system adapter are
// written here, so that each component is held apart from the generated tables,
// which `ATestSuiteRunsThroughItsRunnerTest.kt` runs.
//
// Each case is the twin of the Rust case of the same name in
// backends/rust/tests/tests/a_test_component_runs_through_its_doors.rs
// (`a_test_component_that_is_answered_with_a_matching_reply_passes` is
// `aTestComponentThatIsAnsweredWithAMatchingReplyPasses`), itself the twin of
// the C++ gtest in tests/integration/ATestComponentRunsThroughItsDoorsTest.cpp,
// with the same scenario, verdict, `NOW` and log.

package com.sce.integration

import com.sce.generated.answerer.AnswererComponent
import com.sce.generated.asker.AskerComponent
import com.sce.generated.coordinator.CoordinatorComponent
import com.sce.generated.idler.IdlerComponent
import com.sce.generated.job_reply.JobReplyPayload
import com.sce.generated.job_request.JobRequestPayload
import com.sce.generated.ptc_keeper.PtcKeeperComponent
import com.sce.runtime.test.ComponentMachine
import com.sce.runtime.test.Configuration
import com.sce.runtime.test.Duration
import com.sce.runtime.test.Endpoint
import com.sce.runtime.test.Instant
import com.sce.runtime.test.LogKind
import com.sce.runtime.test.MachineFactory
import com.sce.runtime.test.Message
import com.sce.runtime.test.ModuleParameters
import com.sce.runtime.test.SystemAdapter
import com.sce.runtime.test.TestCaseResult
import com.sce.runtime.test.TestCaseRun
import com.sce.runtime.test.Verdict
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue
import kotlin.test.fail
import com.sce.generated.job_reply.EVENT_NAME as JOB_REPLY
import com.sce.generated.job_request.EVENT_NAME as JOB_REQUEST
import com.sce.generated.reply_ok.TYPE as REPLY_OK_TYPE
import com.sce.generated.reply_ok.canonical as replyOkCanonical

class ATestComponentRunsThroughItsDoorsTest {

    private companion object {
        const val MILLISECOND: Duration = 1000L
        const val SECOND: Duration = 1000L * MILLISECOND

        /** How long the system under test takes to answer a request. */
        const val ANSWER_DELAY: Duration = 500L
    }

    /**
     * The generated machine of each component type a configuration here names,
     * and none for another type, which ends the run with `error`.
     */
    private fun machineOf(componentType: String): ComponentMachine? = when (componentType) {
        "asker" -> AskerComponent()
        "coordinator" -> CoordinatorComponent()
        "answerer" -> AnswererComponent()
        "ptc_keeper" -> PtcKeeperComponent()
        "idler" -> IdlerComponent()
        else -> null
    }

    private fun factory(): MachineFactory = { componentType -> machineOf(componentType) }

    /**
     * What the system under test was handed: each send, its system port and
     * event, and the job each request named.
     */
    private class Seen {
        val sent = mutableListOf<Pair<String, String>>()
        val requested = mutableListOf<UInt>()
    }

    /**
     * The system under test on the mapped port `sut`: it answers each request,
     * ANSWER_DELAY after it, with a reply of [code] for the requested job, or
     * for [job] when one is given. It records what it was sent in [seen].
     */
    private class Responder(
        private val job: UInt?,
        private val code: UInt,
        val seen: Seen = Seen(),
    ) : SystemAdapter {
        private val pending = mutableListOf<Pair<Instant, Message>>()

        override fun send(systemPort: String, message: Message, now: Instant) {
            seen.sent.add(systemPort to message.event)
            val request = message.valueAs<JobRequestPayload>() ?: return
            seen.requested.add(request.job)
            val reply = JobReplyPayload(job = job ?: request.job, code = code)
            // The reply's canonical form in the test log is the one the unit of
            // a template over its record writes.
            pending.add((now + ANSWER_DELAY) to Message.of(JOB_REPLY, reply, REPLY_OK_TYPE, ::replyOkCanonical))
        }

        override fun nextInput(): Instant? = pending.firstOrNull()?.first

        override fun takeInputsDue(now: Instant): List<Pair<String, Message>> {
            val due = pending.takeWhile { (at, _) -> at <= now }
            repeat(due.size) { pending.removeAt(0) }
            return due.map { (_, message) -> "sut" to message }
        }
    }

    /** A system under test nothing is mapped to. */
    private class Silent : SystemAdapter {
        override fun send(systemPort: String, message: Message, now: Instant) {
            fail(
                "no port is mapped, so nothing reaches the system adapter, yet ${message.event} " +
                    "was sent on $systemPort",
            )
        }

        override fun nextInput(): Instant? = null

        override fun takeInputsDue(now: Instant): List<Pair<String, Message>> = emptyList()
    }

    /** The index of the first record of [kind] by [component], or null. */
    private fun firstRecord(result: TestCaseResult, component: String, kind: LogKind): Int? =
        result.log.indexOfFirst { it.component == component && it.kind == kind }.takeIf { it >= 0 }

    /** The index of the first record of [kind] by [component], which the run logged. */
    private fun record(result: TestCaseResult, component: String, kind: LogKind): Int =
        assertNotNull(firstRecord(result, component, kind), "$component logs $kind: ${result.log}")

    private fun run(configuration: Configuration, adapter: SystemAdapter): TestCaseResult =
        TestCaseRun(configuration, factory(), adapter, null, ModuleParameters()).run()

    /** The asker as the MTC, its port `sut` mapped to the system port `sut`. */
    private fun askerConfiguration(): Configuration = Configuration(
        mtcType = "asker",
        portsOfType = mapOf("asker" to listOf("sut")),
        timersOfType = mapOf("asker" to listOf("reply")),
        mappings = listOf(Endpoint("mtc", "sut") to "sut"),
    )

    private fun runAsker(job: UInt?, code: UInt): Pair<TestCaseResult, Seen> {
        val responder = Responder(job, code)
        return run(askerConfiguration(), responder) to responder.seen
    }

    /**
     * P6, P2, P4: the send carries the template's value bound to the
     * component's variable, a reply its receive branch's template matches is
     * taken when it arrives, and the content reads the reply as `_event.data`.
     */
    @Test
    fun aTestComponentThatIsAnsweredWithAMatchingReplyPasses() {
        val (result, seen) = runAsker(null, 0u)
        assertEquals(Verdict.PASS, result.verdict, "${result.log}")
        assertEquals(ANSWER_DELAY, result.endedAt, "the reply is taken in the round it arrives in")
        assertEquals(
            listOf("sut" to JOB_REQUEST),
            seen.sent,
            "a message travels under its template's record's event",
        )
        assertEquals(listOf(7u), seen.requested, "the template's value, given `job`")
        assertNotNull(firstRecord(result, "mtc", LogKind.RECEIVE))
        // P8: the `<log>` of the branch's content is in the test log, with the
        // instant and the component, its label and the job it read.
        val logged = record(result, "mtc", LogKind.LOG)
        assertEquals("answered: 7", result.log[logged].detail)
        assertEquals(ANSWER_DELAY, result.log[logged].at)
    }

    /**
     * P2, P7, §7: a reply the template does not match enables no branch and is
     * not removed, and the timer's timeout branch fails the test case.
     */
    @Test
    fun aTestComponentThatIsAnsweredWithAReplyItsTemplateDoesNotMatchFailsWhenItsTimerFires() {
        val (result, _) = runAsker(null, 3u)
        assertEquals(Verdict.FAIL, result.verdict, "${result.log}")
        assertEquals(SECOND, result.endedAt, "time advances to the timer's instant")
        assertNull(
            firstRecord(result, "mtc", LogKind.RECEIVE),
            "a reply that does not match is never taken",
        )
        assertNotNull(firstRecord(result, "mtc", LogKind.TIMEOUT))
    }

    /**
     * P4: the content reads the message the branch took: a reply of success
     * for another job matches the template, which takes any job, and the
     * content's comparison of the job it read fails the test case.
     */
    @Test
    fun aTestComponentReadsTheReplyItTookAsTheEventsData() {
        val (result, _) = runAsker(8u, 0u)
        assertEquals(Verdict.FAIL, result.verdict, "${result.log}")
        assertEquals(ANSWER_DELAY, result.endedAt)
        assertNotNull(firstRecord(result, "mtc", LogKind.RECEIVE))
    }

    /**
     * P9, P6, P2: the MTC creates a PTC connected to it, which asks it for job
     * 42 as it starts; the MTC answers with the job it read, the PTC passes on
     * the reply and terminates, and the MTC's done branch passes on the next
     * snapshot. Leaving the invoking state kills the PTC, before the done
     * branch's content runs.
     */
    @Test
    fun aTestComponentCreatesAPtcExchangesAMessageWithItAndReadsItsDone() {
        val configuration = Configuration(
            mtcType = "coordinator",
            ptcTypes = mapOf("answerer" to "answerer"),
            portsOfType = mapOf(
                "coordinator" to listOf("peer"),
                "answerer" to listOf("peer"),
            ),
            connections = listOf(Endpoint("mtc", "peer") to Endpoint("answerer", "peer")),
        )
        val result = run(configuration, Silent())

        assertEquals(Verdict.PASS, result.verdict, "${result.log}")
        assertEquals(0L, result.endedAt, "a branch fired in every round, so time never moved")
        val created = record(result, "answerer", LogKind.CREATE)
        val asked = record(result, "answerer", LogKind.SEND)
        val answered = record(result, "mtc", LogKind.SEND)
        val done = record(result, "answerer", LogKind.DONE)
        val killed = record(result, "answerer", LogKind.KILL)
        val passed = record(result, "mtc", LogKind.VERDICT)
        assertTrue(created < asked, "the PTC is created when the macrostep that entered its state ends")
        // P8: a `<log>` that writes only a label is recorded, by the PTC.
        val noted = record(result, "answerer", LogKind.LOG)
        assertEquals("asked", result.log[noted].detail)
        assertTrue(asked < noted, "the log runs in its place, after the send before it")
        assertEquals("peer job.request", result.log[asked].detail)
        assertEquals("peer job.reply", result.log[answered].detail)
        assertTrue(answered < done)
        assertTrue(done < killed, "a done branch fires on a snapshot taken after the PTC terminated")
        assertTrue(
            killed < passed,
            "the invoking state is exited before the done branch's content runs",
        )
    }

    /**
     * P9, P14: leaving the state that invoked a PTC removes it while it still
     * runs, at that instant, and its local verdict still counts.
     */
    @Test
    fun aTestComponentKillsAPtcOnLeavingTheStateThatInvokedIt() {
        val configuration = Configuration(
            mtcType = "ptc_keeper",
            ptcTypes = mapOf("idler" to "idler"),
            portsOfType = mapOf("idler" to listOf("peer")),
            timersOfType = mapOf("ptc_keeper" to listOf("patience", "linger")),
        )
        val result = run(configuration, Silent())

        val killed = record(result, "idler", LogKind.KILL)
        assertEquals(
            10L * MILLISECOND,
            result.log[killed].at,
            "the PTC is removed when `patience` runs out",
        )
        assertNull(firstRecord(result, "idler", LogKind.DONE))
        assertEquals(15L * MILLISECOND, result.endedAt)
        assertEquals(
            Verdict.INCONCLUSIVE,
            result.verdict,
            "the removed PTC's inconclusive outweighs the MTC's pass",
        )
    }
}
