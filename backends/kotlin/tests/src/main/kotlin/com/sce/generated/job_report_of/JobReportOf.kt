// SCE-GENERATED — DO NOT EDIT
// source-hash: 4e65a893782852676bb9b2d81e2da91064cf31e9245b09ee7b72fd1a04b1ed23
// SCE-MAP: job_report_of.scxml:6 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: sce_runtime
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

/**
 * The name of the record [Value] is, as the types module of the suite's TTCN-3 export names it
 * (docs/SCE_TEST_PROFILE.md §9.5): the `type` of a message of it in the test log (§12.5).
 */
const val TYPE: String = "job_status"

/**
 * [value] in the canonical form of the test log (docs/SCE_TEST_PROFILE.md §12.5): each
 * integer as the text of its digits, a byte string in upper-case hex, an enum and a case
 * of a union by their declared names, an absent field omitted. A send of this template
 * carries it beside the value it sends (`com.sce.runtime.test.messageValue`).
 */
fun canonical(value: Value): Any =
    com.sce.runtime.test.Canonical.record("elapsed_ms" to com.sce.runtime.test.Canonical.integer(value.elapsed_ms), "note" to value.note?.let { present_0 -> com.sce.runtime.test.Canonical.text(present_0) }, "codes" to com.sce.runtime.test.Canonical.list(value.codes.map { item_0 -> com.sce.runtime.test.Canonical.integer(item_0) }), "tag" to value.tag?.let { present_0 -> com.sce.runtime.test.Canonical.integer(present_0) }, "retry" to value.retry?.let { present_0 -> com.sce.runtime.test.Canonical.integer(present_0) }, "count" to com.sce.runtime.test.Canonical.integer(value.count))
