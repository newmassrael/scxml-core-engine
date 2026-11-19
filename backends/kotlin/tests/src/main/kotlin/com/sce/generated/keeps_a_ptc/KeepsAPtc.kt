// SCE-GENERATED — DO NOT EDIT
// source-hash: 4c07dde9a70b631ae3726315c43861e16868a63e80a69d0ff7b4427d4a0f63ae
// source: keeps_a_ptc.scxml
// SCE-MAP: keeps_a_ptc.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-case")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.keeps_a_ptc

/** The test case's name in the suite and in the test log (docs/SCE_TEST_PROFILE.md §11.7). */
const val NAME: String = "keeps_a_ptc"

/**
 * The test case `keeps_a_ptc` (docs/SCE_TEST_PROFILE.md §11.7) as the runner reads it:
 * its name, its configuration and the machines of its components, the module
 * parameters those components read, and the reader of the inputs a recorded trace of it
 * carries (docs/SCE_TTCN3_LANE.md §10.5).
 */
fun testCase(): com.sce.runtime.test.TestCase = com.sce.runtime.test.TestCase(
    name = NAME,
    configuration = com.sce.generated.keeping_configuration.configuration(),
    machines = { componentType -> com.sce.generated.keeping_configuration.machine(componentType) },
    moduleParameters = com.sce.generated.keeping_configuration.moduleParameters(),
    inputs = { systemPort, typeName, value -> com.sce.generated.keeping_configuration.input(systemPort, typeName, value) },
)
