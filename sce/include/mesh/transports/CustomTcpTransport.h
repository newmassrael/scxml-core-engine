// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh §mesh-16.8.3 reference transport: TCP loopback with length-prefixed
// CBOR envelope framing. Zero external dependencies — POSIX sockets only.
//
// Wire format (per direction, every envelope):
//   [4 bytes payload_length, network byte order]
//   [payload_length bytes CBOR-encoded MeshEnvelope]
//
// Conformance (SCE Mesh §mesh-10.4):
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
#include <memory>
#include <mutex>
#include <netinet/in.h>
#include <netinet/tcp.h>
#include <optional>
#include <string>
#include <sys/socket.h>
#include <sys/types.h>
#include <thread>
#include <unistd.h>
#include <unordered_map>
#include <utility>
#include <vector>

namespace SCE::Mesh::CustomTcp {

/// Decode-error callback signature: invoked on a per-connection read
/// thread when an inbound frame's CBOR decode fails. Codegen wires
/// this to `raiseCommunicationError(ENVELOPE_CORRUPT,
/// transport="custom_tcp")` so the §mesh-16.7 row 4 catalog row fires at
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
/// loopback-only by design (SCE_MESH.md §mesh-16.8.3).
///
/// Strict numeric parsing: `from_chars` requires the entire port slice
/// to be consumed, so `"127.0.0.1:8080abc"` and `"127.0.0.1:8080 "` are
/// rejected at parse time rather than silently truncating to 8080 (the
/// behaviour `std::stoi` would have).
inline bool parse_endpoint(const std::string &endpoint, sockaddr_in &out) {
    auto colon = endpoint.find(':');
    if (colon == std::string::npos || colon == 0 || colon + 1 >= endpoint.size()) {
        return false;
    }
    const char *port_begin = endpoint.data() + colon + 1;
    const char *port_end = endpoint.data() + endpoint.size();
    int port = 0;
    auto [parse_end, ec] = std::from_chars(port_begin, port_end, port);
    if (ec != std::errc{} || parse_end != port_end) {
        return false;
    }
    if (port < 0 || port > 65535) {
        return false;
    }
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
inline bool read_exact(int fd, void *buf, std::size_t n) {
    auto *p = static_cast<unsigned char *>(buf);
    while (n > 0) {
        ssize_t got = ::recv(fd, p, n, 0);
        if (got == 0) {
            return false;  // EOF: peer closed cleanly
        }
        if (got < 0) {
            if (errno == EINTR) {
                continue;
            }
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
inline bool write_exact(int fd, const void *buf, std::size_t n) {
    const auto *p = static_cast<const unsigned char *>(buf);
    while (n > 0) {
        ssize_t put = ::send(fd, p, n, MSG_NOSIGNAL);
        if (put <= 0) {
            if (put < 0 && errno == EINTR) {
                continue;
            }
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
/// loop AFTER raising §mesh-16.7 row 4 ENVELOPE_CORRUPT). The
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

inline ReadResult read_envelope(int fd, SCE::Mesh::MeshEnvelope &out, std::vector<uint8_t> &scratch) {
    uint32_t net_len = 0;
    if (!read_exact(fd, &net_len, sizeof(net_len))) {
        return ReadResult::SocketClosed;
    }
    uint32_t len = ntohl(net_len);
    // Cap at the SCE-mesh wire ceiling exported by MeshEnvelopeCodec.h.
    // Rebinding the literal here would be ownership-inversion via
    // co-declaration with the CBOR decoder's identical check —
    // consume the single source instead so any future spec change
    // moves both sites together.
    if (len == 0 || static_cast<std::size_t>(len) > ::SCE::Mesh::kMaxEnvelopeBytes) {
        return ReadResult::SocketClosed;
    }
    scratch.resize(len);
    if (!read_exact(fd, scratch.data(), len)) {
        return ReadResult::SocketClosed;
    }
    return SCE::Mesh::decodeEnvelope(scratch.data(), scratch.size(), out) ? ReadResult::Ok : ReadResult::DecodeError;
}

/// Encode `env` and write it as a length-prefixed frame on `fd`.
inline bool write_envelope(int fd, const SCE::Mesh::MeshEnvelope &env) {
    auto buf = SCE::Mesh::encodeEnvelope(env);
    if (buf.empty()) {
        return false;
    }
    uint32_t net_len = htonl(static_cast<uint32_t>(buf.size()));
    if (!write_exact(fd, &net_len, sizeof(net_len))) {
        return false;
    }
    return write_exact(fd, buf.data(), buf.size());
}

/// Process-wide monotonic stream identifier. Ids start at 1 so that 0 is
/// reserved as the "no stream" sentinel a default-constructed `PeerLink`
/// reports.
inline std::uint64_t next_stream_id() {
    static std::atomic<std::uint64_t> counter{0};
    return counter.fetch_add(1, std::memory_order_relaxed) + 1;
}

/// One peer stream — an accepted server connection, or the socket a
/// `Client` dialed. Reference-counted so that a `PeerLink` parked in a
/// generated subscription registry stays safe to call after the peer
/// disconnects: the fd survives until the last reference drops, and every
/// send past `close()` fails instead of touching a recycled descriptor.
///
/// SCE_MESH.md §mesh-10.4: the transport owns framing and stream
/// lifetime, never pattern policy. A `Stream` therefore exposes exactly
/// two operations — write one framed envelope, and close.
struct Stream {
    Stream(int fd, std::uint64_t id) : fd(fd), id(id) {}

    Stream(const Stream &) = delete;
    Stream &operator=(const Stream &) = delete;

    ~Stream() {
        if (fd >= 0) {
            ::close(fd);
        }
    }

    /// Write one length-prefixed frame. Serialised against concurrent
    /// senders on the same stream: two interleaved `write_envelope` calls
    /// would splice their length prefixes and corrupt the peer's framing.
    ///
    /// A failed write closes the stream so the next send fails fast and a
    /// subscription registry holding this link can purge it on the next
    /// fan-out rather than retrying a half-open socket forever.
    [[nodiscard]] bool send(const SCE::Mesh::MeshEnvelope &env) {
        std::lock_guard<std::mutex> lock(write_mutex);
        if (!open.load(std::memory_order_acquire)) {
            return false;
        }
        if (!write_envelope(fd, env)) {
            closeLocked();
            return false;
        }
        return true;
    }

    /// Idempotent. Unblocks a reader parked in `recv` via `::shutdown`
    /// but does NOT close the descriptor — the destructor does that once
    /// every reference (reader thread included) is gone, so no send can
    /// ever address a recycled fd.
    void close() {
        std::lock_guard<std::mutex> lock(write_mutex);
        closeLocked();
    }

    [[nodiscard]] bool is_open() const noexcept {
        return open.load(std::memory_order_acquire);
    }

    int fd = -1;
    std::uint64_t id = 0;

private:
    void closeLocked() {
        bool expected = true;
        if (!open.compare_exchange_strong(expected, false, std::memory_order_acq_rel)) {
            return;
        }
        ::shutdown(fd, SHUT_RDWR);
    }

    std::mutex write_mutex;
    std::atomic<bool> open{true};
};

}  // namespace detail

/// Handle to the peer stream an inbound envelope arrived on.
///
/// SCE_MESH.md §mesh-10.4.4: a brokerless transport answers a request on
/// the stream that carried it, so the receive callback must be told which
/// stream that was. `PeerLink` is that identity — copyable, storable, and
/// safe to outlive the connection (`send` returns false once the peer is
/// gone, `valid()` reports it without a send attempt).
///
/// Holding the link IS the subscription in the generated PubSub registry:
/// a subscriber that dies takes its own registration out of service
/// without any expiry timer or liveness probe, because the very handle
/// the registry stored reports itself invalid.
class PeerLink {
public:
    PeerLink() = default;

    explicit PeerLink(std::shared_ptr<detail::Stream> stream) : stream_(std::move(stream)) {}

    /// Write one framed envelope back to this peer. False when the link is
    /// empty, the peer has disconnected, or the write failed.
    [[nodiscard]] bool send(const SCE::Mesh::MeshEnvelope &env) const {
        return stream_ && stream_->send(env);
    }

    /// True while the stream is still usable. A registry purging on this
    /// predicate never has to distinguish "never subscribed" from
    /// "subscribed then died".
    [[nodiscard]] bool valid() const noexcept {
        return stream_ && stream_->is_open();
    }

    /// Stable per-connection identifier; 0 for a default-constructed link.
    /// Two links compare equal iff they name the same stream, which is
    /// what an unsubscribe has to match on.
    [[nodiscard]] std::uint64_t id() const noexcept {
        return stream_ ? stream_->id : 0;
    }

    [[nodiscard]] bool operator==(const PeerLink &other) const noexcept {
        return id() == other.id();
    }

    [[nodiscard]] bool operator!=(const PeerLink &other) const noexcept {
        return !(*this == other);
    }

private:
    std::shared_ptr<detail::Stream> stream_;
};

/// Receive callback signature: invoked on a transport thread once per
/// successfully decoded envelope, with the link the envelope arrived on.
/// The callback owns dispatch to the engine and any reply; this layer does
/// no policy interpretation.
///
/// Both `Server` and `Client` use this one signature — a client's inbound
/// stream is as much a peer link as an accepted connection, so pattern
/// handling in generated code is written once rather than per role.
using ReceiveCallback = std::function<void(const SCE::Mesh::MeshEnvelope &, const PeerLink &)>;

/// TCP server: binds at construction, accepts connections in a background
/// thread, spawns one read thread per accepted connection. Each decoded
/// envelope is forwarded to `on_receive` on the read thread.
///
/// Construction succeeds iff bind+listen succeed. Use `valid()` to check
/// before relying on the server. Destruction joins all threads after
/// shutting down the listener and any open connections.
class Server {
public:
    Server(const std::string &listen_endpoint, ReceiveCallback on_receive) : on_receive_(std::move(on_receive)) {
        sockaddr_in addr{};
        if (!detail::parse_endpoint(listen_endpoint, addr)) {
            return;  // valid_ stays false
        }
        listen_fd_ = ::socket(AF_INET, SOCK_STREAM | SOCK_CLOEXEC, 0);
        if (listen_fd_ < 0) {
            return;
        }
        // SO_REUSEADDR keeps tests deterministic across rapid teardowns
        // (a port in TIME_WAIT would otherwise reject the next bind).
        int one = 1;
        ::setsockopt(listen_fd_, SOL_SOCKET, SO_REUSEADDR, &one, sizeof(one));
        if (::bind(listen_fd_, reinterpret_cast<sockaddr *>(&addr), sizeof(addr)) != 0) {
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

    ~Server() {
        shutdown();
    }

    Server(const Server &) = delete;
    Server &operator=(const Server &) = delete;

    [[nodiscard]] bool valid() const noexcept {
        return valid_;
    }

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
        if (!valid_ || listen_fd_ < 0) {
            return std::nullopt;
        }
        sockaddr_in addr{};
        socklen_t len = sizeof(addr);
        if (::getsockname(listen_fd_, reinterpret_cast<sockaddr *>(&addr), &len) != 0) {
            return std::nullopt;
        }
        if (addr.sin_family != AF_INET) {
            return std::nullopt;
        }
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
        if (!stopping_.compare_exchange_strong(expected, true)) {
            return;
        }
        if (listen_fd_ >= 0) {
            // Closing the listen socket unblocks the accept thread; it
            // observes accept() failing with EBADF/EINVAL and exits.
            ::shutdown(listen_fd_, SHUT_RDWR);
            ::close(listen_fd_);
            listen_fd_ = -1;
        }
        if (accept_thread_.joinable()) {
            accept_thread_.join();
        }
        // Tear down accepted connections after the accept thread is gone
        // so the connection list cannot grow concurrently.
        std::vector<Connection> taken;
        {
            std::lock_guard<std::mutex> lock(connections_mutex_);
            taken = std::move(connections_);
        }
        // Close every stream first, then join: a reader still inside
        // `on_receive_` may reply on its own link or on a sibling one, and
        // that reply must not block behind this teardown. Closing is
        // non-blocking, so every reader is already unblocked (and every
        // in-flight reply already resolved to a fast `false`) by the time
        // the first join runs.
        for (auto &c : taken) {
            if (c.stream) {
                c.stream->close();
            }
        }
        for (auto &c : taken) {
            if (c.reader.joinable()) {
                c.reader.join();
            }
        }
    }

private:
    struct Connection {
        std::shared_ptr<detail::Stream> stream;
        std::thread reader;
    };

    void acceptLoop() {
        while (!stopping_.load()) {
            sockaddr_in peer{};
            socklen_t plen = sizeof(peer);
            int conn = ::accept(listen_fd_, reinterpret_cast<sockaddr *>(&peer), &plen);
            if (conn < 0) {
                if (stopping_.load()) {
                    return;
                }
                if (errno == EINTR) {
                    continue;
                }
                return;  // unrecoverable: listener is gone
            }
            // Disable Nagle to keep harness latencies deterministic; tests
            // measure end-to-end ordering, not throughput, so coalescing
            // would just add jitter.
            int one = 1;
            ::setsockopt(conn, IPPROTO_TCP, TCP_NODELAY, &one, sizeof(one));
            // The reader owns a reference to the stream, so the fd stays
            // valid for exactly as long as someone can still read or write
            // it — vector reallocation of the slot cannot strand it, and
            // no descriptor is recycled while a `PeerLink` handed to
            // application code still names it.
            auto stream = std::make_shared<detail::Stream>(conn, detail::next_stream_id());
            std::lock_guard<std::mutex> lock(connections_mutex_);
            auto &slot = connections_.emplace_back();
            slot.stream = stream;
            slot.reader = std::thread([this, stream] { readLoop(stream); });
        }
    }

    void readLoop(const std::shared_ptr<detail::Stream> &stream) {
        SCE::Mesh::MeshEnvelope env;
        // Per-reader scratch buffer — see detail::read_envelope contract.
        // Lives on this thread's stack, so no synchronization needed and
        // it is freed automatically when the reader exits.
        std::vector<uint8_t> scratch;
        const PeerLink link(stream);
        const int fd = stream->fd;
        while (!stopping_.load()) {
            auto result = detail::read_envelope(fd, env, scratch);
            if (result == detail::ReadResult::SocketClosed) {
                // Mark the stream down so a registry holding this link
                // stops fanning out to a peer that has hung up.
                stream->close();
                break;
            }
            if (result == detail::ReadResult::DecodeError) {
                // §mesh-16.7 row 4: malformed CBOR on a successfully-read
                // frame. Raise then keep the connection alive — a
                // single bad envelope is recoverable; only a framing-
                // level fault (reported as SocketClosed) tears down
                // the stream.
                if (on_decode_error_) {
                    on_decode_error_();
                }
                env = {};
                continue;
            }
            if (on_receive_) {
                on_receive_(env, link);
            }
            env = {};
        }
    }

public:
    /// Install the decode-error handler (§mesh-16.7 row 4). Invoked on
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
/// for the duration of the connection so the same TCP stream carries both
/// outbound sends and whatever the peer pushes back — replies to this
/// client's requests, and notifications for event groups it subscribed to
/// (full duplex).
///
/// One connection carries every pattern in both directions. A reply never
/// needs a reversed connection back to the requester, which is what lets
/// custom_tcp answer a peer that dialed from an ephemeral port.
class Client {
public:
    Client(std::string connect_endpoint, ReceiveCallback on_receive)
        : connect_endpoint_(std::move(connect_endpoint)), on_receive_(std::move(on_receive)) {}

    ~Client() {
        shutdown();
    }

    Client(const Client &) = delete;
    Client &operator=(const Client &) = delete;

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
        std::lock_guard<std::mutex> lock(dial_mutex_);
        if (stream_ || stopping_.load()) {
            return false;
        }
        connect_endpoint_ = std::move(endpoint);
        return true;
    }

    /// Encode and send the envelope. Connects on demand; subsequent calls
    /// reuse the same socket. Returns false on connect / send failure.
    ///
    /// Re-entrant: an inbound reply handled on this client's reader thread
    /// may trigger a state-entry `<send>` that lands back here. The dial
    /// lock is released before the write, and the write is serialised by
    /// the stream itself, so the re-entrant call contends for nothing that
    /// the reader is holding.
    [[nodiscard]] bool send(const SCE::Mesh::MeshEnvelope &env) {
        auto stream = acquireStream();
        return stream && stream->send(env);
    }

    /// The link to this client's own peer, dialing on demand exactly as
    /// `send` does. Lets a client-role machine answer on the stream it
    /// already owns instead of requiring a second, reversed connection.
    [[nodiscard]] PeerLink link() {
        return PeerLink(acquireStream());
    }

    void shutdown() {
        bool expected = false;
        if (!stopping_.compare_exchange_strong(expected, true)) {
            return;
        }
        std::shared_ptr<detail::Stream> stream;
        std::thread reader;
        std::vector<std::thread> retired;
        {
            std::lock_guard<std::mutex> lock(dial_mutex_);
            stream = std::move(stream_);
            reader = std::move(reader_);
            retired = std::move(retired_readers_);
        }
        if (stream) {
            stream->close();
        }
        // Join OUTSIDE `dial_mutex_`. A reader parked in `on_receive_` may
        // still re-enter `send()`, which needs that lock to observe
        // `stopping_` and fail fast; holding it across the join would put
        // the reader and this thread in a cycle. `stopping_` is already
        // set and the stream already closed, so every such call now
        // returns false promptly and the reader drains.
        if (reader.joinable()) {
            reader.join();
        }
        for (auto &t : retired) {
            if (t.joinable()) {
                t.join();
            }
        }
    }

private:
    /// Return an open stream, dialing if the previous one is gone.
    /// `nullptr` once shutting down or when the dial fails.
    std::shared_ptr<detail::Stream> acquireStream() {
        std::lock_guard<std::mutex> lock(dial_mutex_);
        if (stopping_.load()) {
            return nullptr;
        }
        if (stream_ && stream_->is_open()) {
            return stream_;
        }
        return connect();
    }

    /// Caller MUST hold `dial_mutex_`. Retires the previous reader without
    /// joining it — a join here could be a self-join when the re-dial is
    /// driven by that very reader's callback. Retired threads are joined
    /// in `shutdown()`, on a thread that is never one of them.
    std::shared_ptr<detail::Stream> connect() {
        sockaddr_in addr{};
        if (!detail::parse_endpoint(connect_endpoint_, addr)) {
            return nullptr;
        }
        int s = ::socket(AF_INET, SOCK_STREAM | SOCK_CLOEXEC, 0);
        if (s < 0) {
            return nullptr;
        }
        // Retry connect briefly: in tests the server may still be
        // racing through bind() when the client first tries to dial.
        // Up to 1s of total wait covers ctest startup jitter without
        // turning a real ECONNREFUSED into a hang.
        constexpr int kMaxAttempts = 20;
        constexpr auto kSleep = std::chrono::milliseconds(50);
        bool connected = false;
        for (int attempt = 0; attempt < kMaxAttempts; ++attempt) {
            if (::connect(s, reinterpret_cast<sockaddr *>(&addr), sizeof(addr)) == 0) {
                connected = true;
                break;
            }
            if (errno != ECONNREFUSED && errno != ECONNRESET && errno != EINTR) {
                break;
            }
            std::this_thread::sleep_for(kSleep);
        }
        if (!connected) {
            ::close(s);
            return nullptr;
        }
        int one = 1;
        ::setsockopt(s, IPPROTO_TCP, TCP_NODELAY, &one, sizeof(one));
        if (reader_.joinable()) {
            retired_readers_.push_back(std::move(reader_));
        }
        stream_ = std::make_shared<detail::Stream>(s, detail::next_stream_id());
        if (on_receive_) {
            reader_ = std::thread([this, stream = stream_] { readLoop(stream); });
        }
        return stream_;
    }

    void readLoop(const std::shared_ptr<detail::Stream> &stream) {
        SCE::Mesh::MeshEnvelope env;
        std::vector<uint8_t> scratch;  // reused across iterations
        const PeerLink link(stream);
        const int fd = stream->fd;
        while (!stopping_.load()) {
            auto result = detail::read_envelope(fd, env, scratch);
            if (result == detail::ReadResult::SocketClosed) {
                stream->close();
                break;
            }
            if (result == detail::ReadResult::DecodeError) {
                // §mesh-16.7 row 4: malformed CBOR on a successfully-read
                // frame. Raise then keep the connection alive — see
                // Server::readLoop for the same rationale.
                if (on_decode_error_) {
                    on_decode_error_();
                }
                env = {};
                continue;
            }
            on_receive_(env, link);
            env = {};
        }
    }

public:
    /// Install the decode-error handler (§mesh-16.7 row 4). Invoked on the
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
    std::string connect_endpoint_;
    ReceiveCallback on_receive_;
    DecodeErrorCallback on_decode_error_;
    /// Gates the dial path and the `stream_` / `reader_` handles only.
    /// Frame serialisation lives on the stream, so this lock is never held
    /// across a write, a join, or a user callback — which is what makes an
    /// inbound reply free to re-enter `send()` while teardown runs.
    std::mutex dial_mutex_;
    std::atomic<bool> stopping_{false};
    std::shared_ptr<detail::Stream> stream_;
    std::thread reader_;
    /// Readers of superseded connections. Joined in `shutdown()` rather
    /// than at re-dial time because the re-dial may be driven by the
    /// retiring reader's own callback, and a thread cannot join itself.
    std::vector<std::thread> retired_readers_;
};

}  // namespace SCE::Mesh::CustomTcp
