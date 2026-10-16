// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package com.sce.hostfeatures

/** The most bytes of response headers and of body a result record holds. */
private const val HEADERS_BOUND = 4096
private const val BODY_BOUND = 16384

private val METHODS = listOf("GET", "HEAD", "POST", "PUT", "PATCH", "DELETE", "OPTIONS")

/** One header field: its name and its value. */
data class HttpHeader(val name: String, val value: String)

/** What the host hands its transport: the request, its header lines already split into name and value. */
data class HttpRequest(val method: String, val url: String, val headers: List<HttpHeader>, val body: String)

/** What a transport answers: the response as it came. */
class HttpResponse(val status: Int, val headers: List<HttpHeader>, val body: ByteArray)

/** A transport that got no response: name resolution, connection, TLS, or the connection ended. */
class TransportFailure(message: String) : Exception(message)

/**
 * Carries one exchange. A response is the answer; [TransportFailure] is a transport that got
 * none; `null` is a transport that has not answered, and the host holds the exchange until it
 * is cancelled.
 */
interface Transport {
    fun exchange(request: HttpRequest): HttpResponse?
}

/** An RFC 9110 token: what a header name is made of. */
private fun isToken(name: String): Boolean =
    name.isNotEmpty() &&
        name.all { c ->
            (c in 'a'..'z') || (c in 'A'..'Z') || (c in '0'..'9') || "!#$%&'*+-.^_`|~".indexOf(c) >= 0
        }

/**
 * The header lines of a request, split into name and value, or `null` for a line that is not
 * one. Empty lines are ignored, and the value is without the spaces and tabs around it.
 */
private fun headerLines(text: String): List<HttpHeader>? {
    val headers = mutableListOf<HttpHeader>()
    for (raw in text.split('\n')) {
        val line = raw.removeSuffix("\r")
        if (line.isEmpty()) continue
        val colon = line.indexOf(':')
        if (colon < 0) return null
        val name = line.substring(0, colon)
        if (!isToken(name)) return null
        headers.add(HttpHeader(name, line.substring(colon + 1).trim { it == ' ' || it == '\t' }))
    }
    return headers
}

/** An absolute `http` or `https` URL with a host, and no ASCII space or control character (U+0000 to U+0020 and U+007F) in it. */
private fun isHttpUrl(url: String): Boolean {
    val rest = when {
        url.startsWith("http://") -> url.removePrefix("http://")
        url.startsWith("https://") -> url.removePrefix("https://")
        else -> return false
    }
    val end = rest.indexOfFirst { it == '/' || it == '?' || it == '#' }
    val authority = if (end >= 0) rest.substring(0, end) else rest
    if (authority.isEmpty() || authority.startsWith(":")) return false
    return url.none { it.code <= 0x20 || it.code == 0x7F }
}

/** Whether the arguments of an exchange are ones the operation accepts. */
internal fun acceptable(method: String, url: String, headers: String): Boolean =
    method in METHODS && isHttpUrl(url) && headerLines(headers) != null

internal fun ReferenceHost.httpFetch(id: String, method: String, url: String, headers: String, body: String): Outcome {
    val response = try {
        transport.exchange(HttpRequest(method, url, headerLines(headers) ?: emptyList(), body))
    } catch (_: TransportFailure) {
        return failure("sce-host-http-fetch", "network")
    }
    if (response == null) {
        held.add(id)
        return Outcome.Hold
    }
    val joined = response.headers.joinToString("\n") { "${it.name.lowercase()}: ${it.value.trim()}" }
    // A response to HEAD has no body, whatever the transport carried.
    val payload = if (method == "HEAD") ByteArray(0) else response.body
    if (joined.encodeToByteArray().size > HEADERS_BOUND || payload.size > BODY_BOUND) {
        return failure("sce-host-http-fetch", "response_too_large")
    }
    val text = try {
        payload.decodeToString(throwOnInvalidSequence = true)
    } catch (_: Exception) {
        return failure("sce-host-http-fetch", "invalid_body")
    }
    return record("status" to response.status, "headers" to joined, "body" to text)
}
