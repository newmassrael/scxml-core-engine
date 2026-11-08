// SCE-GENERATED — DO NOT EDIT
// source-hash: 69b0ebe2b258c0c641661c1efeb701cca9163d999b33bc95a7ea77aa603a45be
// SCE-MAP: job_report.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: none
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
