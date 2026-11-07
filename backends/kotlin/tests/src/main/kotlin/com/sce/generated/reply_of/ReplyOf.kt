// SCE-GENERATED — DO NOT EDIT
// source-hash: 021fdf0d0c3ee151c664e2ecf6a7e5a797d68aaf3d399831e8dfd49b9e10bec9
// SCE-MAP: reply_of.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.reply_of

/** The type the template `reply_of` matches: `record:JobReply`, as the unit of its type declares it. */
typealias Value = com.sce.generated.job_reply.JobReplyPayload

/**
 * Whether [value] matches the template `reply_of` (ES 201 873-1 Annex B.1,
 * docs/SCE_TEST_PROFILE.md §6.1).
 *
 * @param param_job the formal parameter `job`, of type `uint32`.
 *
 * @param param_code the formal parameter `code`, of type `uint32`.
 */
@Suppress("UNUSED_PARAMETER", "SENSELESS_COMPARISON", "USELESS_IS_CHECK")
fun matches(value: Value, param_job: UInt, param_code: UInt): Boolean =
    value.job == param_job && value.code == param_code

/**
 * The value the specific template `reply_of` sends (docs/SCE_TEST_PROFILE.md
 * §6.2): every field it names at its value, every field it omits absent. It matches the
 * template by [matches], given the same parameters.
 */
@Suppress("UNUSED_PARAMETER")
fun value(param_job: UInt, param_code: UInt): Value =
    com.sce.generated.job_reply.JobReplyPayload(job = param_job, code = param_code)
