// SCE-GENERATED — DO NOT EDIT
// source-hash: d6ccd465426296be6d3dd55342ede2588f97a29d5d742475fbe13852a51f69a9
// source: answers_the_job.scxml
// SCE-MAP: answers_the_job.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-case")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.answers_the_job

/** The test case's name in the suite and in the test log (docs/SCE_TEST_PROFILE.md §11.7). */
const val NAME: String = "answers_the_job"

/**
 * The test case `answers_the_job` (docs/SCE_TEST_PROFILE.md §11.7) as the runner reads it:
 * its name, its configuration and the machines of its components, the module
 * parameters those components read, and the reader of the inputs a recorded trace of it
 * carries (docs/SCE_TTCN3_LANE.md §10.5).
 */
fun testCase(): com.sce.runtime.test.TestCase = com.sce.runtime.test.TestCase(
    name = NAME,
    configuration = com.sce.generated.asking_configuration.configuration(),
    machines = { componentType -> com.sce.generated.asking_configuration.machine(componentType) },
    moduleParameters = com.sce.generated.asking_configuration.moduleParameters(),
    inputs = { systemPort, typeName, value -> com.sce.generated.asking_configuration.input(systemPort, typeName, value) },
)
