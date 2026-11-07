// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A test suite of the test profile as its runner reads it
// (docs/SCE_TEST_PROFILE.md §4.11, §11.7, §11.8, §12.2): the test cases in run
// order, each with its guard and the selection that runs it in a world of the
// suite's features, and the suite's module parameters; and the runner, which
// runs the cases a world selects one at a time, in that order.
//
// The tables are generated from the `test-case` and `test-suite` documents (a
// case's configuration from its `test-configuration`). They are data: what a
// case does is its components' machines, which the run drives ([TestCaseRun]).
// The runner is a library function, with no `main`: a generated or hand-written
// test calls it with the system adapters that play the SUT.
//
// Cross-language siblings: `sce_rust_runtime::test::run_suite` (Rust),
// `SCE::Test::runSuite` (C++).

package com.sce.runtime.test

/**
 * The features a world of a suite's feature model enables (ADR 0057 §2): a
 * configuration of the model, closed under its `requires` edges.
 */
typealias Features = Set<String>

/** Whether [world] enables [feature]: the one test a selection is made of. */
fun enabled(world: Features, feature: String): Boolean = feature in world

/**
 * A `test-case` (§11.7): its name and what it runs in.
 *
 * The configuration and the machines of its components are its configuration's;
 * the module parameters are the ids every component of the configuration reads,
 * which the suite must give.
 */
data class TestCase(
    /** The test case's name in the suite and in the test log. */
    val name: String,
    /** The configuration it runs. */
    val configuration: Configuration,
    /** The machines of the configuration's component types. */
    val machines: MachineFactory,
    /** The ids of the module parameters its components read. */
    val moduleParameters: List<String>,
)

/**
 * A selection (§11.8): whether a world of the suite's features runs a test
 * case. A condition of ADR 0057's grammar, lowered to the features it names.
 */
typealias Selection = (Features) -> Boolean

/** One `<sce:execute>` (§11.8, P11): a test case, its guard, and its selection. */
data class Execute(
    /** The test case. */
    val testCase: TestCase,
    /** The guard: the test case's time limit, if it has one. */
    val guard: Duration?,
    /** The selection, or null for a test case every world runs. */
    val selection: Selection?,
)

/**
 * A `test-suite` (§11.8): its test cases in run order and its module parameters
 * at their defaults.
 */
class Suite(
    /** The suite's name. */
    val name: String = "",
    /** The module parameters at their defaults. */
    val moduleParameters: ModuleParameters = ModuleParameters(),
    /** The test cases, in run order. */
    val executes: List<Execute> = emptyList(),
)

/** What the runner reports of one test case it ran. */
data class CaseResult(
    /** The test case's name. */
    val testCase: String,
    /** Its verdict and log. */
    val result: TestCaseResult,
)

/**
 * An override of a module parameter the suite does not declare: the caller's
 * mistake, not a verdict of any test case, so no case is run.
 */
class OverrideRefusal(
    /** Why. */
    val reason: String,
) : Exception(reason)

/** P13: the test case cannot start, so it ends at once with `error`. */
private fun refused(testCase: String, why: String): CaseResult = CaseResult(
    testCase = testCase,
    result = TestCaseResult(
        verdict = Verdict.ERROR,
        endedAt = 0,
        log = listOf(LogRecord(at = 0, component = MTC_NAME, kind = LogKind.ERROR, detail = why)),
    ),
)

/**
 * Runs the test cases of [suite] that [world] selects (P11).
 *
 * One at a time, on the calling thread, in the suite's order, each a fresh run
 * from `NOW = 0` with fresh entities against the adapter [adapters] builds for
 * it, given the test case's name, so that no case sees another's trace. Each
 * component reads the suite's module parameters, with the values [overrides]
 * gives in place of their defaults.
 *
 * A test case whose components read a module parameter the run has no value for
 * ends with `error` (P13) before any component is created, as does one for which
 * [adapters] builds no adapter.
 *
 * @throws OverrideRefusal when [overrides] gives a module parameter the suite
 *   does not declare, before any case runs.
 */
fun runSuite(
    suite: Suite,
    world: Features,
    adapters: (testCase: String) -> SystemAdapter?,
    overrides: ModuleParameters,
): List<CaseResult> {
    val parameters = suite.moduleParameters.copy()
    for ((id, value) in overrides.values) {
        if (!parameters.has(id)) {
            throw OverrideRefusal(
                "overrides the module parameter '$id', which the suite '${suite.name}' does not declare",
            )
        }
        parameters.set(id, value)
    }
    val results = mutableListOf<CaseResult>()
    for (execute in suite.executes) {
        val selects = execute.selection
        if (selects != null && !selects(world)) {
            continue
        }
        val testCase = execute.testCase
        val missing = testCase.moduleParameters.firstOrNull { !parameters.has(it) }
        if (missing != null) {
            results.add(
                refused(
                    testCase.name,
                    "the test case reads the module parameter '$missing', which is given no value",
                ),
            )
            continue
        }
        val adapter = adapters(testCase.name)
        if (adapter == null) {
            results.add(refused(testCase.name, "no system adapter is given for the test case"))
            continue
        }
        val run = TestCaseRun(testCase.configuration, testCase.machines, adapter, execute.guard, parameters)
        results.add(CaseResult(testCase = testCase.name, result = run.run()))
    }
    return results
}
