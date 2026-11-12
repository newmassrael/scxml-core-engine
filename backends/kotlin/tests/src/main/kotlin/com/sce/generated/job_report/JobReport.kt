// SCE-GENERATED — DO NOT EDIT
// source-hash: 4e65a893782852676bb9b2d81e2da91064cf31e9245b09ee7b72fd1a04b1ed23
// SCE-MAP: job_report.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.job_report

/** The type the template `job_report` matches: `record:JobStatus`, as the unit of its type declares it. */
typealias Value = com.sce.generated.job_status.JobStatusPayload

/**
 * Whether [value] matches the template `job_report` (ES 201 873-1 Annex B.1,
 * docs/SCE_TEST_PROFILE.md §6.1).
 */
@Suppress("UNUSED_PARAMETER", "SENSELESS_COMPARISON", "USELESS_IS_CHECK")
fun matches(value: Value): Boolean =
    value.elapsed_ms == 0u && value.note.let { present_0 -> present_0 != null && present_0 == "started on time" } && value.codes.size == 1 && value.codes[0] == 7u.toUByte() && value.tag == null && value.retry == null && value.count == 1u

/**
 * The value the specific template `job_report` sends (docs/SCE_TEST_PROFILE.md
 * §6.2): every field it names at its value, every field it omits absent. It matches the
 * template by [matches], given the same parameters.
 */
@Suppress("UNUSED_PARAMETER")
fun value(): Value =
    com.sce.generated.job_status.JobStatusPayload(elapsed_ms = 0u, note = "started on time", codes = listOf(7u.toUByte()), tag = null, retry = null, count = 1u)

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
