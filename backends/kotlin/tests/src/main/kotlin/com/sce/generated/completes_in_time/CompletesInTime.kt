// SCE-GENERATED — DO NOT EDIT
// source-hash: 5610f916540cc7a66f5ffd5f3c581afae0c543595d6e705745f49fe56e124377
// source: completes_in_time.scxml
// SCE-MAP: completes_in_time.scxml:3 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-case")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.completes_in_time

/** The test case's name in the suite and in the test log (docs/SCE_TEST_PROFILE.md §11.7). */
const val NAME: String = "completes_in_time"

/**
 * The test case `completes_in_time` (docs/SCE_TEST_PROFILE.md §11.7) as the runner reads it:
 * its name, its configuration and the machines of its components, the module
 * parameters those components read, and the reader of the inputs a recorded trace of it
 * carries (docs/SCE_TTCN3_LANE.md §10.5).
 */
fun testCase(): com.sce.runtime.test.TestCase = com.sce.runtime.test.TestCase(
    name = NAME,
    configuration = com.sce.generated.tester_and_helper.configuration(),
    machines = { componentType -> com.sce.generated.tester_and_helper.machine(componentType) },
    moduleParameters = com.sce.generated.tester_and_helper.moduleParameters(),
    inputs = { systemPort, typeName, value -> com.sce.generated.tester_and_helper.input(systemPort, typeName, value) },
)
