// SCE-GENERATED — DO NOT EDIT
// source-hash: 16d76c0f9ed42e7e13706b7e291287d0861ab6ac1014d873b4e04ff755d59e01
// SCE-MAP: reply_of.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.reply_of

/** The type the template `reply_of` matches: `record:JobReply`, as the unit of its type declares it. */
typealias Value = com.sce.generated.job_reply.JobReplyPayload

/**
 * Whether [value] matches the template `reply_of` (ES 201 873-1 Annex B.1,
 * docs/SCE_TEST_PROFILE.md §6.1).
 *
 * @param param_answers the formal parameter `answers`, of type `uint32`.
 *
 * @param param_code the formal parameter `code`, of type `uint32`.
 */
@Suppress("UNUSED_PARAMETER", "SENSELESS_COMPARISON", "USELESS_IS_CHECK")
fun matches(value: Value, param_answers: UInt, param_code: UInt): Boolean =
    value.job == param_answers && value.code == param_code

/**
 * The value the specific template `reply_of` sends (docs/SCE_TEST_PROFILE.md
 * §6.2): every field it names at its value, every field it omits absent. It matches the
 * template by [matches], given the same parameters.
 */
@Suppress("UNUSED_PARAMETER")
fun value(param_answers: UInt, param_code: UInt): Value =
    com.sce.generated.job_reply.JobReplyPayload(job = param_answers, code = param_code)

/**
 * The name of the record [Value] is, as the types module of the suite's TTCN-3 export names it
 * (docs/SCE_TEST_PROFILE.md §9.5): the `type` of a message of it in the test log (§12.5).
 */
const val TYPE: String = "job_reply"

/**
 * [value] in the canonical form of the test log (docs/SCE_TEST_PROFILE.md §12.5): each
 * integer as the text of its digits, a byte string in upper-case hex, an enum and a case
 * of a union by their declared names, an absent field omitted. A send of this template
 * carries it beside the value it sends (`com.sce.runtime.test.messageValue`).
 */
fun canonical(value: Value): Any =
    com.sce.runtime.test.Canonical.record("job" to com.sce.runtime.test.Canonical.integer(value.job), "code" to com.sce.runtime.test.Canonical.integer(value.code))
