// SCE-GENERATED — DO NOT EDIT
// source-hash: 2b1a01faa0f39a78981b432fbb0fd087ee06ecdcae837825ff5c891fee416aff
// source: signed_configuration.scxml
// SCE-MAP: signed_configuration.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-configuration")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.signed_configuration

/**
 * The test configuration `signed_configuration` (docs/SCE_TEST_PROFILE.md §11.6) as the
 * profile's runtime reads it: the MTC's component type and each PTC's, the ports and timers
 * each type declares, each `<sce:connect>` and each `<sce:map>`.
 */
fun configuration(): com.sce.runtime.test.Configuration = com.sce.runtime.test.Configuration(
    mtcType = "signed_tester",
    ptcTypes = emptyMap(),
    portsOfType = mapOf(
        "signed_tester" to listOf("sut"),
    ),
    timersOfType = mapOf(
        "signed_tester" to listOf("deadline"),
    ),
    connections = emptyList(),
    mappings = listOf(
        com.sce.runtime.test.Endpoint("mtc", "sut") to "sut",
    ),
)

/**
 * The machine of each component type of the configuration: the component the profile's
 * runtime drives, generated beside its test component's machine (docs/SCE_TEST_PROFILE.md
 * §12.3). Null for a type the configuration does not name, which the run ends with `error`
 * where it is created.
 */
fun machine(componentType: String): com.sce.runtime.test.ComponentMachine? = when (componentType) {
    "signed_tester" -> com.sce.generated.signed_tester.SignedTesterComponent()
    else -> null
}

/**
 * The module parameters the components of the configuration read
 * (docs/SCE_TEST_PROFILE.md §11.5), each of which a suite that runs it gives a value.
 */
fun moduleParameters(): List<String> = emptyList()

/**
 * The record `signed_reading` read from the canonical form of the test log
 * (docs/SCE_TEST_PROFILE.md §12.5), as a recorded input trace holds it (docs/SCE_TTCN3_LANE.md
 * §10.3). The inverse of [canonical_signed_reading].
 *
 * @throws com.sce.runtime.test.TraceRefusal naming where its value does not belong to its type.
 */
fun read_signed_reading(value: Any?): com.sce.generated.signed_reading.SignedReadingPayload =
    com.sce.runtime.test.FromCanonical.record(value, listOf("change", "offset", "level", "stamp", "total"), "signed_reading").let { record_0 -> com.sce.generated.signed_reading.SignedReadingPayload(change = com.sce.runtime.test.FromCanonical.integer(com.sce.runtime.test.FromCanonical.field(record_0, "change", "signed_reading.change"), com.sce.runtime.test.FromCanonical.INT8, "signed_reading.change"), offset = com.sce.runtime.test.FromCanonical.integer(com.sce.runtime.test.FromCanonical.field(record_0, "offset", "signed_reading.offset"), com.sce.runtime.test.FromCanonical.INT16, "signed_reading.offset"), level = com.sce.runtime.test.FromCanonical.integer(com.sce.runtime.test.FromCanonical.field(record_0, "level", "signed_reading.level"), com.sce.runtime.test.FromCanonical.INT32, "signed_reading.level"), stamp = com.sce.runtime.test.FromCanonical.integer(com.sce.runtime.test.FromCanonical.field(record_0, "stamp", "signed_reading.stamp"), com.sce.runtime.test.FromCanonical.INT64, "signed_reading.stamp"), total = com.sce.runtime.test.FromCanonical.integer(com.sce.runtime.test.FromCanonical.field(record_0, "total", "signed_reading.total"), com.sce.runtime.test.FromCanonical.UINT64, "signed_reading.total")) }

/**
 * The record `signed_reading` in the canonical form of the test log
 * (docs/SCE_TEST_PROFILE.md §12.5), which a message of it a trace hands the run carries.
 */
fun canonical_signed_reading(value: com.sce.generated.signed_reading.SignedReadingPayload): Any =
    com.sce.runtime.test.Canonical.record("change" to com.sce.runtime.test.Canonical.integer(value.change), "offset" to com.sce.runtime.test.Canonical.integer(value.offset), "level" to com.sce.runtime.test.Canonical.integer(value.level), "stamp" to com.sce.runtime.test.Canonical.integer(value.stamp), "total" to com.sce.runtime.test.Canonical.integer(value.total))

/**
 * The reader of the inputs of a recorded trace (docs/SCE_TTCN3_LANE.md §10.2,
 * `com.sce.runtime.test.InputReader`): the message an input of the record type [typeName] on the
 * system port [systemPort] carries, its value read from the canonical form.
 *
 * @throws com.sce.runtime.test.TraceRefusal naming why, where the configuration has no such system
 *   port, the `in` list of its type does not name the type, or the value does not belong to it.
 */
fun input(systemPort: String, typeName: String, value: Any?): com.sce.runtime.test.Message = when (systemPort to typeName) {
    "sut" to "signed_reading" -> com.sce.runtime.test.Message.of(com.sce.generated.signed_reading.EVENT_NAME, read_signed_reading(value), "signed_reading", ::canonical_signed_reading)
    else -> throw com.sce.runtime.test.TraceRefusal.notAnInput(systemPort, typeName, listOf("sut"))
}
