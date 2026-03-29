// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// This file is part of SCE (SCXML Core Engine).
//
// Dual Licensed:
// 1. LGPL-2.1: Free for unmodified use (see LICENSE-LGPL-2.1.md)
// 2. Commercial: For modifications (contact newmassrael@gmail.com)
//
// Commercial License:
//   Individual: $100 cumulative
//   Enterprise: $500 cumulative
//   Contact: https://github.com/newmassrael
//
// Full terms: https://github.com/newmassrael/scxml-core-engine/blob/main/LICENSE

#pragma once

#include "core/LogMacros.h"
#include "common/SendHelper.h"
#include "events/EventDescriptor.h"
#ifndef __EMSCRIPTEN__
#include "events/CppHttplibClient.h"
#else
#include "events/EmscriptenFetchClient.h"
#endif
#include <nlohmann/json.hpp>
#include <memory>
#include <string>

namespace SCE::Static {

/**
 * @brief W3C SCXML C.2: BasicHTTP Event I/O Processor send helper for AOT engine
 *
 * Extracts HTTP POST logic from StaticExecutionEngine to keep the engine focused
 * on event queue management. Handles descriptor building, JSON-to-params parsing,
 * request body generation, and platform-specific HTTP client dispatch.
 *
 * ARCHITECTURE.md: Zero Duplication — reuses SendHelper::buildHttpPostBody()
 * for form-encoded body generation, shared with Interpreter's HttpEventTarget.
 */
struct HttpSendHelper {

    /**
     * @brief Check if an event requires HTTP POST delivery
     *
     * @param originType Event originType metadata field
     * @param target Event target URL
     * @return true if this is a BasicHTTP send with a non-empty target
     */
    static bool isHttpSend(const std::string &originType, const std::string &target) {
        return !originType.empty() && !target.empty() &&
               originType.find("BasicHTTPEventProcessor") != std::string::npos;
    }

    /**
     * @brief Execute an HTTP POST send for the AOT engine (W3C SCXML C.2)
     *
     * Builds an EventDescriptor from event metadata, constructs the appropriate
     * HTTP request body (form-encoded, plain text, or JSON), and dispatches via
     * platform-specific HTTP client (CppHttplibClient / EmscriptenFetchClient).
     *
     * @tparam Engine StaticExecutionEngine<StatePolicy> — used for WASM response routing
     * @param eventName Resolved event name string
     * @param data Event data payload
     * @param target HTTP POST target URL
     * @param sendId W3C SCXML sendid
     * @param engine Engine reference for WASM response event routing
     */
    template <typename Engine>
    static void sendHttpPost(const std::string &eventName, const std::string &data, const std::string &target,
                             const std::string &sendId, [[maybe_unused]] Engine &engine) {
        SCE_LOG_DEBUG("AOT HttpSendHelper: Sending HTTP POST (event={}, target={})", eventName, target);

        // W3C SCXML C.2: Build EventDescriptor
        SCE::EventDescriptor descriptor;
        descriptor.eventName = eventName;
        descriptor.target = target;
        descriptor.sendId = sendId;
        descriptor.type = "http";

        // W3C SCXML C.2: For content-only sends (empty event name),
        // the data IS the content to be sent as HTTP POST body
        if (descriptor.eventName.empty()) {
            descriptor.content = data;
        } else {
            descriptor.data = data;

            // W3C SCXML C.2: Parse JSON eventData to params for form-encoded POST (test 519)
            parseJsonToParams(data, descriptor);
        }

        // W3C SCXML C.2: Build HTTP request body and content type
        auto [requestBody, contentType] = buildRequestBody(descriptor);

        // W3C SCXML C.2: Create platform-specific HTTP client and send
#ifndef __EMSCRIPTEN__
        auto httpClient = std::make_unique<SCE::CppHttplibClient>();
#else
        auto httpClient = std::make_unique<SCE::EmscriptenFetchClient>();
#endif

        auto resultFuture = httpClient->sendRequest({.method = "POST",
                                                     .url = target,
                                                     .body = requestBody,
                                                     .contentType = contentType,
                                                     .headers = {}});

        // W3C SCXML C.2: Handle platform-specific response processing
#ifndef __EMSCRIPTEN__
        // Native: Fire and forget — HTTP response handled by W3CHttpTestServer callback
        (void)resultFuture;
#else
        // WASM: Parse HTTP response body to extract event (cross-process communication)
        handleWasmResponse(std::move(resultFuture), engine);
#endif
    }

private:
    /**
     * @brief Parse JSON event data into params map for form-encoded POST (W3C SCXML C.2)
     *
     * When send has <param> elements, eventData contains JSON like {"param1":"1"}.
     * HTTP POST requires params map for application/x-www-form-urlencoded format.
     */
    static void parseJsonToParams(const std::string &data, SCE::EventDescriptor &descriptor) {
        if (data.empty() || data.front() != '{') {
            return;
        }

        try {
            using json = nlohmann::json;
            json dataObj = json::parse(data);
            for (const auto &[key, value] : dataObj.items()) {
                // W3C SCXML C.2: Avoid duplicate _scxmleventname in HTTP POST body
                // W3C requires "single instance" of _scxmleventname parameter
                if (key == "_scxmleventname" && !descriptor.eventName.empty()) {
                    continue;
                }

                std::string valueStr;
                if (value.is_string()) {
                    valueStr = value.template get<std::string>();
                } else if (value.is_number_integer()) {
                    valueStr = std::to_string(value.template get<int>());
                } else if (value.is_number_float()) {
                    valueStr = std::to_string(value.template get<double>());
                } else {
                    valueStr = value.dump();
                }
                descriptor.params[key].push_back(valueStr);
            }
            SCE_LOG_DEBUG("AOT HttpSendHelper: Parsed {} params from JSON eventData", descriptor.params.size());
        } catch (const nlohmann::json::exception &e) {
            SCE_LOG_ERROR("Failed to parse eventData as JSON: {}", e.what());
        }
    }

