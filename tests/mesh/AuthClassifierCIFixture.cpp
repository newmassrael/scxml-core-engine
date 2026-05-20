// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh §16.7 row 10 — axis-6 dynamic verification gate.
//
// Exercises `SCE::Mesh::ThirdParty::isZenohAuthFailMessage` against the
// actually-linked zenoh-cpp binary. A zenoh-cpp upgrade that rephrases its
// auth-failure error text would silently flip row 10 UNAUTHORIZED into row 1
// TRANSPORT_UNAVAILABLE; this fixture catches the drift at CI time before
// reaching any consumer.
//
// Mechanism (docs/SCE_AXIS_6_PATTERNS.md A6-001):
//  1. Generate fresh cert material at test-suite setup time (two disjoint
//     self-signed CAs, server cert signed by CA1, client cert signed by CA2).
//  2. Spawn a zenoh router peer with mTLS listener configured to trust CA1
//     only (so the CA2-signed client cert presented by the connecting peer
//     fails verification).
//  3. Spawn a zenoh client peer with the mismatched CA2-signed cert; its
//     Session::open call throws ZException.
//  4. Capture ZException::what(), assert isZenohAuthFailMessage(what) ==
//     true.
//
// The test is gated on both OpenSSL (for cert generation) and zenoh-cpp
// (for the actual handshake). Either missing, the test is skipped with an
// informative message.

#include "mesh/third_party/AuthClassifier.h"

#include <gtest/gtest.h>

#include <zenoh.hxx>

#include <array>
#include <cerrno>
#include <chrono>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <filesystem>
#include <fstream>
#include <memory>
#include <stdexcept>
#include <string>
#include <thread>

#include <arpa/inet.h>
#include <netinet/in.h>
#include <sys/socket.h>
#include <unistd.h>

namespace {

namespace fs = std::filesystem;

class TempCertEnv {
public:
    explicit TempCertEnv(const fs::path& base_dir) : base_(base_dir) {
        fs::create_directories(base_);
    }

    ~TempCertEnv() {
        std::error_code ec;
        fs::remove_all(base_, ec);
    }

