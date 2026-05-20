// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh §16.8.3 reference transport: TCP loopback with length-prefixed
// CBOR envelope framing. Zero external dependencies — POSIX sockets only.
//
// Wire format (per direction, every envelope):
//   [4 bytes payload_length, network byte order]
//   [payload_length bytes CBOR-encoded MeshEnvelope]
//
// Conformance (SCE Mesh §10.4):
//   * Per-sender FIFO     — TCP stream guarantees, single connection per sender
//   * At-least-once       — TCP reliable delivery
//   * Duplicate tolerance — single TCP stream cannot duplicate; supplies_dedup
//   * Fault signal        — peer close / read EOF / send error visible to caller
//                           (E2 error.communication catalog: out of scope this
//                           session — see Item 7 in the Session E2 plan)
//
// Threading model:
//   * Server: one accept thread + one read thread per accepted connection
//   * Client: one read thread per outbound connection (full duplex)
//   * Engine ownership: the receive callback is invoked on a transport
//     callback thread; the dispatcher (TransportRouter::dispatchToSender)
//     guards engine state per the existing model used by SOME/IP/Zenoh
//
// Lifetime contract: clients are constructed with their connect endpoint
// and dial lazily on first send (avoids ordering coupling between server
// init() and client init()). The server bind happens in the ctor so the
// listen socket is ready before peers attempt to connect.

#pragma once

#include "mesh/MeshEnvelope.h"
#include "mesh/MeshEnvelopeCodec.h"

#include <arpa/inet.h>
#include <atomic>
#include <cerrno>
#include <charconv>
#include <chrono>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <functional>
#include <mutex>
#include <optional>
#include <netinet/in.h>
#include <netinet/tcp.h>
#include <string>
#include <sys/socket.h>
#include <sys/types.h>
#include <thread>
#include <unistd.h>
#include <unordered_map>
#include <utility>
#include <vector>

