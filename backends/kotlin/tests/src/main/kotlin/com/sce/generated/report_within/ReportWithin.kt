// SCE-GENERATED — DO NOT EDIT
// source-hash: 38af95b1cf8d50c8e17ac734db3bf8439c5edbcad09e829327f83dbaea6e3644
// SCE-MAP: report_within.scxml:5 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.report_within

/** The type the template `report_within` matches: `record:JobStatus`, as the unit of its type declares it. */
typealias Value = com.sce.generated.job_status.JobStatusPayload

/**
 * Whether [value] matches the template `report_within` (ES 201 873-1 Annex B.1,
 * docs/SCE_TEST_PROFILE.md §6.1).
 *
 * @param param_limit the formal parameter `limit`, of type `uint32`.
 */
@Suppress("UNUSED_PARAMETER", "SENSELESS_COMPARISON", "USELESS_IS_CHECK")
fun matches(value: Value, param_limit: UInt): Boolean =
    value.elapsed_ms == 0u && value.note.let { present_0 -> present_0 != null && present_0 == "started on time" } && value.codes.size == 1 && value.codes[0] == 7u.toUByte() && value.tag == null && value.retry == null && com.sce.generated.elapsed_at_most.matches(value.count, param_limit)
