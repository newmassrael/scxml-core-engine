// SCE-GENERATED — DO NOT EDIT
// source-hash: d9a20d3d040e3f3b82b50c5dfb616ae2679f31af5d7a387c02d5d27e7c9ed9c1
// SCE-MAP: every_matcher.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.every_matcher

/** The type the template `every_matcher` matches: `record:JobStatus`, as the unit of its type declares it. */
typealias Value = com.sce.generated.job_status.JobStatusPayload

/**
 * Whether [value] matches the template `every_matcher` (ES 201 873-1 Annex B.1,
 * docs/SCE_TEST_PROFILE.md §6.1).
 *
 * @param param_limit the formal parameter `limit`, of type `uint32`.
 */
@Suppress("UNUSED_PARAMETER", "SENSELESS_COMPARISON", "USELESS_IS_CHECK")
fun matches(value: Value, param_limit: UInt): Boolean =
    (value.elapsed_ms == 1u || (value.elapsed_ms >= 10u && value.elapsed_ms <= param_limit)) && (value.note == null || (value.note != null && value.note.let { present_0 -> present_0 == null || com.sce.runtime.test.textLength(present_0) <= 8 })) && value.codes.size == 3 && value.codes[0] == 3u.toUByte() && value.codes[2] != 0u.toUByte() && value.codes.size >= 1 && value.codes.size <= 4 && value.retry == null && com.sce.generated.elapsed_at_most.matches(value.count, param_limit)
