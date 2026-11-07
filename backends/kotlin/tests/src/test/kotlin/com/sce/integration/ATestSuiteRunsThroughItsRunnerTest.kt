// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A test suite generated in Kotlin (docs/SCE_TEST_PROFILE.md §11.6 to §11.8,
// §12.2, §12.6 slice 6), run end to end by the profile's runner,
// `com.sce.runtime.test.runSuite`. The suite, its test cases and their
// configurations are the tables sce-codegen writes from the fixtures under
// `sce-build/tests/fixtures/test_profile/runtime/` into
// `src/main/kotlin/com/sce/generated/` (regen:
// `scripts/regen_static_datamodel_kotlin.sh`), and the components are the
// machines it writes from the components there. Only the system adapter, which
// plays the system under test, is written here.
//
// Each case is the twin of the Rust case of the same name in
// backends/rust/tests/tests/a_test_suite_runs_through_its_runner.rs
// (`runs_every_test_case_of_a_world_with_the_verdict_it_expects` is
// `runsEveryTestCaseOfAWorldWithTheVerdictItExpects`), itself the twin of the
// C++ gtest in tests/integration/ATestSuiteRunsEndToEndTest.cpp, with the same
// suite, world, verdicts and `NOW`.

package com.sce.integration

import com.sce.generated.job_reply.JobReplyPayload
import com.sce.generated.job_request.JobRequestPayload
import com.sce.runtime.SavedValues
import com.sce.runtime.test.CaseResult
import com.sce.runtime.test.Duration
import com.sce.runtime.test.Features
import com.sce.runtime.test.Instant
import com.sce.runtime.test.LogKind
import com.sce.runtime.test.Message
import com.sce.runtime.test.ModuleParameters
import com.sce.runtime.test.SystemAdapter
import com.sce.runtime.test.Verdict
import com.sce.runtime.test.runSuite
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue
import com.sce.generated.answers_the_job.NAME as ANSWERS_THE_JOB
import com.sce.generated.answers_the_job.testCase as answersTheJob
import com.sce.generated.asking_configuration.configuration as askingConfiguration
import com.sce.generated.asking_configuration.machine as askingMachine
import com.sce.generated.asking_configuration.moduleParameters as askingModuleParameters
import com.sce.generated.job_reply.EVENT_NAME as JOB_REPLY
import com.sce.generated.runtime_suite.suite as runtimeSuite

class ATestSuiteRunsThroughItsRunnerTest {

    private companion object {
        const val MILLISECOND: Duration = 1000L
        const val SECOND: Duration = 1000L * MILLISECOND

        /** How long the system under test takes to answer a request. */
        const val ANSWER_DELAY: Duration = 500L
    }

    /**
     * The system under test on the system port `sut`: it answers each request,
     * ANSWER_DELAY after it, with a reply of [code] for the job the request
     * names, and records each job it was asked for in [requested], which the
     * test keeps across the test cases of a run while each run owns its adapter.
     */
    private class Responder(
        private val code: UInt,
        private val requested: MutableList<UInt>,
    ) : SystemAdapter {
        private val pending = mutableListOf<Pair<Instant, Message>>()

        override fun send(systemPort: String, message: Message, now: Instant) {
            assertEquals("sut", systemPort)
            val request = assertNotNull(
                message.valueAs<JobRequestPayload>(),
                "a request travels as its record's value",
            )
            requested.add(request.job)
            val reply = JobReplyPayload(job = request.job, code = code)
            pending.add((now + ANSWER_DELAY) to Message(JOB_REPLY, reply))
        }

        override fun nextInput(): Instant? = pending.firstOrNull()?.first

        override fun takeInputsDue(now: Instant): List<Pair<String, Message>> {
            val due = pending.takeWhile { (at, _) -> at <= now }
            repeat(due.size) { pending.removeAt(0) }
            return due.map { (_, message) -> "sut" to message }
        }
    }

    /**
     * The suite's run in a world of [features], answered with [code] by a fresh
     * Responder for each test case, with [overrides] in place of the defaults
     * of the module parameters they give.
     */
    private fun run(
        features: Features,
        code: UInt,
        requested: MutableList<UInt>,
        overrides: ModuleParameters = ModuleParameters(),
    ): List<CaseResult> = runSuite(runtimeSuite(), features, { _ -> Responder(code, requested) }, overrides)

    private fun names(results: List<CaseResult>): List<String> = results.map { it.testCase }

    private fun logged(result: CaseResult, kind: LogKind): Boolean =
        result.result.log.any { it.kind == kind }

