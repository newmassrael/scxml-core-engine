// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once
#include "AotTestBase.h"
#include "core/LogMacros.h"
#include "AotTestRegistry.h"
#include "events/IHttpClient.h"
#include "common/SendHelper.h"
#ifndef __EMSCRIPTEN__
#include "W3CHttpTestServer.h"
#include "events/CppHttplibClient.h"
#else
#include "events/EmscriptenFetchClient.h"
#include <nlohmann/json.hpp>
#endif
#include "static/StaticExecutionEngine.h"
#include "common/TestUtils.h"
#include <chrono>
#include <thread>

namespace SCE::W3C::AotTests {

/**
 * @brief AOT test base class for W3C SCXML C.2 BasicHTTP Event I/O Processor tests
 *
 * Provides HTTP server infrastructure for tests that require actual HTTP POST operations.
 * Unlike SimpleAotTest, this base class:
 * - Starts W3CHttpTestServer on localhost:8080/test
 * - Routes HTTP response events back to the state machine
 * - Runs async event processing loop until final state or timeout
 *
 * W3C SCXML C.2 BasicHTTP Event I/O Processor tests (518, 519, 520) require this infrastructure.
 *
 * Example usage:
 * struct Test520 : public HttpAotTest<Test520, 520> {
 *     static constexpr const char *DESCRIPTION = "BasicHTTP content element";
 *     using SM = SCE::Generated::test520::test520;
 * };
 */
template <typename Derived, int TestNum> class HttpAotTest : public AotTestBase {
public:
    static constexpr int TEST_ID = TestNum;

    bool run() override {
#ifndef __EMSCRIPTEN__
        // W3C SCXML C.2 BasicHTTPEventProcessor: Docker TSAN environment incompatibility
        // TSAN crashes in getaddrinfo("localhost") due to glibc nscd thread safety issues
        // See DOCKER_TSAN_README.md for nscd workaround details
        // Skip HTTP tests to avoid TSAN false positives in DNS resolution
        if (SCE::Test::Utils::isInDockerTsan()) {
            SCE_LOG_WARN("HttpAotTest {}: Skipping W3C SCXML C.2 test in Docker TSAN environment (getaddrinfo DNS "
                     "resolution incompatible with TSAN)",
                     TestNum);
            return true;  // Report as PASS (skip, not fail)
        }

        using SM = typename Derived::SM;
        SM sm;

        // W3C SCXML C.2: Create and start HTTP server
        W3C::W3CHttpTestServer httpServer(8080, "/test");

        if (!httpServer.start()) {
            SCE_LOG_ERROR("HttpAotTest {}: Failed to start HTTP server on port 8080", TestNum);
            return false;
        }

        SCE_LOG_DEBUG("HttpAotTest {}: HTTP server started on localhost:8080/test", TestNum);

        // W3C SCXML C.2: Setup HTTP event callback to route responses to state machine
        // When HTTP server receives POST response, it raises event to state machine
        httpServer.setEventCallback([&sm](const std::string &eventName, const std::string &eventData) {
            SCE_LOG_DEBUG("HttpAotTest {}: HTTP callback received event '{}' with data '{}'", TestNum, eventName,
                      eventData);

            // W3C SCXML C.2: Map event name string to Event enum
            // Each test has different Event enum values, so we iterate to find match
            using Event = typename SM::Event;
            using Policy = typename SM::PolicyType;

            // Find matching Event enum by comparing event names
            Event matchedEvent = Event::NONE;
            bool found = false;

            // Iterate through all possible Event enum values (typically < 10)
            for (int i = 0; i < 256 && !found; ++i) {
                Event candidateEvent = static_cast<Event>(i);
                if (Policy::getEventName(candidateEvent) == eventName) {
                    matchedEvent = candidateEvent;
                    found = true;
                    SCE_LOG_DEBUG("HttpAotTest {}: Mapped '{}' to Event enum value {}", TestNum, eventName, i);
                }
            }

            if (found && matchedEvent != Event::NONE) {
                typename SM::EventWithMetadata eventMeta(matchedEvent, eventData);
                sm.raiseExternal(eventMeta);
            } else {
                SCE_LOG_WARN("HttpAotTest {}: Unknown HTTP event: {}", TestNum, eventName);
            }
        });

        // W3C SCXML C.2: Wire performHttpSend() callback (Kotlin W3CHttpTestBase parity)
        // Generated code calls engine.performHttpSend() which delegates to this callback.
        // Synchronous send ensures server callback (which injects response event into SM)
        // completes before performHttpSend returns, preserving event ordering.
        sm.setHttpSendCallback([](const SCE::Static::HttpSendRequest &request) {
            SCE_LOG_DEBUG("HttpAotTest {}: performHttpSend target='{}' event='{}' content='{}'",
                      TestNum, request.target, request.eventName, request.content);

            // W3C SCXML C.2: Build HTTP POST body (Zero Duplication with SendHelper)
            std::string body;
            std::string contentType;

            if (!request.eventName.empty() || !request.params.empty()) {
                body = ::SCE::SendHelper::buildHttpPostBody(request.eventName, request.params);
                contentType = "application/x-www-form-urlencoded";
            } else if (!request.content.empty()) {
                body = request.content;
                contentType = "text/plain";
            } else {
                contentType = "application/x-www-form-urlencoded";
            }

            // W3C SCXML C.2: Synchronous HTTP POST (Kotlin httpClient.send parity)
            SCE::CppHttplibClient httpClient;
            SCE::HttpClient::Request httpReq;
            httpReq.method = "POST";
            httpReq.url = request.target;
            httpReq.body = body;
            httpReq.contentType = contentType;

            auto future = httpClient.sendRequest(httpReq);
            try {
                future.get();  // Synchronous: server callback fires before this returns
            } catch (const std::exception &e) {
                SCE_LOG_ERROR("HttpAotTest {}: HTTP POST failed: {}", TestNum, e.what());
            }
        });

        // Initialize state machine
        sm.initialize();
        SCE_LOG_DEBUG("HttpAotTest {}: State machine initialized, starting async event loop", TestNum);

        // W3C SCXML C.2: Async event processing loop
        // HTTP responses come back asynchronously, so we need to poll until final state
        auto startTime = std::chrono::steady_clock::now();
        constexpr auto timeout = std::chrono::seconds(5);

        while (!sm.isInFinalState()) {
            auto elapsed = std::chrono::steady_clock::now() - startTime;
            if (elapsed > timeout) {
                SCE_LOG_ERROR("HttpAotTest {}: Timeout waiting for final state (elapsed: {}ms)", TestNum,
                          std::chrono::duration_cast<std::chrono::milliseconds>(elapsed).count());
                httpServer.stop();
                return false;
            }

            // Process any pending events
            sm.tick();

            // Small sleep to avoid busy-waiting
            std::this_thread::sleep_for(std::chrono::milliseconds(10));
        }

        // Stop HTTP server
        httpServer.stop();
        SCE_LOG_DEBUG("HttpAotTest {}: HTTP server stopped", TestNum);

        // Check if final state is Pass
        bool isPass = sm.getCurrentState() == SM::State::Pass;
        SCE_LOG_DEBUG("HttpAotTest {}: Final state={}, isPass={}", TestNum, static_cast<int>(sm.getCurrentState()), isPass);

        return isPass;
#else
        // W3C SCXML C.2 BasicHTTPEventProcessor: WASM platform
        // External HTTP server started by polyfill_pre.js (standalone_http_server.js)
        // No per-test server instantiation needed - global server handles all HTTP tests

        using SM = typename Derived::SM;
        SM sm;

        // W3C SCXML C.2: Wire performHttpSend() callback for WASM (native parity)
        // Generated code calls engine.performHttpSend() which delegates to this callback.
        // EmscriptenFetchClient sends HTTP POST to external server (standalone_http_server.js),
        // then parses JSON response to route events back into the state machine.
        sm.setHttpSendCallback([&sm](const SCE::Static::HttpSendRequest &request) {
            SCE_LOG_DEBUG("HttpAotTest {}: WASM performHttpSend target='{}' event='{}' content='{}'",
                      TestNum, request.target, request.eventName, request.content);

            // W3C SCXML C.2: Build HTTP POST body (Zero Duplication with SendHelper)
            std::string body;
            std::string contentType;

            if (!request.eventName.empty() || !request.params.empty()) {
                body = ::SCE::SendHelper::buildHttpPostBody(request.eventName, request.params);
                contentType = "application/x-www-form-urlencoded";
            } else if (!request.content.empty()) {
                body = request.content;
                contentType = "text/plain";
            } else {
                contentType = "application/x-www-form-urlencoded";
            }

            // W3C SCXML C.2: Send via EmscriptenFetchClient to external server
            SCE::EmscriptenFetchClient httpClient;
            SCE::HttpClient::Request httpReq;
            httpReq.method = "POST";
            httpReq.url = request.target;
            httpReq.body = body;
            httpReq.contentType = contentType;

            auto future = httpClient.sendRequest(httpReq);
            try {
                auto response = future.get();

                // W3C SCXML C.2: Parse JSON response and route event back to state machine
                if (response.success && !response.body.empty() && response.body.front() == '{') {
                    using json = nlohmann::json;
                    json responseObj = json::parse(response.body);

                    if (responseObj.contains("event")) {
                        std::string eventName = responseObj["event"].template get<std::string>();
                        std::string eventData;
                        if (responseObj.contains("data")) {
                            eventData = responseObj["data"].dump();
                        }
                        sm.raiseExternal(eventName, eventData);
                    }
                } else if (!response.success) {
                    SCE_LOG_ERROR("HttpAotTest {}: WASM HTTP POST failed (status {})",
                              TestNum, response.statusCode);
                }
            } catch (const std::exception &e) {
                SCE_LOG_ERROR("HttpAotTest {}: WASM HTTP POST exception: {}", TestNum, e.what());
            }
        });

        // W3C SCXML C.2: Initialize state machine
        // HTTP POST will be sent during initialize() via performHttpSend callback
        sm.initialize();
        SCE_LOG_DEBUG("HttpAotTest {}: WASM state machine initialized", TestNum);

        // W3C SCXML C.2: Async event processing loop
        // HTTP responses come back asynchronously via EmscriptenFetchClient + external server
        auto startTime = std::chrono::steady_clock::now();
        constexpr auto timeout = std::chrono::seconds(5);

        while (!sm.isInFinalState()) {
            auto elapsed = std::chrono::steady_clock::now() - startTime;
            if (elapsed > timeout) {
                SCE_LOG_ERROR("HttpAotTest {}: WASM timeout waiting for final state (elapsed: {}ms)", TestNum,
                          std::chrono::duration_cast<std::chrono::milliseconds>(elapsed).count());
                SCE_LOG_ERROR("HttpAotTest {}: Make sure external HTTP server is running (started by polyfill_pre.js)",
                          TestNum);
                return false;
            }

            // Process any pending events
            sm.tick();

            // Small sleep to avoid busy-waiting
            std::this_thread::sleep_for(std::chrono::milliseconds(10));
        }

        // Check if final state is Pass
        bool isPass = sm.getCurrentState() == SM::State::Pass;
        SCE_LOG_DEBUG("HttpAotTest {}: WASM final state={}, isPass={}", TestNum, static_cast<int>(sm.getCurrentState()),
                  isPass);

        return isPass;
#endif
    }

    int getTestId() const override {
        return TEST_ID;
    }

    const char *getDescription() const override {
        // Lazy load description from metadata.txt (Single Source of Truth)
        // Cached to avoid repeated file I/O
        if (cachedDescription_.empty()) {
            cachedDescription_ = AotTestBase::loadMetadataDescription(TEST_ID);
        }
        return cachedDescription_.c_str();
    }

private:
    mutable std::string cachedDescription_;
};

}  // namespace SCE::W3C::AotTests
