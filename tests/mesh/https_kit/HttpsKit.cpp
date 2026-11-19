// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The conformance kit of the https binding's port, the C++ kit of SCE_MESH.md
// §mesh-22.7; its cases are stated in HttpsKit.h. The servers here speak
// HTTP/1.1 with a Content-Length on every message and offer no ALPN, so an
// adapter that would speak HTTP/2 falls back to HTTP/1.1 against them, as the
// port allows. TLS is OpenSSL's, which is the kit's own dependency and none of
// the adapters'.

#include "HttpsKit.h"

#include "Mailbox.h"

#include "mesh/MeshEnvelopeCodec.h"
#include "mesh/transports/HttpsClient.h"

// The sockets first: on Windows, winsock2.h must come before anything that
// may include windows.h, which would otherwise bring the older winsock.h.
#if defined(_WIN32)
#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#ifndef NOMINMAX
#define NOMINMAX
#endif
#include <winsock2.h>
#include <ws2tcpip.h>
#else
#include <arpa/inet.h>
#include <netinet/in.h>
#include <poll.h>
#include <sys/socket.h>
#include <sys/time.h>
#include <unistd.h>
#endif

#include <openssl/bio.h>
#include <openssl/bn.h>
#include <openssl/evp.h>
#include <openssl/pem.h>
#include <openssl/rand.h>
#include <openssl/ssl.h>
#include <openssl/x509.h>
#include <openssl/x509v3.h>

#include <atomic>
#include <chrono>
#include <condition_variable>
#include <cstdint>
#include <filesystem>
#include <fstream>
#include <functional>
#include <mutex>
#include <optional>
#include <string>
#include <thread>
#include <utility>
#include <vector>

namespace SCE::Mesh::HttpsKit {

namespace {

using namespace std::chrono_literals;
namespace Https = SCE::Mesh::Https;

/// The token every case's client presents and every case's server expects.
constexpr const char *kToken = "sce-kit-token";
/// The client's machine name.
constexpr const char *kClient = "phone";
/// The name only the proxies resolve (to 127.0.0.1).
constexpr const char *kProxiedHost = "sce-kit.test";
/// How long a case waits for what it expects before it fails.
constexpr std::chrono::milliseconds kPatience{10000};

// ── Sockets ──────────────────────────────────────────────────────────────────

#if defined(_WIN32)
using Socket = SOCKET;
using SockLen = int;
const Socket kNoSocket = INVALID_SOCKET;

void closeSocket(Socket s) {
    closesocket(s);
}

int pollSockets(WSAPOLLFD *fds, unsigned long count, int timeoutMs) {
    return WSAPoll(fds, count, timeoutMs);
}

using PollFd = WSAPOLLFD;

/// Winsock is started once for the kit's life.
struct Winsock {
    Winsock() {
        WSADATA data;
        WSAStartup(MAKEWORD(2, 2), &data);
    }

    ~Winsock() {
        WSACleanup();
    }
};
#else
using Socket = int;
using SockLen = socklen_t;
const Socket kNoSocket = -1;

void closeSocket(Socket s) {
    ::close(s);
}

int pollSockets(pollfd *fds, unsigned long count, int timeoutMs) {
    return ::poll(fds, static_cast<nfds_t>(count), timeoutMs);
}

using PollFd = pollfd;
#endif

/// Whether `s` has something to read within `timeoutMs`.
bool readable(Socket s, int timeoutMs) {
    PollFd fd{};
    fd.fd = s;
    fd.events = POLLIN;
    return pollSockets(&fd, 1, timeoutMs) > 0;
}

/// Bounds a blocking receive, so a peer that says nothing cannot hold a
/// handler thread past the kit's patience.
void boundReceive(Socket s) {
#if defined(_WIN32)
    const DWORD ms = static_cast<DWORD>(kPatience.count());
    setsockopt(s, SOL_SOCKET, SO_RCVTIMEO, reinterpret_cast<const char *>(&ms), static_cast<SockLen>(sizeof(ms)));
#else
    timeval tv{};
    tv.tv_sec = static_cast<time_t>(kPatience.count() / 1000);
    setsockopt(s, SOL_SOCKET, SO_RCVTIMEO, reinterpret_cast<const char *>(&tv), static_cast<SockLen>(sizeof(tv)));
#endif
}

Socket listenLoopback(std::uint16_t &port) {
    const Socket s = socket(AF_INET, SOCK_STREAM, IPPROTO_TCP);
    if (s == kNoSocket) {
        return kNoSocket;
    }
    const int one = 1;
    setsockopt(s, SOL_SOCKET, SO_REUSEADDR, reinterpret_cast<const char *>(&one), static_cast<SockLen>(sizeof(one)));
    sockaddr_in address{};
    address.sin_family = AF_INET;
    address.sin_addr.s_addr = htonl(INADDR_LOOPBACK);
    address.sin_port = 0;
    SockLen length = static_cast<SockLen>(sizeof(address));
    if (bind(s, reinterpret_cast<sockaddr *>(&address), length) != 0 || listen(s, 16) != 0 ||
        getsockname(s, reinterpret_cast<sockaddr *>(&address), &length) != 0) {
        closeSocket(s);
        return kNoSocket;
    }
    port = ntohs(address.sin_port);
    return s;
}

Socket connectLoopback(std::uint16_t port) {
    const Socket s = socket(AF_INET, SOCK_STREAM, IPPROTO_TCP);
    if (s == kNoSocket) {
        return kNoSocket;
    }
    sockaddr_in address{};
    address.sin_family = AF_INET;
    address.sin_addr.s_addr = htonl(INADDR_LOOPBACK);
    address.sin_port = htons(port);
    if (connect(s, reinterpret_cast<sockaddr *>(&address), static_cast<SockLen>(sizeof(address))) != 0) {
        closeSocket(s);
        return kNoSocket;
    }
    boundReceive(s);
    return s;
}

long long sendSome(Socket s, const char *data, std::size_t size) {
#if defined(_WIN32)
    return send(s, data, static_cast<int>(size), 0);
#elif defined(MSG_NOSIGNAL)
    return ::send(s, data, size, MSG_NOSIGNAL);
#else
    return ::send(s, data, size, 0);
#endif
}

long long receiveSome(Socket s, char *data, std::size_t size) {
#if defined(_WIN32)
    return recv(s, data, static_cast<int>(size), 0);
#else
    return ::recv(s, data, size, 0);
#endif
}

// ── Streams: a socket, or TLS over one ───────────────────────────────────────

class Stream {
public:
    explicit Stream(Socket s) : socket_(s) {}

