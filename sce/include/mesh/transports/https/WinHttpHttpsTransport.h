// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The reference adapter of the https binding's port on WinHTTP, the Windows
// HTTP stack (docs/adr/0107, D2). The target SCE::mesh_https_winhttp
// (cmake/SCEMeshHttps.cmake) builds it on Windows only.
//
// What it takes from the platform, as the port requires:
//   * trust: the Windows certificate store, through WinHTTP's own validation
//     of the server's chain, so an authority an administrator deployed (a
//     network that inspects TLS among them) is honoured;
//   * proxy: the current user's proxy configuration
//     (WinHttpGetIEProxyConfigForCurrentUser), its automatic configuration
//     script and WPAD resolved per URL with WinHttpGetProxyForUrl, tunnelling
//     to the proxy with CONNECT;
//   * HTTP/2 where the system's WinHTTP has it, HTTP/1.1 otherwise.
//
// The header names no Windows type and includes no Windows header, so a host
// that includes it receives none of the macros windows.h defines.

#pragma once

#include "mesh/transports/HttpsTransport.h"

#include <memory>
#include <string>

namespace SCE::Mesh {

class WinHttpHttpsTransport final : public HttpsTransport {
public:
    /// What the host may set beyond the platform's defaults.
    struct Options {
        /// A proxy that replaces the user's configuration, `host:port` (an
        /// explicit proxy the host's own policy names). Empty: the user's
        /// configuration, its script and WPAD included.
        std::string proxy;
        /// The `mtls` authenticator's client certificate: the SHA-1
        /// thumbprint, in hexadecimal, of a certificate with its private key
        /// in the current user's personal store (`MY`). Empty: none.
        std::string client_certificate_thumbprint;
        /// The User-Agent the session sends.
        std::string user_agent = "SCE-Mesh-HTTPS/1";
    };

    WinHttpHttpsTransport();
    explicit WinHttpHttpsTransport(Options options);
    ~WinHttpHttpsTransport() override;

    WinHttpHttpsTransport(const WinHttpHttpsTransport &) = delete;
    WinHttpHttpsTransport &operator=(const WinHttpHttpsTransport &) = delete;

    /// Whether the WinHTTP session opened; an adapter whose session did not
    /// open answers every exchange with Failure::Other.
    bool valid() const noexcept;

    HttpsExchange exchange(const HttpsRequest &request) override;

private:
    struct Impl;
    std::unique_ptr<Impl> impl_;
};

}  // namespace SCE::Mesh
