// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The canonical form of a value in the test log, a message's value, and the
// records of the test log (docs/SCE_TEST_PROFILE.md §12.5), each written in the
// JSON Canonicalization Scheme. Each case is the twin of the Rust case of the
// same name in backends/rust/runtime/src/test/canonical.rs, port.rs and
// test_log.rs (`an_integer_is_the_text_of_its_decimal_digits` is
// `an integer is the text of its decimal digits`), with the same bytes.

package com.sce.runtime.test

import com.sce.runtime.Json
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull

private fun written(value: Any): String = Json.writeJcs(value)

class CanonicalTest {

    /**
     * An integer is the text of its digits at every width, the ends of the
     * 64-bit ones among them, which a double does not hold exactly.
     */
    @Test
    fun `an integer is the text of its decimal digits`() {
        assertEquals("\"0\"", written(Canonical.integer(0u.toUByte())))
        assertEquals("\"-128\"", written(Canonical.integer(Byte.MIN_VALUE)))
        assertEquals("\"18446744073709551615\"", written(Canonical.integer(ULong.MAX_VALUE)))
        assertEquals("\"-9223372036854775808\"", written(Canonical.integer(Long.MIN_VALUE)))
    }

    @Test
    fun `a bool is a truth value and a text a text`() {
        assertEquals("true", written(Canonical.boolean(true)))
        assertEquals("false", written(Canonical.boolean(false)))
        assertEquals("\"a\\\"b\"", written(Canonical.text("a\"b")))
    }

    /** Two hex digits an octet, upper case, the leading zero kept. */
    @Test
    fun `a byte string is its octets in upper case hex`() {
        assertEquals(
            "\"00AB0FFF\"",
            written(Canonical.bytes(byteArrayOf(0x00, 0xab.toByte(), 0x0f, 0xff.toByte()))),
        )
        assertEquals("\"\"", written(Canonical.bytes(byteArrayOf())))
    }

    /**
     * A closed enum by its declared name; an open one by its name, or by its
     * number where no variant declares it.
     */
    @Test
    fun `an enum is its declared name`() {
        assertEquals("\"red\"", written(Canonical.enumerated("red")))
        assertEquals("\"red\"", written(Canonical.openEnumerated("red", Canonical.integer(0u.toUByte()))))
        assertEquals("\"7\"", written(Canonical.openEnumerated(null, Canonical.integer(7u.toUByte()))))
    }

    /**
     * An absent field is omitted, and the fields are sorted when written,
     * whatever order they were given in.
     */
    @Test
    fun `a record omits an absent field`() {
        val value = Canonical.record(
            "quantity" to Canonical.integer(2u.toUShort()),
            "note" to null,
            "color" to Canonical.enumerated("red"),
        )
        assertEquals("{\"color\":\"red\",\"quantity\":\"2\"}", written(value))
        assertEquals("{}", written(Canonical.record("note" to null)))
    }

    @Test
    fun `a list is its elements in order`() {
        val value = Canonical.list(listOf(3u, 1u, 2u).map { Canonical.integer(it.toUByte()) })
        assertEquals("[\"3\",\"1\",\"2\"]", written(value))
        assertEquals("[]", written(Canonical.list(emptyList())))
    }

    /**
     * An absent element of a list is `null` in its place, and a present one
     * its value: the array keeps every position.
     */
    @Test
    fun `an absent element of a list is null`() {
        val tags: List<UByte?> = listOf(1u.toUByte(), null)
        val value = Canonical.list(tags.map { tag -> Canonical.optionalElement(tag) { n -> Canonical.integer(n) } })
        assertEquals("[\"1\",null]", written(value))
    }

    /**
     * A union is one member named by its case's variant: the case's record, or
     * the empty object for a case that holds none.
     */
    @Test
    fun `a union is one member named by its variant`() {
        val card = Canonical.unionCase("card", Canonical.record("amount" to Canonical.integer(1u)))
        assertEquals("{\"card\":{\"amount\":\"1\"}}", written(card))
        assertEquals("{\"cash\":{}}", written(Canonical.unionCase("cash", null)))
    }
}

class MessageValueTest {

