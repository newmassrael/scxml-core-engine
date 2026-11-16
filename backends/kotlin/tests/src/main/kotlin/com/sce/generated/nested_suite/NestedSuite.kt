// SCE-GENERATED — DO NOT EDIT
// source-hash: 12a7dc72335a819ac5590a1b6b46aee0f5b664147671271baaaa11524ee4c7b9
// SCE-MAP: nested_suite.scxml:5 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-suite")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.nested_suite

/**
 * The test suite `nested_suite` (docs/SCE_TEST_PROFILE.md §11.8) as the runner reads it
 * (`com.sce.runtime.test.runSuite`): each module parameter at its default, held as the saved value
 * of its declared type, and each `<sce:execute>` in run order, with its test case, its guard
 * and the selection that runs it in a world of the suite's features.
 */
fun suite(): com.sce.runtime.test.Suite {
    val moduleParameters = com.sce.runtime.test.ModuleParameters()
    return com.sce.runtime.test.Suite(
        name = "nested_suite",
        moduleParameters = moduleParameters,
        executes = listOf(
            com.sce.runtime.test.Execute(
                testCase = com.sce.generated.nested_case.testCase(),
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
@Suppress("UNUSED_PARAMETER")
fun moduleParameter(id: String, literal: String): com.sce.runtime.test.ModuleParameterValue =
    throw com.sce.runtime.test.undeclaredParameter(id)

/**
 * The runner program of the suite `nested_suite` (docs/SCE_TTCN3_LANE.md §4): every test
 * case the world of `--world` selects, run against its recorded input trace from
 * `--trace-dir`, its canonical test log written to `--log-dir` (`com.sce.runtime.test.runnerMain`).
 * The class of this unit is the program.
 */
fun main(args: Array<String>) {
    com.sce.runtime.test.runnerMain(suite(), ::moduleParameter, args)
}
