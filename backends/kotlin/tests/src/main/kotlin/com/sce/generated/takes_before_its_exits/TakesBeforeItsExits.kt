// SCE-GENERATED — DO NOT EDIT
// source-hash: da8cfaa8149c596ce55c8e1b8c2853ee2c20bb8f1999394376d21de2c156738a
// source: takes_before_its_exits.scxml
// SCE-MAP: takes_before_its_exits.scxml:6 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-case")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.takes_before_its_exits

/** The test case's name in the suite and in the test log (docs/SCE_TEST_PROFILE.md §11.7). */
const val NAME: String = "takes_before_its_exits"

/**
 * The test case `takes_before_its_exits` (docs/SCE_TEST_PROFILE.md §11.7) as the runner reads it:
 * its name, its configuration and the machines of its components, the module
 * parameters those components read, and the reader of the inputs a recorded trace of it
 * carries (docs/SCE_TTCN3_LANE.md §10.5).
 */
fun testCase(): com.sce.runtime.test.TestCase = com.sce.runtime.test.TestCase(
    name = NAME,
    configuration = com.sce.generated.leaving_configuration.configuration(),
    machines = { componentType -> com.sce.generated.leaving_configuration.machine(componentType) },
    moduleParameters = com.sce.generated.leaving_configuration.moduleParameters(),
    inputs = { systemPort, typeName, value -> com.sce.generated.leaving_configuration.input(systemPort, typeName, value) },
)
