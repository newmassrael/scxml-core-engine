// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The reader and the writer of the canonical form a test configuration's unit
// generates for each record its system ports take (docs/SCE_TTCN3_LANE.md
// §10.3, §10.5; docs/SCE_TEST_PROFILE.md §12.5), compiled here over records of
// every form: order_placed (a list of records with a closed enum, an optional
// text and integer, a record with a truth value, a union of a case with a
// record, one with none and one with a byte string), job_status (a list of
// integers, optional integers) and form_extras (an enum of an open set, a list
// of optional elements, a 64-bit integer, a list with a lower count, an
// optional list and a list of lists). The units are generated from
// `sce-build/tests/fixtures/test_profile/trace_forms/` (regen:
// `scripts/regen_static_datamodel_kotlin.sh`).
//
// The twin of backends/rust/tests/tests/a_record_reads_back_the_form_it_writes.rs,
// with the same values, the same malformed spellings and the same refusals
// (`a_value_of_each_form_reads_back_from_its_canonical_text` is
// `aValueOfEachFormReadsBackFromItsCanonicalText`).

package com.sce.integration

import com.sce.generated.card_payment.CardPaymentPayload
import com.sce.generated.form_extras.FormExtrasPayload
import com.sce.generated.job_status.JobStatusPayload
import com.sce.generated.order_customer.OrderCustomerPayload
import com.sce.generated.order_line.OrderLinePayload
import com.sce.generated.order_payment.OrderPayment
import com.sce.generated.order_placed.OrderPlacedPayload
import com.sce.generated.paint_color.PaintColor
import com.sce.generated.shape_kind_open.ShapeKindOpen
import com.sce.generated.voucher_payment.VoucherPaymentPayload
import com.sce.runtime.Json
import com.sce.runtime.test.Execute
import com.sce.runtime.test.ModuleParameters
import com.sce.runtime.test.Suite
import com.sce.runtime.test.Trace
import com.sce.runtime.test.TraceInput
import com.sce.runtime.test.TraceRefusal
import com.sce.runtime.test.Verdict
import com.sce.runtime.test.runSuiteWithTraces
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertTrue
import com.sce.generated.forms_configuration.canonical_form_extras as canonicalFormExtras
import com.sce.generated.forms_configuration.canonical_job_status as canonicalJobStatus
import com.sce.generated.forms_configuration.canonical_order_placed as canonicalOrderPlaced
import com.sce.generated.forms_configuration.read_form_extras as readFormExtras
import com.sce.generated.forms_configuration.read_job_status as readJobStatus
import com.sce.generated.forms_configuration.read_order_placed as readOrderPlaced
import com.sce.generated.replays_every_form.NAME as REPLAYS_EVERY_FORM
import com.sce.generated.replays_every_form.testCase as replaysEveryForm

class ARecordReadsBackTheFormItWritesTest {

    /**
     * An order paid by [payment]: two lines, one with a text of a character of
     * two bytes and an escaped quote, the widest order id, a note and no
     * priority.
     */
    private fun order(payment: OrderPayment) = OrderPlacedPayload(
        order_id = UInt.MAX_VALUE,
        lines = listOf(
            OrderLinePayload(sku = "A-1", quantity = 2u, color = PaintColor.GREEN),
            OrderLinePayload(sku = "\"q\" é", quantity = UShort.MAX_VALUE, color = PaintColor.BLUE),
        ),
        note = "ring twice",
        priority = null,
        customer = OrderCustomerPayload(full_name = "Ada", vip = true),
        payment = payment,
    )

    /** The three cases of the payment: one of a record, one of none, one of a byte string. */
    private fun payments(): List<OrderPayment> = listOf(
        OrderPayment.Card(CardPaymentPayload(last4 = "4242", amount = 1u)),
        OrderPayment.Cash,
        OrderPayment.Voucher(VoucherPaymentPayload(voucher_code = byteArrayOf(0x00, 0xab.toByte(), 0xff.toByte()))),
    )

    private fun statuses(): List<JobStatusPayload> = listOf(
        JobStatusPayload(elapsed_ms = 0u, note = null, codes = listOf(), tag = UByte.MAX_VALUE, retry = null, count = 7u),
        JobStatusPayload(
            elapsed_ms = 12u,
            note = "late",
            codes = listOf(3u, 9u, 5u, 0u).map { it.toUByte() },
            tag = null,
            retry = 0u,
            count = 0u,
        ),
    )

