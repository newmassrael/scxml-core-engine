// SCE-GENERATED — DO NOT EDIT
// source-hash: 09f5c1752a44bc80450b6558b46b82c35fec8d2e2bc36fe630ffd183079ea6c2
// source: sets_no_verdict.scxml
// SCE-MAP: sets_no_verdict.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-case")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.sets_no_verdict

/** The test case's name in the suite and in the test log (docs/SCE_TEST_PROFILE.md §11.7). */
const val NAME: String = "sets_no_verdict"

/**
 * The test case `sets_no_verdict` (docs/SCE_TEST_PROFILE.md §11.7) as the runner reads it:
 * its name, its configuration and the machines of its components, the module
 * parameters those components read, and the reader of the inputs a recorded trace of it
 * carries (docs/SCE_TTCN3_LANE.md §10.5).
 */
fun testCase(): com.sce.runtime.test.TestCase = com.sce.runtime.test.TestCase(
    name = NAME,
    configuration = com.sce.generated.unset_configuration.configuration(),
    machines = { componentType -> com.sce.generated.unset_configuration.machine(componentType) },
    moduleParameters = com.sce.generated.unset_configuration.moduleParameters(),
    inputs = { systemPort, typeName, value -> com.sce.generated.unset_configuration.input(systemPort, typeName, value) },
)
