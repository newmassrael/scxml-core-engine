// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The recorded input trace, the readers of the canonical form and the runner's
// command line (docs/SCE_TTCN3_LANE.md §4, §10). Each case is the twin of the
// Rust case of the same name in backends/rust/runtime/src/test/trace.rs,
// from_canonical.rs and runner.rs (`a_trace_is_read_from_its_canonical_text_and_written_back`
// is `a trace is read from its canonical text and written back`), with the same
// texts and the same refusals.

package com.sce.runtime.test

import com.sce.runtime.Json
import com.sce.runtime.SavedValues
import java.io.File
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertNull
import kotlin.test.assertTrue

/** The directory of the repository, above the one the suite runs in. */
private fun repoRoot(): File =
    generateSequence(File(System.getProperty("user.dir")).absoluteFile) { it.parentFile }
        .firstOrNull { File(it, "sce-build").isDirectory }
        ?: error("no repository root above ${System.getProperty("user.dir")}")

class TraceTest {

    private companion object {
        const val ONE_INPUT: String =
            "{\"inputs\":[{\"at\":\"500\",\"port\":\"sut\",\"type\":\"job_reply\"," +
                "\"value\":{\"code\":\"0\",\"job\":\"7\"}}]," +
                "\"schema\":\"sce-test-trace.v1\",\"suite\":\"runtime_suite\"," +
                "\"test_case\":\"answers_the_job\"}"
    }

    /**
     * The status the runtime declares is the one the trace's schema file
     * declares, and the schema pins the name of the shape the runtime reads.
     */
    @Test
    fun `the schema file declares the status the runtime does`() {
        val schema = Json.parse(File(repoRoot(), "schemas/sce-test-trace.v1.schema.json").readText()) as Map<*, *>
        assertEquals(
            TEST_TRACE_SCHEMA_STATUS,
            schema["x-sce-schema-status"],
            "schemas/sce-test-trace.v1.schema.json x-sce-schema-status disagrees with " +
                "TEST_TRACE_SCHEMA_STATUS; SCE_WIRE_CONTRACTS.md requires one commit to move both",
        )
        val properties = schema["properties"] as Map<*, *>
        val shape = properties["schema"] as Map<*, *>
        assertEquals(TEST_TRACE_SCHEMA, shape["const"])
    }

    /**
     * A trace is read from its canonical text, with or without the line feed
     * a file ends in, and written back as the same bytes.
     */
    @Test
    fun `a trace is read from its canonical text and written back`() {
        val trace = Trace.parse(ONE_INPUT)
        assertEquals("runtime_suite", trace.suite)
        assertEquals("answers_the_job", trace.testCase)
        assertEquals(
            listOf(
                TraceInput(
                    at = 500,
                    port = "sut",
                    messageType = "job_reply",
                    value = Canonical.record("code" to Canonical.integer(0u), "job" to Canonical.integer(7u)),
                ),
            ),
            trace.inputs,
        )
        assertEquals(ONE_INPUT, trace.canonicalText())
        assertEquals(trace, Trace.parse("$ONE_INPUT\n"))
    }

    private fun refused(text: String): String = assertFailsWith<TraceRefusal>(text) { Trace.parse(text) }.reason

    private fun changed(from: String, to: String): String {
        assertTrue(ONE_INPUT.contains(from), from)
        return ONE_INPUT.replaceFirst(from, to)
    }

    /** Each change of one member of a trace it first reads is refused, by name. */
    @Test
    fun `a trace that is not of its shape is refused by name`() {
        for ((text, expected) in listOf(
            changed("sce-test-trace.v1", "sce-test-trace.v2") to "schema is not sce-test-trace.v1",
            changed("\"500\"", "500") to "the instant of input 0",
            changed("\"500\"", "\"0500\"") to "the instant of input 0",
            changed("\"500\"", "\"-5\"") to "the instant of input 0",
            changed("\"500\"", "\"9223372036854775808\"") to "past the microseconds",
            changed("\"port\":\"sut\"", "\"port\":\"\"") to "the port of input 0",
            changed("\"type\"", "\"kind\"") to "input 0 has no member 'kind'",
            changed(",\"suite\":\"runtime_suite\"", "") to "the trace has no 'suite'",
            changed("{\"inputs\"", "{ \"inputs\"") to "one canonical text",
            changed("[{", "{") to "not JSON",
        )) {
            val reason = refused(text)
            assertTrue(reason.contains(expected), "$expected: $reason")
        }
        val outOfOrder =
            "{\"inputs\":[{\"at\":\"5\",\"port\":\"sut\",\"type\":\"t\",\"value\":{}}," +
                "{\"at\":\"4\",\"port\":\"sut\",\"type\":\"t\",\"value\":{}}]," +
                "\"schema\":\"sce-test-trace.v1\",\"suite\":\"s\",\"test_case\":\"c\"}"
        assertTrue(refused(outOfOrder).contains("input 1, at 4, comes after an input at 5"))
    }

