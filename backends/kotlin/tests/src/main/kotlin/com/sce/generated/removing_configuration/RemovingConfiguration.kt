// SCE-GENERATED — DO NOT EDIT
// source-hash: 982a23a5ec15b8216f29e9e8e4ba6c1047dee0d66671c7835c2776a4de5a13d2
// SCE-MAP: removing_configuration.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-configuration")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.removing_configuration

/**
 * The test configuration `removing_configuration` (docs/SCE_TEST_PROFILE.md §11.6) as the
 * profile's runtime reads it: the MTC's component type and each PTC's, the ports and timers
 * each type declares, each `<sce:connect>` and each `<sce:map>`.
 */
fun configuration(): com.sce.runtime.test.Configuration = com.sce.runtime.test.Configuration(
    mtcType = "ptc_remover",
    ptcTypes = mapOf(
        "listener" to "request_listener",
    ),
    portsOfType = mapOf(
        "ptc_remover" to listOf("peer"),
        "request_listener" to listOf("peer"),
    ),
    timersOfType = mapOf(
        "ptc_remover" to listOf("patience"),
    ),
    connections = listOf(
        com.sce.runtime.test.Endpoint("mtc", "peer") to com.sce.runtime.test.Endpoint("listener", "peer"),
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
    "ptc_remover" -> com.sce.generated.ptc_remover.PtcRemoverComponent()
    "request_listener" -> com.sce.generated.request_listener.RequestListenerComponent()
    else -> null
}

/**
 * The module parameters the components of the configuration read
 * (docs/SCE_TEST_PROFILE.md §11.5), each of which a suite that runs it gives a value.
 */
fun moduleParameters(): List<String> = emptyList()

/**
 * The reader of the inputs of a recorded trace (docs/SCE_TTCN3_LANE.md §10.2,
 * `com.sce.runtime.test.InputReader`): the message an input of the record type [typeName] on the
 * system port [systemPort] carries, its value read from the canonical form.
 *
 * @throws com.sce.runtime.test.TraceRefusal naming why, where the configuration has no such system
 *   port, the `in` list of its type does not name the type, or the value does not belong to it.
 */
@Suppress("UNUSED_PARAMETER")
fun input(systemPort: String, typeName: String, value: Any?): com.sce.runtime.test.Message =
    throw com.sce.runtime.test.TraceRefusal.notAnInput(systemPort, typeName, listOf())
