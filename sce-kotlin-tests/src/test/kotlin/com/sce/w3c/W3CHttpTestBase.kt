// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML C.2: HTTP test harness for BasicHTTPEventProcessor conformance tests.
// Port of C++ HttpAotTest — provides HTTP server lifecycle and send/receive wiring.

package com.sce.w3c

import com.sce.http.W3CHttpTestServer
import com.sce.runtime.Event
import com.sce.runtime.EventMetadata
import com.sce.runtime.State
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Test
import java.net.URI
import java.net.URLEncoder
import java.net.http.HttpClient
import java.net.http.HttpRequest
import java.net.http.HttpResponse

/**
 * Abstract base class for W3C SCXML BasicHTTP Event I/O Processor tests.
 *
 * Matches C++ HttpAotTest pattern:
 * 1. Start W3CHttpTestServer on dynamic port (parallel test safety)
 * 2. Set SM's onHttpSend callback to dispatch HTTP POST (rewriting port)
 * 3. Set server's event callback to inject response events into SM
 * 4. Initialize SM and poll until final state or timeout
 *
 * @param S State sealed interface type
 * @param E Event sealed interface type
 */
abstract class W3CHttpTestBase<S : State, E : Event> : W3CTestBase<S, E>() {

    override val timeoutMs: Long = 5000L

    @Test
    override fun testW3CConformance() {
        // W3C SCXML C.2: Dynamic port allocation for parallel test safety
        val server = W3CHttpTestServer(port = 0, path = "/test")
        if (!server.start()) {
            throw AssertionError("W3CHttpTestServer failed to start")
        }
        val actualPort = server.actualPort

        try {
            val sm = createStateMachine()
            val httpClient = HttpClient.newHttpClient()

            // W3C SCXML C.2: Wire SM HTTP send -> HTTP POST to test server
            // Rewrite SCXML-hardcoded port to actual dynamic port for parallel test safety
            sm.onHttpSend = { request ->
                val originalUri = URI.create(request.target)
                val actualTarget = URI(
                    originalUri.scheme, null, originalUri.host, actualPort,
                    originalUri.path, originalUri.query, originalUri.fragment
                ).toString()
                val (body, contentType) = buildHttpPostBody(
                    request.eventName, request.content, request.params
                )
                try {
                    val httpRequest = HttpRequest.newBuilder()
                        .uri(URI.create(actualTarget))
                        .header("Content-Type", contentType)
                        .POST(HttpRequest.BodyPublishers.ofString(body))
                        .build()
                    // W3C SCXML C.2: Synchronous send ensures server callback (which injects
                    // the response event into SM queue) completes BEFORE performHttpSend returns.
                    // W3CHttpTestServer delivers callback before HTTP response (line 131),
                    // so event ordering is preserved when multiple sends share an onentry block.
                    httpClient.send(httpRequest, HttpResponse.BodyHandlers.ofString())
                } catch (_: Exception) {
                    // Match C++ pattern: log and continue
                }
            }

            // W3C SCXML C.2: Wire server response -> SM event injection
            server.setEventCallback { eventName, eventData ->
                sm.sendEventByName(eventName, EventMetadata.external(data = eventData))
            }

            sm.initialize()

            // C++ HttpAotTest pattern: async polling with timeout
            if (!sm.isInFinalState) {
                val deadline = System.currentTimeMillis() + timeoutMs
                while (!sm.isInFinalState && System.currentTimeMillis() < deadline) {
                    Thread.sleep(10)
                    sm.tick()
                }
            }

            try {
                assertEquals(
                    expectedPassState,
                    sm.currentState.value,
                    "W3C HTTP conformance failed: expected Pass but reached ${sm.currentState.value}"
                )
            } finally {
                sm.cleanup()
            }
        } finally {
            server.stop()
        }
    }

    companion object {
        /**
         * W3C SCXML C.2: Build HTTP POST body from send request data.
         *
         * Matches C++ SendHelper::buildHttpPostBody + HttpSendHelper::buildRequestBody:
         * - If event name or params present: form-encoded (application/x-www-form-urlencoded)
         * - If content present: text/plain
         * - Otherwise: empty form
         */
        fun buildHttpPostBody(
            eventName: String,
            content: String,
            params: Map<String, List<String>>
        ): Pair<String, String> {
            if (eventName.isNotEmpty() || params.isNotEmpty()) {
                // Form-encoded (W3C SCXML C.2)
                val parts = mutableListOf<String>()
                if (eventName.isNotEmpty()) {
                    parts.add("_scxmleventname=${urlEncode(eventName)}")
                }
                for ((key, values) in params) {
                    // Skip _scxmleventname if already added via eventName
                    if (key == "_scxmleventname" && eventName.isNotEmpty()) continue
                    for (value in values) {
                        parts.add("${urlEncode(key)}=${urlEncode(value)}")
                    }
                }
                return parts.joinToString("&") to "application/x-www-form-urlencoded"
            } else if (content.isNotEmpty()) {
                return content to "text/plain"
            }
            return "" to "application/x-www-form-urlencoded"
        }

        private fun urlEncode(value: String): String =
            URLEncoder.encode(value, "UTF-8")
    }
}
