// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-RSM-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#ifdef __EMSCRIPTEN__

#include "events/EmscriptenFetchClient.h"
#include "common/Logger.h"
#include <cstring>
#include <emscripten.h>
#include <emscripten/fetch.h>
#include <sstream>
#include <vector>

// EM_JS functions for Node.js HTTP support (W3C SCXML C.2 BasicHTTP via Node.js)

// Runtime environment detection: returns 1 if Node.js, 0 if browser
EM_JS(int, em_is_nodejs, (), {
    return (typeof process != = 'undefined' && typeof process.versions !=
            = 'undefined' && typeof process.versions.node != = 'undefined')
               ? 1
               : 0;
});

// W3C SCXML C.2: Node.js HTTP request via EM_ASYNC_JS (returns Promise)
// ASYNCIFY automatically pauses/resumes execution to wait for Promise resolution
// MEMORY LEAK FIX: Use stack-based stringToUTF8 instead of heap-based allocateUTF8
EM_ASYNC_JS(int, em_nodejs_http_request_async,
            (const char *method, const char *url, const char *headers_json, const char *body, int body_length,
             char *out_body_buffer, int buffer_size, int *out_status),
            {
                const urlStr = UTF8ToString(url);
                const methodStr = UTF8ToString(method);
                const headersJson = UTF8ToString(headers_json);

                console.log("[DEBUG] EM_ASYNC_JS: Starting Node.js HTTP request:", methodStr, urlStr);

                let headers = {};
                try {
                    headers = JSON.parse(headersJson);
                } catch (e) {
                    console.error("[ERROR] Failed to parse headers JSON:", e);
                }

                const isHttps = urlStr.startsWith("https://");
                const http = require(isHttps ? "https" : "http");
                const urlModule = require("url");
                const parsedUrl = new urlModule.URL(urlStr);

                const options = {
                    method : methodStr,
                    hostname : parsedUrl.hostname,
                    port : parsedUrl.port || (isHttps ? 443 : 80),
                    path : parsedUrl.pathname + parsedUrl.search,
                    headers : headers
                };

                // Return Promise for HTTP request - EM_ASYNC_JS automatically awaits it
                const result = await new Promise((resolve, reject) = > {
                    const req = http.request(
                        options, (res) = > {
                            let responseBody = "";

                            res.setEncoding("utf8");
                            res.on("data", (chunk) = > { responseBody += chunk; });

                            res.on(
                                "end", () = > {
                                    console.log("[DEBUG] EM_ASYNC_JS: HTTP response:", res.statusCode,
                                                "body length:", responseBody.length);

                                    // MEMORY LEAK FIX: Clean up event listeners to prevent memory accumulation
                                    res.removeAllListeners();
                                    req.removeAllListeners();
                                    req.destroy();  // Close socket connection

                                    resolve({statusCode : res.statusCode, body : responseBody});
                                });
                        });

                    req.on(
                        "error", (error) = > {
                            console.error("[ERROR] EM_ASYNC_JS: HTTP request failed:", error.message);

                            // MEMORY LEAK FIX: Clean up on error
                            req.removeAllListeners();
                            req.destroy();

                            reject(error);
                        });

                    if (body_length > 0) {
                        const bodyData = HEAPU8.subarray(body, body + body_length);
                        req.write(Buffer.from(bodyData));
                    }

                    req.end();
                });

                // MEMORY LEAK FIX: Use pre-allocated buffer (no heap allocation)
                // Check buffer size to prevent overflow
                const bodyBytes = lengthBytesUTF8(result.body);
                if (bodyBytes + 1 > buffer_size) {
                    console.error("[ERROR] EM_ASYNC_JS: Response body too large:", bodyBytes,
                                  "bytes, buffer:", buffer_size, "bytes");
                    setValue(out_status, 500, 'i32');
                    return 0;  // Buffer overflow - return error
                }

                // Copy response to pre-allocated buffer (stack-based, no malloc)
                stringToUTF8(result.body, out_body_buffer, buffer_size);
                setValue(out_status, result.statusCode, 'i32');

                return 1;  // Success
            });

