// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package com.sce.hostfeatures

import com.sun.net.httpserver.HttpServer
import java.net.InetSocketAddress
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue

class ReferenceHostJvmTest {

    private class Unreachable : Transport {
        override fun exchange(request: HttpRequest): HttpResponse = throw TransportFailure("no network")
    }

    private fun done(outcome: Outcome?): String {
        assertTrue(outcome is Outcome.Done, "the host did not complete: $outcome")
        return outcome.data
    }

    /** Numbers the starts [invocation] gives, so no two of them are one invocation asked twice. */
    private var startsGiven = 0L

    /** A start of the `<invoke>` [id] of one session, numbered apart from every other start. */
    private fun invocation(id: String): Invocation = Invocation("session", id, ++startsGiven)

    private fun putIf(key: String, value: String, expectedVersion: String) =
        mapOf("key" to key, "value" to value, "expected_version" to expectedVersion)

    /**
     * docs/adr/0107, D4: a put-if started again under the identity it had is
     * answered with its first result and writes nothing, and a start of another
     * number, of another session or of another `<invoke>` is an operation of its
     * own.
     */
    @Test
    fun aPutIfRepeatedUnderItsIdentityIsAnsweredWithItsFirstResult() {
        val host = ReferenceHost(ScriptedClock(), Unreachable())
        val first = Invocation("s1", "putif", 1)
        val wrote = done(host.start("sce-host-kv-put-if", first, putIf("k", "a", "0")))
        assertTrue("\"written\":true" in wrote && "\"version\":\"1\"" in wrote, wrote)
        // The repeat finds the key at version 1, which would answer written false; it is
        // answered as the first start was, and the store is as it was.
        assertEquals(wrote, done(host.start("sce-host-kv-put-if", first, putIf("k", "a", "0"))))
        assertEquals(listOf("k" to "a"), host.storeEntries())
        for (other in listOf(Invocation("s1", "putif", 2), Invocation("s2", "putif", 1), Invocation("s1", "other", 1))) {
            val lost = done(host.start("sce-host-kv-put-if", other, putIf("k", "b", "0")))
            assertTrue("\"written\":false" in lost && "\"version\":\"1\"" in lost, "$other: $lost")
        }
        val starts = host.takeStarts()
        assertEquals(5, starts.size)
        assertEquals(Triple("s1", "putif", 1L), Triple(starts[1].sessionId, starts[1].id, starts[1].number))
    }

    /** A refusal is no result, and a failure is a result a repeat answers. */
    @Test
    fun aPutIfKeepsItsFailureAndNotItsRefusal() {
        val host = ReferenceHost(ScriptedClock(), Unreachable())
        val refused = Invocation("s1", "putif", 1)
        repeat(2) { assertTrue(host.start("sce-host-kv-put-if", refused, putIf("", "a", "0")) is Outcome.Refuse) }
        host.setQuota(0)
        val failed = Invocation("s1", "putif", 2)
        val first = done(host.start("sce-host-kv-put-if", failed, putIf("k", "a", "0")))
        assertTrue("\"error\":\"quota_exceeded\"" in first, first)
        host.setQuota(8)
        assertEquals(first, done(host.start("sce-host-kv-put-if", failed, putIf("k", "a", "0"))))
        assertTrue(host.storeEntries().isEmpty())
    }

    /** The system clock's monotonic reading never goes below one already answered. */
    @Test
    fun theSystemClockAnswersAMonotonicReadingThatDoesNotDecrease() {
        val host = ReferenceHost(SystemClock(), Unreachable())
        var last = -1L
        repeat(50) {
            val data = done(host.start("sce-host-clock-now", invocation("now"), emptyMap()))
            val mono = Regex("\"monotonic_ms\":\"(\\d+)\"").find(data)!!.groupValues[1].toLong()
            assertTrue(mono >= last, "the monotonic reading went from $last to $mono")
            last = mono
        }
        assertTrue("\"error\":\"\"" in done(host.start("sce-host-clock-zone", invocation("zone"), emptyMap())))
    }

