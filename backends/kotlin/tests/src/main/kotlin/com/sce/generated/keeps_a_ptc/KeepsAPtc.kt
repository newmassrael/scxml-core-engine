// SCE-GENERATED — DO NOT EDIT
// source-hash: 011d804f887126b888ea6ba5b4213d56e0f5c8740555a154c83e9b8d0105c4b6
// SCE-MAP: keeps_a_ptc.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-case")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.keeps_a_ptc

/** The test case's name in the suite and in the test log (docs/SCE_TEST_PROFILE.md §11.7). */
const val NAME: String = "keeps_a_ptc"

/**
 * The test case `keeps_a_ptc` (docs/SCE_TEST_PROFILE.md §11.7) as the runner reads it:
 * its name, its configuration and the machines of its components, and the module
 * parameters those components read.
 */
fun testCase(): com.sce.runtime.test.TestCase = com.sce.runtime.test.TestCase(
    name = NAME,
    configuration = com.sce.generated.keeping_configuration.configuration(),
    machines = { componentType -> com.sce.generated.keeping_configuration.machine(componentType) },
    moduleParameters = com.sce.generated.keeping_configuration.moduleParameters(),
)