    virtual ~Stream() = default;
    /// Bytes read (> 0), the end of the stream (0), or an error (< 0).
    virtual long long readSome(char *out, std::size_t size) = 0;
    virtual bool writeAll(const std::string &data) = 0;
    /// Whether bytes already read from the socket wait to be returned.
    virtual bool buffered() const = 0;

    /// Wait for something to read while `running` holds.
    bool await(const std::atomic<bool> &running) const {
        while (running.load()) {
            if (buffered() || readable(socket_, 100)) {
                return true;
            }
        }
        return false;
    }

protected:
    Socket socket_;
};

class PlainStream final : public Stream {
public:
    using Stream::Stream;

    long long readSome(char *out, std::size_t size) override {
        return receiveSome(socket_, out, size);
    }

    bool writeAll(const std::string &data) override {
        std::size_t sent = 0;
        while (sent < data.size()) {
            const long long n = sendSome(socket_, data.data() + sent, data.size() - sent);
            if (n <= 0) {
                return false;
            }
            sent += static_cast<std::size_t>(n);
        }
        return true;
    }

    bool buffered() const override {
        return false;
    }
};

class TlsStream final : public Stream {
public:
    TlsStream(Socket s, SSL *ssl) : Stream(s), ssl_(ssl) {}

    ~TlsStream() override {
        SSL_free(ssl_);
    }

    TlsStream(const TlsStream &) = delete;
    TlsStream &operator=(const TlsStream &) = delete;

    long long readSome(char *out, std::size_t size) override {
        const int n = SSL_read(ssl_, out, static_cast<int>(size));
        if (n > 0) {
            return n;
        }
        return SSL_get_error(ssl_, n) == SSL_ERROR_ZERO_RETURN ? 0 : -1;
    }

    bool writeAll(const std::string &data) override {
        std::size_t sent = 0;
        while (sent < data.size()) {
            const int n = SSL_write(ssl_, data.data() + sent, static_cast<int>(data.size() - sent));
            if (n <= 0) {
                return false;
            }
            sent += static_cast<std::size_t>(n);
        }
        return true;
    }