    @Test
    fun anOperationOutsideTheStandardSetIsNotServed() {
        val host = ReferenceHost(ScriptedClock(), Unreachable())
        assertNull(host.start("x-app-host", invocation("id"), emptyMap()))
    }

    @Test
    fun aKeyWithANulIsRefused() {
        val host = ReferenceHost(ScriptedClock(), Unreachable())
        assertTrue(host.start("sce-host-kv-get", invocation("id"), mapOf("key" to "a\u0000b")) is Outcome.Refuse)
    }

    /** A put-if is refused for a key with U+0000 as a get is, and writes nothing. */
    @Test
    fun aPutIfWithANulInItsKeyIsRefused() {
        val host = ReferenceHost(ScriptedClock(), Unreachable())
        val params = mapOf("key" to "a\u0000b", "value" to "v", "expected_version" to "0")
        assertTrue(host.start("sce-host-kv-put-if", invocation("putif"), params) is Outcome.Refuse)
        assertTrue(host.storeEntries().isEmpty())
    }

    /** Keys are ordered by their UTF-8 bytes, which differs from UTF-16 order past the basic plane. */
    @Test
    fun keysAreOrderedByTheirUtf8Bytes() {
        val host = ReferenceHost(ScriptedClock(), Unreachable())
        // U+FF5E is one UTF-16 unit above the surrogates' range and three UTF-8 bytes; U+1F600 is a surrogate
        // pair in UTF-16 and four UTF-8 bytes, which sorts after it by bytes and before it by UTF-16 units.
        for (key in listOf("～", "😀")) {
            host.start("sce-host-kv-put", invocation("put"), mapOf("key" to key, "value" to "v"))
        }
        val data = done(host.start("sce-host-kv-next-key", invocation("walk"), mapOf("after" to "", "prefix" to "")))
        assertTrue("\"key\":\"～\"" in data, data)
    }

    /** A local server answers; the host carries the headers and body, follows no redirect, and fixes the field order. */
    @Test
    fun theJavaTransportCarriesAnExchangeToALocalServer() {
        val server = HttpServer.create(InetSocketAddress("127.0.0.1", 0), 0)
        var seenBody = ""
        var seenTrace: String? = null
        server.createContext("/") { exchange ->
            seenBody = exchange.requestBody.readBytes().decodeToString()
            seenTrace = exchange.requestHeaders.getFirst("X-Trace")
            when (exchange.requestURI.path) {
                "/moved" -> {
                    exchange.responseHeaders.add("Location", "/elsewhere")
                    exchange.sendResponseHeaders(301, -1)
                }
                "/missing" -> exchange.sendResponseHeaders(404, -1)
                else -> {
                    exchange.responseHeaders.add("X-Reply", "yes")
                    exchange.sendResponseHeaders(201, 4)
                    exchange.responseBody.write("made".encodeToByteArray())
                }
            }
            exchange.close()
        }
        server.start()
        val base = "http://127.0.0.1:${server.address.port}"
        try {
            val host = ReferenceHost(ScriptedClock(), JavaHttpTransport())
            fun fetch(method: String, path: String, headers: String = "", body: String = ""): String =
                done(
                    host.start(
                        "sce-host-http-fetch",
                        invocation("fetch"),
                        mapOf("method" to method, "url" to base + path, "headers" to headers, "body" to body),
                    ),
                )

            val made = fetch("POST", "/items", "X-Trace: 7\nContent-Type: text/plain", "payload")
            assertTrue("\"error\":\"\"" in made && "\"status\":201" in made && "\"body\":\"made\"" in made, made)
            assertTrue("x-reply: yes" in made, made)
            assertEquals("payload", seenBody)
            assertNotNull(seenTrace)
            assertEquals("7", seenTrace)
            val moved = fetch("GET", "/moved")
            assertTrue("\"status\":301" in moved && "location: /elsewhere" in moved, moved)
            assertTrue("\"status\":404" in fetch("GET", "/missing"))
        } finally {
            server.stop(0)
        }
        val host = ReferenceHost(ScriptedClock(), JavaHttpTransport())
        val gone = done(
            host.start(
                "sce-host-http-fetch",
                invocation("fetch"),
                mapOf("method" to "GET", "url" to "$base/items", "headers" to "", "body" to ""),
            ),
        )
        assertTrue("\"error\":\"network\"" in gone, gone)
    }

