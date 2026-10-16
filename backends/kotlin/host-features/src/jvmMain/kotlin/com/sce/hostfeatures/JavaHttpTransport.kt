// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package com.sce.hostfeatures

import java.io.IOException
import java.net.URI
import java.net.http.HttpClient
import java.net.http.HttpRequest as JavaRequest
import java.net.http.HttpResponse as JavaResponse
import java.time.Duration

/**
 * An exchange over the JDK's HTTP client. It follows no redirect and keeps no cookie.
 *
 * The client keeps a response's fields in a map, so the order they were received in is lost:
 * they are given sorted by name, which is the order a transport that cannot give the received
 * one gives. A request header the client reserves (`host`, `content-length`, `connection`,
 * `expect`, `upgrade`) cannot be carried, and a request that names one fails as `network`.
 */
class JavaHttpTransport(private val timeout: Duration = Duration.ofSeconds(30)) : Transport {
    private val client: HttpClient = HttpClient.newBuilder()
        .followRedirects(HttpClient.Redirect.NEVER)
        .connectTimeout(timeout)
        .build()

    override fun exchange(request: HttpRequest): HttpResponse {
        try {
            val publisher = if (request.body.isEmpty()) {
                JavaRequest.BodyPublishers.noBody()
            } else {
                JavaRequest.BodyPublishers.ofString(request.body)
            }
            val builder = JavaRequest.newBuilder(URI.create(request.url)).timeout(timeout)
                .method(request.method, publisher)
            for (header in request.headers) builder.header(header.name, header.value)
            val response = client.send(builder.build(), JavaResponse.BodyHandlers.ofByteArray())
            val headers = response.headers().map().entries
                .sortedBy { it.key.lowercase() }
                .flatMap { (name, values) -> values.map { HttpHeader(name, it) } }
            return HttpResponse(response.statusCode(), headers, response.body())
        } catch (error: IOException) {
            throw TransportFailure(error.message ?: "no response")
        } catch (error: IllegalArgumentException) {
            throw TransportFailure(error.message ?: "the request cannot be carried")
        } catch (error: InterruptedException) {
            Thread.currentThread().interrupt()
            throw TransportFailure("interrupted")
        }
    }
}