    bool buffered() const override {
        return SSL_pending(ssl_) > 0;
    }

private:
    SSL *ssl_;
};

// ── HTTP/1.1 messages ────────────────────────────────────────────────────────

struct Message {
    /// The request line or the status line.
    std::string start;
    std::vector<HttpsHeader> headers;
    std::vector<std::uint8_t> body;
};

/// Read one message from `stream`; `buffer` keeps what was read past it.
bool readMessage(Stream &stream, std::string &buffer, const std::atomic<bool> &running, Message &out) {
    std::size_t end = 0;
    char chunk[4096];
    while ((end = buffer.find("\r\n\r\n")) == std::string::npos) {
        if (buffer.size() > 65536 || !stream.await(running)) {
            return false;
        }
        const long long n = stream.readSome(chunk, sizeof(chunk));
        if (n <= 0) {
            return false;
        }
        buffer.append(chunk, static_cast<std::size_t>(n));
    }
    const std::string head = buffer.substr(0, end);
    buffer.erase(0, end + 4);
    out = Message();
    std::size_t start = 0;
    bool first = true;
    while (start <= head.size()) {
        std::size_t lineEnd = head.find("\r\n", start);
        if (lineEnd == std::string::npos) {
            lineEnd = head.size();
        }
        const std::string line = head.substr(start, lineEnd - start);
        start = lineEnd + 2;
        if (first) {
            out.start = line;
            first = false;
            continue;
        }
        const std::size_t colon = line.find(':');
        if (colon == std::string::npos) {
            continue;
        }
        std::size_t value = colon + 1;
        while (value < line.size() && line[value] == ' ') {
            ++value;
        }
        out.headers.emplace_back(line.substr(0, colon), line.substr(value));
    }
    std::uint64_t length = 0;
    if (const auto declared = Https::headerValue(out.headers, "Content-Length")) {
        length = Https::parseUnsigned(*declared).value_or(0);
    }
    while (buffer.size() < length) {
        if (!stream.await(running)) {
            return false;
        }
        const long long n = stream.readSome(chunk, sizeof(chunk));
        if (n <= 0) {
            return false;
        }
        buffer.append(chunk, static_cast<std::size_t>(n));
    }
    out.body.assign(buffer.begin(), buffer.begin() + static_cast<std::ptrdiff_t>(length));
    buffer.erase(0, static_cast<std::size_t>(length));
    return true;
}

/// A message on the wire, its length stated once.
std::string serialize(const Message &message) {
    std::string out = message.start + "\r\n";
    for (const auto &header : message.headers) {
        const std::string &name = header.first;
        if (Https::headerValue({header}, "Content-Length") || Https::headerValue({header}, "Connection")) {
            continue;
        }
        out += name + ": " + header.second + "\r\n";
    }
    out += "Content-Length: " + std::to_string(message.body.size()) + "\r\n\r\n";
    out.append(message.body.begin(), message.body.end());
    return out;
}

const char *reasonPhrase(std::uint16_t status) {
    switch (status) {
    case 200:
        return "OK";
    case 400:
        return "Bad Request";
    case 401:
        return "Unauthorized";
    case 404:
        return "Not Found";
    case 405:
        return "Method Not Allowed";
    case 413:
        return "Content Too Large";
    case 429:
        return "Too Many Requests";
    case 503:
        return "Service Unavailable";
    default:
        return "Status";
    }
}

// ── TLS material ─────────────────────────────────────────────────────────────

struct KeyFree {
    void operator()(EVP_PKEY *key) const {
        EVP_PKEY_free(key);
    }
};

struct CertFree {
    void operator()(X509 *cert) const {
        X509_free(cert);
    }
};

struct ContextFree {
    void operator()(SSL_CTX *context) const {
        SSL_CTX_free(context);
    }
};

using Key = std::unique_ptr<EVP_PKEY, KeyFree>;
using Cert = std::unique_ptr<X509, CertFree>;
using Context = std::unique_ptr<SSL_CTX, ContextFree>;

Key newKey() {
    EVP_PKEY_CTX *context = EVP_PKEY_CTX_new_id(EVP_PKEY_EC, nullptr);
    EVP_PKEY *key = nullptr;
    if (context != nullptr && EVP_PKEY_keygen_init(context) > 0 &&
        EVP_PKEY_CTX_set_ec_paramgen_curve_nid(context, NID_X9_62_prime256v1) > 0) {
        if (EVP_PKEY_keygen(context, &key) <= 0) {
            key = nullptr;
        }
    }
    EVP_PKEY_CTX_free(context);
    return Key(key);
}

bool addExtension(X509 *cert, X509 *issuer, int nid, const char *value) {
    X509V3_CTX context;
    X509V3_set_ctx_nodb(&context);
    X509V3_set_ctx(&context, issuer, cert, nullptr, nullptr, 0);
    X509_EXTENSION *extension = X509V3_EXT_conf_nid(nullptr, &context, nid, value);
    if (extension == nullptr) {
        return false;
    }
    const bool added = X509_add_ext(cert, extension, -1) == 1;
    X509_EXTENSION_free(extension);
    return added;
}

/// A certificate for `key`, issued by `issuer` with `issuerKey` (itself when
/// `issuer` is null): an authority, or a server for the kit's names.
Cert issue(const char *commonName, EVP_PKEY *key, X509 *issuer, EVP_PKEY *issuerKey, bool authority) {
    Cert cert(X509_new());
    if (!cert || key == nullptr || issuerKey == nullptr) {
        return Cert();
    }
    X509_set_version(cert.get(), 2);
    unsigned char serial[8];
    if (RAND_bytes(serial, static_cast<int>(sizeof(serial))) != 1) {
        return Cert();
    }
    serial[0] &= 0x7f;
    BIGNUM *number = BN_bin2bn(serial, static_cast<int>(sizeof(serial)), nullptr);
    BN_to_ASN1_INTEGER(number, X509_get_serialNumber(cert.get()));
    BN_free(number);
    X509_gmtime_adj(X509_getm_notBefore(cert.get()), -3600);
    X509_gmtime_adj(X509_getm_notAfter(cert.get()), 7L * 24 * 3600);
    X509_set_pubkey(cert.get(), key);
    X509_NAME *name = X509_get_subject_name(cert.get());
    X509_NAME_add_entry_by_txt(name, "CN", MBSTRING_ASC, reinterpret_cast<const unsigned char *>(commonName), -1, -1,
                               0);
    X509 *signer = issuer != nullptr ? issuer : cert.get();
    X509_set_issuer_name(cert.get(), X509_get_subject_name(signer));
    bool ok = addExtension(cert.get(), signer, NID_subject_key_identifier, "hash") &&
              addExtension(cert.get(), signer, NID_authority_key_identifier, "keyid:always");
    if (authority) {
        ok = ok && addExtension(cert.get(), signer, NID_basic_constraints, "critical,CA:TRUE") &&
             addExtension(cert.get(), signer, NID_key_usage, "critical,keyCertSign,cRLSign");
    } else {
        ok = ok && addExtension(cert.get(), signer, NID_basic_constraints, "critical,CA:FALSE") &&
             addExtension(cert.get(), signer, NID_key_usage, "critical,digitalSignature") &&
             addExtension(cert.get(), signer, NID_ext_key_usage, "serverAuth") &&
             addExtension(cert.get(), signer, NID_subject_alt_name, "DNS:localhost,DNS:sce-kit.test,IP:127.0.0.1");
    }
    if (!ok || X509_sign(cert.get(), issuerKey, EVP_sha256()) <= 0) {
        return Cert();
    }
    return cert;
}

bool writePem(const std::filesystem::path &path, X509 *cert) {
    BIO *file = BIO_new_file(path.string().c_str(), "wb");
    if (file == nullptr) {
        return false;
    }
    const bool written = PEM_write_bio_X509(file, cert) == 1;
    BIO_free(file);
    return written;
}

bool writeDer(const std::filesystem::path &path, X509 *cert) {
    const int length = i2d_X509(cert, nullptr);
    if (length <= 0) {
        return false;
    }
    std::vector<unsigned char> der(static_cast<std::size_t>(length));
    unsigned char *cursor = der.data();
    i2d_X509(cert, &cursor);
    std::ofstream out(path, std::ios::binary);
    out.write(reinterpret_cast<const char *>(der.data()), static_cast<std::streamsize>(der.size()));
    return static_cast<bool>(out);
}

Context serverContext(X509 *cert, EVP_PKEY *key) {
    Context context(SSL_CTX_new(TLS_server_method()));
    if (!context) {
        return context;
    }
    SSL_CTX_set_min_proto_version(context.get(), TLS1_2_VERSION);
    if (SSL_CTX_use_certificate(context.get(), cert) != 1 || SSL_CTX_use_PrivateKey(context.get(), key) != 1) {
        return Context();
    }
    return context;
}

Context clientContext(X509 *trusted) {
    Context context(SSL_CTX_new(TLS_client_method()));
    if (!context) {
        return context;
    }
    SSL_CTX_set_min_proto_version(context.get(), TLS1_2_VERSION);
    X509_STORE_add_cert(SSL_CTX_get_cert_store(context.get()), trusted);
    SSL_CTX_set_verify(context.get(), SSL_VERIFY_PEER, nullptr);
    return context;
}

// ── Servers ──────────────────────────────────────────────────────────────────

/// Accepts connections on 127.0.0.1 and hands each to `handle` on a thread of
/// its own, until stopped.
class Acceptor {
public:
    using Handler = std::function<void(Socket, const std::atomic<bool> &)>;

