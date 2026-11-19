// SCE-GENERATED — DO NOT EDIT
// source-hash: 945f3336b5577424df7d0d55b748b92fb98290481f319eb98f4c161a0343addf
// source: request_of.scxml
// SCE-MAP: request_of.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.request_of

/** The type the template `request_of` matches: `record:JobRequest`, as the unit of its type declares it. */
typealias Value = com.sce.generated.job_request.JobRequestPayload

/**
 * Whether [value] matches the template `request_of` (ES 201 873-1 Annex B.1,
 * docs/SCE_TEST_PROFILE.md §6.1).
 *
 * @param param_wanted the formal parameter `wanted`, of type `uint32`.
 */
@Suppress("UNUSED_PARAMETER", "SENSELESS_COMPARISON", "USELESS_IS_CHECK")
fun matches(value: Value, param_wanted: UInt): Boolean =
    value.job == param_wanted

/**
 * The value the specific template `request_of` sends (docs/SCE_TEST_PROFILE.md
 * §6.2): every field it names at its value, every field it omits absent. It matches the
 * template by [matches], given the same parameters.
 */
@Suppress("UNUSED_PARAMETER")
fun value(param_wanted: UInt): Value =
    com.sce.generated.job_request.JobRequestPayload(job = param_wanted)

/**
 * The name of the record [Value] is, as the types module of the suite's TTCN-3 export names it
 * (docs/SCE_TEST_PROFILE.md §9.5): the `type` of a message of it in the test log (§12.5).
 */
const val TYPE: String = "job_request"

/**
 * [value] in the canonical form of the test log (docs/SCE_TEST_PROFILE.md §12.5): each
 * integer as the text of its digits, a byte string in upper-case hex, an enum and a case
 * of a union by their declared names, an absent field omitted. A send of this template
 * carries it beside the value it sends (`com.sce.runtime.test.messageValue`).
 */
fun canonical(value: Value): Any =
    com.sce.runtime.test.Canonical.record("job" to com.sce.runtime.test.Canonical.integer(value.job))