    /**
     * A number past the range of a double has no canonical text, so a trace
     * whose value holds one is refused as not written in it, not thrown out of
     * the reader.
     */
    @Test
    fun `a trace whose value holds a number past a double is refused`() {
        val text = changed("{\"code\":\"0\",\"job\":\"7\"}", "1e400")
        // The Rust reader refuses the same text in the same words
        // (`a_trace_whose_value_holds_a_number_past_a_double_is_refused`).
        assertEquals(
            "the trace is not written in its one canonical text, the JSON Canonicalization Scheme",
            refused(text),
        )
    }

    /**
     * An input on a port the configuration does not have, or of a type its
     * port does not take, is refused, naming which.
     */
    @Test
    fun `an input the configuration does not take is refused by name`() {
        assertEquals(
            "'job_request' is not on the `in` list of the type of the system port 'sut'",
            TraceRefusal.notAnInput("sut", "job_request", listOf("sut")).reason,
        )
        assertEquals(
            "'oracle' is not a system port of the test case's configuration",
            TraceRefusal.notAnInput("oracle", "job_reply", listOf("sut")).reason,
        )
        val trace = Trace.parse(ONE_INPUT)
        val refused = assertFailsWith<TraceRefusal> { TraceAdapter.replaying(trace, ::noInputs) }
        assertEquals(
            "input 0, at 500: 'sut' is not a system port of the test case's configuration",
            refused.reason,
        )
    }
}

class FromCanonicalTest {

    /** The reason of the refusal [read] throws. */
    private fun reason(read: () -> Any?): String = assertFailsWith<TraceRefusal> { read() }.reason

    /**
     * An integer is read from the one spelling the writer gives it, at every
     * width, and any other spelling, or a value past its type, is refused,
     * naming where it stands.
     */
    @Test
    fun `an integer is read from the text of its digits alone`() {
        assertEquals(0u.toUByte(), FromCanonical.integer(Canonical.integer(0u.toUByte()), FromCanonical.UINT8, "n"))
        assertEquals(ULong.MAX_VALUE, FromCanonical.integer(Canonical.integer(ULong.MAX_VALUE), FromCanonical.UINT64, "n"))
        assertEquals(Long.MIN_VALUE, FromCanonical.integer(Canonical.integer(Long.MIN_VALUE), FromCanonical.INT64, "n"))
        for (spelling in listOf("007", "+7", "-0", "7.0", "", " 7")) {
            val refusal = reason { FromCanonical.integer(spelling, FromCanonical.UINT32, "job_reply.code") }
            assertTrue(refusal.startsWith("job_reply.code: "), refusal)
            assertTrue(refusal.contains("decimal digits"), "$spelling: $refusal")
        }
        assertTrue(reason { FromCanonical.integer("256", FromCanonical.UINT8, "n") }.contains("past the range"))
        assertTrue(reason { FromCanonical.integer("-1", FromCanonical.UINT32, "n") }.contains("past the range"))
        assertTrue(
            reason { FromCanonical.integer(Json.Number("7"), FromCanonical.UINT32, "n") }
                .contains("a number is not an integer"),
        )
        // Past the range of a 128-bit integer, as the Rust reader of the same
        // text, the digits are no integer's at all.
        assertTrue(
            reason { FromCanonical.integer("1" + "0".repeat(39), FromCanonical.UINT64, "n") }
                .contains("is not the text of an integer's decimal digits"),
        )
    }

    @Test
    fun `a bool is a truth value`() {
        assertEquals(true, FromCanonical.boolean(Canonical.boolean(true), "b"))
        assertTrue(reason { FromCanonical.boolean("true", "b") }.contains("a text is not a truth value"))
    }

