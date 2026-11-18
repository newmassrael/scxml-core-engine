// SCE-GENERATED — DO NOT EDIT
// source-hash: 4fc3202cd282b89817f9b6bc17d27c57e6f71fb81651493178a84da56c1cae6a
// SCE-MAP: job_suite.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-suite")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.job_suite

/**
 * The test suite `job_suite` (docs/SCE_TEST_PROFILE.md §11.8) as the runner reads it
 * (`com.sce.runtime.test.runSuite`): each module parameter at its default, held as the saved value
 * of its declared type, and each `<sce:execute>` in run order, with its test case, its guard
 * and the selection that runs it in a world of the suite's features.
 */
fun suite(): com.sce.runtime.test.Suite {
    val moduleParameters = com.sce.runtime.test.ModuleParameters()
    moduleParameters.set("elapsed_limit", com.sce.runtime.SavedValues.of(500u))
    moduleParameters.set("run_label", com.sce.runtime.SavedValues.of("nightly"))
    moduleParameters.set("key", com.sce.runtime.SavedValues.of(byteArrayOf(0x0a.toByte(), 0xff.toByte())))
    return com.sce.runtime.test.Suite(
        name = "job_suite",
        moduleParameters = moduleParameters,
        executes = listOf(
            com.sce.runtime.test.Execute(
                testCase = com.sce.generated.completes_in_time.testCase(),
                guard = 10000000L,
                selection = { world -> com.sce.runtime.test.enabled(world, "job-timeout") && com.sce.runtime.test.enabled(world, "job-retry") },
            ),
        ),
    )
}

/**
 * The value the runner is given for the module parameter [id] (docs/SCE_TTCN3_LANE.md §4,
 * `--module-parameter id=value`), a literal of its declared type as the `value` of a
 * `<sce:module-parameter>` writes one (docs/SCE_TEST_PROFILE.md §11.4), read as the saved value
 * of that type.
 *
 * @throws com.sce.runtime.test.OverrideRefusal naming why, where the suite declares no such parameter
 *   or the literal is no value of its type.
 */
fun moduleParameter(id: String, literal: String): com.sce.runtime.test.ModuleParameterValue = when (id) {
    "elapsed_limit" -> com.sce.runtime.SavedValues.of(com.sce.runtime.test.integerParameter(id, literal, com.sce.runtime.test.FromCanonical.UINT32))
    "run_label" -> com.sce.runtime.test.textParameter(literal)
    "key" -> com.sce.runtime.test.bytesParameter(id, literal)
    else -> throw com.sce.runtime.test.undeclaredParameter(id)
}

/**
 * The runner program of the suite `job_suite` (docs/SCE_TTCN3_LANE.md §4): every test
 * case the world of `--world` selects, run against its recorded input trace from
 * `--trace-dir`, its canonical test log written to `--log-dir` (`com.sce.runtime.test.runnerMain`).
 * The class of this unit is the program.
 */
fun main(args: Array<String>) {
    com.sce.runtime.test.runnerMain(suite(), ::moduleParameter, args)
}
