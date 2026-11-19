// SCE-GENERATED — DO NOT EDIT
// source-hash: c31794321c13b21ef2b620f7ccbeb9f40af783b7909594b3c0b15b0dd6bee19c
// source: signed_reading_within.scxml
// SCE-MAP: signed_reading_within.scxml:5 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.signed_reading_within

/** The type the template `signed_reading_within` matches: `record:SignedReading`, as the unit of its type declares it. */
typealias Value = com.sce.generated.signed_reading.SignedReadingPayload

/**
 * Whether [value] matches the template `signed_reading_within` (ES 201 873-1 Annex B.1,
 * docs/SCE_TEST_PROFILE.md §6.1).
 */
@Suppress("UNUSED_PARAMETER", "SENSELESS_COMPARISON", "USELESS_IS_CHECK")
fun matches(value: Value): Boolean =
    value.delta <= (-1).toByte() && value.level <= -1 && value.stamp <= -4294967296L && value.total >= 9223372036854775808uL

/**
 * The name of the record [Value] is, as the types module of the suite's TTCN-3 export names it
 * (docs/SCE_TEST_PROFILE.md §9.5): the `type` of a message of it in the test log (§12.5).
 */
const val TYPE: String = "signed_reading"

/**
 * [value] in the canonical form of the test log (docs/SCE_TEST_PROFILE.md §12.5): each
 * integer as the text of its digits, a byte string in upper-case hex, an enum and a case
 * of a union by their declared names, an absent field omitted. A send of this template
 * carries it beside the value it sends (`com.sce.runtime.test.messageValue`).
 */
fun canonical(value: Value): Any =
    com.sce.runtime.test.Canonical.record("delta" to com.sce.runtime.test.Canonical.integer(value.delta), "offset" to com.sce.runtime.test.Canonical.integer(value.offset), "level" to com.sce.runtime.test.Canonical.integer(value.level), "stamp" to com.sce.runtime.test.Canonical.integer(value.stamp), "total" to com.sce.runtime.test.Canonical.integer(value.total))
