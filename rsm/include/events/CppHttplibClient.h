// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-RSM-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#ifndef __EMSCRIPTEN__

#include "events/IHttpClient.h"
#include <regex>

namespace RSM {

/**
 * @brief Native HTTP client using cpp-httplib
 *
 * Platform: Linux/macOS/Windows (Native builds)
 * Dependencies: cpp-httplib (POSIX sockets, pthread)
 *
 * ARCHITECTURE.md Compliance:
 * - Zero Duplication: Implements IHttpClient interface
 * - Platform Abstraction: Native-specific implementation
 * - All-or-Nothing: Pure Native (no WASM mixing)
 *
 * W3C SCXML C.2 BasicHTTP Event I/O Processor:
 * - Sends HTTP POST to external servers
 * - Supports HTTP/HTTPS with SSL verification
 */
class CppHttplibClient : public IHttpClient {
public:
    CppHttplibClient();
    ~CppHttplibClient() override = default;

    std::future<HttpClient::Response> sendRequest(const HttpClient::Request &request) override;
    void setTimeout(std::chrono::milliseconds timeout) override;

    void setSSLVerification(bool verify);
    void setCustomHeaders(const std::map<std::string, std::string> &headers);

private:
    std::tuple<std::string, std::string, int, std::string> parseUrl(const std::string &url) const;

    std::chrono::milliseconds timeout_{5000};
    bool sslVerification_{true};
    std::map<std::string, std::string> customHeaders_;
};

}  // namespace RSM

#endif  // !__EMSCRIPTEN__