    /**
     * The extras: a declared variant and a number no variant declares, a list
     * with an absent element, the least 64-bit integer, a list at its lower
     * count and at its bound, an optional list present and absent, and a list
     * of lists with an empty one.
     */
    private fun extras(): List<FormExtrasPayload> = listOf(
        FormExtrasPayload(
            shape = ShapeKindOpen.CIRCLE,
            slots = listOf(1u.toUShort(), null, UShort.MAX_VALUE),
            least = Long.MIN_VALUE,
            flags = listOf(true),
            spare = listOf(0u.toUByte(), UByte.MAX_VALUE),
            grid = listOf(listOf(1u.toUByte(), 2u.toUByte()), listOf()),
        ),
        FormExtrasPayload(
            shape = ShapeKindOpen.fromUnderlying(9u.toUByte()),
            slots = listOf(),
            least = Long.MAX_VALUE,
            flags = listOf(false, true),
            spare = null,
            grid = listOf(),
        ),
    )

    /**
     * [value] written by [write] in its canonical text, read back from that
     * text by [read], and written again: the same text, and, where [equal]
     * compares the two, the same value.
     */
    private fun <V> roundTrip(value: V, write: (V) -> Any, read: (Any?) -> V, equal: (V, V) -> Boolean) {
        val written = Json.writeJcs(write(value))
        val back = read(Json.parse(written))
        assertTrue(equal(value, back), written)
        assertEquals(written, Json.writeJcs(write(back)))
    }

    /**
     * docs/SCE_TEST_PROFILE.md §12.5: a value of each form reads back from the
     * text its writer gives as the value it was. A Kotlin data class compares a
     * byte string by its identity, so the voucher's octets are compared by
     * their content.
     */
    @Test
    fun aValueOfEachFormReadsBackFromItsCanonicalText() {
        for (payment in payments()) {
            roundTrip(order(payment), ::canonicalOrderPlaced, ::readOrderPlaced) { sent, back ->
                val held = sent.payment
                if (held is OrderPayment.Voucher) {
                    val read = back.payment as OrderPayment.Voucher
                    assertContentEquals(held.value.voucher_code, read.value.voucher_code)
                    sent.copy(payment = OrderPayment.Cash) == back.copy(payment = OrderPayment.Cash)
                } else {
                    sent == back
                }
            }
        }
        for (status in statuses()) {
            roundTrip(status, ::canonicalJobStatus, ::readJobStatus) { sent, back -> sent == back }
        }
        for (extra in extras()) {
            roundTrip(extra, ::canonicalFormExtras, ::readFormExtras) { sent, back -> sent == back }
        }
    }

    /**
     * The reason [read] refuses the canonical text of [value] with [from]
     * replaced once by [to] for.
     */
    private fun <V> refused(value: V, write: (V) -> Any, read: (Any?) -> V, from: String, to: String): String {
        val written = Json.writeJcs(write(value))
        assertTrue(written.contains(from), "`$from` in $written")
        val changed = written.replaceFirst(from, to)
        val parsed = Json.parse(changed)
        return assertFailsWith<TraceRefusal>("$changed is read") { read(parsed) }.reason
    }