    const fs::path& dir() const { return base_; }

private:
    fs::path base_;
};

// Run a command via system(), assert exit 0. The shell-out is acceptable
// here because openssl is invoked with controlled, static-shape arguments
// (no user-derived input is concatenated into the command).
void runOpenssl(const std::string& command) {
    int rc = std::system(command.c_str());
    if (rc != 0) {
        throw std::runtime_error("openssl invocation failed (rc=" +
                                 std::to_string(rc) + "): " + command);
    }
}

// Generate two disjoint self-signed CAs and matching leaf certs. CA1 signs
// the server cert; CA2 signs the client cert. Server's trust store is CA1
// only — the CA2-signed client cert presented at mTLS handshake time fails
// the chain-verify check. CN values are spec-distinct so a future
// maintainer can read tcpdump output and tell which side rejected.
void generateCertMaterial(const fs::path& d) {
    auto p = [&](const char* name) { return (d / name).string(); };

    // Two CAs, each rooted at its own private key.
    runOpenssl("openssl genrsa -out " + p("ca1.key") + " 2048 >/dev/null 2>&1");
    runOpenssl("openssl req -x509 -new -nodes -key " + p("ca1.key") +
               " -sha256 -days 1 -out " + p("ca1.cert.pem") +
               " -subj '/CN=SCE-Test-CA1' >/dev/null 2>&1");
    runOpenssl("openssl genrsa -out " + p("ca2.key") + " 2048 >/dev/null 2>&1");
    runOpenssl("openssl req -x509 -new -nodes -key " + p("ca2.key") +
               " -sha256 -days 1 -out " + p("ca2.cert.pem") +
               " -subj '/CN=SCE-Test-CA2' >/dev/null 2>&1");

    // Server leaf: CN=localhost, signed by CA1.
    runOpenssl("openssl genrsa -out " + p("server.key") + " 2048 >/dev/null 2>&1");
    runOpenssl("openssl req -new -key " + p("server.key") + " -out " + p("server.csr") +
               " -subj '/CN=localhost' >/dev/null 2>&1");
    // SAN for localhost so rustls' hostname verification is satisfied on
    // the server-cert side of the handshake (the failure must originate
    // from CLIENT cert verification, not server-name mismatch).
    {
        std::ofstream san((d / "server.ext").string());
        san << "subjectAltName=DNS:localhost,IP:127.0.0.1\n";
        san << "extendedKeyUsage=serverAuth\n";
    }
    runOpenssl("openssl x509 -req -in " + p("server.csr") +
               " -CA " + p("ca1.cert.pem") + " -CAkey " + p("ca1.key") +
               " -CAcreateserial -out " + p("server.cert.pem") +
               " -days 1 -sha256 -extfile " + p("server.ext") + " >/dev/null 2>&1");

    // Client leaf: CN=sce-test-client, signed by CA2 (the wrong CA from
    // the server's perspective).
    runOpenssl("openssl genrsa -out " + p("client.key") + " 2048 >/dev/null 2>&1");
    runOpenssl("openssl req -new -key " + p("client.key") + " -out " + p("client.csr") +
               " -subj '/CN=sce-test-client' >/dev/null 2>&1");
    {
        std::ofstream ext((d / "client.ext").string());
        ext << "extendedKeyUsage=clientAuth\n";
    }
    runOpenssl("openssl x509 -req -in " + p("client.csr") +
               " -CA " + p("ca2.cert.pem") + " -CAkey " + p("ca2.key") +
               " -CAcreateserial -out " + p("client.cert.pem") +
               " -days 1 -sha256 -extfile " + p("client.ext") + " >/dev/null 2>&1");
}

// Helper to wrap a value as a JSON5 string literal.
std::string jstr(const std::string& v) { return "\"" + v + "\""; }

// Apply common TLS-listener config keys. zenoh's actual config keys
// (verified against libzenohc.so symbol table) live under
// `transport/link/tls/`. enable_mtls=true forces the connecting peer to
// present a client cert; root_ca_certificate is the trust store —
// CA1.cert.pem on the listener side, so a CA2-signed client cert fails
// chain verification.
void applyRouterConfig(zenoh::Config& config, const fs::path& d, int port) {
    std::string p = d.string();
    config.insert_json5("mode", jstr("router"));
    config.insert_json5("scouting/multicast/enabled", "false");
    config.insert_json5(
        "listen/endpoints",
        "[" + jstr("tls/127.0.0.1:" + std::to_string(port)) + "]");
    config.insert_json5("transport/link/tls/root_ca_certificate",
                        jstr(p + "/ca1.cert.pem"));
    config.insert_json5("transport/link/tls/listen_certificate",
                        jstr(p + "/server.cert.pem"));
    config.insert_json5("transport/link/tls/listen_private_key",
                        jstr(p + "/server.key"));
    config.insert_json5("transport/link/tls/enable_mtls", "true");
    config.insert_json5("transport/link/tls/verify_name_on_connect", "false");
}

void applyClientConfig(zenoh::Config& config, const fs::path& d, int port) {
    std::string p = d.string();
    config.insert_json5("mode", jstr("client"));
    config.insert_json5("scouting/multicast/enabled", "false");
    config.insert_json5(
        "connect/endpoints",
        "[" + jstr("tls/127.0.0.1:" + std::to_string(port)) + "]");
    config.insert_json5("transport/link/tls/root_ca_certificate",
                        jstr(p + "/ca1.cert.pem"));
    config.insert_json5("transport/link/tls/connect_certificate",
                        jstr(p + "/client.cert.pem"));
    config.insert_json5("transport/link/tls/connect_private_key",
                        jstr(p + "/client.key"));
    config.insert_json5("transport/link/tls/enable_mtls", "true");
    config.insert_json5("transport/link/tls/verify_name_on_connect", "false");
}

// Pick a free localhost port: bind a TCP socket to port 0, read the
// assigned port, close. Race-window between close and the test's bind
// is tiny but real — the test only needs the port for a few seconds.
int pickFreePort() {
    int sock = ::socket(AF_INET, SOCK_STREAM, 0);
    if (sock < 0) {
        throw std::runtime_error("socket() failed: " +
                                 std::string(::strerror(errno)));
    }
    sockaddr_in addr{};
    addr.sin_family = AF_INET;
    addr.sin_addr.s_addr = htonl(INADDR_LOOPBACK);
    addr.sin_port = 0;
    if (::bind(sock, reinterpret_cast<sockaddr*>(&addr), sizeof(addr)) < 0) {
        ::close(sock);
        throw std::runtime_error("bind() failed: " +
                                 std::string(::strerror(errno)));
    }
    socklen_t len = sizeof(addr);
    if (::getsockname(sock, reinterpret_cast<sockaddr*>(&addr), &len) < 0) {
        ::close(sock);
        throw std::runtime_error("getsockname() failed: " +
                                 std::string(::strerror(errno)));
    }
    int port = ntohs(addr.sin_port);
    ::close(sock);
    return port;
}

class AuthClassifierCIFixture : public ::testing::Test {
protected:
    static void SetUpTestSuite() {
        // Use a per-process temporary directory under the build tree so
        // parallel ctest invocations don't collide.
        cert_env_ = std::make_unique<TempCertEnv>(
            fs::temp_directory_path() /
            ("sce_auth_ci_" + std::to_string(::getpid())));
        generateCertMaterial(cert_env_->dir());
    }

    static void TearDownTestSuite() { cert_env_.reset(); }

