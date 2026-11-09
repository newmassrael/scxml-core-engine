// SCE-GENERATED — DO NOT EDIT
// source-hash: 16d76c0f9ed42e7e13706b7e291287d0861ab6ac1014d873b4e04ff755d59e01
// SCE-MAP: keeping_configuration.scxml:6 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-configuration")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.keeping_configuration

/**
 * The test configuration `keeping_configuration` (docs/SCE_TEST_PROFILE.md §11.6) as the
 * profile's runtime reads it: the MTC's component type and each PTC's, the ports and timers
 * each type declares, each `<sce:connect>` and each `<sce:map>`.
 */
fun configuration(): com.sce.runtime.test.Configuration = com.sce.runtime.test.Configuration(
    mtcType = "ptc_keeper",
    ptcTypes = mapOf(
        "idler" to "idler",
    ),
    portsOfType = mapOf(
        "idler" to listOf("peer"),
    ),
    timersOfType = mapOf(
        "ptc_keeper" to listOf("patience", "linger"),
    ),
    connections = emptyList(),
    mappings = listOf(
        com.sce.runtime.test.Endpoint("idler", "peer") to "quiet",
    ),
)

/**
 * The machine of each component type of the configuration: the component the profile's
 * runtime drives, generated beside its test component's machine (docs/SCE_TEST_PROFILE.md
 * §12.3). Null for a type the configuration does not name, which the run ends with `error`
 * where it is created.
 */
fun machine(componentType: String): com.sce.runtime.test.ComponentMachine? = when (componentType) {
    "ptc_keeper" -> com.sce.generated.ptc_keeper.PtcKeeperComponent()
    "idler" -> com.sce.generated.idler.IdlerComponent()
    else -> null
}

/**
 * The module parameters the components of the configuration read
 * (docs/SCE_TEST_PROFILE.md §11.5), each of which a suite that runs it gives a value.
 */
fun moduleParameters(): List<String> = emptyList()