    /**
     * P11, P12, P13, P14: in a world with every feature the suite runs every
     * test case in its order, and each ends with the verdict it expects: a job
     * answered in time passes, a PTC served passes, a removed PTC's
     * inconclusive outweighs its MTC's pass, and a guard shorter than the
     * answer ends in error.
     */
    @Test
    fun runsEveryTestCaseOfAWorldWithTheVerdictItExpects() {
        val requested = mutableListOf<UInt>()
        val results = run(setOf("ptcs", "removal", "guard"), 0u, requested)
        assertEquals(
            listOf("answers_the_job", "serves_a_ptc", "keeps_a_ptc", "answers_before_the_guard"),
            names(results),
        )
        val outcome = results.map { it.result }
        assertEquals(Verdict.PASS, outcome[0].verdict, "${outcome[0].log}")
        assertEquals(ANSWER_DELAY, outcome[0].endedAt, "the reply is taken in the round it arrives in")
        assertEquals(Verdict.PASS, outcome[1].verdict, "${outcome[1].log}")
        assertEquals(0L, outcome[1].endedAt, "a branch fired in every round, so time never moved")
        assertEquals(Verdict.INCONCLUSIVE, outcome[2].verdict, "${outcome[2].log}")
        assertEquals(15L * MILLISECOND, outcome[2].endedAt)
        assertEquals(Verdict.ERROR, outcome[3].verdict, "${outcome[3].log}")
        assertEquals(100L, outcome[3].endedAt, "the guard, 0.1ms, ends the case before the answer")
        assertTrue(logged(results[3], LogKind.ERROR))
        assertEquals(
            listOf(7u, 7u),
            requested,
            "each asking case asks for the job the suite's module parameter gives",
        )
    }

    /**
     * P2, P7: a system under test that answers with a failure code fails the
     * test case when its timer runs out. A world with no feature runs only the
     * test case no selection guards.
     */
    @Test
    fun failsAJobTheSystemUnderTestAnswersWithAFailure() {
        val results = run(emptySet(), 3u, mutableListOf())
        assertEquals(listOf("answers_the_job"), names(results))
        assertEquals(Verdict.FAIL, results[0].result.verdict)
        assertEquals(SECOND, results[0].result.endedAt, "time advances to the timer's instant")
        assertTrue(logged(results[0], LogKind.TIMEOUT))
    }

    /**
     * §11.8: a world runs the test cases whose selection it satisfies, in the
     * suite's order: `ptcs` alone selects the served PTC and not the removed
     * one, and `guard` selects the guarded case.
     */
    @Test
    fun runsTheTestCasesAWorldSelects() {
        val requested = mutableListOf<UInt>()
        assertEquals(
            listOf("answers_the_job", "serves_a_ptc"),
            names(run(setOf("ptcs"), 0u, requested)),
        )
        assertEquals(
            listOf("answers_the_job", "answers_before_the_guard"),
            names(run(setOf("guard"), 0u, requested)),
        )
    }

    /**
     * -1 §8.2.1, §11.5: a value the caller gives the module parameter replaces
     * the suite's default, and the component asks for the job it names.
     */
    @Test
    fun asksForTheJobAnOverrideOfItsModuleParameterNames() {
        val requested = mutableListOf<UInt>()
        val overrides = ModuleParameters()
        overrides.set("job", SavedValues.of(9u))
        val results = run(emptySet(), 0u, requested, overrides)
        assertEquals(1, results.size)
        assertEquals(Verdict.PASS, results[0].result.verdict, "${results[0].result.log}")
        assertEquals(listOf(9u), requested)
    }

    /**
     * P13: a value of the module parameter that is not of the type the
     * component declares ends the test case in error before the component
     * starts.
     */
    @Test
    fun endsInErrorWhenAModuleParameterIsNotOfItsType() {
        val requested = mutableListOf<UInt>()
        val overrides = ModuleParameters()
        overrides.set("job", SavedValues.of("seven"))
        val results = run(emptySet(), 0u, requested, overrides)
        assertEquals(1, results.size)
        assertEquals(Verdict.ERROR, results[0].result.verdict)
        assertTrue(logged(results[0], LogKind.ERROR))
        assertTrue(requested.isEmpty(), "the component did not start, so it sent nothing")
    }

    /**
     * §11.6, §11.7: the configuration's tables name the module parameters its
     * components read, which a suite that runs it must give, and the test case
     * names itself and its configuration.
     */
    @Test
    fun readsTheModuleParametersItsConfigurationNames() {
        assertEquals(listOf("job"), askingModuleParameters())
        assertEquals("param_asker", askingConfiguration().mtcType)
        assertNull(askingMachine("nobody"))
        assertNotNull(askingMachine("param_asker"))
        val case = answersTheJob()
        assertEquals(ANSWERS_THE_JOB, case.name)
        assertEquals("answers_the_job", case.name)
        assertEquals(askingConfiguration(), case.configuration)
        assertEquals(listOf("job"), case.moduleParameters)
    }
}