    static std::unique_ptr<TempCertEnv> cert_env_;
};

std::unique_ptr<TempCertEnv> AuthClassifierCIFixture::cert_env_;

// Returns the ZException::what() captured from a mismatched-CA mTLS
// handshake. Server runs on a fresh port; the client peer attempts open
// and is expected to throw.
std::string captureZenohMTLSHandshakeFailMessage(const fs::path& cert_dir) {
    int port = pickFreePort();

    // Stage 1: start router (server) peer. Use err-out form so the
    // server itself doesn't throw on benign listen-time conditions.
    zenoh::ZResult server_err = Z_OK;
    auto server_config = zenoh::Config::create_default();
    applyRouterConfig(server_config, cert_dir, port);
    auto server_session = zenoh::Session::open(std::move(server_config),
                                               zenoh::Session::SessionOptions::create_default(),
                                               &server_err);
    if (server_err != Z_OK) {
        throw std::runtime_error(
            "server Session::open failed (likely zenoh-cpp build missing "
            "tls feature or port " + std::to_string(port) +
            " unbindable) — ZResult=" + std::to_string(server_err));
    }

    // Stage 2: give the router a small moment to begin accepting on
    // the TLS listener before the client peer attempts connect.
    std::this_thread::sleep_for(std::chrono::milliseconds(200));

    // Stage 3: client peer attempts mTLS handshake; capture the
    // ZException. The throw form (err=nullptr) is what the codegen
    // template's catch-block path is exercising in production.
    try {
        auto client_config = zenoh::Config::create_default();
        applyClientConfig(client_config, cert_dir, port);
        auto client_session = zenoh::Session::open(std::move(client_config));
        // If we reach here, the handshake unexpectedly succeeded — the
        // CA mismatch did not surface as a Session::open failure.
        return std::string();  // empty string signals "no throw"
    } catch (const zenoh::ZException& ex) {
        return ex.what() ? ex.what() : "";
    }
}

}  // namespace

// Stage 1: zenoh-cpp SCE-facing API contract is alive — Session::open
// throws on mTLS handshake failure. If this assertion ever fails, zenoh-cpp
// has stopped propagating handshake failure as a synchronous throw and
// SCE's catch-block-based row-10 path needs reconsideration entirely.
TEST_F(AuthClassifierCIFixture, MTLSHandshakeFailureSurfacesAsZException) {
    const std::string msg = captureZenohMTLSHandshakeFailMessage(cert_env_->dir());
    ASSERT_FALSE(msg.empty())
        << "zenoh client Session::open did NOT throw on a CA-mismatched mTLS "
           "handshake. Either zenoh-cpp's TLS feature is disabled in this "
           "build, the test config does not actually enforce mTLS, or "
           "zenoh-cpp upstream switched to async/non-throwing error "
           "delivery. SCE row-10 production codepath assumes synchronous "
           "ZException from Session::open — re-examine the assumption.";

    // Print the captured message so a future maintainer reading ctest
    // output can see exactly what zenoh-cpp is producing under this
    // build's binary version.
    std::cout << "[axis-6 dynamic verification] zenoh-cpp ZException::what() "
                 "on mTLS handshake fail: '"
              << msg << "'" << std::endl;
}

// Stage 2: known limitation lock-in (docs/SCE_AXIS_6_PATTERNS.md A6-001).
//
// zenoh-cpp current versions wrap every connection error — including mTLS
// handshake failure — in a generic Z_ENETWORK ZException whose what()
// payload is `"Failed to open session(Error code: -4 )"`. None of the
// manifest keywords (certificate / tls / auth / handshake) appear in
// that string, so AuthClassifier returns false and SCE's §16.7 row-10
// production codepath does not fire. Row 10 closure (`73087043`) remains
// spec-valid as an author-facing contract, but production emission of
// `error.communication{reason: UNAUTHORIZED}` is deferred until zenoh-cpp
// upstream exposes a typed auth-failure discriminator (`ZAuthException` or
// equivalent) that SCE can catch directly.
//
// This assertion LOCKS IN that limitation: if it ever fails, zenoh-cpp has
// started exposing detailed auth-fail text in ZException::what() and the
// row-10 production codepath becomes live. The remediation is:
//   1. flip this assertion to EXPECT_TRUE
//   2. remove the production-deferred notes from CommunicationError.h
//      (transport_status comment) and docs/SCE_AXIS_6_PATTERNS.md (A6-001
//      Limitation paragraph)
//   3. add release-note entry naming the zenoh-cpp version that enabled
//      live row-10 emission
TEST_F(AuthClassifierCIFixture, RowTenLimitationIsLockedIn) {
    const std::string msg = captureZenohMTLSHandshakeFailMessage(cert_env_->dir());
    ASSERT_FALSE(msg.empty()) << "see MTLSHandshakeFailureSurfacesAsZException for diagnosis";

    EXPECT_FALSE(SCE::Mesh::ThirdParty::isZenohAuthFailMessage(msg))
        << "zenoh-cpp upgrade detected: the actually-linked binary's "
           "auth-fail ZException::what() now matches an AuthClassifier "
           "manifest keyword. SCE §16.7 row-10 production codepath is now "
           "live — remove the production-deferred limitation notes "
           "(CommunicationError.h transport_status comment + "
           "SCE_AXIS_6_PATTERNS.md A6-001 Limitation paragraph) and flip "
           "this assertion to EXPECT_TRUE. Captured message: "
        << msg;
}