    explicit Acceptor(Handler handle) : handle_(std::move(handle)) {
        listener_ = listenLoopback(port_);
        if (listener_ != kNoSocket) {
            accepting_ = std::thread([this] { acceptLoop(); });
        }
    }

    ~Acceptor() {
        stop();
    }

    Acceptor(const Acceptor &) = delete;
    Acceptor &operator=(const Acceptor &) = delete;

    bool valid() const {
        return listener_ != kNoSocket;
    }

    std::uint16_t port() const {
        return port_;
    }

    void stop() {
        if (!running_.exchange(false)) {
            return;
        }
        if (accepting_.joinable()) {
            accepting_.join();
        }
        for (auto &connection : connections_) {
            if (connection.joinable()) {
                connection.join();
            }
        }
        if (listener_ != kNoSocket) {
            closeSocket(listener_);
            listener_ = kNoSocket;
        }
    }

private:
    void acceptLoop() {
        while (running_.load()) {
            if (!readable(listener_, 100)) {
                continue;
            }
            const Socket connection = accept(listener_, nullptr, nullptr);
            if (connection == kNoSocket) {
                continue;
            }
            boundReceive(connection);
            connections_.emplace_back([this, connection] {
                handle_(connection, running_);
                closeSocket(connection);
            });
        }
    }

    Handler handle_;
    Socket listener_ = kNoSocket;
    std::uint16_t port_ = 0;
    std::atomic<bool> running_{true};
    std::thread accepting_;
    std::vector<std::thread> connections_;
};

/// The binding's wire over TLS: a Mailbox behind a socket.
class MailboxServer {
public:
    MailboxServer(Mailbox &mailbox, SSL_CTX *context)
        : mailbox_(mailbox), context_(context),
          acceptor_([this](Socket s, const std::atomic<bool> &running) { serve(s, running); }) {}

    ~MailboxServer() {
        mailbox_.wakeAll();
        acceptor_.stop();
    }

    bool valid() const {
        return acceptor_.valid();
    }

    std::uint16_t port() const {
        return acceptor_.port();
    }

private:
    void serve(Socket s, const std::atomic<bool> &running) {
        SSL *ssl = SSL_new(context_);
        if (ssl == nullptr) {
            return;
        }
        SSL_set_fd(ssl, static_cast<int>(s));
        TlsStream stream(s, ssl);
        if (SSL_accept(ssl) != 1) {
            return;
        }
        std::string buffer;
        Message request;
        while (readMessage(stream, buffer, running, request)) {
            ServerRequest serverRequest;
            const std::size_t space = request.start.find(' ');
            const std::size_t second = request.start.find(' ', space + 1);
            serverRequest.method = request.start.substr(0, space);
            serverRequest.target = request.start.substr(space + 1, second - space - 1);
            serverRequest.headers = request.headers;
            serverRequest.body = request.body;
            const ServerAnswer answer = mailbox_.answer(serverRequest, [&running] { return !running.load(); });
            if (answer.kind == ServerAnswer::Kind::Drop) {
                // The server acted; the response is lost with the connection.
                return;
            }
            Message response;
            response.start = "HTTP/1.1 " + std::to_string(answer.status) + " " + reasonPhrase(answer.status);
            response.headers = answer.headers;
            response.body = answer.body;
            if (!stream.writeAll(serialize(response))) {
                return;
            }
        }
    }

