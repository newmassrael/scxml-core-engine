// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The reference adapter of the https binding's port on the system's libcurl
// (docs/adr/0107, D2): the platform's HTTP client where the platform has no
// HTTP stack of its own, Linux among them. The target SCE::mesh_https_curl
// (cmake/SCEMeshHttps.cmake) builds it against the libcurl find_package(CURL)
// finds; SCE vendors none, so the distribution's security fixes reach it.
//
// What it takes from the platform, as the port requires:
//   * trust: the CA bundle libcurl was built to read (the distribution's),
//     unless the host names another;
//   * proxy: libcurl's own reading of `https_proxy` / `HTTPS_PROXY` and
//     `no_proxy` / `NO_PROXY`, tunnelling to the proxy with CONNECT;
//   * HTTP/2 over TLS where the libcurl has it, HTTP/1.1 otherwise.
//
// The header names no libcurl type, so a host that includes it receives none
// of libcurl's declarations.

#pragma once

#include "mesh/transports/HttpsTransport.h"

#include <string>

namespace SCE::Mesh {

class CurlHttpsTransport final : public HttpsTransport {
public:
    /// What the host may set beyond the platform's defaults.
    struct Options {
        /// A CA bundle (PEM) to trust instead of the one libcurl was built to
        /// read. Empty: the distribution's.
        std::string ca_bundle;
        /// A TLS client certificate (PEM) for the `mtls` authenticator, and
        /// its private key. Empty: none.
        std::string client_certificate;
        std::string client_key;
    };

    CurlHttpsTransport();
    explicit CurlHttpsTransport(Options options);
    ~CurlHttpsTransport() override;

    CurlHttpsTransport(const CurlHttpsTransport &) = delete;
    CurlHttpsTransport &operator=(const CurlHttpsTransport &) = delete;

    HttpsExchange exchange(const HttpsRequest &request) override;

private:
    Options options_;
};

}  // namespace SCE::Mesh
