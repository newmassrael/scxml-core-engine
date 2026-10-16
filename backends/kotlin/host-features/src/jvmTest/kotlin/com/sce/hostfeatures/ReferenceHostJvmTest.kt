// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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

    /** The system clock's monotonic reading never goes below one already answered. */
    @Test
    fun theSystemClockAnswersAMonotonicReadingThatDoesNotDecrease() {
        val host = ReferenceHost(SystemClock(), Unreachable())
        var last = -1L
        repeat(50) {
            val data = done(host.start("sce-host-clock-now", "now", emptyMap()))
            val mono = Regex("\"monotonic_ms\":\"(\\d+)\"").find(data)!!.groupValues[1].toLong()
            assertTrue(mono >= last, "the monotonic reading went from $last to $mono")
            last = mono
        }
        assertTrue("\"error\":\"\"" in done(host.start("sce-host-clock-zone", "zone", emptyMap())))
    }

    @Test
    fun anOperationOutsideTheStandardSetIsNotServed() {
        val host = ReferenceHost(ScriptedClock(), Unreachable())
        assertNull(host.start("x-app-host", "id", emptyMap()))
    }

    @Test
    fun aKeyWithANulIsRefused() {
        val host = ReferenceHost(ScriptedClock(), Unreachable())
        assertTrue(host.start("sce-host-kv-get", "id", mapOf("key" to "a\u0000b")) is Outcome.Refuse)
    }

    /** Keys are ordered by their UTF-8 bytes, which differs from UTF-16 order past the basic plane. */
    @Test
    fun keysAreOrderedByTheirUtf8Bytes() {
        val host = ReferenceHost(ScriptedClock(), Unreachable())
        // U+FF5E is one UTF-16 unit above the surrogates' range and three UTF-8 bytes; U+1F600 is a surrogate
        // pair in UTF-16 and four UTF-8 bytes, which sorts after it by bytes and before it by UTF-16 units.
        for (key in listOf("～", "😀")) {
            host.start("sce-host-kv-put", "put", mapOf("key" to key, "value" to "v"))
        }
        val data = done(host.start("sce-host-kv-next-key", "walk", mapOf("after" to "", "prefix" to "")))
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
                        "fetch",
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
                "fetch",
                mapOf("method" to "GET", "url" to "$base/items", "headers" to "", "body" to ""),
            ),
        )
        assertTrue("\"error\":\"network\"" in gone, gone)
    }
}