namespace SCE::Mesh::CustomTcp {

/// Receive callback signature: invoked on a transport thread once per
/// successfully decoded envelope. The callback owns dispatch to the
/// engine; this layer does no policy interpretation.
using ReceiveCallback = std::function<void(const SCE::Mesh::MeshEnvelope&)>;

/// Decode-error callback signature: invoked on a per-connection read
/// thread when an inbound frame's CBOR decode fails. Codegen wires
/// this to `raiseCommunicationError(ENVELOPE_CORRUPT,
/// transport="custom_tcp")` so the §16.7 row 4 catalog row fires at
/// this hand-written endpoint tier the same way codegen
/// `decodeEnvelope` sites do. Distinct from a stream-level fault
/// (`ReadResult::SocketClosed`) which tears down the connection
/// silently — a single bad envelope leaves the connection alive.
using DecodeErrorCallback = std::function<void()>;

/// Runtime override for endpoints the generated TransportRouter::init()
/// would otherwise take from codegen-baked deploy.yaml values. The
/// two-process cross-device harness populates this before calling
/// init() so the dialing side connects to the listener's kernel-
/// assigned ephemeral port instead of the deploy.yaml placeholder.
/// Default-constructed means "no overrides, use codegen values".
struct PortOverride {
    /// Map from peer name (the key of deploy.yaml `bindings["#<peer>"]`)
    /// to the `"host:port"` endpoint that should replace the codegen-
    /// baked `peer.connect_endpoint`. Only custom_tcp peers honour it;
    /// other transports ignore entries silently.
    std::unordered_map<std::string, std::string> peer_connect_endpoints;
};

namespace detail {

/// Parse `host:port` (IPv4 only). Returns false on malformed input or a
/// port outside [0, 65535]. Port 0 is the BSD-sockets sentinel that asks
/// the kernel to pick an ephemeral port at bind() time; listen-side
/// callers read the assigned port back via `Server::local_endpoint()`.
/// On the connect side a literal `:0` parses but ::connect then fails
/// cleanly (no peer listens on 0), so Client::connect returns false
/// without a silent fault. Caller is responsible for ensuring the host
/// portion is an IPv4 dotted quad — the harness reference transport is
/// loopback-only by design (SCE_MESH.md §16.8.3).
///
/// Strict numeric parsing: `from_chars` requires the entire port slice
/// to be consumed, so `"127.0.0.1:8080abc"` and `"127.0.0.1:8080 "` are
/// rejected at parse time rather than silently truncating to 8080 (the
/// behaviour `std::stoi` would have).
inline bool parse_endpoint(const std::string& endpoint, sockaddr_in& out) {
    auto colon = endpoint.find(':');
    if (colon == std::string::npos || colon == 0 || colon + 1 >= endpoint.size()) {
        return false;
    }
    const char* port_begin = endpoint.data() + colon + 1;
    const char* port_end = endpoint.data() + endpoint.size();
    int port = 0;
    auto [parse_end, ec] = std::from_chars(port_begin, port_end, port);
    if (ec != std::errc{} || parse_end != port_end) return false;
    if (port < 0 || port > 65535) return false;
    std::string host = endpoint.substr(0, colon);
    std::memset(&out, 0, sizeof(out));
    out.sin_family = AF_INET;
    out.sin_port = htons(static_cast<uint16_t>(port));
    if (::inet_pton(AF_INET, host.c_str(), &out.sin_addr) != 1) {
        return false;
    }
    return true;
}

/// Read exactly `n` bytes from `fd` into `buf`. TCP is a stream protocol
/// — `recv` may return short reads; callers that need a full record must
/// loop. Returns false on EOF (peer closed) or unrecoverable error so the
/// reader thread can exit cleanly. `EINTR` is retried (signal during a
/// blocking read is not a transport fault).
inline bool read_exact(int fd, void* buf, std::size_t n) {
    auto* p = static_cast<unsigned char*>(buf);
    while (n > 0) {
        ssize_t got = ::recv(fd, p, n, 0);
        if (got == 0) return false;          // EOF: peer closed cleanly
        if (got < 0) {
            if (errno == EINTR) continue;
            return false;
        }
        p += got;
        n -= static_cast<std::size_t>(got);
    }
    return true;
}

/// Write exactly `n` bytes from `buf` to `fd`. Mirrors `read_exact` for
/// writers. `MSG_NOSIGNAL` suppresses SIGPIPE so a broken peer surfaces
/// as `EPIPE` to the caller rather than terminating the process.
inline bool write_exact(int fd, const void* buf, std::size_t n) {
    const auto* p = static_cast<const unsigned char*>(buf);
    while (n > 0) {
        ssize_t put = ::send(fd, p, n, MSG_NOSIGNAL);
        if (put <= 0) {
            if (put < 0 && errno == EINTR) continue;
            return false;
        }
        p += put;
        n -= static_cast<std::size_t>(put);
    }
    return true;
}

/// Read one length-prefixed envelope from `fd` and decode it. Returns
/// false on EOF, decode failure, or socket error so the caller exits the
/// receive loop.
///
/// `scratch` is a caller-owned reusable buffer. The reader threads
/// (Server::readLoop, Client::readLoop) hold one per thread and reuse
/// it across iterations, so a steady-state stream of envelopes does not
/// allocate per receive. `vector::resize` keeps any prior capacity, so
/// the buffer grows monotonically toward the largest seen envelope and
/// then becomes amortized-free for subsequent reads of equal-or-smaller
/// size.
/// Three-state result for an inbound frame attempt: a successful decode
/// (continue the loop), a clean socket close (exit the loop, no error),
/// or a CBOR decode failure on a successfully-read frame (continue the
/// loop AFTER raising §16.7 row 4 ENVELOPE_CORRUPT). The
/// SocketClosed/StreamFramingError split lets the read loop continue
/// on a recoverable decode error rather than dropping the connection,
/// while keeping the catalog-row raise distinguishable from a peer
/// hangup. A framing-level fault (length validation, partial read)
/// is reported as SocketClosed because the stream is no longer
/// trustable — the loop must tear down.
enum class ReadResult {
    Ok,
    SocketClosed,
    DecodeError,
};

inline ReadResult read_envelope(int fd, SCE::Mesh::MeshEnvelope& out,
                                std::vector<uint8_t>& scratch) {
    uint32_t net_len = 0;
    if (!read_exact(fd, &net_len, sizeof(net_len))) return ReadResult::SocketClosed;
    uint32_t len = ntohl(net_len);
    // Cap at the SCE-mesh wire ceiling exported by MeshEnvelopeCodec.h.
    // Rebinding the literal here would be ownership-inversion via
    // co-declaration with the CBOR decoder's identical check —
    // consume the single source instead so any future spec change
    // moves both sites together.
    if (len == 0 ||
        static_cast<std::size_t>(len) > ::SCE::Mesh::kMaxEnvelopeBytes) {
        return ReadResult::SocketClosed;
    }
    scratch.resize(len);
    if (!read_exact(fd, scratch.data(), len)) return ReadResult::SocketClosed;
    return SCE::Mesh::decodeEnvelope(scratch.data(), scratch.size(), out)
               ? ReadResult::Ok
               : ReadResult::DecodeError;
}

/// Encode `env` and write it as a length-prefixed frame on `fd`.
inline bool write_envelope(int fd, const SCE::Mesh::MeshEnvelope& env) {
    auto buf = SCE::Mesh::encodeEnvelope(env);
    if (buf.empty()) return false;
    uint32_t net_len = htonl(static_cast<uint32_t>(buf.size()));
    if (!write_exact(fd, &net_len, sizeof(net_len))) return false;
    return write_exact(fd, buf.data(), buf.size());
}

}  // namespace detail

/// TCP server: binds at construction, accepts connections in a background
/// thread, spawns one read thread per accepted connection. Each decoded
/// envelope is forwarded to `on_receive` on the read thread.
///
/// Construction succeeds iff bind+listen succeed. Use `valid()` to check
/// before relying on the server. Destruction joins all threads after
/// shutting down the listener and any open connections.
class Server {
public:
    Server(const std::string& listen_endpoint, ReceiveCallback on_receive)
        : on_receive_(std::move(on_receive)) {
        sockaddr_in addr{};
        if (!detail::parse_endpoint(listen_endpoint, addr)) {
            return;  // valid_ stays false
        }
        listen_fd_ = ::socket(AF_INET, SOCK_STREAM | SOCK_CLOEXEC, 0);
        if (listen_fd_ < 0) return;
        // SO_REUSEADDR keeps tests deterministic across rapid teardowns
        // (a port in TIME_WAIT would otherwise reject the next bind).
        int one = 1;
        ::setsockopt(listen_fd_, SOL_SOCKET, SO_REUSEADDR, &one, sizeof(one));
        if (::bind(listen_fd_, reinterpret_cast<sockaddr*>(&addr), sizeof(addr)) != 0) {
            ::close(listen_fd_);
            listen_fd_ = -1;
            return;
        }
        if (::listen(listen_fd_, 16) != 0) {
            ::close(listen_fd_);
            listen_fd_ = -1;
            return;
        }
        valid_ = true;
        accept_thread_ = std::thread([this] { acceptLoop(); });
    }