    /**
     * docs/SCE_TTCN3_LANE.md §10.2: a malformed spelling of each form is
     * refused, naming the place it stands at: the record's type, each field by
     * its id and `[]` for an element of a list.
     */
    @Test
    fun aMalformedSpellingOfEachFormIsRefusedByName() {
        val (card, cash, voucher) = payments().map { order(it) }
        val ofOrder = { value: OrderPlacedPayload, from: String, to: String ->
            refused(value, ::canonicalOrderPlaced, ::readOrderPlaced, from, to)
        }
        val ofExtras = { from: String, to: String ->
            refused(extras()[0], ::canonicalFormExtras, ::readFormExtras, from, to)
        }
        val ofStatus = { from: String, to: String ->
            refused(statuses()[0], ::canonicalJobStatus, ::readJobStatus, from, to)
        }
        val longName = "\"full_name\":\"${"x".repeat(25)}\""
        val threeLines = "\"lines\":[" +
            "{\"color\":\"red\",\"quantity\":\"1\",\"sku\":\"s\"}," +
            "{\"color\":\"red\",\"quantity\":\"1\",\"sku\":\"s\"}," +
            "{\"color\":\"red\",\"quantity\":\"1\",\"sku\":\"s\"},"
        for ((reason, expected) in listOf(
            ofOrder(card, "\"order_id\":\"4294967295\"", "\"order_id\":\"007\"") to
                "order_placed.order_id: '007' is not the text of an integer's decimal digits",
            ofOrder(card, "\"quantity\":\"2\"", "\"quantity\":\"65536\"") to
                "order_placed.lines[].quantity: 65536 is past the range of its type",
            ofOrder(card, "\"vip\":true", "\"vip\":\"true\"") to
                "order_placed.customer.vip: a text is not a truth value",
            ofOrder(card, "\"full_name\":\"Ada\"", longName) to
                "order_placed.customer.full_name: a text of 25 bytes is past the 24 its type holds",
            ofOrder(card, "\"color\":\"green\"", "\"color\":\"purple\"") to
                "order_placed.lines[].color: 'purple' names no variant of its enum",
            ofOrder(card, "\"note\":\"ring twice\"", "\"note\":null") to
                "order_placed.note: null is not a text",
            ofOrder(card, "{\"customer\":", "{\"rush\":\"1\",\"customer\":") to
                "order_placed: 'rush' is no field of its record",
            ofOrder(card, "\"order_id\":\"4294967295\",", "") to
                "order_placed.order_id: the field is absent, and its type is not optional",
            ofOrder(card, "\"lines\":[", threeLines) to
                "order_placed.lines: a list of 5 elements is outside its bounds, 0 to 4",
            ofOrder(card, "{\"card\":", "{\"cheque\":") to
                "order_placed.payment: 'cheque' names no case of its union",
            ofOrder(card, "\"last4\":\"4242\"}}", "\"last4\":\"4242\"},\"cash\":{}}") to
                "order_placed.payment: a value of a union is an object of one member, and this one has 2",
            ofOrder(cash, "{\"cash\":{}}", "{\"cash\":{\"x\":\"1\"}}") to
                "order_placed.payment.cash: a case that holds no record is the empty object",
            ofOrder(voucher, "\"voucher_code\":\"00ABFF\"", "\"voucher_code\":\"00abff\"") to
                "order_placed.payment.voucher_code: '00abff' is not octets in upper-case hex, two digits each",
            ofOrder(voucher, "\"voucher_code\":\"00ABFF\"", "\"voucher_code\":\"0011223344\"") to
                "order_placed.payment.voucher_code: a byte string of 5 octets is past the 4 its type holds",
            ofStatus("\"tag\":\"255\"", "\"tag\":\"256\"") to
                "job_status.tag: 256 is past the range of its type",
            ofStatus("\"codes\":[]", "\"codes\":[\"x\"]") to
                "job_status.codes[]: 'x' is not the text of an integer's decimal digits",
            ofExtras("\"shape\":\"circle\"", "\"shape\":\"1\"") to
                "form_extras.shape: 1 is the number of a declared variant, which is written by its name",
            ofExtras("\"shape\":\"circle\"", "\"shape\":\"hexagon\"") to
                "form_extras.shape: 'hexagon' names no variant of its enum, and is no number",
            ofExtras("\"slots\":[\"1\",null,", "\"slots\":[\"1\",\"x\",") to
                "form_extras.slots[]: 'x' is not the text of an integer's decimal digits",
            ofExtras("\"flags\":[true]", "\"flags\":[]") to
                "form_extras.flags: a list of 0 elements is outside its bounds, 1 to 2",
            ofExtras("\"grid\":[[\"1\",\"2\"]", "\"grid\":[[\"1\",\"2\",\"3\"]") to
                "form_extras.grid[]: a list of 3 elements is outside its bounds, 0 to 2",
            ofExtras("\"least\":\"-9223372036854775808\"", "\"least\":\"-9223372036854775809\"") to
                "form_extras.least: -9223372036854775809 is past the range of its type",
            ofExtras("\"spare\":[\"0\",\"255\"]", "\"spare\":null") to
                "form_extras.spare: null is not a list, which is an array",
        )) {
            assertEquals(expected, reason)
        }
    }

    /**
     * docs/SCE_TTCN3_LANE.md §10.5: a trace of one record of each type the
     * system port takes, two at the start and one later, replays to the
     * listener, which takes one a round and passes; the log holds each value
     * as the trace does.
     */
    @Test
    fun aTraceOfEveryFormReplaysToTheValuesItHolds() {
        val input = { at: Long, messageType: String, value: Any ->
            TraceInput(at = at, port = "sut", messageType = messageType, value = value)
        }
        val trace = Trace(
            suite = "forms_suite",
            testCase = REPLAYS_EVERY_FORM,
            inputs = listOf(
                input(0L, "order_placed", canonicalOrderPlaced(order(payments()[0]))),
                input(0L, "job_status", canonicalJobStatus(statuses()[0])),
                input(250L, "form_extras", canonicalFormExtras(extras()[0])),
            ),
        )
        val text = trace.canonicalText()
        val suite = Suite(
            name = "forms_suite",
            moduleParameters = ModuleParameters(),
            executes = listOf(Execute(testCase = replaysEveryForm(), guard = null, selection = null)),
        )
        val results = runSuiteWithTraces(suite, emptySet(), { _ -> Trace.parse(text) }, ModuleParameters())
        assertEquals(1, results.size)
        assertEquals(Verdict.PASS, results[0].result.verdict, "${results[0].result.log}")
        assertEquals(250L, results[0].result.endedAt)
        val log = results[0].canonicalLog("forms_suite")
        for (held in trace.inputs) {
            val received = "\"kind\":\"message-received\",\"port\":\"sut\",\"type\":\"${held.messageType}\"," +
                "\"value\":${Json.writeJcs(held.value)}"
            assertTrue(log.contains(received), "`$received` in $log")
        }
    }
}
