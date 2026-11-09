// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The Kotlin units of the test profile's templates (docs/SCE_TEST_PROFILE.md §6,
// §11.4, §12.6 slice 6), matched against concrete values: each mechanism of
// ES 201 873-1 Annex B.1 the profile holds, true and false, the value of each
// specific template matching its own template, and a union matched by the class
// of its case (docs/adr/0092). The Kotlin half of the Rust suite's
// `a_test_template_matches_its_values.rs`, which holds the same templates to the
// same values. The committed units under `src/main/kotlin/com/sce/generated/` are
// generated from the fixtures under `sce-build/tests/fixtures/test_profile/` and
// `sce-build/tests/fixtures/union/` (regen: `scripts/regen_static_datamodel_kotlin.sh`).

package com.sce.integration

import com.sce.generated.card_payment.CardPaymentPayload
import com.sce.generated.job_status.JobStatusPayload
import com.sce.generated.order_customer.OrderCustomerPayload
import com.sce.generated.order_line.OrderLinePayload
import com.sce.generated.order_payment.OrderPayment
import com.sce.generated.order_placed.OrderPlacedPayload
import com.sce.generated.paint_color.PaintColor
import com.sce.generated.shape_radius_minimal.ShapeRadiusMinimalPayload
import com.sce.generated.shape_rect_minimal.ShapeRectMinimalPayload
import com.sce.generated.union_shape_minimal.UnionShapeMinimal
import com.sce.generated.voucher_payment.VoucherPaymentPayload
import com.sce.runtime.Json
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue
import com.sce.generated.card_within.matches as cardWithin
import com.sce.generated.elapsed_at_most.matches as elapsedAtMost
import com.sce.generated.every_matcher.matches as everyMatcher
import com.sce.generated.job_report.matches as jobReport
import com.sce.generated.job_report.value as jobReportValue
import com.sce.generated.job_report_of.matches as jobReportOf
import com.sce.generated.job_report_of.value as jobReportOfValue
import com.sce.generated.order_line_bounded.matches as orderLineBounded
import com.sce.generated.order_line_exact.matches as orderLineExact
import com.sce.generated.order_line_exact.value as orderLineExactValue
import com.sce.generated.order_placed_match.TYPE as ORDER_PLACED_TYPE
import com.sce.generated.order_placed_match.canonical as orderPlacedCanonical
import com.sce.generated.order_placed_match.matches as orderPlacedMatch
import com.sce.generated.report_within.matches as reportWithin
import com.sce.generated.shape_is_circle.matches as shapeIsCircle
import com.sce.generated.voucher_is.matches as voucherIs
import com.sce.generated.voucher_is.value as voucherIsValue

class ATestTemplateMatchesItsValuesTest {

    private companion object {
        /** U+00E9, one character of two UTF-8 bytes. */
        const val E_ACUTE = "é"

        /** The limit every_matcher's range and reference are given. */
        const val LIMIT: UInt = 100u

        /** The quantity limit order_placed_match is given. */
        val QUANTITY_LIMIT: UShort = 5u
    }

    /** [items] as the codes of a job status, each a `uint8`. */
    private fun codes(vararg items: Int): List<UByte> = items.map { it.toUByte() }

    /**
     * A job status every_matcher matches with the limit 100: each field at a value its
     * matcher holds, each optional field absent.
     */
    private fun matching() =
        JobStatusPayload(
            elapsed_ms = 1u,
            note = null,
            codes = codes(3, 9, 5),
            tag = null,
            retry = null,
            count = 100u,
        )

    @Test
    fun aStatusEachOfWhoseFieldsItsMatcherHoldsMatches() {
        assertTrue(everyMatcher(matching(), LIMIT))
    }

    /** value-list (B.1.2.1) of a specific value and a range (B.1.2.5), both bounds inclusive, the upper one a formal parameter. */
    @Test
    fun aValueListMatchesAnyOfItsItemsAndARangeHoldsItsBounds() {
        for (elapsed in listOf(1u, 10u, 50u, LIMIT)) {
            assertTrue(everyMatcher(matching().copy(elapsed_ms = elapsed), LIMIT), "$elapsed")
        }
        for (elapsed in listOf(0u, 2u, 9u, LIMIT + 1u)) {
            assertFalse(everyMatcher(matching().copy(elapsed_ms = elapsed), LIMIT), "$elapsed")
        }
    }