    Mailbox &mailbox_;
    SSL_CTX *context_;
    Acceptor acceptor_;
};

/// Read a CONNECT request and answer it: the authority asked for, or empty
/// when the request was none.
std::string takeConnect(Stream &stream, std::string &buffer, const std::atomic<bool> &running) {
    Message request;
    if (!readMessage(stream, buffer, running, request) || request.start.rfind("CONNECT ", 0) != 0) {
        return std::string();
    }
    const std::size_t end = request.start.find(' ', 8);
    const std::string authority = request.start.substr(8, end - 8);
    if (!stream.writeAll("HTTP/1.1 200 Connection established\r\n\r\n")) {
        return std::string();
    }
    return authority;
}

/// The port of `host:port`.
std::uint16_t portOf(const std::string &authority) {
    const std::size_t colon = authority.rfind(':');
    const auto port = colon == std::string::npos ? std::nullopt : Https::parseUnsigned(authority.substr(colon + 1));
    return port && *port <= 65535 ? static_cast<std::uint16_t>(*port) : 0;
}

/// An explicit proxy: CONNECT to any name, tunnelled to that port on
/// 127.0.0.1, its bytes carried as they are.
class ConnectProxy {
public:
    ConnectProxy() : acceptor_([this](Socket s, const std::atomic<bool> &running) { serve(s, running); }) {}

    bool valid() const {
        return acceptor_.valid();
    }

    std::uint16_t port() const {
        return acceptor_.port();
    }

    std::vector<std::string> tunnels() const {
        std::lock_guard<std::mutex> lock(mutex_);
        return tunnels_;
    }

private:
    void serve(Socket client, const std::atomic<bool> &running) {
        PlainStream stream(client);
        std::string buffer;
        const std::string authority = takeConnect(stream, buffer, running);
        if (authority.empty()) {
            return;
        }
        {
            std::lock_guard<std::mutex> lock(mutex_);
            tunnels_.push_back(authority);
        }
        const Socket upstream = connectLoopback(portOf(authority));
        if (upstream == kNoSocket) {
            return;
        }
        if (!buffer.empty() && sendSome(upstream, buffer.data(), buffer.size()) <= 0) {
            closeSocket(upstream);
            return;
        }
        char chunk[8192];
        while (running.load()) {
            PollFd fds[2] = {};
            fds[0].fd = client;
            fds[0].events = POLLIN;
            fds[1].fd = upstream;
            fds[1].events = POLLIN;
            if (pollSockets(fds, 2, 100) <= 0) {
                continue;
            }
            bool open = true;
            for (int i = 0; i < 2 && open; ++i) {
                if ((fds[i].revents & (POLLIN | POLLHUP | POLLERR)) == 0) {
                    continue;
                }
                const Socket from = i == 0 ? client : upstream;
                const Socket to = i == 0 ? upstream : client;
                const long long n = receiveSome(from, chunk, sizeof(chunk));
                open = n > 0 && sendSome(to, chunk, static_cast<std::size_t>(n)) > 0;
            }
            if (!open) {
                break;
            }
        }
        closeSocket(upstream);
    }

    mutable std::mutex mutex_;
    std::vector<std::string> tunnels_;
    Acceptor acceptor_;
};

/// A proxy that inspects TLS: it takes CONNECT, ends the client's TLS with
/// the network authority's certificate, reads each request and carries it to
/// the server over TLS of its own, trusting the server's authority.
class InspectingProxy {
public:
    InspectingProxy(SSL_CTX *towardClient, SSL_CTX *towardServer)
        : toward_client_(towardClient), toward_server_(towardServer),
          acceptor_([this](Socket s, const std::atomic<bool> &running) { serve(s, running); }) {}

    bool valid() const {
        return acceptor_.valid();
    }

    std::uint16_t port() const {
        return acceptor_.port();
    }

    std::vector<std::string> inspected() const {
        std::lock_guard<std::mutex> lock(mutex_);
        return inspected_;
    }

private:
    void serve(Socket client, const std::atomic<bool> &running) {
        std::string authority;
        {
            PlainStream plain(client);
            std::string buffer;
            authority = takeConnect(plain, buffer, running);
            if (authority.empty() || !buffer.empty()) {
                return;
            }
        }
        SSL *clientSsl = SSL_new(toward_client_);
        if (clientSsl == nullptr) {
            return;
        }
        SSL_set_fd(clientSsl, static_cast<int>(client));
        TlsStream fromClient(client, clientSsl);
        if (SSL_accept(clientSsl) != 1) {
            return;
        }
        const Socket upstream = connectLoopback(portOf(authority));
        if (upstream == kNoSocket) {
            return;
        }
        const std::string host = authority.substr(0, authority.rfind(':'));
        SSL *serverSsl = SSL_new(toward_server_);
        if (serverSsl == nullptr) {
            closeSocket(upstream);
            return;
        }
        SSL_set_fd(serverSsl, static_cast<int>(upstream));
        SSL_set_tlsext_host_name(serverSsl, host.c_str());
        SSL_set1_host(serverSsl, host.c_str());
        {
            TlsStream toServer(upstream, serverSsl);
            if (SSL_connect(serverSsl) == 1) {
                relay(fromClient, toServer, running);
            }
        }
        closeSocket(upstream);
    }