    ~Server() { shutdown(); }

    Server(const Server&) = delete;
    Server& operator=(const Server&) = delete;

    [[nodiscard]] bool valid() const noexcept { return valid_; }

    /// Return the actually-bound `host:port` after bind. When the server
    /// was constructed with `host:0` the kernel assigns an ephemeral port
    /// at bind time; this getter surfaces the kernel's choice via
    /// `getsockname` so two-process harnesses can export the endpoint to
    /// peers at runtime instead of baking a static port into codegen.
    /// Returns `std::nullopt` when the server never became valid (bind or
    /// listen failed) or when `shutdown()` has already closed the listen
    /// fd. Not synchronized against `shutdown()` — a caller racing with
    /// teardown may observe a nullopt; serialise externally if a stable
    /// readback across teardown is required.
    [[nodiscard]] std::optional<std::string> local_endpoint() const {
        if (!valid_ || listen_fd_ < 0) return std::nullopt;
        sockaddr_in addr{};
        socklen_t len = sizeof(addr);
        if (::getsockname(listen_fd_, reinterpret_cast<sockaddr*>(&addr), &len) != 0) {
            return std::nullopt;
        }
        if (addr.sin_family != AF_INET) return std::nullopt;
        char host[INET_ADDRSTRLEN] = {};
        if (::inet_ntop(AF_INET, &addr.sin_addr, host, sizeof(host)) == nullptr) {
            return std::nullopt;
        }
        std::string out(host);
        out.push_back(':');
        out.append(std::to_string(ntohs(addr.sin_port)));
        return out;
    }