    /**
     * any-value with ifpresent (B.1.4.2) and a length restriction (B.1.4.1) on an optional
     * text: absent, or present with at most eight characters.
     */
    @Test
    fun ifpresentTakesAnAbsentFieldAndALengthCountsCharacters() {
        val withNote = { note: String? -> matching().copy(note = note) }
        assertTrue(everyMatcher(withNote(null), LIMIT))
        assertTrue(everyMatcher(withNote("abcdefgh"), LIMIT))
        assertFalse(everyMatcher(withNote("abcdefghi"), LIMIT))
        // Eight characters of two bytes each: sixteen bytes, eight characters.
        assertTrue(everyMatcher(withNote(E_ACUTE.repeat(8)), LIMIT))
        assertFalse(everyMatcher(withNote(E_ACUTE.repeat(9)), LIMIT))
    }

    /**
     * list-match (B.1.1): exactly three elements, the first 3, the second any value
     * (B.1.3.1), the third not 0 (complement, B.1.2.2).
     */
    @Test
    fun aListMatchHoldsItsCountAndEachElementInOrder() {
        for (items in listOf(codes(3, 9, 5), codes(3, 0, 1))) {
            assertTrue(everyMatcher(matching().copy(codes = items), LIMIT), "$items")
        }
        val refused = listOf(codes(3, 9, 0), codes(4, 9, 5), codes(3, 9), codes(3, 9, 5, 5), codes())
        for (items in refused) {
            assertFalse(everyMatcher(matching().copy(codes = items), LIMIT), "$items")
        }
    }

    /** any-value-or-none (B.1.2.4) and omit (B.1.2.8). */
    @Test
    fun anyValueOrNoneTakesBothAndOmitOnlyAnAbsentField() {
        assertTrue(everyMatcher(matching().copy(tag = 7.toUByte()), LIMIT))
        assertTrue(everyMatcher(matching(), LIMIT))
        assertFalse(everyMatcher(matching().copy(retry = 1.toUByte()), LIMIT))
    }

    /** template-ref (-1 §15.1): the referenced template's match, its formal parameter bound to this template's own. */
    @Test
    fun aReferenceMatchesByItsTemplateWithTheArgumentItBinds() {
        val status = matching().copy(count = 150u)
        assertFalse(everyMatcher(status, LIMIT))
        assertTrue(everyMatcher(status, 150u))
        assertTrue(elapsedAtMost(150u, 150u))
        assertFalse(elapsedAtMost(151u, 150u))
    }

    /** A specific template's value (§6.2) matches the template, and a value that differs in one field does not. */
    @Test
    fun aSpecificTemplatesValueMatchesItsTemplate() {
        val report = jobReportValue()
        assertEquals(0u, report.elapsed_ms)
        assertEquals("started on time", report.note)
        assertEquals(codes(7), report.codes)
        assertNull(report.tag)
        assertNull(report.retry)
        assertEquals(1u, report.count)
        assertTrue(jobReport(report))

        assertFalse(jobReport(report.copy(note = null)))
        assertFalse(jobReport(report.copy(retry = 0.toUByte())))
        assertFalse(jobReport(report.copy(count = 2u)))
    }

    /** A specific template whose values are partly its parameters. */
    @Test
    fun aSpecificTemplatesValueHoldsTheParametersItIsGiven() {
        val report = jobReportOfValue(5u, "late", 9u)
        assertEquals(5u, report.elapsed_ms)
        assertEquals("late", report.note)
        assertEquals(codes(9, 255), report.codes)
        assertNull(report.tag)
        assertEquals(2u.toUByte(), report.retry)
        assertEquals(4_294_967_295u, report.count)
        assertTrue(jobReportOf(report, 5u, "late", 9u))
        assertFalse(jobReportOf(report, 6u, "late", 9u))
        assertFalse(jobReportOf(report, 5u, "early", 9u))
        assertFalse(jobReportOf(report, 5u, "late", 8u))
    }

    /** An enum literal, and a reference to a specific template, which builds the field with that template's value. */
    @Test
    fun anEnumLiteralAndAReferenceToASpecificTemplateBuildAndMatch() {
        val line = orderLineExactValue()
        assertEquals("A-1", line.sku)
        assertEquals(2u.toUShort(), line.quantity)
        assertEquals(PaintColor.RED, line.color)
        assertTrue(orderLineExact(line))

        assertFalse(orderLineExact(line.copy(color = PaintColor.BLUE)))
        assertFalse(orderLineExact(line.copy(quantity = 3u)))
    }

