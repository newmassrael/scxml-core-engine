// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-RSM-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#ifdef __EMSCRIPTEN__

#include "events/IHttpClient.h"

namespace RSM {

/**
 * @brief WASM HTTP client using Emscripten Fetch API
 *
 * Platform: WebAssembly (Emscripten/Browser)
 * Dependencies: Emscripten Fetch API (browser-native)
 *
 * ARCHITECTURE.md Compliance:
 * - Zero Duplication: Implements IHttpClient interface
 * - Platform Abstraction: WASM-specific implementation
 * - All-or-Nothing: Pure WASM (no Native mixing)
 *
 * W3C SCXML C.2 BasicHTTP Event I/O Processor:
 * - Sends HTTP POST via browser Fetch API
 * - CORS-compliant (browser handles automatically)
 *
 * Build Requirements:
 * - Emscripten flag: -sFETCH=1
 */
class EmscriptenFetchClient : public IHttpClient {
public:
    EmscriptenFetchClient();
    ~EmscriptenFetchClient() override = default;

    std::future<HttpClient::Response> sendRequest(const HttpClient::Request &request) override;
    void setTimeout(std::chrono::milliseconds timeout) override;

private:
    std::chrono::milliseconds timeout_{5000};
};

}  // namespace RSM

#endif  // __EMSCRIPTEN__
