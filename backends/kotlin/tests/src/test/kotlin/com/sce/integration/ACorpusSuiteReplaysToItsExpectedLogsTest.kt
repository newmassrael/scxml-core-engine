// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// docs/SCE_TTCN3_LANE.md §12, §13.1 S2: every trace of a suite of the corpus
// replays, through the suite's runner ([runRunner], the program of §4 run in
// place), to its expected log, byte for byte. The suites are the ones
// sce-codegen writes into `src/main/kotlin/com/sce/generated/` (regen:
// `scripts/regen_static_datamodel_kotlin.sh`); runtime_suite's own traces are
// replayed by ATestSuiteRunsThroughItsRunnerTest.kt, beside the rest of what
// that suite is held to.
//
// The expected logs are the files every backend is held to, the canonical logs
// a reviewed run of the Rust runner wrote. Each case is the twin of the Rust case
// of the same name in
// backends/rust/tests/tests/the_corpus_replays_to_its_expected_logs.rs.

package com.sce.integration

import com.sce.runtime.test.JvmRunnerFiles
import com.sce.runtime.test.ParameterReader
import com.sce.runtime.test.Suite
import com.sce.runtime.test.runRunner
import java.io.File
import java.nio.file.Files
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue
import com.sce.generated.boundary_suite.moduleParameter as boundaryModuleParameter
import com.sce.generated.boundary_suite.suite as boundarySuite
import com.sce.generated.job_suite.moduleParameter as jobModuleParameter
import com.sce.generated.job_suite.suite as jobSuite
import com.sce.generated.nested_suite.moduleParameter as nestedModuleParameter
import com.sce.generated.nested_suite.suite as nestedSuite
import com.sce.generated.orders_suite.moduleParameter as ordersModuleParameter
import com.sce.generated.orders_suite.suite as ordersSuite

class ACorpusSuiteReplaysToItsExpectedLogsTest {

    /** A suite of the corpus and how its runner is run over its traces. */
    private class Replayed(
        /** The suite's unit name, which names the directory of its runs. */
        val name: String,
        /** The suite's table, which the runner runs. */
        val suite: Suite,
        /** The reader of a module parameter's value from the command line. */
        val parameter: ParameterReader,
        /** The world that selects every test case, as `--world` takes it. */
        val world: String,
        /** Each test case its world selects, in the suite's order, with its verdict. */
        val verdicts: List<String>,
    )

    private fun repoRoot(): File =
        generateSequence(File(System.getProperty("user.dir")).absoluteFile) { it.parentFile }
            .firstOrNull { File(it, "sce-build").isDirectory }
            ?: error("no repository root above ${System.getProperty("user.dir")}")

    /** The directory of the runs of [suite]: its `traces/` and `expected_logs/`. */
    private fun runsDir(suite: String): File =
        File(repoRoot(), "sce-build/tests/fixtures/test_profile/runs/$suite")

    /** The name of every file in [dir], sorted. */
    private fun fileNames(dir: File): List<String> =
        (dir.list() ?: error("the directory $dir is read")).sorted()

    /**
     * Runs the suite's runner over every trace of its directory, in the world
     * that selects every test case, and holds what it prints to the verdicts it
     * expects, every log it writes to its expected log, and the set of logs to
     * the set of expected logs.
     */
    private fun replays(replayed: Replayed) {
        val runs = runsDir(replayed.name)
        val logs = File(Files.createTempDirectory("sce-corpus-runner-${replayed.name}").toFile(), "logs")
        val out = mutableListOf<String>()
        val err = mutableListOf<String>()
        val status = runRunner(
            suite = replayed.suite,
            parameter = replayed.parameter,
            args = listOf(
                "--trace-dir",
                File(runs, "traces").path,
                "--log-dir",
                logs.path,
                "--world",
                replayed.world,
            ),
            files = JvmRunnerFiles,
            out = { out.add(it) },
            err = { err.add(it) },
        )
        assertEquals(0, status, "$err")
        assertEquals(replayed.verdicts, out, "$err")
        val expectedDir = File(runs, "expected_logs")
        val expected = fileNames(expectedDir).map { name ->
            assertTrue(name.endsWith(".json"), "the expected log $name is a .json file")
            name.removeSuffix(".json")
        }
        val written = fileNames(logs).map { name ->
            assertTrue(name.endsWith(".log.json"), "the runner wrote $name, which is no log of a case")
            val testCase = name.removeSuffix(".log.json")
            val want = File(expectedDir, "$testCase.json").readText(Charsets.UTF_8)
            assertTrue(want.endsWith("\n"), "the expected log of $testCase ends in one newline")
            assertEquals(
                want.removeSuffix("\n"),
                File(logs, name).readText(Charsets.UTF_8),
                "the log of $testCase of ${replayed.name} is not its expected log",
            )
            testCase
        }
        assertEquals(
            expected,
            written,
            "the runner wrote a log of each case that has an expected log, and of no other",
        )
        logs.parentFile.deleteRecursively()
    }

    /**
     * docs/SCE_TTCN3_LANE.md §12.2: signed and 64-bit integers in a message,
     * the verdicts `fail` and `none`, the two ways to `error` a document leads
     * a run to, a test case that can never end and a dynamic error, and a
     * branch taken before the states it leaves log their exits
     * (docs/SCE_TEST_PROFILE.md §4.4).
     */
    @Test
    fun `the boundary suite replays to its expected logs`() {
        replays(
            Replayed(
                name = "boundary_suite",
                suite = boundarySuite(),
                parameter = ::boundaryModuleParameter,
                world = "",
                verdicts = listOf(
                    "exchanges_signed_numbers pass",
                    "answers_another_job fail",
                    "sets_no_verdict none",
                    "never_ends error",
                    "sends_to_a_removed_ptc error",
                    "takes_before_its_exits pass",
                ),
            ),
        )
    }

    /**
     * docs/SCE_TEST_PROFILE.md §11.8: a status every matcher of
     * `every_matcher` accepts, at 1 ms, which the tester's branch reads as a
     * composed message; the helper is told and passes, and the tester settles
     * for the duration its start gives and passes at 501 ms.
     */
    @Test
    fun `the job suite replays to its expected logs`() {
        replays(
            Replayed(
                name = "job_suite",
                suite = jobSuite(),
                parameter = ::jobModuleParameter,
                world = "job-timeout,job-retry",
                verdicts = listOf("completes_in_time pass"),
            ),
        )
    }

    /**
     * docs/SCE_TEST_PROFILE.md §9.5: an order of two lines, a customer and a
     * card, which `order_placed_match` accepts over its union, its list of
     * records and its template references, at 1 ms.
     */
    @Test
    fun `the orders suite replays to its expected logs`() {
        replays(
            Replayed(
                name = "orders_suite",
                suite = ordersSuite(),
                parameter = ::ordersModuleParameter,
                world = "",
                verdicts = listOf("orders_case pass"),
            ),
        )
    }

    /**
     * docs/SCE_TEST_PROFILE.md §4.5 (P5), §4.7 (P7): an order at 100 ms, which
     * the atomic state's own targetless branch takes before its ancestor's, and
     * one at 500 ms, which the ancestor's internal branch takes, restarting the
     * running timer; the inner timer then runs out twice and the tester passes
     * at 1250 ms.
     */
    @Test
    fun `the nested suite replays to its expected logs`() {
        replays(
            Replayed(
                name = "nested_suite",
                suite = nestedSuite(),
                parameter = ::nestedModuleParameter,
                world = "",
                verdicts = listOf("nested_case pass"),
            ),
        )
    }
}