    void relay(Stream &fromClient, Stream &toServer, const std::atomic<bool> &running) {
        std::string clientBuffer;
        std::string serverBuffer;
        Message request;
        while (readMessage(fromClient, clientBuffer, running, request)) {
            {
                std::lock_guard<std::mutex> lock(mutex_);
                const std::size_t space = request.start.find(' ');
                inspected_.push_back(request.start.substr(space + 1, request.start.find(' ', space + 1) - space - 1));
            }
            Message response;
            if (!toServer.writeAll(serialize(request)) || !readMessage(toServer, serverBuffer, running, response) ||
                !fromClient.writeAll(serialize(response))) {
                return;
            }
        }
    }

    SSL_CTX *toward_client_;
    SSL_CTX *toward_server_;
    mutable std::mutex mutex_;
    std::vector<std::string> inspected_;
    Acceptor acceptor_;
};

// ── What a case runs ─────────────────────────────────────────────────────────

class KitCredentials final : public Https::Credentials {
public:
    std::optional<std::string> bearerToken(const std::string &) override {
        return std::string(kToken);
    }

    std::optional<Https::InstallSecret> loadInstallSecret(const std::string &) override {
        return std::nullopt;
    }

    bool storeInstallSecret(const std::string &, const Https::InstallSecret &) override {
        return false;
    }
};

/// What a client told the case.
class Recorder {
public:
    Https::ClientEvents events() {
        Https::ClientEvents e;
        e.receive = [this](MeshEnvelope env) {
            std::lock_guard<std::mutex> lock(mutex_);
            received_.push_back(std::move(env));
        };
        e.ready = [this] {
            std::lock_guard<std::mutex> lock(mutex_);
            ++ready_;
        };
        e.notReady = [](CommunicationError) {};
        e.fault = [this](CommunicationError err) {
            std::lock_guard<std::mutex> lock(mutex_);
            faults_.push_back(std::move(err));
        };
        return e;
    }

    std::size_t received() const {
        std::lock_guard<std::mutex> lock(mutex_);
        return received_.size();
    }

    int ready() const {
        std::lock_guard<std::mutex> lock(mutex_);
        return ready_;
    }

    std::size_t faults() const {
        std::lock_guard<std::mutex> lock(mutex_);
        return faults_.size();
    }

private:
    mutable std::mutex mutex_;
    std::vector<MeshEnvelope> received_;
    int ready_ = 0;
    std::vector<CommunicationError> faults_;
};

/// Wait until `done` holds, at most the kit's patience.
bool eventually(const std::function<bool()> &done, std::chrono::milliseconds bound = kPatience) {
    const auto until = std::chrono::steady_clock::now() + bound;
    while (!done()) {
        if (std::chrono::steady_clock::now() >= until) {
            return false;
        }
        std::this_thread::sleep_for(20ms);
    }
    return true;
}

MeshEnvelope kitEnvelope(const std::string &type, std::uint8_t tag) {
    MeshEnvelope env;
    env.id[0] = tag;
    env.id[15] = 0x6b;
    env.source = kClient;
    env.type = type;
    return env;
}

HttpsRequest exchangeRequest(const std::string &base, std::vector<std::uint8_t> body,
                             std::chrono::milliseconds timeout) {
    HttpsRequest request;
    request.url = Https::exchangeUrl(base, kClient);
    request.headers.emplace_back("Content-Type", Https::kContentType);
    request.headers.emplace_back("Accept", Https::kContentType);
    request.headers.emplace_back(Https::kAckHeader, "0");
    request.headers.emplace_back(Https::kHoldHeader, "0");
    request.headers.emplace_back("Authorization", std::string("Bearer ") + kToken);
    request.body = std::move(body);
    request.timeout = timeout;
    return request;
}

std::string describe(const HttpsExchange &answer) {
    if (answer.kind == HttpsExchange::Kind::Response) {
        return "a response " + std::to_string(answer.status);
    }
    return std::string("a failure (") + failureName(answer.failure) + "): " + answer.detail;
}

CaseResult pass() {
    return CaseResult{true, std::string()};
}

CaseResult fail(std::string why) {
    return CaseResult{false, std::move(why)};
}

MailboxOptions kitMailbox() {
    MailboxOptions options;
    options.client = kClient;
    options.bearer_token = kToken;
    options.max_hold = 2000ms;
    return options;
}

}  // namespace

struct Kit::Impl {
#if defined(_WIN32)
    Winsock winsock;
#endif
    std::filesystem::path directory;
    Key server_ca_key;
    Cert server_ca;
    Key server_key;
    Cert server_cert;
    Key network_ca_key;
    Cert network_ca;
    Key network_key;
    Cert network_cert;
    Context server_context;
    Context network_context;
    Context proxy_upstream_context;
    bool made = false;

