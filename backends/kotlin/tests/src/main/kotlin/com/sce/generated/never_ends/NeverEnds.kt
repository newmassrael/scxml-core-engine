// SCE-GENERATED — DO NOT EDIT
// source-hash: 86c6dd30cc120de03266ccea6cca0d437b9db87ab00fd8a8b7241bb2567cde50
// source: never_ends.scxml
// SCE-MAP: never_ends.scxml:5 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-case")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.never_ends

/** The test case's name in the suite and in the test log (docs/SCE_TEST_PROFILE.md §11.7). */
const val NAME: String = "never_ends"

/**
 * The test case `never_ends` (docs/SCE_TEST_PROFILE.md §11.7) as the runner reads it:
 * its name, its configuration and the machines of its components, the module
 * parameters those components read, and the reader of the inputs a recorded trace of it
 * carries (docs/SCE_TTCN3_LANE.md §10.5).
 */
fun testCase(): com.sce.runtime.test.TestCase = com.sce.runtime.test.TestCase(
    name = NAME,
    configuration = com.sce.generated.stalled_configuration.configuration(),
    machines = { componentType -> com.sce.generated.stalled_configuration.machine(componentType) },
    moduleParameters = com.sce.generated.stalled_configuration.moduleParameters(),
    inputs = { systemPort, typeName, value -> com.sce.generated.stalled_configuration.input(systemPort, typeName, value) },
)
