// SCE-GENERATED — DO NOT EDIT
// source-hash: 011d804f887126b888ea6ba5b4213d56e0f5c8740555a154c83e9b8d0105c4b6
// SCE-MAP: serving_configuration.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-configuration")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.serving_configuration

/**
 * The test configuration `serving_configuration` (docs/SCE_TEST_PROFILE.md §11.6) as the
 * profile's runtime reads it: the MTC's component type and each PTC's, the ports and timers
 * each type declares, each `<sce:connect>` and each `<sce:map>`.
 */
fun configuration(): com.sce.runtime.test.Configuration = com.sce.runtime.test.Configuration(
    mtcType = "coordinator",
    ptcTypes = mapOf(
        "answerer" to "answerer",
    ),
    portsOfType = mapOf(
        "coordinator" to listOf("peer"),
        "answerer" to listOf("peer"),
    ),
    timersOfType = emptyMap(),
    connections = listOf(
        com.sce.runtime.test.Endpoint("mtc", "peer") to com.sce.runtime.test.Endpoint("answerer", "peer"),
    ),
    mappings = emptyList(),
)

/**
 * The machine of each component type of the configuration: the component the profile's
 * runtime drives, generated beside its test component's machine (docs/SCE_TEST_PROFILE.md
 * §12.3). Null for a type the configuration does not name, which the run ends with `error`
 * where it is created.
 */
fun machine(componentType: String): com.sce.runtime.test.ComponentMachine? = when (componentType) {
    "coordinator" -> com.sce.generated.coordinator.CoordinatorComponent()
    "answerer" -> com.sce.generated.answerer.AnswererComponent()
    else -> null
}

/**
 * The module parameters the components of the configuration read
 * (docs/SCE_TEST_PROFILE.md §11.5), each of which a suite that runs it gives a value.
 */
fun moduleParameters(): List<String> = emptyList()