    /** A reference whose template matches a range up to the argument it binds. */
    @Test
    fun aReferenceToARangeHoldsTheBoundItsArgumentGives() {
        val line = OrderLinePayload(sku = "A-1", quantity = 5u, color = PaintColor.BLUE)
        assertTrue(orderLineBounded(line, 5u))
        assertFalse(orderLineBounded(line, 4u))
        assertFalse(orderLineBounded(line.copy(color = PaintColor.GREEN), 5u))
    }

    /** A template that is specific in itself and references one that is not matches by the referenced template's range. */
    @Test
    fun aTemplateMatchesThroughAReferenceToOneThatIsNotSpecific() {
        val status = jobReportValue().copy(count = 60u)
        assertTrue(reportWithin(status, 60u))
        assertFalse(reportWithin(status, 59u))
    }

    /**
     * A length restriction on a text counts characters, so a text whose bytes pass its
     * upper bound matches while its characters do not; a range with a literal lower bound.
     */
    @Test
    fun aLengthOnATextCountsAMultiByteCharacterAsOne() {
        val card = { last4: String, amount: UInt -> CardPaymentPayload(last4 = last4, amount = amount) }
        // Two characters in three bytes, and in four.
        for (last4 in listOf("${E_ACUTE}1", E_ACUTE.repeat(2), "123")) {
            assertTrue(cardWithin(card(last4, 50u), 100u), last4)
        }
        for (last4 in listOf("1", E_ACUTE, "1234")) {
            assertFalse(cardWithin(card(last4, 50u), 100u), last4)
        }
        assertTrue(cardWithin(card("12", 1u), 100u))
        assertFalse(cardWithin(card("12", 0u), 100u))
        assertTrue(cardWithin(card("12", 100u), 100u))
        assertFalse(cardWithin(card("12", 101u), 100u))
    }

    /** A byte string written in hex, compared by its octets rather than by the array that holds them. */
    @Test
    fun aByteStringLiteralIsItsOctets() {
        val voucher = voucherIsValue()
        assertContentEquals(
            byteArrayOf(0x00, 0xff.toByte(), 0x10, 0xa0.toByte()),
            voucher.voucher_code,
        )
        assertTrue(voucherIs(voucher))
        // Another array of the same octets matches too.
        assertTrue(voucherIs(VoucherPaymentPayload(voucher.voucher_code.copyOf())))
        val other = voucher.voucher_code.copyOf()
        other[3] = 0xa1.toByte()
        assertFalse(voucherIs(VoucherPaymentPayload(other)))
    }

    /** union-match (§11.4): a union matches in the case its matcher names, and in no other, whatever the case holds. */
    @Test
    fun aUnionMatchesInTheCaseItsMatcherNames() {
        assertTrue(shapeIsCircle(UnionShapeMinimal.Circle(ShapeRadiusMinimalPayload(radius = 3u))))
        assertFalse(shapeIsCircle(UnionShapeMinimal.Dot))
        assertFalse(shapeIsCircle(UnionShapeMinimal.Rect(ShapeRectMinimalPayload(width = 1u, height = 2u))))
    }

    /**
     * An order order_placed_match matches with the quantity limit 5: two lines, the second
     * the exact one, no note and no priority, a VIP customer, paid by a card of four
     * characters.
     */
    private fun order() =
        OrderPlacedPayload(
            order_id = 7u,
            lines = listOf(OrderLinePayload(sku = "A-1", quantity = 5u, color = PaintColor.GREEN), orderLineExactValue()),
            note = null,
            priority = null,
            customer = OrderCustomerPayload(full_name = "Ada", vip = true),
            payment = OrderPayment.Card(CardPaymentPayload(last4 = "1234", amount = 1u)),
        )

    /**
     * A value list of two union matches: a card whose record the case's matcher holds, and
     * the case that holds no record; any other case, or a card its matcher does not hold,
     * matches neither.
     */
    @Test
    fun aUnionFieldMatchesOneOfTheCasesItsValueListNames() {
        assertTrue(orderPlacedMatch(order(), QUANTITY_LIMIT))
        assertTrue(orderPlacedMatch(order().copy(payment = OrderPayment.Cash), QUANTITY_LIMIT))
        assertFalse(
            orderPlacedMatch(order().copy(payment = OrderPayment.Voucher(voucherIsValue())), QUANTITY_LIMIT),
        )
        for ((last4, amount) in listOf("123" to 1u, "1234" to 0u)) {
            val card = order().copy(payment = OrderPayment.Card(CardPaymentPayload(last4 = last4, amount = amount)))
            assertFalse(orderPlacedMatch(card, QUANTITY_LIMIT), "$last4 $amount")
        }
    }

