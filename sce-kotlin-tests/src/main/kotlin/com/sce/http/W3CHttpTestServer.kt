// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML C.2: BasicHTTP Event I/O Processor test server.
// Port of C++ W3CHttpTestServer — lightweight HTTP server for W3C conformance tests.

package com.sce.http

import com.sun.net.httpserver.HttpExchange
import com.sun.net.httpserver.HttpServer
import java.net.InetSocketAddress
import java.net.URLDecoder
import java.util.concurrent.Executors

/**
 * W3C SCXML C.2: Test HTTP server for BasicHTTPEventProcessor conformance tests.
 *
 * Receives HTTP POST requests and extracts event information following W3C rules:
 * 1. Form data with `_scxmleventname` param -> use param value as event name
 * 2. Form data without `_scxmleventname` -> use "event1" default
 * 3. Non-form content without event -> use HTTP method name ("HTTP.POST")
 *
 * Matches C++ W3CHttpTestServer behavior for test parity.
 */
class W3CHttpTestServer(
    private val port: Int = 0,
    private val path: String = "/test"
) {
    private var server: HttpServer? = null
    private var executor: java.util.concurrent.ExecutorService? = null
    private var callback: ((eventName: String, data: String) -> Unit)? = null
    private val pendingEvents = mutableListOf<Pair<String, String>>()
    private val lock = Any()

    /** Actual port the server is listening on (valid after [start]). */
    val actualPort: Int get() = server?.address?.port ?: 0

    /**
     * Start the HTTP server. Blocks until the server is ready to accept connections.
     * Uses port 0 by default for OS-assigned dynamic port (parallel test safety).
     */
    fun start(): Boolean {
        return try {
            val httpServer = HttpServer.create(InetSocketAddress(port), 0)
            val exec = Executors.newSingleThreadExecutor()
            httpServer.executor = exec
            httpServer.createContext(path) { exchange -> handlePost(exchange) }
            httpServer.start()
            server = httpServer
            executor = exec
            true
        } catch (e: Exception) {
            false
        }
    }

    /**
     * Stop the HTTP server and release the port.
     */
    fun stop() {
        server?.stop(0)
        server = null
        executor?.shutdown()
        executor = null
        // C++ pattern: sleep briefly to allow OS to fully release port
        Thread.sleep(100)
    }

    /**
     * Set the event callback. Any pending events queued before the callback
     * was set are delivered immediately.
     */
    fun setEventCallback(cb: (eventName: String, data: String) -> Unit) {
        synchronized(lock) {
            callback = cb
            // Deliver any events that arrived before callback was set
            for ((name, data) in pendingEvents) {
                cb(name, data)
            }
            pendingEvents.clear()
        }
    }

    /**
     * W3C SCXML C.2: Handle incoming HTTP POST request.
     *
     * Parses the request body to extract event name and data following
     * the W3C BasicHTTP Event I/O Processor specification.
     */
    private fun handlePost(exchange: HttpExchange) {
        try {
            if (exchange.requestMethod != "POST") {
                exchange.sendResponseHeaders(405, -1)
                exchange.close()
                return
            }

            val contentType = exchange.requestHeaders.getFirst("Content-Type") ?: ""
            val body = exchange.requestBody.bufferedReader().readText()
            var eventName: String
            var eventData = ""

            if (contentType.contains("application/x-www-form-urlencoded")) {
                // W3C SCXML C.2: Form-encoded data
                val params = parseFormData(body)
                // _scxmleventname has highest priority for event name
                eventName = params["_scxmleventname"]?.firstOrNull() ?: "event1"

                // Build JSON from all params for event data
                val jsonEntries = mutableListOf<String>()
                for ((key, values) in params) {
                    for (value in values) {
                        val jsonValue = tryNumeric(value)
                        jsonEntries.add("\"$key\":$jsonValue")
                    }
                }
                if (jsonEntries.isNotEmpty()) {
                    eventData = "{${jsonEntries.joinToString(",")}}"
                }
            } else if (body.trimStart().startsWith("{") || body.trimStart().startsWith("[")) {
                // JSON content — extract "event" field
                eventName = extractJsonEventField(body) ?: "HTTP.POST"
                eventData = body
            } else {
                // Non-JSON, non-form content — use HTTP method name as event
                eventName = "HTTP.POST"
                eventData = body
            }

            // C++ W3CHttpTestServer pattern: deliver callback BEFORE sending HTTP response
            synchronized(lock) {
                val cb = callback
                if (cb != null) {
                    cb(eventName, eventData)
                } else {
                    pendingEvents.add(eventName to eventData)
                }
            }

            // Build JSON response (C++ W3CHttpTestServer pattern)
            val sendId = "w3c_test_${System.currentTimeMillis()}"
            val safeEventName = eventName.replace("\\", "\\\\").replace("\"", "\\\"")
            val responseJson = """{"status":"ok","event":"$safeEventName","sendId":"$sendId"}"""

            // Send response
            val responseBytes = responseJson.toByteArray()
            exchange.responseHeaders.add("Content-Type", "application/json")
            exchange.responseHeaders.add("Cache-Control", "no-cache")
            exchange.responseHeaders.add("Access-Control-Allow-Origin", "*")
            exchange.responseHeaders.add("Access-Control-Allow-Methods", "POST, OPTIONS")
            exchange.responseHeaders.add("Access-Control-Allow-Headers", "Content-Type")
            exchange.sendResponseHeaders(200, responseBytes.size.toLong())
            exchange.responseBody.write(responseBytes)
            exchange.responseBody.close()
        } catch (e: Exception) {
            try {
                val safeMsg = (e.message ?: "unknown").replace("\\", "\\\\").replace("\"", "\\\"")
                val errorJson = """{"status":"error","message":"$safeMsg"}"""
                val errorBytes = errorJson.toByteArray()
                exchange.sendResponseHeaders(500, errorBytes.size.toLong())
                exchange.responseBody.write(errorBytes)
                exchange.responseBody.close()
            } catch (_: Exception) {
                exchange.close()
            }
        }
    }

    /**
     * Parse URL-encoded form data into a multi-value parameter map.
     * Supports duplicate parameter names (W3C test 178).
     */
    private fun parseFormData(body: String): Map<String, List<String>> {
        if (body.isBlank()) return emptyMap()
        val result = mutableMapOf<String, MutableList<String>>()
        for (pair in body.split("&")) {
            val parts = pair.split("=", limit = 2)
            val key = URLDecoder.decode(parts[0], "UTF-8")
            val value = if (parts.size > 1) URLDecoder.decode(parts[1], "UTF-8") else ""
            result.getOrPut(key) { mutableListOf() }.add(value)
        }
        return result
    }

    /**
     * Try to parse a value as numeric for JSON representation.
     * Returns JSON-encoded value (number or quoted string).
     */
    private fun tryNumeric(value: String): String {
        value.toIntOrNull()?.let { return it.toString() }
        value.toDoubleOrNull()?.let { return it.toString() }
        return "\"${value.replace("\\", "\\\\").replace("\"", "\\\"")}\""
    }

    /**
     * Extract "event" field from JSON string via simple string parsing.
     * Avoids JSON library dependency (matches C++ pattern).
     */
    private fun extractJsonEventField(json: String): String? {
        val key = "\"event\""
        val idx = json.indexOf(key)
        if (idx < 0) return null
        val colonIdx = json.indexOf(':', idx + key.length)
        if (colonIdx < 0) return null
        val afterColon = json.substring(colonIdx + 1).trimStart()
        if (afterColon.startsWith("\"")) {
            val endQuote = afterColon.indexOf('"', 1)
            if (endQuote > 0) return afterColon.substring(1, endQuote)
        }
        return null
    }
}