    void shutdown() {
        bool expected = false;
        if (!stopping_.compare_exchange_strong(expected, true)) return;
        if (listen_fd_ >= 0) {
            // Closing the listen socket unblocks the accept thread; it
            // observes accept() failing with EBADF/EINVAL and exits.
            ::shutdown(listen_fd_, SHUT_RDWR);
            ::close(listen_fd_);
            listen_fd_ = -1;
        }
        if (accept_thread_.joinable()) accept_thread_.join();
        // Tear down accepted connections after the accept thread is gone
        // so the connection list cannot grow concurrently.
        std::vector<Connection> taken;
        {
            std::lock_guard<std::mutex> lock(connections_mutex_);
            taken = std::move(connections_);
        }
        for (auto& c : taken) {
            if (c.fd >= 0) {
                ::shutdown(c.fd, SHUT_RDWR);
                ::close(c.fd);
            }
            if (c.reader.joinable()) c.reader.join();
        }
    }

private:
    struct Connection {
        int fd = -1;
        std::thread reader;
    };

    void acceptLoop() {
        while (!stopping_.load()) {
            sockaddr_in peer{};
            socklen_t plen = sizeof(peer);
            int conn = ::accept(listen_fd_, reinterpret_cast<sockaddr*>(&peer), &plen);
            if (conn < 0) {
                if (stopping_.load()) return;
                if (errno == EINTR) continue;
                return;  // unrecoverable: listener is gone
            }
            // Disable Nagle to keep harness latencies deterministic; tests
            // measure end-to-end ordering, not throughput, so coalescing
            // would just add jitter.
            int one = 1;
            ::setsockopt(conn, IPPROTO_TCP, TCP_NODELAY, &one, sizeof(one));
            // Two-step construct so the reader thread sees `slot.fd` set
            // before it runs. emplace_back returns a stable reference for
            // the assignment; the lambda captures `conn` by value so a
            // future vector reallocation does not invalidate the fd it
            // reads (the slot itself is move-relocated, but the fd value
            // it carries was already copied into the closure).
            std::lock_guard<std::mutex> lock(connections_mutex_);
            auto& slot = connections_.emplace_back();
            slot.fd = conn;
            slot.reader = std::thread([this, conn] { readLoop(conn); });
        }
    }