    /**
     * The other fields of the order: a list of records, a reference with the argument it
     * binds, an optional text with ifpresent and a length, an optional number with
     * ifpresent and a range, and a nested record's `bool`.
     */
    @Test
    fun everyFieldOfAnOrderIsHeldToItsMatcher() {
        val first = { quantity: UShort, color: PaintColor ->
            OrderLinePayload(sku = "A-1", quantity = quantity, color = color)
        }
        val withLines = { lines: List<OrderLinePayload> -> order().copy(lines = lines) }
        val more = withLines(listOf(first(6u, PaintColor.GREEN), orderLineExactValue()))
        assertFalse(orderPlacedMatch(more, QUANTITY_LIMIT))
        assertTrue(orderPlacedMatch(more, 6u))
        val blue = withLines(listOf(first(5u, PaintColor.BLUE), orderLineExactValue()))
        assertFalse(orderPlacedMatch(blue, QUANTITY_LIMIT))
        val inexact = withLines(listOf(first(5u, PaintColor.GREEN), first(5u, PaintColor.GREEN)))
        assertFalse(orderPlacedMatch(inexact, QUANTITY_LIMIT))
        val one = withLines(listOf(first(5u, PaintColor.GREEN)))
        assertFalse(orderPlacedMatch(one, QUANTITY_LIMIT))

        for ((note, held) in listOf("abcdefgh" to true, "abcdefghi" to false)) {
            assertEquals(held, orderPlacedMatch(order().copy(note = note), QUANTITY_LIMIT), note)
        }
        for ((priority, held) in listOf(1 to true, 5 to true, 0 to false, 6 to false)) {
            val prioritised = order().copy(priority = priority.toUByte())
            assertEquals(held, orderPlacedMatch(prioritised, QUANTITY_LIMIT), "$priority")
        }
        val ordinary = order().copy(customer = OrderCustomerPayload(full_name = "Ada", vip = false))
        assertFalse(orderPlacedMatch(ordinary, QUANTITY_LIMIT))
    }

    /**
     * docs/SCE_TEST_PROFILE.md §12.5: the unit's `canonical` writes an order by the types
     * its documents declare, in the JSON Canonicalization Scheme: every integer as the
     * text of its digits, an enum and a union's case by their declared names, a byte
     * string in upper-case hex, an absent field omitted and a present one written, a case
     * that holds no record as the empty object. `TYPE` is the unit name of the record's
     * event schema.
     */
    @Test
    fun anOrdersCanonicalFormIsWrittenByTheTypesItsDocumentsDeclare() {
        val lines = "\"lines\":[{\"color\":\"green\",\"quantity\":\"5\",\"sku\":\"A-1\"}," +
            "{\"color\":\"red\",\"quantity\":\"2\",\"sku\":\"A-1\"}]"
        assertEquals(
            "{\"customer\":{\"full_name\":\"Ada\",\"vip\":true},$lines,\"order_id\":\"7\"," +
                "\"payment\":{\"card\":{\"amount\":\"1\",\"last4\":\"1234\"}}}",
            Json.writeJcs(orderPlacedCanonical(order())),
        )
        val noted = order().copy(
            note = "late",
            priority = 3.toUByte(),
            payment = OrderPayment.Voucher(voucherIsValue()),
        )
        assertEquals(
            "{\"customer\":{\"full_name\":\"Ada\",\"vip\":true},$lines,\"note\":\"late\"," +
                "\"order_id\":\"7\",\"payment\":{\"voucher\":{\"voucher_code\":\"00FF10A0\"}}," +
                "\"priority\":\"3\"}",
            Json.writeJcs(orderPlacedCanonical(noted)),
        )
        val cash = Json.writeJcs(orderPlacedCanonical(order().copy(payment = OrderPayment.Cash)))
        assertTrue(cash.endsWith(",\"payment\":{\"cash\":{}}}"), cash)
        assertEquals("order_placed", ORDER_PLACED_TYPE)
    }
}
