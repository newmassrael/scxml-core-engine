// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-RSM-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#ifndef __EMSCRIPTEN__

#include "events/CppHttplibClient.h"
#include "common/Logger.h"
#include <algorithm>
#include <httplib.h>

namespace RSM {

CppHttplibClient::CppHttplibClient() {
    LOG_DEBUG("CppHttplibClient: Created Native HTTP client");
}

std::future<HttpClient::Response> CppHttplibClient::sendRequest(const HttpClient::Request &request) {
    auto timeout = timeout_;
    auto sslVerify = sslVerification_;
    auto headers = customHeaders_;

    return std::async(std::launch::async, [request, timeout, sslVerify, headers]() -> HttpClient::Response {
        try {
            // Parse URL
            std::regex uriPattern(R"(^(https?)://([^:/\s]+)(?::(\d+))?(/.*)?$)", std::regex_constants::icase);
            std::smatch match;

            if (!std::regex_match(request.url, match, uriPattern)) {
                LOG_ERROR("CppHttplibClient: Invalid URL: {}", request.url);
                return HttpClient::Response{false, 0, "", {}};
            }

            std::string scheme = match[1].str();
            std::transform(scheme.begin(), scheme.end(), scheme.begin(), ::tolower);

            std::string host = match[2].str();

            int port;
            if (match[3].matched) {
                try {
                    port = std::stoi(match[3].str());
                } catch (const std::exception &) {
                    LOG_ERROR("CppHttplibClient: Invalid port in URL: {}", request.url);
                    return HttpClient::Response{false, 0, "", {}};
                }
            } else {
                port = (scheme == "https") ? 443 : 80;
            }

            std::string path = match[4].matched ? match[4].str() : "/";

            // Create base URL
            std::string baseUrl = scheme + "://" + host;
            if ((scheme == "http" && port != 80) || (scheme == "https" && port != 443)) {
                baseUrl += ":" + std::to_string(port);
            }

            LOG_DEBUG("CppHttplibClient: {} {} (timeout={}ms)", request.method, baseUrl + path, timeout.count());

            // Create cpp-httplib client
            httplib::Client client(baseUrl);

            // Set timeout
            auto timeoutSec = std::chrono::duration_cast<std::chrono::seconds>(timeout);
            client.set_connection_timeout(timeoutSec.count());
            client.set_read_timeout(timeoutSec.count());
            client.set_write_timeout(timeoutSec.count());

            // SSL settings
            if (scheme == "https") {
                client.enable_server_certificate_verification(sslVerify);
            }

            // Set headers
            httplib::Headers httplibHeaders;
            for (const auto &[key, value] : headers) {
                httplibHeaders.emplace(key, value);
            }
            for (const auto &[key, value] : request.headers) {
                httplibHeaders.emplace(key, value);
            }
            client.set_default_headers(httplibHeaders);

            // Execute request
            httplib::Result result;
            if (request.method == "POST") {
                result = client.Post(path.c_str(), request.body, request.contentType.c_str());
            } else if (request.method == "GET") {
                result = client.Get(path.c_str());
            } else if (request.method == "PUT") {
                result = client.Put(path.c_str(), request.body, request.contentType.c_str());
            } else if (request.method == "DELETE") {
                result = client.Delete(path.c_str());
            } else {
                LOG_ERROR("CppHttplibClient: Unsupported HTTP method: {}", request.method);
                return HttpClient::Response{false, 0, "", {}};
            }

            // Convert result
            HttpClient::Response response;
            if (result) {
                response.success = (result->status >= 200 && result->status < 300);
                response.statusCode = result->status;
                response.body = result->body;

                for (const auto &[key, value] : result->headers) {
                    response.headers[key] = value;
                }

                LOG_DEBUG("CppHttplibClient: Response {} {} (body {} bytes)", result->status,
                          response.success ? "OK" : "ERROR", response.body.size());
            } else {
                response.success = false;
                response.statusCode = 0;
                auto err = result.error();
                LOG_ERROR("CppHttplibClient: Request failed - error code {}", static_cast<int>(err));
            }

            return response;

        } catch (const std::exception &e) {
            LOG_ERROR("CppHttplibClient: Exception: {}", e.what());
            return HttpClient::Response{false, 0, "", {}};
        }
    });
}

void CppHttplibClient::setTimeout(std::chrono::milliseconds timeout) {
    timeout_ = timeout;
    LOG_DEBUG("CppHttplibClient: Set timeout to {}ms", timeout.count());
}

void CppHttplibClient::setSSLVerification(bool verify) {
    sslVerification_ = verify;
    LOG_DEBUG("CppHttplibClient: SSL verification {}", verify ? "enabled" : "disabled");
}

void CppHttplibClient::setCustomHeaders(const std::map<std::string, std::string> &headers) {
    customHeaders_ = headers;
    LOG_DEBUG("CppHttplibClient: Set {} custom headers", headers.size());
}

std::tuple<std::string, std::string, int, std::string> CppHttplibClient::parseUrl(const std::string &url) const {
    std::regex uriPattern(R"(^(https?)://([^:/\s]+)(?::(\d+))?(/.*)?$)", std::regex_constants::icase);
    std::smatch match;

    if (!std::regex_match(url, match, uriPattern)) {
        return {"", "", 0, ""};
    }

    std::string scheme = match[1].str();
    std::transform(scheme.begin(), scheme.end(), scheme.begin(), ::tolower);

    std::string host = match[2].str();

    int port;
    if (match[3].matched) {
        try {
            port = std::stoi(match[3].str());
        } catch (const std::exception &) {
            return {"", "", 0, ""};
        }
    } else {
        port = (scheme == "https") ? 443 : 80;
    }

    std::string path = match[4].matched ? match[4].str() : "/";

    return {scheme, host, port, path};
}

}  // namespace RSM

#endif  // !__EMSCRIPTEN__