    void readLoop(int fd) {
        SCE::Mesh::MeshEnvelope env;
        // Per-reader scratch buffer — see detail::read_envelope contract.
        // Lives on this thread's stack, so no synchronization needed and
        // it is freed automatically when the reader exits.
        std::vector<uint8_t> scratch;
        while (!stopping_.load()) {
            auto result = detail::read_envelope(fd, env, scratch);
            if (result == detail::ReadResult::SocketClosed) break;
            if (result == detail::ReadResult::DecodeError) {
                // §16.7 row 4: malformed CBOR on a successfully-read
                // frame. Raise then keep the connection alive — a
                // single bad envelope is recoverable; only a framing-
                // level fault (reported as SocketClosed) tears down
                // the stream.
                if (on_decode_error_) on_decode_error_();
                env = {};
                continue;
            }
            if (on_receive_) on_receive_(env);
            env = {};
        }
    }

public:
    /// Install the decode-error handler (§16.7 row 4). Invoked on
    /// per-connection read threads when an inbound frame decodes
    /// to malformed CBOR. Caller wires this to
    /// `raiseCommunicationError(ENVELOPE_CORRUPT, transport="custom_tcp")`.
    /// Per `Server`'s thread-safety contract, the handler MUST be
    /// installed BEFORE any client connects (i.e. before accept
    /// returns the first connection). May be left unset — in that
    /// case decode failures revert to the pre-row-4 silent-drop
    /// (still keeping the connection alive).
    void setDecodeErrorHandler(DecodeErrorCallback handler) {
        on_decode_error_ = std::move(handler);
    }

private:
    ReceiveCallback on_receive_;
    DecodeErrorCallback on_decode_error_;
    int listen_fd_ = -1;
    bool valid_ = false;
    std::atomic<bool> stopping_{false};
    std::thread accept_thread_;
    std::mutex connections_mutex_;
    std::vector<Connection> connections_;
};

/// TCP client: dials lazily on first send. Spawns a single read thread
/// for the duration of the connection so the same TCP stream carries
/// both outbound sends and any inbound traffic the peer may push back
/// (full duplex). FireForget is the only pattern in this session, but
/// the duplex reader is wired now so RPC patterns can be added without
/// reshuffling the lifecycle.
class Client {
public:
    Client(std::string connect_endpoint, ReceiveCallback on_receive)
        : connect_endpoint_(std::move(connect_endpoint)),
          on_receive_(std::move(on_receive)) {}

    ~Client() { shutdown(); }

    Client(const Client&) = delete;
    Client& operator=(const Client&) = delete;

    /// Replace the connect endpoint before the first `send()` has been
    /// issued. Intended for runtime overrides: deploy.yaml bakes a
    /// placeholder endpoint (often `"host:0"` for ephemeral peers) and
    /// the harness reassigns the actual endpoint after discovering the
    /// peer's listener. Returns false if a socket is already open
    /// (connect() already ran) or the client is shutting down; in
    /// either case the existing endpoint is preserved. Acquires the
    /// same `send_mutex_` that guards `connect()`, so races between
    /// this setter and a concurrent `send()` resolve deterministically
    /// — one side wins the lock and the other observes the decided
    /// state on entry.
    [[nodiscard]] bool set_connect_endpoint(std::string endpoint) {
        std::lock_guard<std::mutex> lock(send_mutex_);
        if (fd_ >= 0 || stopping_.load()) return false;
        connect_endpoint_ = std::move(endpoint);
        return true;
    }

    /// Encode and send the envelope. Connects on demand; subsequent calls
    /// reuse the same socket. Returns false on connect / send failure.
    [[nodiscard]] bool send(const SCE::Mesh::MeshEnvelope& env) {
        std::lock_guard<std::mutex> lock(send_mutex_);
        if (fd_ < 0 && !connect()) return false;
        if (!detail::write_envelope(fd_, env)) {
            // Tear down on first send error so a subsequent send() retries
            // cleanly rather than reusing a broken half-open socket. The
            // reader thread observes the close and exits.
            tearDownLocked();
            return false;
        }
        return true;
    }

    void shutdown() {
        bool expected = false;
        if (!stopping_.compare_exchange_strong(expected, true)) return;
        std::lock_guard<std::mutex> lock(send_mutex_);
        tearDownLocked();
    }

private:
    bool connect() {
        sockaddr_in addr{};
        if (!detail::parse_endpoint(connect_endpoint_, addr)) return false;
        int s = ::socket(AF_INET, SOCK_STREAM | SOCK_CLOEXEC, 0);
        if (s < 0) return false;
        // Retry connect briefly: in tests the server may still be
        // racing through bind() when the client first tries to dial.
        // Up to 1s of total wait covers ctest startup jitter without
        // turning a real ECONNREFUSED into a hang.
        constexpr int kMaxAttempts = 20;
        constexpr auto kSleep = std::chrono::milliseconds(50);
        bool connected = false;
        for (int attempt = 0; attempt < kMaxAttempts; ++attempt) {
            if (::connect(s, reinterpret_cast<sockaddr*>(&addr), sizeof(addr)) == 0) {
                connected = true;
                break;
            }
            if (errno != ECONNREFUSED && errno != ECONNRESET && errno != EINTR) break;
            std::this_thread::sleep_for(kSleep);
        }
        if (!connected) {
            ::close(s);
            return false;
        }
        int one = 1;
        ::setsockopt(s, IPPROTO_TCP, TCP_NODELAY, &one, sizeof(one));
        fd_ = s;
        if (on_receive_) {
            reader_ = std::thread([this] { readLoop(); });
        }
        return true;
    }

