// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#pragma once

#include <cstddef>
#include <cstdio>
#include <stdexcept>
#include <string>

#include "basic_http_test_endpoint.h"

namespace SCE {
namespace W3C {

/// C++ ergonomics over the endpoint fact, which lives in
/// `basic_http_test_endpoint.h` because the C11 AOT runners need it too — a
/// `constexpr` here put it out of their reach and each of them wrote the
/// address again.
///
/// §scxml-C-2-3: this same address is handed to the state machine as the
/// BasicHTTP processor's 'location', and the converted W3C documents read that
/// entry to address their sends. Bind parameters and published address are
/// therefore one fact — a document that posts somewhere the listener never
/// claimed would fail for a reason unrelated to what it tests.
///
/// Three listeners answer here across the harness: W3CHttpTestServer for the
/// native interpreter and AOT runs, and standalone_http_server.js (started by
/// polyfill_pre.js) for the WASM run. W3CHttpTestServer is in the process of
/// the machine it serves, so it binds the port the kernel assigns unless
/// `SCE_W3C_HTTP_PORT` names one, and publishes what it bound
/// (basic_http_test_endpoint.h says why).
///
/// These are functions rather than the constants they replaced. The endpoint is
/// read from the environment (`SCE_W3C_HTTP_PORT`) so two checkouts of this
/// repository can be given different ones: the listener is a machine-global
/// resource, and a compiled-in number makes concurrent runs impossible. Every
/// use site passed the old constants as values and none in a constant
/// expression, so nothing needed the `constexpr`.

/// The port an out-of-process fixture listener binds (the WASM host's): the
/// one `SCE_W3C_HTTP_PORT` names, or the default.
inline int basicHttpTestPort() {
    return sce_w3c_http_test_port();
}

/// The port the in-process W3CHttpTestServer binds: the one
/// `SCE_W3C_HTTP_PORT` names, or 0 for one the kernel assigns. The location a
/// machine is handed is then the server's (W3CHttpTestServer::accessUri).
inline int basicHttpInProcessPort() {
    return sce_w3c_http_in_process_port();
}

/// The path the fixture listener answers on.
inline const char *basicHttpTestPath() {
    return sce_w3c_http_test_path();
}

/// The BasicHTTP location of a listener bound to `port`, answering on `path`:
/// `http://localhost:<port><path>`, in the endpoint's one format.
inline std::string basicHttpAccessUri(int port, const std::string &path) {
    const int length = std::snprintf(nullptr, 0, SCE_W3C_HTTP_URI_FORMAT, port, path.c_str());
    if (length < 0) {
        throw std::runtime_error("basicHttpAccessUri: the endpoint format could not be written");
    }
    std::string uri(static_cast<std::size_t>(length) + 1, '\0');
    std::snprintf(&uri[0], uri.size(), SCE_W3C_HTTP_URI_FORMAT, port, path.c_str());
    uri.resize(static_cast<std::size_t>(length));
    return uri;
}

/// The published BasicHTTP location of the out-of-process listener:
/// `http://localhost:<port><path>`.
inline std::string basicHttpTestAccessUri() {
    char buffer[SCE_W3C_HTTP_URI_MAX];
    return std::string(sce_w3c_http_test_access_uri(buffer, sizeof buffer));
}

}  // namespace W3C
}  // namespace SCE