    /**
     * A message's value is read back by its type, and by no other, and its
     * canonical form is the one its writer gives, whichever reader asks.
     */
    @Test
    fun `a message value is read by its type and written by its writer`() {
        val message = Message.of("number", 7u, "number") { n -> Canonical.integer(n) }
        assertEquals(7u, message.valueAs<UInt>())
        assertNull(message.valueAs<Int>())
        assertEquals("7", message.canonicalValue())
        assertEquals("number", message.typeName)
        assertEquals("TestValue(typeName=number, ..)", message.value.toString())
    }
}

class TestLogRecordTest {

    private fun record(event: TestLogEvent): String = written(TestLogRecord(at = 250, event = event).toValue())

    /**
     * A record of each shape: `at` and every integer as the text of its
     * digits, an absent reason omitted, the members sorted when written.
     */
    @Test
    fun `a record is written with the members of its kind`() {
        assertEquals(
            "{\"at\":\"250\",\"component\":\"mtc\",\"duration_us\":\"1000000\"," +
                "\"kind\":\"timer-started\",\"timer\":\"reply\"}",
            record(TestLogEvent.TimerStarted("mtc", "reply", 1_000_000)),
        )
        assertEquals(
            "{\"at\":\"250\",\"component\":\"echo\",\"kind\":\"verdict-set\"," +
                "\"verdict\":\"inconc\"}",
            record(TestLogEvent.VerdictSet("echo", Verdict.INCONCLUSIVE, null)),
        )
        assertEquals(
            "{\"at\":\"250\",\"component\":\"echo\",\"kind\":\"verdict-set\"," +
                "\"reason\":\"late\",\"verdict\":\"fail\"}",
            record(TestLogEvent.VerdictSet("echo", Verdict.FAIL, "late")),
        )
        assertEquals(
            "{\"at\":\"250\",\"component\":\"mtc\",\"kind\":\"message-received\"," +
                "\"port\":\"sut\",\"type\":\"job_reply\",\"value\":{\"job\":\"7\"}}",
            record(
                TestLogEvent.MessageReceived(
                    "mtc",
                    "sut",
                    "job_reply",
                    Canonical.record("job" to Canonical.integer(7u)),
                ),
            ),
        )
    }

    /** Every kind the runtime writes, each spelled as the schema names it. */
    @Test
    fun `every kind is spelled as the schema names it`() {
        val kinds = listOf(
            TestLogEvent.TestCaseStarted,
            TestLogEvent.TestCaseEnded(Verdict.NONE),
            TestLogEvent.ComponentCreated("", ""),
            TestLogEvent.ComponentDone("", Verdict.NONE),
            TestLogEvent.MessageSent("", "", "", ""),
            TestLogEvent.MessageReceived("", "", "", ""),
            TestLogEvent.TimerStarted("", "", 0),
            TestLogEvent.TimerStopped("", ""),
            TestLogEvent.TimerTimedOut("", ""),
            TestLogEvent.VerdictSet("", Verdict.NONE, null),
            TestLogEvent.Log("", ""),
        ).map { it.kind }
        assertEquals(
            listOf(
                "test-case-started",
                "test-case-ended",
                "component-created",
                "component-done",
                "message-sent",
                "message-received",
                "timer-started",
                "timer-stopped",
                "timer-timed-out",
                "verdict-set",
                "log",
            ),
            kinds,
        )
    }

    /**
     * The log of a run: its shape, suite, test case and verdict before its
     * records, all sorted, with no whitespace.
     */
    @Test
    fun `a log is its shape suite case verdict and records`() {
        val result = TestCaseResult(
            verdict = Verdict.ERROR,
            endedAt = 0,
            records = listOf(
                TestLogRecord(at = 0, event = TestLogEvent.TestCaseStarted),
                TestLogRecord(at = 0, event = TestLogEvent.TestCaseEnded(Verdict.ERROR)),
            ),
        )
        assertEquals(
            "{\"records\":[{\"at\":\"0\",\"kind\":\"test-case-started\"}," +
                "{\"at\":\"0\",\"kind\":\"test-case-ended\",\"verdict\":\"error\"}]," +
                "\"schema\":\"sce-test-log.v1\",\"suite\":\"a_suite\",\"test_case\":\"a_case\"," +
                "\"verdict\":\"error\"}",
            result.canonicalLog("a_suite", "a_case"),
        )
    }
}
