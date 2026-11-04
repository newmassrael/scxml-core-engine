// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once
#include "AotTestBase.h"
#include "AotTestRegistry.h"
#include "BasicHttpTestEndpoint.h"
#include "common/SendHelper.h"
#include "core/LogMacros.h"
#include "events/IHttpClient.h"
#ifndef __EMSCRIPTEN__
#include "W3CHttpTestServer.h"
#include "events/CppHttplibClient.h"
#else
#include "events/EmscriptenFetchClient.h"
#include <nlohmann/json.hpp>
#endif
#include "common/TestUtils.h"
#include "scripting/ScriptEngineProvider.h"
#include "static/StaticExecutionEngine.h"
#include <chrono>
#include <memory>
#include <thread>

namespace SCE::W3C::AotTests {

/**
 * @brief AOT test base class for W3C SCXML C.2 BasicHTTP Event I/O Processor tests
 *
 * Runs a generated machine whose sends POST to the BasicHTTP fixture endpoint,
 * routes the event the listener makes of each POST back to the machine, and
 * drives the machine until a final state or a timeout.
 *
 * One run on every platform; only the listener and the client differ:
 * - natively the test starts a W3CHttpTestServer in process, which hands each
 *   event to a callback on its own thread, and posts with CppHttplibClient;
 * - under WASM the host starts the listener before main() (w3c/wasm_polyfill.js
 *   runs w3c/standalone_http_server.js), which answers each POST with the event
 *   it made, and the test posts with EmscriptenFetchClient.
 * The two were separate runs once, and the WASM one fell behind: it never set
 * the script engine an ECMAScript machine needs, and the machine's destructor
 * then ran into a session it was never given.
 *
 * Example usage:
 * struct Test520 : public HttpAotTest<Test520, 520> {
 *     using SM = SCE::Generated::test520::test520;
 * };
 */
template <typename Derived, int TestNum> class HttpAotTest : public AotTestBase {
public:
    static constexpr int TEST_ID = TestNum;

    bool run() override {
#ifndef __EMSCRIPTEN__
        // W3C SCXML C.2 BasicHTTPEventProcessor: ThreadSanitizer build incompatibility
        // TSAN crashes in getaddrinfo("localhost") due to glibc nscd thread safety issues
        // See DOCKER_TSAN_README.md for nscd workaround details
        // Skip HTTP tests to avoid TSAN false positives in DNS resolution
        if (SCE::Test::Utils::isThreadSanitizerBuild()) {
            SCE_LOG_WARN("HttpAotTest {}: Skipping W3C SCXML C.2 test under ThreadSanitizer (getaddrinfo DNS "
                         "resolution incompatible with TSAN)",
                         TestNum);
            return true;  // Report as PASS (skip, not fail)
        }
#endif

        using SM = typename Derived::SM;
        using Event = typename SM::Event;
        using Policy = typename SM::PolicyType;
        SM sm;
        // W3C SCXML B.1: Inject script engine handle (Path B+ Q1=(d) C++=shared_ptr).
        if constexpr (SM::PolicyType::NEEDS_SCRIPT_ENGINE) {
            sm.setScriptEngine(::std::shared_ptr<::SCE::IScriptEngine>(&::SCE::ScriptEngineProvider::getScriptEngine(),
                                                                       [](::SCE::IScriptEngine *) {}));
        }

        // W3C SCXML C.2: the event the listener made of a POST, by name, as
        // the machine's own event, posted to the machine's inbox as a host
        // posts what arrives for a machine it does not drive on that thread
        // (ARCHITECTURE.md "Engine Threading"). Natively the listener's thread
        // posts it; under WASM the send's answer carries it, and the owner
        // posts it inside the send. One rule for both: the post is applied
        // before the machine puts an event of its own on its external queue
        // (docs/adr/0084, D2), and the POST returns only after the listener has
        // made the event, so the event joins the queue ahead of anything the
        // document queues after the send. W3C test 201 sends `timeout` to
        // itself right after the POST and must see `event1` first. The handle
        // is a copy, safe on any thread and valid for as long as the listener
        // runs.
        const auto inbox = sm.inbox();
        const auto deliver = [inbox](const std::string &eventName, const std::string &eventData) {
            SCE_LOG_DEBUG("HttpAotTest {}: the listener made event '{}' with data '{}'", TestNum, eventName, eventData);
            for (int i = 0; i < 256; ++i) {
                const auto candidate = static_cast<Event>(i);
                if (candidate != Event::NONE && Policy::getEventName(candidate) == eventName) {
                    if (!inbox.post(
                            typename SM::InboxExternalEvent{typename SM::EventWithMetadata(candidate, eventData)})) {
                        SCE_LOG_ERROR("HttpAotTest {}: the machine's inbox refused event '{}'", TestNum, eventName);
                    }
                    return;
                }
            }
            SCE_LOG_WARN("HttpAotTest {}: Unknown HTTP event: {}", TestNum, eventName);
        };

#ifndef __EMSCRIPTEN__
        // The listener, in process: it answers on a thread of its own and
        // hands each event over before the POST that made it returns.
        W3C::W3CHttpTestServer httpServer(W3C::basicHttpTestPort(), W3C::basicHttpTestPath());
        if (!httpServer.start()) {
            SCE_LOG_ERROR("HttpAotTest {}: Failed to start HTTP server on port {}", TestNum, W3C::basicHttpTestPort());
            return false;
        }
        httpServer.setEventCallback(deliver);
        using HttpClientType = SCE::CppHttplibClient;
#else
        // The listener, the host's: started before main(), at the same port
        // and path, it answers each POST with the event it made.
        using HttpClientType = SCE::EmscriptenFetchClient;
#endif
        SCE_LOG_DEBUG("HttpAotTest {}: listening at {}", TestNum, W3C::basicHttpTestAccessUri());

        // §scxml-C-2-3: the test owns the listener, so it declares the access
        // URI the machine publishes. The converted documents address their
        // sends through that published entry.
        sm.setBasicHttpAccessUri(W3C::basicHttpTestAccessUri());

        // W3C SCXML C.2: Wire performHttpSend() callback (Kotlin W3CHttpTestBase parity)
        // Generated code calls engine.performHttpSend() which delegates to this callback.
        sm.setHttpSendCallback([&deliver](const SCE::Static::HttpSendRequest &request) {
            SCE_LOG_DEBUG("HttpAotTest {}: performHttpSend target='{}' event='{}' content='{}'", TestNum,
                          request.target, request.eventName, request.content);

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
            HttpClientType httpClient;
            SCE::HttpClient::Request httpReq;
            httpReq.method = "POST";
            httpReq.url = request.target;
            httpReq.body = body;
            httpReq.contentType = contentType;

            auto future = httpClient.sendRequest(httpReq);
            try {
                const auto response = future.get();
                if (!response.success) {
                    SCE_LOG_ERROR("HttpAotTest {}: HTTP POST failed (status {})", TestNum, response.statusCode);
                }
#ifdef __EMSCRIPTEN__
                // The host's listener answers with the event it made.
                if (response.success && !response.body.empty() && response.body.front() == '{') {
                    const auto answer = nlohmann::json::parse(response.body);
                    if (answer.contains("event")) {
                        deliver(answer["event"].template get<std::string>(),
                                answer.contains("data") ? answer["data"].dump() : std::string());
                    }
                }
#else
                (void)deliver;
#endif
            } catch (const std::exception &e) {
                SCE_LOG_ERROR("HttpAotTest {}: HTTP POST failed: {}", TestNum, e.what());
            }
        });

        sm.initialize();
        SCE_LOG_DEBUG("HttpAotTest {}: State machine initialized, starting event loop", TestNum);

        // W3C SCXML C.2: drive the machine until a final state or the timeout.
        auto startTime = std::chrono::steady_clock::now();
        constexpr auto timeout = std::chrono::seconds(5);
        bool timedOut = false;

        while (!sm.isInFinalState()) {
            auto elapsed = std::chrono::steady_clock::now() - startTime;
            if (elapsed > timeout) {
                SCE_LOG_ERROR("HttpAotTest {}: Timeout waiting for final state (elapsed: {}ms)", TestNum,
                              std::chrono::duration_cast<std::chrono::milliseconds>(elapsed).count());
                timedOut = true;
                break;
            }

            // Process any pending events; what the listener posted since the
            // last tick is applied at the macrostep boundary
            sm.tick();

            // Small sleep to avoid busy-waiting
            std::this_thread::sleep_for(std::chrono::milliseconds(10));
        }

#ifndef __EMSCRIPTEN__
        httpServer.stop();
        SCE_LOG_DEBUG("HttpAotTest {}: HTTP server stopped", TestNum);
#endif
        if (timedOut) {
            return false;
        }

        // Check the run ENDED in Pass (§scxml-D-exitInterpreter empties the configuration)
        bool isPass = sm.terminalState() == SM::State::Pass;
        SCE_LOG_DEBUG("HttpAotTest {}: Final state={}, isPass={}", TestNum, static_cast<int>(sm.getCurrentState()),
                      isPass);

        return isPass;
    }

    int getTestId() const override {
        return TEST_ID;
    }
};

}  // namespace SCE::W3C::AotTests
