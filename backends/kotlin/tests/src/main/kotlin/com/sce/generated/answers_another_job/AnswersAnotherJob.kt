// SCE-GENERATED — DO NOT EDIT
// source-hash: 7abea7973bcdc2d068064b2dd6062cfe96348ca03b00db9f639c12ef4c2a89b3
// source: answers_another_job.scxml
// SCE-MAP: answers_another_job.scxml:5 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-case")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.answers_another_job

/** The test case's name in the suite and in the test log (docs/SCE_TEST_PROFILE.md §11.7). */
const val NAME: String = "answers_another_job"

/**
 * The test case `answers_another_job` (docs/SCE_TEST_PROFILE.md §11.7) as the runner reads it:
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
