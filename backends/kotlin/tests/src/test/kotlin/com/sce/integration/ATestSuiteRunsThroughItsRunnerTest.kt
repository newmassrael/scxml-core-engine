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
// plays the system under test, is written here; the same suite is also replayed
// from the committed trace of `answers_the_job` (docs/SCE_TTCN3_LANE.md §10.5)
// through `com.sce.runtime.test.runSuiteWithTraces`, and from every committed
// trace, each to its expected log, through the suite's runner program (§4).
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
import com.sce.runtime.test.JvmRunnerFiles
import com.sce.runtime.test.LogKind
import com.sce.runtime.test.Message
import com.sce.runtime.test.ModuleParameters
import com.sce.runtime.test.RUNNER_USAGE
import com.sce.runtime.test.SystemAdapter
import com.sce.runtime.test.Trace
import com.sce.runtime.test.TraceRefusal
import com.sce.runtime.test.Verdict
import com.sce.runtime.test.runRunner
import com.sce.runtime.test.runSuite
import com.sce.runtime.test.runSuiteWithTraces
import java.io.File
import java.nio.file.Files
import java.util.concurrent.TimeUnit
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue
import com.sce.generated.answers_the_job.NAME as ANSWERS_THE_JOB
import com.sce.generated.answers_the_job.testCase as answersTheJob
import com.sce.generated.asking_configuration.canonical_job_reply as canonicalJobReply
import com.sce.generated.asking_configuration.configuration as askingConfiguration
import com.sce.generated.asking_configuration.input as askingInput
import com.sce.generated.asking_configuration.machine as askingMachine
import com.sce.generated.asking_configuration.moduleParameters as askingModuleParameters
import com.sce.generated.asking_configuration.read_job_reply as readJobReply
import com.sce.generated.job_reply.EVENT_NAME as JOB_REPLY
import com.sce.generated.reply_ok.TYPE as REPLY_OK_TYPE
import com.sce.generated.reply_ok.canonical as replyOkCanonical
import com.sce.generated.runtime_suite.moduleParameter as runtimeModuleParameter
import com.sce.generated.runtime_suite.suite as runtimeSuite

class ATestSuiteRunsThroughItsRunnerTest {

