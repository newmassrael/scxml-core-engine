// SCE-GENERATED — DO NOT EDIT
// source-hash: 6ea2a2e9fa7aacdc8d25262d487485ed2e56685af55d9ad67516c207181b7a16
// source: replays_every_form.scxml
// SCE-MAP: replays_every_form.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-case")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.replays_every_form

/** The test case's name in the suite and in the test log (docs/SCE_TEST_PROFILE.md §11.7). */
const val NAME: String = "replays_every_form"

/**
 * The test case `replays_every_form` (docs/SCE_TEST_PROFILE.md §11.7) as the runner reads it:
 * its name, its configuration and the machines of its components, the module
 * parameters those components read, and the reader of the inputs a recorded trace of it
 * carries (docs/SCE_TTCN3_LANE.md §10.5).
 */
fun testCase(): com.sce.runtime.test.TestCase = com.sce.runtime.test.TestCase(
    name = NAME,
    configuration = com.sce.generated.forms_configuration.configuration(),
    machines = { componentType -> com.sce.generated.forms_configuration.machine(componentType) },
    moduleParameters = com.sce.generated.forms_configuration.moduleParameters(),
    inputs = { systemPort, typeName, value -> com.sce.generated.forms_configuration.input(systemPort, typeName, value) },
)