namespace RSM {

EmscriptenFetchClient::EmscriptenFetchClient() {
    LOG_DEBUG("EmscriptenFetchClient: Created WASM HTTP client");
}

std::future<HttpClient::Response> EmscriptenFetchClient::sendRequest(const HttpClient::Request &request) {
    auto promise = std::make_shared<std::promise<HttpClient::Response>>();
    auto future = promise->get_future();

    LOG_DEBUG("EmscriptenFetchClient: {} {} (timeout={}ms)", request.method, request.url, timeout_.count());

    // Detect runtime environment: Node.js vs browser
    bool isNodeJs = em_is_nodejs();

    if (isNodeJs) {
        // W3C SCXML C.2: Use Node.js HTTP module via EM_ASYNC_JS
        // ASYNCIFY automatically pauses execution until Promise resolves
        LOG_DEBUG("EmscriptenFetchClient: Using Node.js http module (EM_ASYNC_JS)");

        // Build headers JSON
        std::ostringstream headersJson;
        headersJson << "{";
        bool first = true;

        if (!request.contentType.empty()) {
            headersJson << "\"Content-Type\":\"" << request.contentType << "\"";
            first = false;
        }

        for (const auto &[key, value] : request.headers) {
            if (!first) {
                headersJson << ",";
            }
            headersJson << "\"" << key << "\":\"" << value << "\"";
            first = false;
        }

        headersJson << "}";

        // W3C SCXML C.2: Call EM_ASYNC_JS - appears synchronous but yields to event loop via ASYNCIFY
        // MEMORY LEAK FIX: Reuse static buffer to eliminate heap fragmentation
        // W3C test responses are typically small JSON (< 1KB), 4KB is sufficient
        // WASM: Single-threaded, safe to reuse static buffer across requests
        constexpr int MAX_RESPONSE_SIZE = 4096;  // 4KB buffer for HTTP response body
        static std::vector<char> responseBuffer(MAX_RESPONSE_SIZE);
        int statusCode = 0;

        LOG_DEBUG("EmscriptenFetchClient: Calling EM_ASYNC_JS (will pause until HTTP response)");

        int result = em_nodejs_http_request_async(request.method.c_str(), request.url.c_str(),
                                                  headersJson.str().c_str(), request.body.c_str(), request.body.size(),
                                                  responseBuffer.data(), responseBuffer.size(), &statusCode);

        LOG_DEBUG("EmscriptenFetchClient: EM_ASYNC_JS returned (HTTP complete)");

        // Build response from EM_ASYNC_JS output
        HttpClient::Response response;

        if (result == 0) {
            // Buffer overflow error (response too large)
            LOG_ERROR("EmscriptenFetchClient: Response buffer overflow (response > 4KB)");
            response.success = false;
            response.statusCode = 500;
        } else {
            response.success = (statusCode >= 200 && statusCode < 300);
            response.statusCode = statusCode;
            response.body = std::string(responseBuffer.data());  // Copy from stack buffer to std::string
        }

        LOG_DEBUG("EmscriptenFetchClient (Node.js): {} {} → {} (body {} bytes)", request.method, request.url,
                  response.statusCode, response.body.size());

        promise->set_value(std::move(response));
        return future;
    }

    // Browser environment: use emscripten_fetch (callback-based, requires context)
    LOG_DEBUG("EmscriptenFetchClient: Using browser Fetch API");

    // W3C SCXML C.2: Browser-only context for emscripten_fetch callbacks
    struct BrowserFetchContext {
        std::shared_ptr<std::promise<HttpClient::Response>> promise;
        std::string requestUrl;
        std::string requestMethod;
        std::string requestBody;
        std::vector<std::string> headerStrings;
        std::vector<const char *> headerPtrs;

        BrowserFetchContext(std::shared_ptr<std::promise<HttpClient::Response>> p, std::string url, std::string method)
            : promise(std::move(p)), requestUrl(std::move(url)), requestMethod(std::move(method)) {}
    };

    auto *context = new BrowserFetchContext(promise, request.url, request.method);

    emscripten_fetch_attr_t attr;
    emscripten_fetch_attr_init(&attr);

    strncpy(attr.requestMethod, request.method.c_str(), sizeof(attr.requestMethod) - 1);
    attr.requestMethod[sizeof(attr.requestMethod) - 1] = '\0';

    // W3C SCXML C.2: Store request body in context to keep alive
    context->requestBody = request.body;
    attr.requestData = context->requestBody.c_str();
    attr.requestDataSize = context->requestBody.size();

    // Build headers array - store in context to keep alive
    if (!request.contentType.empty()) {
        context->headerStrings.push_back("Content-Type");
        context->headerStrings.push_back(request.contentType);
    }

    for (const auto &[key, value] : request.headers) {
        context->headerStrings.push_back(key);
        context->headerStrings.push_back(value);
    }

    // Build pointer array from stored strings
    for (const auto &str : context->headerStrings) {
        context->headerPtrs.push_back(str.c_str());
    }
    context->headerPtrs.push_back(nullptr);

    attr.requestHeaders = context->headerPtrs.data();

    // W3C SCXML C.2: Use synchronous mode for Node.js compatibility with ASYNCIFY
    attr.attributes = EMSCRIPTEN_FETCH_LOAD_TO_MEMORY | EMSCRIPTEN_FETCH_SYNCHRONOUS;
    attr.timeoutMSecs = static_cast<unsigned long>(timeout_.count());

    // Success callback
    attr.onsuccess = [](emscripten_fetch_t *fetch) {
        // W3C SCXML C.2: Reconstruct context type for callback
        struct BrowserFetchContext {
            std::shared_ptr<std::promise<HttpClient::Response>> promise;
            std::string requestUrl;
            std::string requestMethod;
            std::string requestBody;
            std::vector<std::string> headerStrings;
            std::vector<const char *> headerPtrs;
        };
        auto *ctx = static_cast<BrowserFetchContext *>(fetch->userData);

        HttpClient::Response response;
        response.success = true;
        response.statusCode = fetch->status;

        if (fetch->numBytes > 0) {
            response.body = std::string(fetch->data, fetch->numBytes);
        }

        // Parse response headers
        size_t headersLength = emscripten_fetch_get_response_headers_length(fetch);
        if (headersLength > 0) {
            std::vector<char> headersBuffer(headersLength + 1);
            emscripten_fetch_get_response_headers(fetch, headersBuffer.data(), headersLength + 1);

            std::string headersStr(headersBuffer.data());
            std::istringstream stream(headersStr);
            std::string line;

            while (std::getline(stream, line)) {
                if (line.empty() || line == "\r") {
                    continue;
                }

                size_t colonPos = line.find(':');
                if (colonPos != std::string::npos) {
                    std::string key = line.substr(0, colonPos);
                    std::string value = line.substr(colonPos + 1);

                    value.erase(0, value.find_first_not_of(" \t\r"));
                    value.erase(value.find_last_not_of(" \t\r") + 1);

                    response.headers[key] = value;
                }
            }
        }

        LOG_DEBUG("EmscriptenFetchClient: {} {} → {} (body {} bytes)", ctx->requestMethod, ctx->requestUrl,
                  response.statusCode, response.body.size());

        ctx->promise->set_value(std::move(response));
        emscripten_fetch_close(fetch);
        delete ctx;
    };

    // Error callback
    attr.onerror = [](emscripten_fetch_t *fetch) {
        struct BrowserFetchContext {
            std::shared_ptr<std::promise<HttpClient::Response>> promise;
            std::string requestUrl;
            std::string requestMethod;
            std::string requestBody;
            std::vector<std::string> headerStrings;
            std::vector<const char *> headerPtrs;
        };
        auto *ctx = static_cast<BrowserFetchContext *>(fetch->userData);

        HttpClient::Response response;
        response.success = false;
        response.statusCode = fetch->status;

        LOG_ERROR("EmscriptenFetchClient: {} {} → FAILED (status {})", ctx->requestMethod, ctx->requestUrl,
                  fetch->status);

        ctx->promise->set_value(std::move(response));
        emscripten_fetch_close(fetch);
        delete ctx;
    };

    attr.userData = context;

    // W3C SCXML C.2: Synchronous fetch blocks until complete (with ASYNCIFY support)
    emscripten_fetch_t *fetch = emscripten_fetch(&attr, request.url.c_str());

    // Check if fetch completed successfully
    if (fetch) {
        // Manually trigger the appropriate callback based on status
        if (fetch->status >= 200 && fetch->status < 300) {
            attr.onsuccess(fetch);
        } else {
            attr.onerror(fetch);
        }
    } else {
        // Fetch failed immediately
        HttpClient::Response response;
        response.success = false;
        response.statusCode = 0;
        LOG_ERROR("EmscriptenFetchClient: {} {} → FAILED (fetch returned null)", request.method, request.url);
        context->promise->set_value(std::move(response));
        delete context;
    }

    return future;
}

void EmscriptenFetchClient::setTimeout(std::chrono::milliseconds timeout) {
    timeout_ = timeout;
    LOG_DEBUG("EmscriptenFetchClient: Set timeout to {}ms", timeout.count());
}

}  // namespace RSM

#endif  // __EMSCRIPTEN__