    Impl() {
        std::uint64_t tag = 0;
        RAND_bytes(reinterpret_cast<unsigned char *>(&tag), static_cast<int>(sizeof(tag)));
        directory = std::filesystem::temp_directory_path() / ("sce-https-kit-" + std::to_string(tag));
        std::error_code error;
        std::filesystem::create_directories(directory, error);
        if (error) {
            return;
        }
        server_ca_key = newKey();
        server_ca = issue("SCE https kit server authority", server_ca_key.get(), nullptr, server_ca_key.get(), true);
        server_key = newKey();
        server_cert = issue("localhost", server_key.get(), server_ca.get(), server_ca_key.get(), false);
        network_ca_key = newKey();
        network_ca =
            issue("SCE https kit inspecting network", network_ca_key.get(), nullptr, network_ca_key.get(), true);
        network_key = newKey();
        network_cert = issue("localhost", network_key.get(), network_ca.get(), network_ca_key.get(), false);
        if (!server_ca || !server_cert || !network_ca || !network_cert) {
            return;
        }
        server_context = serverContext(server_cert.get(), server_key.get());
        network_context = serverContext(network_cert.get(), network_key.get());
        proxy_upstream_context = clientContext(server_ca.get());
        made = server_context && network_context && proxy_upstream_context &&
               writePem(directory / "server-ca.pem", server_ca.get()) &&
               writeDer(directory / "server-ca.der", server_ca.get()) &&
               writePem(directory / "network-ca.pem", network_ca.get()) &&
               writeDer(directory / "network-ca.der", network_ca.get());
    }

    ~Impl() {
        std::error_code error;
        std::filesystem::remove_all(directory, error);
    }

    Network trustingServer(const std::string &proxy) const {
        return Network{(directory / "server-ca.pem").string(), (directory / "server-ca.der").string(), proxy};
    }

