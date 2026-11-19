// SCE-GENERATED — DO NOT EDIT
// source-hash: 3f07cb01aecace47d954fb98fd440e9fe0933cedeb33f75c177718596532e7b1
// source: boundary_suite.scxml
// SCE-MAP: boundary_suite.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-suite")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.boundary_suite

/**
 * The test suite `boundary_suite` (docs/SCE_TEST_PROFILE.md §11.8) as the runner reads it
 * (`com.sce.runtime.test.runSuite`): each module parameter at its default, held as the saved value
 * of its declared type, and each `<sce:execute>` in run order, with its test case, its guard
 * and the selection that runs it in a world of the suite's features.
 */
fun suite(): com.sce.runtime.test.Suite {
    val moduleParameters = com.sce.runtime.test.ModuleParameters()
    moduleParameters.set("job", com.sce.runtime.SavedValues.of(7u))
    return com.sce.runtime.test.Suite(
        name = "boundary_suite",
        moduleParameters = moduleParameters,
        executes = listOf(
            com.sce.runtime.test.Execute(
                testCase = com.sce.generated.exchanges_signed_numbers.testCase(),
                guard = 5000000L,
                selection = null,
            ),
            com.sce.runtime.test.Execute(
                testCase = com.sce.generated.answers_another_job.testCase(),
                guard = 5000000L,
                selection = null,
            ),
            com.sce.runtime.test.Execute(
                testCase = com.sce.generated.sets_no_verdict.testCase(),
                guard = null,
                selection = null,
            ),
            com.sce.runtime.test.Execute(
                testCase = com.sce.generated.never_ends.testCase(),
                guard = null,
                selection = null,
            ),
            com.sce.runtime.test.Execute(
                testCase = com.sce.generated.sends_to_a_removed_ptc.testCase(),
                guard = null,
                selection = null,
            ),
            com.sce.runtime.test.Execute(
                testCase = com.sce.generated.takes_before_its_exits.testCase(),
                guard = 5000000L,
                selection = null,
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
    "job" -> com.sce.runtime.SavedValues.of(com.sce.runtime.test.integerParameter(id, literal, com.sce.runtime.test.FromCanonical.UINT32))
    else -> throw com.sce.runtime.test.undeclaredParameter(id)
}

/**
 * The runner program of the suite `boundary_suite` (docs/SCE_TTCN3_LANE.md §4): every test
 * case the world of `--world` selects, run against its recorded input trace from
 * `--trace-dir`, its canonical test log written to `--log-dir` (`com.sce.runtime.test.runnerMain`).
 * The class of this unit is the program.
 */
fun main(args: Array<String>) {
    com.sce.runtime.test.runnerMain(suite(), ::moduleParameter, args)
}
