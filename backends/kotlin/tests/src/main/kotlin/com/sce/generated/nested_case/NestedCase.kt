// SCE-GENERATED — DO NOT EDIT
// source-hash: 1e281a8396692195e709bec100279034d644b5bab0a3c6d9b461fbae9b46fce6
// source: nested_case.scxml
// SCE-MAP: nested_case.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-case")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.nested_case

/** The test case's name in the suite and in the test log (docs/SCE_TEST_PROFILE.md §11.7). */
const val NAME: String = "nested_case"

/**
 * The test case `nested_case` (docs/SCE_TEST_PROFILE.md §11.7) as the runner reads it:
 * its name, its configuration and the machines of its components, the module
 * parameters those components read, and the reader of the inputs a recorded trace of it
 * carries (docs/SCE_TTCN3_LANE.md §10.5).
 */
fun testCase(): com.sce.runtime.test.TestCase = com.sce.runtime.test.TestCase(
    name = NAME,
    configuration = com.sce.generated.nested_configuration.configuration(),
    machines = { componentType -> com.sce.generated.nested_configuration.machine(componentType) },
    moduleParameters = com.sce.generated.nested_configuration.moduleParameters(),
    inputs = { systemPort, typeName, value -> com.sce.generated.nested_configuration.input(systemPort, typeName, value) },
)
