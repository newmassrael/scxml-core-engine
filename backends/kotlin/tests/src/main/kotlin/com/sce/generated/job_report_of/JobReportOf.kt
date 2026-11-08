// SCE-GENERATED — DO NOT EDIT
// source-hash: f37425ae3865d6dd180df44e52fd892d2910b6883720df85bc9f25b65e2d89d3
// SCE-MAP: job_report_of.scxml:6 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.job_report_of

/** The type the template `job_report_of` matches: `record:JobStatus`, as the unit of its type declares it. */
typealias Value = com.sce.generated.job_status.JobStatusPayload

/**
 * Whether [value] matches the template `job_report_of` (ES 201 873-1 Annex B.1,
 * docs/SCE_TEST_PROFILE.md §6.1).
 *
 * @param param_elapsed the formal parameter `elapsed`, of type `uint32`.
 *
 * @param param_note the formal parameter `note`, of type `string[32]`.
 *
 * @param param_code the formal parameter `code`, of type `uint8`.
 */
@Suppress("UNUSED_PARAMETER", "SENSELESS_COMPARISON", "USELESS_IS_CHECK")
fun matches(value: Value, param_elapsed: UInt, param_note: String, param_code: UByte): Boolean =
    value.elapsed_ms == param_elapsed && value.note.let { present_0 -> present_0 != null && present_0 == param_note } && value.codes.size == 2 && value.codes[0] == param_code && value.codes[1] == 255u.toUByte() && value.tag == null && value.retry.let { present_0 -> present_0 != null && present_0 == 2u.toUByte() } && value.count == 4294967295u

/**
 * The value the specific template `job_report_of` sends (docs/SCE_TEST_PROFILE.md
 * §6.2): every field it names at its value, every field it omits absent. It matches the
 * template by [matches], given the same parameters.
 */
@Suppress("UNUSED_PARAMETER")
fun value(param_elapsed: UInt, param_note: String, param_code: UByte): Value =
    com.sce.generated.job_status.JobStatusPayload(elapsed_ms = param_elapsed, note = param_note, codes = listOf(param_code, 255u.toUByte()), tag = null, retry = 2u.toUByte(), count = 4294967295u)