    Network trustingNetwork(const std::string &proxy) const {
        return Network{(directory / "network-ca.pem").string(), (directory / "network-ca.der").string(), proxy};
    }
};

Kit::Kit() : impl_(std::make_unique<Impl>()) {}

Kit::~Kit() = default;

bool Kit::ready() const {
    return impl_->made;
}

// SCE_MESH.md §mesh-22.7: the cases every adapter is held to, one function
// each, each against servers and proxies on loopback ports of its own under
// the authorities the kit made.
CaseResult Kit::directExchange(AdapterFactory &factory) {
    Mailbox mailbox(kitMailbox());
    MailboxServer server(mailbox, impl_->server_context.get());
    if (!server.valid()) {
        return fail("the kit's server did not listen");
    }
    AdapterUnderTest adapter = factory.make(impl_->trustingServer(""));
    if (!adapter.transport) {
        return fail("the factory made no adapter");
    }
    KitCredentials credentials;
    Recorder recorder;
    Https::ClientConfig config;
    config.base_url = "https://localhost:" + std::to_string(server.port());
    config.client = kClient;
    config.peer = "cloud";
    config.target = "#cloud";
    config.hold = 1000ms;
    Https::Client client(config, Https::ClientHost{adapter.transport.get(), &credentials}, recorder.events());
    if (!client.start()) {
        return fail("the client did not start");
    }
    if (!eventually([&] { return recorder.ready() >= 1; })) {
        return fail("no exchange was answered 200 over the adapter");
    }
    mailbox.post(kitEnvelope("service.response.kit", 1));
    if (!eventually([&] { return recorder.received() == 1; })) {
        return fail("a mailbox envelope did not reach the client");
    }
    if (!client.send(kitEnvelope("service.request.kit", 2))) {
        return fail("the client refused a send");
    }
    if (!eventually([&] { return mailbox.received().size() == 1; })) {
        return fail("a send did not reach the server");
    }
    client.stop();
    if (recorder.faults() != 0) {
        return fail("the client raised a fault on a healthy network");
    }
    return pass();
}

CaseResult Kit::throughConnectProxy(AdapterFactory &factory) {
    Mailbox mailbox(kitMailbox());
    MailboxServer server(mailbox, impl_->server_context.get());
    ConnectProxy proxy;
    if (!server.valid() || !proxy.valid()) {
        return fail("the kit's server or proxy did not listen");
    }
    AdapterUnderTest adapter = factory.make(impl_->trustingServer("127.0.0.1:" + std::to_string(proxy.port())));
    // A name only the proxy resolves: an adapter that went direct cannot
    // reach the server at all.
    const std::string authority = std::string(kProxiedHost) + ":" + std::to_string(server.port());
    const HttpsExchange answer = adapter.transport->exchange(
        exchangeRequest("https://" + authority, Https::encodeBody({kitEnvelope("service.request.kit", 3)}), kPatience));
    if (answer.kind != HttpsExchange::Kind::Response || answer.status != 200) {
        return fail("through the proxy the adapter got " + describe(answer));
    }
    const auto tunnels = proxy.tunnels();
    bool tunnelled = false;
    for (const auto &tunnel : tunnels) {
        tunnelled = tunnelled || tunnel == authority;
    }
    if (!tunnelled) {
        return fail("the proxy saw no CONNECT for " + authority);
    }
    if (mailbox.received().size() != 1) {
        return fail("the envelope sent through the proxy did not reach the server");
    }
    return pass();
}

CaseResult Kit::throughInspectingProxy(AdapterFactory &factory) {
    Mailbox mailbox(kitMailbox());
    MailboxServer server(mailbox, impl_->server_context.get());
    InspectingProxy proxy(impl_->network_context.get(), impl_->proxy_upstream_context.get());
    if (!server.valid() || !proxy.valid()) {
        return fail("the kit's server or proxy did not listen");
    }
    // The platform trusts the network's authority, and not the server's: what
    // reaches the adapter is the proxy's certificate.
    AdapterUnderTest adapter = factory.make(impl_->trustingNetwork("127.0.0.1:" + std::to_string(proxy.port())));
    const std::string authority = std::string(kProxiedHost) + ":" + std::to_string(server.port());
    const HttpsExchange answer = adapter.transport->exchange(
        exchangeRequest("https://" + authority, Https::encodeBody({kitEnvelope("service.request.kit", 4)}), kPatience));
    if (answer.kind != HttpsExchange::Kind::Response || answer.status != 200) {
        return fail("through the inspecting proxy the adapter got " + describe(answer));
    }
    const auto inspected = proxy.inspected();
    const std::string path = std::string("/sce-mesh/") + Https::kBindingVersion + "/" + kClient;
    if (inspected.size() != 1 || inspected[0] != path) {
        return fail("the inspecting proxy did not read the exchange it carried");
    }
    if (mailbox.received().size() != 1) {
        return fail("the envelope sent through the inspecting proxy did not reach the server");
    }
    return pass();
}

CaseResult Kit::untrustedServerIsTls(AdapterFactory &factory) {
    Mailbox mailbox(kitMailbox());
    MailboxServer server(mailbox, impl_->server_context.get());
    if (!server.valid()) {
        return fail("the kit's server did not listen");
    }
    // The network's authority is trusted, and it did not issue the server's
    // certificate.
    AdapterUnderTest adapter = factory.make(impl_->trustingNetwork(""));
    const HttpsExchange answer = adapter.transport->exchange(
        exchangeRequest("https://localhost:" + std::to_string(server.port()), {}, kPatience));
    if (answer.kind != HttpsExchange::Kind::Failure || answer.failure != HttpsExchange::Failure::Tls) {
        return fail("a server the platform does not trust gave " + describe(answer));
    }
    if (mailbox.exchanges() != 0) {
        return fail("an exchange reached a server the platform does not trust");
    }
    return pass();
}

CaseResult Kit::droppedResponseAfterServerActed(AdapterFactory &factory) {
    Mailbox mailbox(kitMailbox());
    MailboxServer server(mailbox, impl_->server_context.get());
    if (!server.valid()) {
        return fail("the kit's server did not listen");
    }
    AdapterUnderTest adapter = factory.make(impl_->trustingServer(""));
    const std::string base = "https://localhost:" + std::to_string(server.port());

    // The adapter: no response is a failure, though the server took it in. A
    // stack that carries a request again on a fresh connection when a reused
    // one closed before any response (WinHTTP may) answers with that second
    // exchange's response instead; then the server took the body in twice,
    // which the binding allows (the receiver's window drops the copy). What
    // no adapter may do is report a response the server never sent.
    mailbox.dropAfterActing(1);
    const auto body = Https::encodeBody({kitEnvelope("service.request.kit", 5)});
    const HttpsExchange lost = adapter.transport->exchange(exchangeRequest(base, body, kPatience));
    const std::size_t takenIn = mailbox.received().size();
    if (lost.kind == HttpsExchange::Kind::Failure) {
        if (takenIn != 1) {
            return fail("the server did not take in the exchange whose response it dropped");
        }
    } else if (lost.status != 200 || takenIn != 2) {
        return fail("a response the server dropped gave " + describe(lost) + " with " + std::to_string(takenIn) +
                    " copies taken in");
    }
    const HttpsExchange again = adapter.transport->exchange(exchangeRequest(base, body, kPatience));
    if (again.kind != HttpsExchange::Kind::Response || again.status != 200) {
        return fail("the exchange after a dropped one gave " + describe(again));
    }

    // The client over the adapter: what the lost response carried is carried
    // again and delivered once.
    KitCredentials credentials;
    Recorder recorder;
    Https::ClientConfig config;
    config.base_url = base;
    config.client = kClient;
    config.peer = "cloud";
    config.target = "#cloud";
    config.hold = 1000ms;
    Https::Client client(config, Https::ClientHost{adapter.transport.get(), &credentials}, recorder.events());
    if (!client.start() || !eventually([&] { return recorder.ready() >= 1; })) {
        return fail("the client over the adapter did not become ready");
    }
    mailbox.dropAfterActing(1);
    mailbox.post(kitEnvelope("service.response.kit", 6));
    if (!eventually([&] { return recorder.received() == 1 && recorder.ready() >= 2; })) {
        return fail("what a lost response carried was not delivered after it");
    }
    std::this_thread::sleep_for(1500ms);
    client.stop();
    if (recorder.received() != 1) {
        return fail("what a lost response carried was delivered " + std::to_string(recorder.received()) + " times");
    }
    return pass();
}

CaseResult Kit::heldPastBound(AdapterFactory &factory) {
    Mailbox mailbox(kitMailbox());
    MailboxServer server(mailbox, impl_->server_context.get());
    if (!server.valid()) {
        return fail("the kit's server did not listen");
    }
    AdapterUnderTest adapter = factory.make(impl_->trustingServer(""));
    mailbox.holdPastBound(5000ms);
    const auto started = std::chrono::steady_clock::now();
    const HttpsExchange answer =
        adapter.transport->exchange(exchangeRequest("https://localhost:" + std::to_string(server.port()), {}, 1000ms));
    const auto took = std::chrono::steady_clock::now() - started;
    if (answer.kind != HttpsExchange::Kind::Failure || answer.failure != HttpsExchange::Failure::Timeout) {
        return fail("an exchange held past its bound gave " + describe(answer));
    }
    if (took > 3000ms) {
        return fail("an exchange bound to 1 s ended after " +
                    std::to_string(std::chrono::duration_cast<std::chrono::milliseconds>(took).count()) + " ms");
    }
    return pass();
}

}  // namespace SCE::Mesh::HttpsKit
