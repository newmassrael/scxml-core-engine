// SCE-GENERATED — DO NOT EDIT
// source-hash: be94512ba39db699a331b27cc31692bda58f20eb3a2d7dac3ed4ad84dab78fc5
// source: orders_case.scxml
// SCE-MAP: orders_case.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-case")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.orders_case

/** The test case's name in the suite and in the test log (docs/SCE_TEST_PROFILE.md §11.7). */
const val NAME: String = "orders_case"

/**
 * The test case `orders_case` (docs/SCE_TEST_PROFILE.md §11.7) as the runner reads it:
 * its name, its configuration and the machines of its components, the module
 * parameters those components read, and the reader of the inputs a recorded trace of it
 * carries (docs/SCE_TTCN3_LANE.md §10.5).
 */
fun testCase(): com.sce.runtime.test.TestCase = com.sce.runtime.test.TestCase(
    name = NAME,
    configuration = com.sce.generated.orders_configuration.configuration(),
    machines = { componentType -> com.sce.generated.orders_configuration.machine(componentType) },
    moduleParameters = com.sce.generated.orders_configuration.moduleParameters(),
    inputs = { systemPort, typeName, value -> com.sce.generated.orders_configuration.input(systemPort, typeName, value) },
)
