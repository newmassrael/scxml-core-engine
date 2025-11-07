// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-RSM-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include <chrono>
#include <future>
#include <map>
#include <memory>
#include <string>

namespace RSM {

// Forward declare to avoid conflict with IEventBridge's HttpRequest/HttpResponse
// IEventBridge: Server-side (incoming requests, outgoing responses)
// HttpClient: Client-side (outgoing requests, incoming responses)
namespace HttpClient {

/**
 * @brief HTTP client request data
 *
 * Platform-agnostic representation for outgoing HTTP requests.
 * Used by both Native (cpp-httplib) and WASM (Emscripten Fetch API).
 *
 * ARCHITECTURE.md: Zero Duplication Principle
 * - Separate namespace from IEventBridge to avoid name collision
 * - IEventBridge::HttpRequest: Server-side incoming HTTP requests
 * - HttpClient::Request: Client-side outgoing HTTP requests
 */
struct Request {
    std::string method;                          // "POST", "GET", "PUT", "DELETE"
    std::string url;                             // Full URL: "http://example.com:8080/api/test"
    std::string body;                            // Request payload
    std::string contentType;                     // "application/json", "application/x-www-form-urlencoded", etc.
    std::map<std::string, std::string> headers;  // Custom HTTP headers
};

/**
 * @brief HTTP client response data
 *
 * Platform-agnostic representation for incoming HTTP responses.
 * Returned by both CppHttplibClient and EmscriptenFetchClient.
 *
 * ARCHITECTURE.md: Zero Duplication Principle
 * - Separate namespace from IEventBridge to avoid name collision
 * - IEventBridge::HttpResponse: Server-side outgoing HTTP responses
 * - HttpClient::Response: Client-side incoming HTTP responses
 */
struct Response {
    bool success;                                // true if HTTP 200-299 and no network error
    int statusCode;                              // HTTP status code (0 if network error)
    std::string body;                            // Response payload
    std::map<std::string, std::string> headers;  // Response headers
};

}  // namespace HttpClient

/**
 * @brief Platform-agnostic HTTP client interface
 *
 * ARCHITECTURE.md Compliance:
 * - Zero Duplication: Single interface for Native and WASM HTTP client operations
 * - Platform Abstraction: Native (cpp-httplib) vs WASM (Emscripten Fetch API)
 * - All-or-Nothing: Pure Native or Pure WASM, no mixing
 *
 * W3C SCXML C.2 BasicHTTP Event I/O Processor:
 * - Sends HTTP POST requests to external servers
 * - Receives HTTP responses and converts to SCXML events
 * - Client-only functionality (no server in SCXML engine)
 *
 * Platform Implementations:
 * - Native: CppHttplibClient using cpp-httplib::Client
 * - WASM: EmscriptenFetchClient using Emscripten Fetch API
 */
class IHttpClient {
public:
    virtual ~IHttpClient() = default;

    /**
     * @brief Send HTTP request asynchronously
     *
     * @param request HTTP request data
     * @return Future with HTTP response
     *
     * Thread Safety:
     * - Native: Safe from any thread (std::async creates worker thread)
     * - WASM: Must be called from main thread (browser restriction)
     */
    virtual std::future<HttpClient::Response> sendRequest(const HttpClient::Request &request) = 0;

    /**
     * @brief Set request timeout
     *
     * @param timeout Maximum wait time for HTTP response
     */
    virtual void setTimeout(std::chrono::milliseconds timeout) = 0;
};

/**
 * @brief Factory function for platform-specific HTTP client
 *
 * Compile-time selection:
 * - __EMSCRIPTEN__ defined: Returns EmscriptenFetchClient
 * - __EMSCRIPTEN__ not defined: Returns CppHttplibClient
 *
 * @return Platform-specific HTTP client instance
 */
std::unique_ptr<IHttpClient> createHttpClient();

}  // namespace RSM