    /**
     * @brief Build HTTP request body and content type from descriptor (W3C SCXML C.2)
     *
     * @return pair of (requestBody, contentType)
     */
    static std::pair<std::string, std::string> buildRequestBody(const SCE::EventDescriptor &descriptor) {
        if (!descriptor.eventName.empty() || !descriptor.params.empty()) {
            // W3C SCXML C.2: Form-encoded format (test 518, 519)
            return {SendHelper::buildHttpPostBody(descriptor.eventName, descriptor.params),
                    "application/x-www-form-urlencoded"};
        } else if (!descriptor.content.empty()) {
            // W3C SCXML C.2: Content-only send (test 520)
            return {descriptor.content, "text/plain"};
        } else {
            // JSON format
            return {descriptor.data, "application/json"};
        }
    }

#ifdef __EMSCRIPTEN__
    /**
     * @brief Handle WASM HTTP response: parse JSON body and route event back to engine
     */
    template <typename Engine>
    static void handleWasmResponse(std::future<SCE::HttpClient::Response> resultFuture, Engine &engine) {
        try {
            auto response = resultFuture.get();

            if (response.success && !response.body.empty() && response.body.front() == '{') {
                using json = nlohmann::json;
                json responseObj = json::parse(response.body);

                SCE_LOG_DEBUG("AOT HttpSendHelper WASM: HTTP response body: {}", response.body);

                if (responseObj.contains("event")) {
                    std::string eventName = responseObj["event"].template get<std::string>();
                    SCE_LOG_DEBUG("AOT HttpSendHelper WASM: Received HTTP response event '{}'", eventName);

                    std::string eventData;
                    if (responseObj.contains("data")) {
                        eventData = responseObj["data"].dump();
                        SCE_LOG_DEBUG("AOT HttpSendHelper WASM: Extracted eventData: {}", eventData);
                    }
                    engine.raiseExternal(eventName, eventData);
                }
            } else if (!response.success) {
                SCE_LOG_ERROR("AOT HttpSendHelper WASM: HTTP POST failed (status {})", response.statusCode);
            }
        } catch (const std::exception &e) {
            SCE_LOG_ERROR("AOT HttpSendHelper WASM: Exception while getting HTTP result: {}", e.what());
        }
    }
#endif
};

}  // namespace SCE::Static
