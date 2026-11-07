// SCE-GENERATED — DO NOT EDIT
// source-hash: 021fdf0d0c3ee151c664e2ecf6a7e5a797d68aaf3d399831e8dfd49b9e10bec9
// SCE-MAP: runtime_suite.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-suite")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.runtime_suite

/**
 * The test suite `runtime_suite` (docs/SCE_TEST_PROFILE.md §11.8) as the runner reads it
 * (`com.sce.runtime.test.runSuite`): each module parameter at its default, held as the saved value
 * of its declared type, and each `<sce:execute>` in run order, with its test case, its guard
 * and the selection that runs it in a world of the suite's features.
 */
fun suite(): com.sce.runtime.test.Suite {
    val moduleParameters = com.sce.runtime.test.ModuleParameters()
    moduleParameters.set("job", com.sce.runtime.SavedValues.of(7u))
    return com.sce.runtime.test.Suite(
        name = "runtime_suite",
        moduleParameters = moduleParameters,
        executes = listOf(
            com.sce.runtime.test.Execute(
                testCase = com.sce.generated.answers_the_job.testCase(),
                guard = 10000000L,
                selection = null,
            ),
            com.sce.runtime.test.Execute(
                testCase = com.sce.generated.serves_a_ptc.testCase(),
                guard = null,
                selection = { world -> com.sce.runtime.test.enabled(world, "ptcs") },
            ),
            com.sce.runtime.test.Execute(
                testCase = com.sce.generated.keeps_a_ptc.testCase(),
                guard = null,
                selection = { world -> com.sce.runtime.test.enabled(world, "ptcs") && com.sce.runtime.test.enabled(world, "removal") },
            ),
            com.sce.runtime.test.Execute(
                testCase = com.sce.generated.answers_before_the_guard.testCase(),
                guard = 100L,
                selection = { world -> com.sce.runtime.test.enabled(world, "guard") || com.sce.runtime.test.enabled(world, "removal") },
            ),
        ),
    )
}