    private fun ask(prompt: String, schema: String = "", maxTokens: String = "8", temperature: String = "0") =
        mapOf("prompt" to prompt, "answer_schema" to schema, "max_tokens" to maxTokens, "temperature_milli" to temperature)

    private fun modelHost(provider: ModelProvider): ReferenceHost =
        ReferenceHost(ScriptedClock(), Unreachable()).apply {
            setAnswerSchemas(listOf(com.sce.runtime.AnswerSchema("urn:sce:schema:answer:1", "{}", "root ::= \"{}\"")))
            setModelProvider(provider)
        }

    /** Each argument the operation does not accept is refused before the provider is asked. */
    @Test
    fun aGenerationTheOperationDoesNotAcceptIsRefused() {
        val provider = ScriptedProvider().apply {
            constrained = true
            answers.addLast(ScriptedAnswer("kept"))
        }
        val host = modelHost(provider)
        for (params in listOf(ask(""), ask("p", maxTokens = "0"), ask("p", temperature = "2001"), ask("p", "urn:sce:schema:other:1"))) {
            assertTrue(host.start(MODEL_GENERATE, invocation("gen"), params) is Outcome.Refuse, "$params was not refused")
        }
        val kept = done(host.start(MODEL_GENERATE, invocation("gen"), ask("abc", "urn:sce:schema:answer:1")))
        assertTrue("\"text\":\"kept\"" in kept && "\"constrained\":true" in kept && "\"prompt_tokens\":3" in kept, kept)
        assertTrue("\"error\":\"unavailable\"" in done(host.start(MODEL_GENERATE, invocation("gen"), ask("p"))))
    }

    /** An answer past the bound of `text` is never cut; a request past the context takes no answer. */
    @Test
    fun anAnswerPastItsBoundOrItsContextFailsWhole() {
        val provider = ScriptedProvider().apply {
            context = 10
            answers.addLast(ScriptedAnswer("a".repeat(ANSWER_MAX_BYTES + 1)))
            answers.addLast(ScriptedAnswer("kept"))
        }
        val host = modelHost(provider)
        assertTrue("\"error\":\"answer_too_large\"" in done(host.start(MODEL_GENERATE, invocation("gen"), ask("p"))))
        assertTrue("\"error\":\"context_exceeded\"" in done(host.start(MODEL_GENERATE, invocation("gen"), ask("abc"))))
        assertTrue("\"text\":\"kept\"" in done(host.start(MODEL_GENERATE, invocation("gen"), ask("abc", maxTokens = "7"))))
    }

    /** The exhaustive provider answers each class it is named, in order. */
    @Test
    fun theExhaustiveProviderAnswersEachClassInTurn() {
        val host = modelHost(
            ExhaustiveProvider(
                listOf(
                    OutcomeClass.Answer("t", "length"),
                    OutcomeClass.Failure("busy"),
                    OutcomeClass.Refusal,
                    OutcomeClass.Hold,
                ),
            ),
        )
        val answer = done(host.start(MODEL_GENERATE, invocation("gen"), ask("p")))
        assertTrue("\"finish\":\"length\"" in answer && "\"model\":\"exhaustive\"" in answer, answer)
        assertTrue("\"error\":\"busy\"" in done(host.start(MODEL_GENERATE, invocation("gen"), ask("p"))))
        assertTrue(host.start(MODEL_GENERATE, invocation("gen"), ask("p")) is Outcome.Refuse)
        assertEquals(Outcome.Hold, host.start(MODEL_GENERATE, invocation("gen"), ask("p")))
        assertTrue("\"error\":\"unavailable\"" in done(host.start(MODEL_GENERATE, invocation("gen"), ask("p"))))
    }
}