    /** A text is read within its bound in UTF-8 bytes, not in characters. */
    @Test
    fun `a text is read within its bound`() {
        assertEquals("abc", FromCanonical.text(Canonical.text("abc"), 3, "t"))
        assertTrue(reason { FromCanonical.text(Canonical.text("abcd"), 3, "t") }.contains("past the 3"))
        assertEquals(
            "t: a text of 2 bytes is past the 1 its type holds",
            reason { FromCanonical.text(Canonical.text("é"), 1, "t") },
        )
    }

    /**
     * A byte string is read from upper-case hex alone, the writer's spelling,
     * and back to the same text.
     */
    @Test
    fun `a byte string is read from upper case hex`() {
        val octets = byteArrayOf(0x00, 0xab.toByte(), 0x0f, 0xff.toByte())
        val written = Canonical.bytes(octets)
        val read = FromCanonical.bytes(written, 4, "b")
        assertContentEquals(octets, read)
        assertEquals(written, Canonical.bytes(read))
        for (spelling in listOf("00ab", "0", "0G")) {
            assertTrue(reason { FromCanonical.bytes(spelling, 4, "b") }.contains("upper-case hex"), spelling)
        }
        assertTrue(reason { FromCanonical.bytes(written, 3, "b") }.contains("past the 3"))
    }

    /**
     * A closed enum is read by its declared names; an open one by its names
     * and by the numbers no variant declares, a declared number being the
     * second spelling of its variant.
     */
    @Test
    fun `an enum is read by its declared name`() {
        val declared = listOf("red" to 0u.toUByte(), "green" to 1u.toUByte())
        assertEquals(1u.toUByte(), FromCanonical.enumerated("green", declared, "c"))
        assertTrue(reason { FromCanonical.enumerated("blue", declared, "c") }.contains("'blue' names no"))
        val open = { text: String -> FromCanonical.openEnumerated(text, declared, FromCanonical.UINT8, "c") { it } }
        assertEquals(0u.toUByte(), open("red"))
        assertEquals(7u.toUByte(), open("7"))
        assertTrue(reason { open("1") }.contains("written by its name"))
        assertTrue(reason { open("300") }.contains("past the range"))
    }

    /**
     * A record holds only the fields it declares; a field whose type is not
     * optional is present, and an optional one that is present as `null` is
     * read, and refused, as its type's reader refuses `null`.
     */
    @Test
    fun `a record holds its declared fields`() {
        val value = Json.parse("{\"code\":\"0\",\"job\":\"7\"}")
        val record = FromCanonical.record(value, listOf("job", "code"), "job_reply")
        assertEquals(
            7u,
            FromCanonical.integer(FromCanonical.field(record, "job", "job_reply.job"), FromCanonical.UINT32, "job"),
        )
        assertNull(FromCanonical.optionalField(record, "note") { FromCanonical.text(it, 8, "job_reply.note") })
        val stray = Json.parse("{\"job\":\"7\",\"jobs\":\"1\"}")
        assertTrue(
            reason { FromCanonical.record(stray, listOf("job", "code"), "job_reply") }.contains("'jobs' is no field"),
        )
        val short = FromCanonical.record(Json.parse("{\"job\":\"7\"}"), listOf("job", "code"), "job_reply")
        assertEquals(
            "job_reply.code: the field is absent, and its type is not optional",
            reason { FromCanonical.field(short, "code", "job_reply.code") },
        )
        val written = FromCanonical.record(Json.parse("{\"note\":null}"), listOf("note"), "job_reply")
        assertEquals(
            "job_reply.note: null is not a text",
            reason { FromCanonical.optionalField(written, "note") { FromCanonical.text(it, 8, "job_reply.note") } },
        )
    }

    /** A list is read within its bounds, an absent element as null. */
    @Test
    fun `a list is read within its bounds`() {
        val value = Json.parse("[\"1\",null]")
        val read = FromCanonical.elements(value, 1, 2, "l").map { item ->
            FromCanonical.nullable(item) { FromCanonical.integer(it, FromCanonical.UINT8, "l[]") }
        }
        assertEquals(listOf(1u.toUByte(), null), read)
        assertTrue(reason { FromCanonical.elements(value, 3, 4, "l") }.contains("outside its bounds, 3 to 4"))
    }

