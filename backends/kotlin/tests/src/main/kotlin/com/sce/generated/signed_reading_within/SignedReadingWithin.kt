// SCE-GENERATED — DO NOT EDIT
// source-hash: 982a23a5ec15b8216f29e9e8e4ba6c1047dee0d66671c7835c2776a4de5a13d2
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
