// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The conformance kit of the https binding's port (docs/adr/0107, D2): the
// cases every adapter of mesh/transports/HttpsTransport.h is held to, over
// real sockets and real TLS, so an adapter is judged by what it does on a
// network rather than by what it calls. SCE's adapters run it in SCE's lanes
// (the curl adapter on Linux, the WinHTTP adapter on Windows); an embedding's
// adapter runs it in the embedding's tests by implementing AdapterFactory.
//
// What the kit stands up for each case, all on 127.0.0.1:
//   * a TLS server of the binding's wire (a Mailbox behind a socket), with a
//     certificate a test authority issued for `localhost`, `127.0.0.1` and
//     `sce-kit.test`;
//   * an explicit proxy that takes CONNECT and tunnels to that server, for
//     any host name the client asks for;
//   * a proxy that inspects TLS: it takes CONNECT, ends the client's TLS with
//     a certificate a second authority (the network's) issued, reads the HTTP
//     inside and carries it to the server over TLS of its own.
//
// The cases:
//   directExchange            the client of the binding over the adapter:
//                             ready, a mailbox envelope delivered, a send
//                             taken in;
//   throughConnectProxy       an exchange tunnelled through the explicit
//                             proxy, to a name only the proxy resolves;
//   throughInspectingProxy    an exchange through the inspecting proxy, with
//                             the network's authority the one trusted;
//   untrustedServerIsTls      a server whose authority the platform does not
//                             trust is Failure::Tls, not a response;
//   droppedResponseAfterServerActed
//                             the server takes a request in and closes
//                             without answering: a Failure, the envelope
//                             taken in, and a client that delivers what a
//                             lost response carried exactly once;
//   heldPastBound             a server holding an exchange past its timeout
//                             is Failure::Timeout, within the bound.

#pragma once

#include "mesh/transports/HttpsTransport.h"

#include <memory>
#include <optional>
#include <string>

namespace SCE::Mesh::HttpsKit {

/// What a case tells the adapter factory of the network it runs on.
struct Network {
    /// A PEM file holding the authorities the platform must trust for this
    /// case, and nothing else.
    std::string trusted_pem;
    /// The same authority, DER encoded, for a platform that installs one into
    /// a certificate store.
    std::string trusted_der;
    /// The explicit proxy to tunnel through, `127.0.0.1:<port>`; empty for a
    /// case that goes direct.
    std::string proxy;
};

/// An adapter made for one case, and what the factory set up for it (a
/// trusted authority, a proxy setting), undone when this is destroyed.
struct AdapterUnderTest {
    std::unique_ptr<HttpsTransport> transport;
    std::shared_ptr<void> scope;
};

/// Makes the adapter under test for each case. The factory owns how its
/// platform is told to trust an authority and to use a proxy: an adapter on
/// a platform's HTTP stack reads both from the platform.
class AdapterFactory {
public:
    virtual ~AdapterFactory() = default;
    virtual AdapterUnderTest make(const Network &network) = 0;
};

/// What a case found.
struct CaseResult {
    bool passed = false;
    std::string why;
};

/// The kit: a test authority, the network's authority and their
/// certificates, made once for the kit's life in a directory of its own.
class Kit {
public:
    Kit();
    ~Kit();
    Kit(const Kit &) = delete;
    Kit &operator=(const Kit &) = delete;

    /// Whether the authorities and certificates were made.
    bool ready() const;

    CaseResult directExchange(AdapterFactory &factory);
    CaseResult throughConnectProxy(AdapterFactory &factory);
    CaseResult throughInspectingProxy(AdapterFactory &factory);
    CaseResult untrustedServerIsTls(AdapterFactory &factory);
    CaseResult droppedResponseAfterServerActed(AdapterFactory &factory);
    CaseResult heldPastBound(AdapterFactory &factory);

    struct Impl;

private:
    std::unique_ptr<Impl> impl_;
};

}  // namespace SCE::Mesh::HttpsKit