    private companion object {
        const val MILLISECOND: Duration = 1000L
        const val SECOND: Duration = 1000L * MILLISECOND

        /** How long the system under test takes to answer a request. */
        const val ANSWER_DELAY: Duration = 500L

        /**
         * The world that selects every test case of the suite, so that the
         * runner replays every trace of the directory.
         */
        const val EVERY_FEATURE = "ptcs,removal,guard"

        /**
         * The lines the runner prints in the world of every feature: each case
         * of the suite, in its order, with the verdict it ends with.
         */
        val EVERY_VERDICT = listOf(
            "answers_the_job pass",
            "serves_a_ptc pass",
            "keeps_a_ptc inconc",
            "answers_before_the_guard error",
        )
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

    private fun repoRoot(): File =
        generateSequence(File(System.getProperty("user.dir")).absoluteFile) { it.parentFile }
            .firstOrNull { File(it, "sce-build").isDirectory }
            ?: error("no repository root above ${System.getProperty("user.dir")}")

    /**
     * The test log every runtime writes of `answers_the_job` in a world with no
     * feature, answered with success: one file the Rust, C++ and Kotlin runs are
     * held to (`a_generated_test_case_writes_its_test_log` in
     * backends/rust/tests/tests/a_test_suite_runs_through_its_runner.rs), so the
     * agreement of the backends is one set of bytes. The file ends in one
     * newline, which no canonical text holds.
     */
    private fun answersTheJobLog(): String {
        val text = File(
            repoRoot(),
            "sce-build/tests/fixtures/test_profile/runtime/expected_logs/answers_the_job.json",
        ).readText()
        assertTrue(text.endsWith("\n") && !text.endsWith("\n\n"), "the shared log ends in one newline")
        return text.removeSuffix("\n")
    }

    /**
     * docs/SCE_TEST_PROFILE.md §12.5: the test log of a generated test case, in
     * its one canonical text, the bytes of the shared file. The request the
     * component sends carries the value its template's unit writes, the job of
     * the suite's module parameter; the reply arrives at 500 and is taken in
     * that round, which stops the timer, passes and ends the MTC.
     */
    @Test
    fun aGeneratedTestCaseWritesItsTestLog() {
        val results = run(emptySet(), 0u, mutableListOf())
        assertEquals(listOf("answers_the_job"), names(results))
        assertEquals(answersTheJobLog(), results[0].canonicalLog("runtime_suite"))
    }

    /** The directory of the suite's committed traces (docs/SCE_TTCN3_LANE.md §10.2). */
    private fun tracesDir(): File =
        File(repoRoot(), "sce-build/tests/fixtures/test_profile/runtime/traces")

    /**
     * The directory of the expected log of each committed trace, `<test
     * case>.json` each: the canonical log a reviewed run wrote of it, with one
     * newline after it.
     */
    private fun expectedLogsDir(): File =
        File(repoRoot(), "sce-build/tests/fixtures/test_profile/runtime/expected_logs")

    /**
     * docs/SCE_TTCN3_LANE.md §12, §13.1 S2: every log the runner wrote into
     * [logs], `<case>.log.json`, is the expected log of its case byte for byte,
     * and the runner wrote a log for every expected log and nothing else. The
     * twin of `assert_every_log_is_its_expected_log` (Rust).
     */
    private fun assertEveryLogIsItsExpectedLog(logs: File) {
        val expected = (expectedLogsDir().list() ?: error("${expectedLogsDir()} is not a directory")).sorted().map {
            assertTrue(it.endsWith(".json"), "the expected log $it is a .json file")
            it.removeSuffix(".json")
        }
        val written = (logs.list() ?: error("$logs is not a directory")).sorted().map { name ->
            assertTrue(name.endsWith(".log.json"), "the runner wrote $name, which is no log of a case")
            val testCase = name.removeSuffix(".log.json")
            val want = File(expectedLogsDir(), "$testCase.json")
            assertTrue(want.isFile, "the case $testCase has no expected log $want")
            val text = want.readText()
            assertTrue(
                text.endsWith("\n") && !text.endsWith("\n\n"),
                "the expected log of $testCase ends in one newline",
            )
            assertEquals(
                text.removeSuffix("\n"),
                File(logs, name).readText(),
                "the log of $testCase is not its expected log",
            )
            testCase
        }
        assertEquals(expected, written, "the runner wrote a log of each case that has an expected log, and of no other")
    }

    /**
     * docs/SCE_TTCN3_LANE.md §10.5: the generated suite replayed from the
     * committed trace of `answers_the_job` writes the test log every runtime
     * writes of it, byte for byte: the configuration's unit reads the reply from
     * its canonical form and writes it back the same, and the clock advances to
     * the trace's instant as it did to the Responder's. The twin of
     * `a_recorded_trace_replays_to_the_test_log_every_runtime_writes` (Rust).
     */
    @Test
    fun aRecordedTraceReplaysToTheTestLogEveryRuntimeWrites() {
        val text = File(tracesDir(), "answers_the_job.json").readText()
        val results = runSuiteWithTraces(runtimeSuite(), emptySet(), { _ -> Trace.parse(text) }, ModuleParameters())
        assertEquals(listOf("answers_the_job"), names(results))
        assertEquals(Verdict.PASS, results[0].result.verdict, "${results[0].result.log}")
        assertEquals(answersTheJobLog(), results[0].canonicalLog("runtime_suite"))
    }

    /**
     * §10.2, §10.5: the configuration's unit reads the reply from its canonical
     * form into the record its components take, and writes it back the same;
     * an input on a port it does not have, of a type the port does not take,
     * or of a value that does not belong to the type is refused by name.
     */
    @Test
    fun theConfigurationReadsTheInputsOfARecordedTrace() {
        val reply = readJobReply(com.sce.runtime.Json.parse("{\"code\":\"0\",\"job\":\"7\"}"))
        assertEquals(JobReplyPayload(job = 7u, code = 0u), reply)
        assertEquals("{\"code\":\"0\",\"job\":\"7\"}", com.sce.runtime.Json.writeJcs(canonicalJobReply(reply)))
        val message = askingInput("sut", "job_reply", canonicalJobReply(reply))
        assertEquals(JOB_REPLY, message.event)
        assertEquals("job_reply", message.typeName)
        assertEquals(reply, message.valueAs<JobReplyPayload>())
        assertEquals(
            "'job_request' is not on the `in` list of the type of the system port 'sut'",
            assertFailsWith<TraceRefusal> { askingInput("sut", "job_request", null) }.reason,
        )
        assertEquals(
            "'oracle' is not a system port of the test case's configuration",
            assertFailsWith<TraceRefusal> { askingInput("oracle", "job_reply", null) }.reason,
        )
        assertEquals(
            "job_reply.job: '07' is not the text of an integer's decimal digits",
            assertFailsWith<TraceRefusal> {
                askingInput("sut", "job_reply", com.sce.runtime.Json.parse("{\"code\":\"0\",\"job\":\"07\"}"))
            }.reason,
        )
    }

    /** What one run of the runner program printed and the status it exited with. */
    private class ProgramRun(val status: Int, val out: List<String>, val err: List<String>)

    /** A fresh directory, apart from every other test's, that the runner makes its logs in. */
    private fun logDir(): File = File(Files.createTempDirectory("sce-runtime-suite-runner").toFile(), "logs")

    /**
     * The suite's runner program run in place ([runRunner], over the
     * platform's files), given [args] after the directories of the traces,
     * the committed ones unless [traces] names others, and of the logs.
     */
    private fun runInPlace(logs: File, vararg args: String, traces: File = tracesDir()): ProgramRun {
        val out = mutableListOf<String>()
        val err = mutableListOf<String>()
        val status = runRunner(
            suite = runtimeSuite(),
            parameter = ::runtimeModuleParameter,
            args = listOf("--trace-dir", traces.path, "--log-dir", logs.path) + args,
            files = JvmRunnerFiles,
            out = { out.add(it) },
            err = { err.add(it) },
        )
        return ProgramRun(status, out, err)
    }

    /**
     * docs/SCE_TTCN3_LANE.md §4: the suite's runner program runs each test case
     * its world selects against its trace and writes `<case>.log.json`, the
     * canonical log with nothing after it, and prints each case's verdict. In
     * the world of every feature it replays every trace of the directory, and
     * each log it writes is its case's expected log (§12, §13.1 S2). A module
     * parameter's value on its command line replaces its default, and one the
     * suite does not declare, or that is no value of its type, is refused with
     * exit status 2 before any case runs. The twin of
     * `the_suites_runner_program_writes_the_log_of_each_trace` (Rust).
     */
    @Test
    fun theSuitesRunnerProgramWritesTheLogOfEachTrace() {
        val logs = logDir()
        val run = runInPlace(logs, "--world", EVERY_FEATURE)
        assertEquals(0, run.status, "${run.err}")
        assertEquals(EVERY_VERDICT, run.out)
        assertEveryLogIsItsExpectedLog(logs)
        logs.parentFile.deleteRecursively()

        val overridden = logDir()
        val given = runInPlace(overridden, "--world", "", "--module-parameter", "job=9")
        assertEquals(0, given.status, "${given.err}")
        val log = File(overridden, "answers_the_job.log.json").readText()
        assertTrue(
            log.contains(
                "\"kind\":\"message-sent\",\"port\":\"sut\",\"type\":\"job_request\",\"value\":{\"job\":\"9\"}",
            ),
            "the request names the job of the value given: $log",
        )
        overridden.parentFile.deleteRecursively()

        for ((literal, why) in listOf(
            "limit=1" to "the suite declares no module parameter 'limit'",
            "job=seven" to "--module-parameter job=seven: job: 'seven' is not the text of an integer's decimal digits",
        )) {
            val refused = logDir()
            val attempt = runInPlace(refused, "--module-parameter", literal)
            assertEquals(2, attempt.status, literal)
            assertTrue(attempt.err.any { it.contains(why) }, "${attempt.err}")
            assertEquals(RUNNER_USAGE, attempt.err.last())
            assertFalse(refused.exists(), "no case ran, so no log was written")
            refused.parentFile.deleteRecursively()
        }
    }

    /**
     * P13, §4: a test case whose trace is missing ends with `error` and its log
     * is written, the reason on the standard error, and the program exits 0,
     * since the verdict is the log's; a command line that is not the runner's,
     * and a log directory that cannot be made, exit 2. The directory of the
     * traces holds the committed trace of `answers_the_job` alone, so
     * `serves_a_ptc`'s is missing.
     */
    @Test
    fun aTestCaseWhoseTraceIsMissingEndsInErrorAndItsLogIsWritten() {
        val traces = Files.createTempDirectory("sce-runtime-suite-traces").toFile()
        File(tracesDir(), "answers_the_job.json").copyTo(File(traces, "answers_the_job.json"))
        val logs = logDir()
        val run = runInPlace(logs, "--world", "ptcs", traces = traces)
        assertEquals(0, run.status, "${run.err}")
        assertEquals(listOf("answers_the_job pass", "serves_a_ptc error"), run.out)
        assertEquals(
            "{\"records\":[{\"at\":\"0\",\"kind\":\"test-case-started\"},{\"at\":\"0\",\"kind\":\"test-case-ended\"," +
                "\"verdict\":\"error\"}],\"schema\":\"sce-test-log.v1\",\"suite\":\"runtime_suite\"," +
                "\"test_case\":\"serves_a_ptc\",\"verdict\":\"error\"}",
            File(logs, "serves_a_ptc.log.json").readText(),
        )
        assertTrue(
            run.err.any { it.startsWith("serves_a_ptc: mtc: the trace is refused: ") && it.contains("is not read") },
            "${run.err}",
        )
        logs.parentFile.deleteRecursively()
        traces.deleteRecursively()

        val usage = runRunner(
            suite = runtimeSuite(),
            parameter = ::runtimeModuleParameter,
            args = listOf("--log-dir", "logs"),
            files = JvmRunnerFiles,
            out = { error("a refused command line runs nothing: $it") },
            err = { },
        )
        assertEquals(2, usage)

        val blocked = Files.createTempFile("sce-runtime-suite-runner", ".log").toFile()
        val unmade = runInPlace(blocked)
        assertEquals(2, unmade.status)
        assertTrue(unmade.err.any { it.contains("is not made") }, "${unmade.err}")
        blocked.delete()
    }

    /**
     * §4: the suite's runner is a program: the class of the suite's unit, whose
     * `main` the generated code declares, run as its own JVM on the classpath of
     * this suite, prints the verdict of each case, writes its log and exits
     * with the status the command line asks for.
     */
    @Test
    fun theSuitesUnitIsTheRunnerProgram() {
        val scratch = Files.createTempDirectory("sce-runtime-suite-program").toFile()
        val program = { name: String, args: List<String> ->
            val out = File(scratch, "$name.out")
            val err = File(scratch, "$name.err")
            val process = ProcessBuilder(
                listOf(
                    File(System.getProperty("java.home"), "bin/java").path,
                    "-cp",
                    System.getProperty("java.class.path"),
                    "com.sce.generated.runtime_suite.RuntimeSuiteKt",
                    "--trace-dir",
                    tracesDir().path,
                    "--log-dir",
                    File(scratch, "$name-logs").path,
                ) + args,
            ).redirectOutput(out).redirectError(err).start()
            assertTrue(process.waitFor(60, TimeUnit.SECONDS), "the runner program ends")
            ProgramRun(process.exitValue(), out.readLines(), err.readLines())
        }
        val run = program("every", listOf("--world", EVERY_FEATURE))
        assertEquals(0, run.status, "${run.err}")
        assertEquals(EVERY_VERDICT, run.out, "the standard output is the contract's lines alone")
        // The standard error may hold diagnostics, whatever they say: none of
        // its lines is one of the standard output's.
        assertTrue(run.err.filter { it.isNotEmpty() }.none { it in run.out }, "${run.err}")
        assertEveryLogIsItsExpectedLog(File(scratch, "every-logs"))
        val refused = program("undeclared", listOf("--module-parameter", "limit=1"))
        assertEquals(2, refused.status)
        assertTrue(refused.err.any { it.contains("the suite declares no module parameter 'limit'") }, "${refused.err}")
        assertFalse(File(scratch, "undeclared-logs").exists(), "no case ran, so no log was written")
        scratch.deleteRecursively()
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
