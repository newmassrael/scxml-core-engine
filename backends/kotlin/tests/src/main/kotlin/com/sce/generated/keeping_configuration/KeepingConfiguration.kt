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

/**
 * The record `job_reply` read from the canonical form of the test log
 * (docs/SCE_TEST_PROFILE.md §12.5), as a recorded input trace holds it (docs/SCE_TTCN3_LANE.md
 * §10.3). The inverse of [canonical_job_reply].
 *
 * @throws com.sce.runtime.test.TraceRefusal naming where its value does not belong to its type.
 */
fun read_job_reply(value: Any?): com.sce.generated.job_reply.JobReplyPayload =
    com.sce.runtime.test.FromCanonical.record(value, listOf("job", "code"), "job_reply").let { record_0 -> com.sce.generated.job_reply.JobReplyPayload(job = com.sce.runtime.test.FromCanonical.integer(com.sce.runtime.test.FromCanonical.field(record_0, "job", "job_reply.job"), com.sce.runtime.test.FromCanonical.UINT32, "job_reply.job"), code = com.sce.runtime.test.FromCanonical.integer(com.sce.runtime.test.FromCanonical.field(record_0, "code", "job_reply.code"), com.sce.runtime.test.FromCanonical.UINT32, "job_reply.code")) }

/**
 * The record `job_reply` in the canonical form of the test log
 * (docs/SCE_TEST_PROFILE.md §12.5), which a message of it a trace hands the run carries.
 */
fun canonical_job_reply(value: com.sce.generated.job_reply.JobReplyPayload): Any =
    com.sce.runtime.test.Canonical.record("job" to com.sce.runtime.test.Canonical.integer(value.job), "code" to com.sce.runtime.test.Canonical.integer(value.code))

/**
 * The reader of the inputs of a recorded trace (docs/SCE_TTCN3_LANE.md §10.2,
 * `com.sce.runtime.test.InputReader`): the message an input of the record type [typeName] on the
 * system port [systemPort] carries, its value read from the canonical form.
 *
 * @throws com.sce.runtime.test.TraceRefusal naming why, where the configuration has no such system
 *   port, the `in` list of its type does not name the type, or the value does not belong to it.
 */
fun input(systemPort: String, typeName: String, value: Any?): com.sce.runtime.test.Message = when (systemPort to typeName) {
    "quiet" to "job_reply" -> com.sce.runtime.test.Message.of(com.sce.generated.job_reply.EVENT_NAME, read_job_reply(value), "job_reply", ::canonical_job_reply)
    else -> throw com.sce.runtime.test.TraceRefusal.notAnInput(systemPort, typeName, listOf("quiet"))
}