    /**
     * A union is one member named by its case; a case that holds no record is
     * the empty object.
     */
    @Test
    fun `a union is one member named by its case`() {
        val card = Canonical.unionCase("card", Canonical.record("amount" to Canonical.integer(1u)))
        val cases = listOf("card", "cash")
        val read = FromCanonical.unionCase(card, cases, "p")
        assertEquals("card", read.variant)
        assertEquals("{\"amount\":\"1\"}", Json.writeJcs(read.value))
        val cash = FromCanonical.unionCase(Canonical.unionCase("cash", null), cases, "p")
        assertEquals("cash", FromCanonical.noRecord(cash.value, "p", "cash"))
        assertTrue(reason { FromCanonical.noRecord(card, "p", "cash") }.contains("is the empty object"))
        val two = Json.parse("{\"card\":{},\"cash\":{}}")
        assertTrue(reason { FromCanonical.unionCase(two, cases, "p") }.contains("this one has 2"))
        assertEquals(
            "p: 'cheque' names no case of its union",
            reason { FromCanonical.unionCase(Canonical.unionCase("cheque", null), cases, "p") },
        )
    }
}

class RunnerOptionsTest {

    private fun refused(args: List<String>): String =
        assertFailsWith<RunnerUsageRefusal>("$args") { RunnerOptions.parse(args) }.reason

    /** The command line every backend's runner takes. */
    @Test
    fun `the command line is read as every runner reads it`() {
        val options = RunnerOptions.parse(
            listOf(
                "--trace-dir", "traces",
                "--log-dir", "logs",
                "--world", "ptcs,guard",
                "--module-parameter", "job=9",
                "--module-parameter", "label=a=b",
            ),
        )
        assertEquals("traces", options.traceDir)
        assertEquals("logs", options.logDir)
        assertEquals(setOf("ptcs", "guard"), options.world)
        assertEquals(listOf("job" to "9", "label" to "a=b"), options.moduleParameters)
        val bare = RunnerOptions.parse(listOf("--log-dir", "l", "--trace-dir", "t"))
        assertTrue(bare.world.isEmpty())
        assertTrue(RunnerOptions.parse(listOf("--trace-dir", "t", "--log-dir", "l", "--world", "")).world.isEmpty())
        for ((line, why) in listOf(
            listOf("--log-dir", "l") to "--trace-dir is not given",
            listOf("--trace-dir", "t") to "--log-dir is not given",
            listOf("--trace-dir", "t", "--log-dir", "l", "--trace-dir", "u") to "--trace-dir is given twice",
            listOf("--trace-dir", "t", "--log-dir", "l", "--seed", "1") to "--seed is no option",
            listOf("--trace-dir") to "--trace-dir is given no value",
            listOf("--trace-dir", "t", "--log-dir", "l", "--module-parameter", "job") to "is not <id>=<value>",
            listOf(
                "--trace-dir", "t", "--log-dir", "l",
                "--module-parameter", "job=1", "--module-parameter", "job=2",
            ) to "'job' is given twice",
        )) {
            val reason = refused(line)
            assertTrue(reason.contains(why), "$why: $reason")
        }
    }

    /**
     * A module parameter's literal is read as the value of its type, and one
     * that is no value of it is refused, naming the parameter.
     */
    @Test
    fun `a module parameter is read as the saved value of its type`() {
        assertEquals(9u, integerParameter("job", "9", FromCanonical.UINT32))
        assertEquals(ULong.MAX_VALUE, integerParameter("wide", "18446744073709551615", FromCanonical.UINT64))
        assertTrue(
            assertFailsWith<OverrideRefusal> { integerParameter("job", "256", FromCanonical.UINT8) }
                .reason.startsWith("job: "),
        )
        assertFailsWith<OverrideRefusal> { integerParameter("job", "09", FromCanonical.UINT32) }
        assertEquals(true, boolParameter("armed", "true"))
        assertFailsWith<OverrideRefusal> { boolParameter("armed", "yes") }
        assertEquals("nightly", textParameter("nightly"))
        assertEquals("green", enumParameter("green"))
        assertEquals(SavedValues.of(byteArrayOf(0x0a, 0xff.toByte())), bytesParameter("key", "0aFF"))
        for (literal in listOf("0a0", "0g", "+a", "０１")) {
            assertFailsWith<OverrideRefusal>(literal) { bytesParameter("key", literal) }
        }
        assertEquals("the suite declares no module parameter 'limit'", undeclaredParameter("limit").reason)
    }
}
