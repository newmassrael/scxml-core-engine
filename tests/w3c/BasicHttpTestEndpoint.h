// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#pragma once

#include <string>

namespace SCE {
namespace W3C {

/// Where the harness's inbound BasicHTTP listener answers.
///
/// §scxml-C-2-3: this same address is handed to the state machine as the
/// BasicHTTP processor's 'location', and the converted W3C documents read that
/// entry to address their sends. Bind parameters and published address are
/// therefore one fact — a document that posts somewhere the listener never
/// claimed would fail for a reason unrelated to what it tests.
///
/// Three listeners answer here across the harness: W3CHttpTestServer for the
/// native interpreter and AOT runs, and standalone_http_server.js (started by
/// polyfill_pre.js) for the WASM run, which takes the same port and path as
/// its argv defaults.
constexpr int BASIC_HTTP_TEST_PORT = 8080;
constexpr const char *BASIC_HTTP_TEST_PATH = "/test";

inline std::string basicHttpTestAccessUri() {
    return "http://localhost:" + std::to_string(BASIC_HTTP_TEST_PORT) + BASIC_HTTP_TEST_PATH;
}

}  // namespace W3C
}  // namespace SCE
