// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Accepted Subset §2.12 and docs/adr/0055, step 10 — a host-run `<invoke>`
// whose `sce:result` record holds values that may be absent. Kotlin compile+run
// gate.
//
// The committed machine (com/sce/integration/statechart_host_invoker_optional/)
// is generated from
// sce-build/tests/fixtures/host_processor/statechart_host_invoker_optional.scxml
// (regen: scripts/regen_host_processor_kotlin.sh) and constructed with NO script
// engine: a `datamodel="sce-static"` machine holds its variables as fields.
//
// The host answers through the generated typed interface, so the record it hands
// in holds each field that may be absent as a nullable; the engine delivers the
// record as JSON, an absent value as `null` with its key written all the same;
// and the machine reads each field where a test proves it present. What these
// hold apart is the three states a field has — absent, present and zero or
// empty, and present — which a sentinel could not: `""` is a label, `false` is a
// flag that was answered, and `0L` is a day.

package com.sce.integration

import com.sce.integration.statechart_host_invoker_optional.StatechartHostInvokerOptionalEvent
import com.sce.integration.statechart_host_invoker_optional.StatechartHostInvokerOptionalLookupResult
import com.sce.integration.statechart_host_invoker_optional.StatechartHostInvokerOptionalStateMachine
import com.sce.integration.statechart_host_invoker_optional.StatechartHostInvokerOptionalXSceHostInvoker
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class AHostResultMayHoldAValueThatIsAbsentTest {

    // The type the fixture was compiled for; `scripts/regen_host_processor_kotlin.sh`
    // passes the same string to the invoker declaration.
    private val declaredType = "x-sce-host"

    /**
     * A host implementing the generated interface; its work outlives the call, so
     * each test completes the start it was handed.
     */
    private class LookupHost : StatechartHostInvokerOptionalXSceHostInvoker {
        val tokens = mutableListOf<Long>()

        override fun startLookup(token: Long): StatechartHostInvokerOptionalLookupResult? {
            tokens.add(token)
            return null
        }

        override fun cancelLookup(token: Long) {}
    }

    /**
     * What the machine published after a completion: `answered` counts the
     * completions it read, and the rest is what each field of the record held.
     */
    private data class Answer(
        val answered: Int = 0,
        val rowsSeen: Int = 0,
        val totalSeen: ULong = 0uL,
        val atPresent: Boolean = false,
        val atValue: Long = 0L,
        val labelPresent: Boolean = false,
        val labelValue: String = "",
        val hitKnown: Boolean = false,
        val hitTrue: Boolean = false,
    )

    private fun read(sm: StatechartHostInvokerOptionalStateMachine) = Answer(
        answered = sm.answered.toInt(),
        rowsSeen = sm.rowsSeen.toInt(),
        totalSeen = sm.totalSeen,
        atPresent = sm.atPresent,
        atValue = sm.atValue,
        labelPresent = sm.labelPresent,
        labelValue = sm.labelValue,
        hitKnown = sm.hitKnown,
        hitTrue = sm.hitTrue,
    )

    /** A machine standing at `asking`, with the token the host was started with. */
    private fun <T> asking(body: (StatechartHostInvokerOptionalStateMachine, Long) -> T): T {
        val host = LookupHost()
        // No engine argument: the document is `sce-static`, so codegen emits a
        // machine with no script engine at all.
        val sm = StatechartHostInvokerOptionalStateMachine()
        try {
            sm.registerXSceHostInvoker(host)
            sm.initialize()
            sm.send(StatechartHostInvokerOptionalEvent.Ask)
            sm.tick()
            assertEquals(1, host.tokens.size, "the lookup was started once")
            return body(sm, host.tokens[0])
        } finally {
            sm.cleanup()
        }
    }

    /** Ask, complete the lookup with [result] through the typed interface, and settle the machine. */
    private fun complete(result: StatechartHostInvokerOptionalLookupResult): Answer =
        asking { sm, token ->
            assertTrue(sm.completeLookup(token, result), "a running invocation's typed completion was refused")
            sm.tick()
            read(sm)
        }

    /** Every field that may be absent is: the machine reads three absent values and the count every answer carries. */
    @Test
    fun anAbsentFieldIsReadAsAbsent() {
        val got = complete(StatechartHostInvokerOptionalLookupResult(rows = 3u, total = 30uL, at = null, label = null, hit = null))
        assertEquals(Answer(answered = 1, rowsSeen = 3, totalSeen = 30uL), got, "an all-absent answer")
    }

    /** Every field is present: the machine reads each value. */
    @Test
    fun aPresentFieldIsReadAsItsValue() {
        val got = complete(StatechartHostInvokerOptionalLookupResult(rows = 4u, total = 40uL, at = 41L, label = "ab", hit = true))
        assertEquals(
            Answer(
                answered = 1,
                rowsSeen = 4,
                totalSeen = 40uL,
                atPresent = true,
                atValue = 41L,
                labelPresent = true,
                labelValue = "ab",
                hitKnown = true,
                hitTrue = true,
            ),
            got,
            "an all-present answer",
        )
    }

    /** A zero, an empty text and a false flag are values, not absences: each is present, and is read as itself. */
    @Test
    fun aZeroAnEmptyTextAndAFalseFlagAreValuesAndNotAbsences() {
        val got = complete(StatechartHostInvokerOptionalLookupResult(rows = 1u, total = 0uL, at = 0L, label = "", hit = false))
        assertEquals(
            Answer(answered = 1, rowsSeen = 1, atPresent = true, labelPresent = true, hitKnown = true),
            got,
            "zero, empty and false are present",
        )
    }

    /** Each field is independent: a day with no label and a flag. */
    @Test
    fun eachFieldIsAbsentOrPresentOnItsOwn() {
        val got = complete(StatechartHostInvokerOptionalLookupResult(rows = 2u, total = 20uL, at = 7L, label = null, hit = true))
        assertEquals(
            Answer(
                answered = 1,
                rowsSeen = 2,
                totalSeen = 20uL,
                atPresent = true,
                atValue = 7L,
                hitKnown = true,
                hitTrue = true,
            ),
            got,
            "a mixed answer",
        )
    }

    /**
     * The record's JSON writes the key of every field, `null` where the value is
     * absent, in the order the schema declares them. The day is absent in both, so
     * that what is held is the key, the `null` and the other values' own spelling;
     * how a present 64-bit integer is written is held by the case after it.
     */
    @Test
    fun theRecordIsWrittenWithNullForAnAbsentValue() {
        val absent = StatechartHostInvokerOptionalLookupResult(rows = 3u, total = 5uL, at = null, label = null, hit = null)
        assertEquals("""{"rows":3,"total":"5","at":null,"label":null,"hit":null}""", absent.wire())
        val mixed = StatechartHostInvokerOptionalLookupResult(rows = 3u, total = 5uL, at = null, label = "ab", hit = false)
        assertEquals("""{"rows":3,"total":"5","at":null,"label":"ab","hit":false}""", mixed.wire())
    }

    /**
     * A 64-bit integer, signed or not, is written as the text of its digits, a
     * JSON string, and a smaller integer stays a bare number: the day and the total
     * here are past 2^53 - 1, where a bare number would lose its low digits.
     */
    @Test
    fun aSixtyFourBitIntegerIsWrittenAsTheTextOfItsDigits() {
        val record = StatechartHostInvokerOptionalLookupResult(
            rows = 1u,
            total = 18446744073709551615uL,
            at = 9007199254740993L,
            label = null,
            hit = null,
        )
        assertEquals(
            """{"rows":1,"total":"18446744073709551615","at":"9007199254740993","label":null,"hit":null}""",
            record.wire(),
        )
    }

    /** A day and a total past 2^53 - 1 cross the typed interface unchanged: 2^53 + 1 and the largest unsigned. */
    @Test
    fun aSixtyFourBitIntegerPastTheExactDoubleRangeIsPublishedExactly() {
        val got = complete(
            StatechartHostInvokerOptionalLookupResult(
                rows = 1u,
                total = 18446744073709551615uL,
                at = 9007199254740993L,
                label = null,
                hit = null,
            ),
        )
        assertEquals(
            Answer(
                answered = 1,
                rowsSeen = 1,
                totalSeen = 18446744073709551615uL,
                atPresent = true,
                atValue = 9007199254740993L,
            ),
            got,
            "a day and a total past 2^53 - 1",
        )
    }

    /** The extremes of a signed 64-bit day, each its own completion, are published exactly. */
    @Test
    fun theSignedSixtyFourBitExtremesArePublishedExactly() {
        for (day in listOf(Long.MIN_VALUE, Long.MAX_VALUE)) {
            val got = complete(
                StatechartHostInvokerOptionalLookupResult(rows = 1u, total = 0uL, at = day, label = null, hit = null),
            )
            assertEquals(
                Answer(answered = 1, rowsSeen = 1, atPresent = true, atValue = day),
                got,
                "the day $day",
            )
        }
    }

    /**
     * A key left out is not an absent value: every field of the first revision of a
     * schema is carried by every delivery, and `null` is the one spelling of absence,
     * so a completion that lacks `at` is refused as any typed payload the data does
     * not fit is, and the machine reads no answer.
     */
    @Test
    fun aCompletionThatLeavesAKeyOutIsRefused() {
        asking { sm, token ->
            assertTrue(
                sm.completeHostInvoke(declaredType, "lookup", token, """{"rows":1,"total":"2","label":null,"hit":null}"""),
                "a running invocation's completion was refused",
            )
            sm.tick()
            assertEquals(0, read(sm).answered, "a record without `at` was read as an answer")
        }
    }
}
