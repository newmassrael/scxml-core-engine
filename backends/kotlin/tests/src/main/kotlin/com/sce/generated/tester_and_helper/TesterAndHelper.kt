// SCE-GENERATED — DO NOT EDIT
// source-hash: 6fffef0974273f6b0d81b98b338020882f5206e2b18811e18fbb23b721e7585c
// source: tester_and_helper.scxml
// SCE-MAP: tester_and_helper.scxml:5 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-configuration")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.tester_and_helper

/**
 * The test configuration `tester_and_helper` (docs/SCE_TEST_PROFILE.md §11.6) as the
 * profile's runtime reads it: the MTC's component type and each PTC's, the ports and timers
 * each type declares, each `<sce:connect>` and each `<sce:map>`.
 */
fun configuration(): com.sce.runtime.test.Configuration = com.sce.runtime.test.Configuration(
    mtcType = "job_tester",
    ptcTypes = mapOf(
        "helper" to "helper",
    ),
    portsOfType = mapOf(
        "job_tester" to listOf("sut", "ctl"),
        "helper" to listOf("ctl"),
    ),
    timersOfType = mapOf(
        "job_tester" to listOf("deadline", "settle"),
    ),
    connections = listOf(
        com.sce.runtime.test.Endpoint("mtc", "ctl") to com.sce.runtime.test.Endpoint("helper", "ctl"),
    ),
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
    "job_tester" -> com.sce.generated.job_tester.JobTesterComponent()
    "helper" -> com.sce.generated.helper.HelperComponent()
    else -> null
}

/**
 * The module parameters the components of the configuration read
 * (docs/SCE_TEST_PROFILE.md §11.5), each of which a suite that runs it gives a value.
 */
fun moduleParameters(): List<String> = listOf("elapsed_limit")

/**
 * The record `job_status` read from the canonical form of the test log
 * (docs/SCE_TEST_PROFILE.md §12.5), as a recorded input trace holds it (docs/SCE_TTCN3_LANE.md
 * §10.3). The inverse of [canonical_job_status].
 *
 * @throws com.sce.runtime.test.TraceRefusal naming where its value does not belong to its type.
 */
fun read_job_status(value: Any?): com.sce.generated.job_status.JobStatusPayload =
    com.sce.runtime.test.FromCanonical.record(value, listOf("elapsed_ms", "note", "codes", "tag", "retry", "count"), "job_status").let { record_0 -> com.sce.generated.job_status.JobStatusPayload(elapsed_ms = com.sce.runtime.test.FromCanonical.integer(com.sce.runtime.test.FromCanonical.field(record_0, "elapsed_ms", "job_status.elapsed_ms"), com.sce.runtime.test.FromCanonical.UINT32, "job_status.elapsed_ms"), note = com.sce.runtime.test.FromCanonical.optionalField(record_0, "note") { present_0 -> com.sce.runtime.test.FromCanonical.text(present_0, 32, "job_status.note") }, codes = com.sce.runtime.test.FromCanonical.elements(com.sce.runtime.test.FromCanonical.field(record_0, "codes", "job_status.codes"), 0, 4, "job_status.codes").map { item_0 -> com.sce.runtime.test.FromCanonical.integer(item_0, com.sce.runtime.test.FromCanonical.UINT8, "job_status.codes[]") }, tag = com.sce.runtime.test.FromCanonical.optionalField(record_0, "tag") { present_0 -> com.sce.runtime.test.FromCanonical.integer(present_0, com.sce.runtime.test.FromCanonical.UINT8, "job_status.tag") }, retry = com.sce.runtime.test.FromCanonical.optionalField(record_0, "retry") { present_0 -> com.sce.runtime.test.FromCanonical.integer(present_0, com.sce.runtime.test.FromCanonical.UINT8, "job_status.retry") }, count = com.sce.runtime.test.FromCanonical.integer(com.sce.runtime.test.FromCanonical.field(record_0, "count", "job_status.count"), com.sce.runtime.test.FromCanonical.UINT32, "job_status.count")) }

/**
 * The record `job_status` in the canonical form of the test log
 * (docs/SCE_TEST_PROFILE.md §12.5), which a message of it a trace hands the run carries.
 */
fun canonical_job_status(value: com.sce.generated.job_status.JobStatusPayload): Any =
    com.sce.runtime.test.Canonical.record("elapsed_ms" to com.sce.runtime.test.Canonical.integer(value.elapsed_ms), "note" to value.note?.let { present_0 -> com.sce.runtime.test.Canonical.text(present_0) }, "codes" to com.sce.runtime.test.Canonical.list(value.codes.map { item_0 -> com.sce.runtime.test.Canonical.integer(item_0) }), "tag" to value.tag?.let { present_0 -> com.sce.runtime.test.Canonical.integer(present_0) }, "retry" to value.retry?.let { present_0 -> com.sce.runtime.test.Canonical.integer(present_0) }, "count" to com.sce.runtime.test.Canonical.integer(value.count))

/**
 * The reader of the inputs of a recorded trace (docs/SCE_TTCN3_LANE.md §10.2,
 * `com.sce.runtime.test.InputReader`): the message an input of the record type [typeName] on the
 * system port [systemPort] carries, its value read from the canonical form.
 *
 * @throws com.sce.runtime.test.TraceRefusal naming why, where the configuration has no such system
 *   port, the `in` list of its type does not name the type, or the value does not belong to it.
 */
fun input(systemPort: String, typeName: String, value: Any?): com.sce.runtime.test.Message = when (systemPort to typeName) {
    "sut" to "job_status" -> com.sce.runtime.test.Message.of(com.sce.generated.job_status.EVENT_NAME, read_job_status(value), "job_status", ::canonical_job_status)
    else -> throw com.sce.runtime.test.TraceRefusal.notAnInput(systemPort, typeName, listOf("sut"))
}