    void readLoop() {
        int fd_snapshot = fd_;
        SCE::Mesh::MeshEnvelope env;
        std::vector<uint8_t> scratch;  // reused across iterations
        while (!stopping_.load() && fd_snapshot >= 0) {
            auto result = detail::read_envelope(fd_snapshot, env, scratch);
            if (result == detail::ReadResult::SocketClosed) break;
            if (result == detail::ReadResult::DecodeError) {
                // §16.7 row 4: malformed CBOR on a successfully-read
                // frame. Raise then keep the connection alive — see
                // Server::readLoop for the same rationale.
                if (on_decode_error_) on_decode_error_();
                env = {};
                continue;
            }
            on_receive_(env);
            env = {};
        }
    }

public:
    /// Install the decode-error handler (§16.7 row 4). Invoked on the
    /// reader thread when an inbound frame's CBOR decode fails.
    /// Wires to `raiseCommunicationError(ENVELOPE_CORRUPT,
    /// transport="custom_tcp")`. Must be installed BEFORE any
    /// `send()` that triggers the lazy `connect()` so the reader
    /// thread sees the handler when it starts. May be left unset
    /// — in that case decode failures revert to the pre-row-4
    /// silent-drop (connection kept alive).
    void setDecodeErrorHandler(DecodeErrorCallback handler) {
        on_decode_error_ = std::move(handler);
    }

private:

    /// Caller MUST hold `send_mutex_`. Idempotent; safe to call from
    /// shutdown() and the failure path of send().
    ///
    /// DEADLOCK INVARIANT (caller-maintained, not structural):
    /// joining the reader while holding `send_mutex_` is safe ONLY while
    /// the reader's exit path is purely the `recv()` unblock from the
    /// `::shutdown` above. This holds today because the on_receive_
    /// callback installed by codegen is `(env){ dispatchToSender(env); }`,
    /// which queues the envelope on the sender engine's external queue
    /// and returns — it does NOT synchronously call back into this
    /// `Client::send`.
    ///
    /// Adding RPC patterns will violate this invariant if the inbound
    /// reply triggers a state-entry `<send>` that re-enters the same
    /// Client. Concrete failure mode:
    ///   1. main thread: shutdown() → tearDownLocked() takes send_mutex_
    ///   2. reader thread: in on_receive_ → engine.processEvent → onentry
    ///      <send> → setMeshSendCallback → route_send → Client::send
    ///      → blocks on send_mutex_
    ///   3. main thread: reader_.join() blocks waiting for reader to exit
    ///   → deadlock
    /// The fix when RPC lands is to drop send_mutex_ before joining (or
    /// use a separate connect_mutex_ that gates only the dial path).
    void tearDownLocked() {
        if (fd_ >= 0) {
            ::shutdown(fd_, SHUT_RDWR);
            ::close(fd_);
            fd_ = -1;
        }
        if (reader_.joinable()) {
            reader_.join();
        }
    }

    std::string connect_endpoint_;
    ReceiveCallback on_receive_;
    DecodeErrorCallback on_decode_error_;
    std::mutex send_mutex_;
    std::atomic<bool> stopping_{false};
    int fd_ = -1;
    std::thread reader_;
};

}  // namespace SCE::Mesh::CustomTcp
