// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#pragma once

/**
 * @file HttplibTransport.h
 * @brief An exchange of the standard host features' HTTP operation (docs/SCE_HOST_FEATURES.md,
 *        section 5) over cpp-httplib, the client the runtime already links.
 *
 * It follows no redirect and keeps no cookie. cpp-httplib keeps a response's fields in a map
 * ordered by name, so the order they were received in is lost: they are given sorted by name, the
 * order a transport that cannot give the received one gives. An `https` URL is carried only where
 * cpp-httplib was built with TLS; where it was not, such an exchange fails as `network`.
 */

#include <chrono>
#include <httplib.h>
#include <string>

#include "host/StandardHost.h"

namespace SCE::StandardHost {

class HttplibTransport : public Transport {
public:
    explicit HttplibTransport(std::chrono::seconds timeout = std::chrono::seconds(30)) : timeout_(timeout) {}

    Exchange exchange(const HttpRequest &request) override {
        // The URL was accepted as an absolute http or https one with a host: its path is what follows the
        // authority.
        const size_t schemeEnd = request.url.find("://") + 3;
        const size_t pathStart = request.url.find_first_of("/?#", schemeEnd);
        const std::string base = request.url.substr(0, pathStart);
        std::string target = pathStart == std::string::npos ? "/" : request.url.substr(pathStart);
        if (target.front() != '/') {
            target.insert(0, "/");
        }
        const size_t fragment = target.find('#');
        if (fragment != std::string::npos) {
            target.erase(fragment);
        }
        httplib::Client client(base);
        client.set_follow_location(false);
        client.set_connection_timeout(timeout_);
        client.set_read_timeout(timeout_);
        httplib::Request outgoing;
        outgoing.method = request.method;
        outgoing.path = target;
        for (const auto &header : request.headers) {
            outgoing.headers.emplace(header.name, header.value);
        }
        outgoing.body = request.body;
        const auto result = client.send(outgoing);
        if (!result) {
            return {Exchange::Kind::Failure, {}};
        }
        HttpResponse response;
        response.status = result->status;
        for (const auto &[name, value] : result->headers) {
            response.headers.push_back({name, value});
        }
        response.body = result->body;
        return {Exchange::Kind::Response, std::move(response)};
    }

private:
    std::chrono::seconds timeout_;
};

}  // namespace SCE::StandardHost
