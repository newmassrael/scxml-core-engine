// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The port of the Mesh binding over HTTP request and response (docs/adr/0107,
// D2; SCE_MESH.md §mesh-22.6, the C++ interface of the port that section
// names): one exchange at the level of HTTP semantics (RFC 9110), which is the
// level the binding is defined on. A POST with its headers and body and a
// bound on the wait goes in; a response, or a failure of a named kind, comes
// out.
//
// The binding is three layers. Its semantics (framing, acknowledgement,
// holding, readiness, status codes, authentication) are SCE's and live in
// mesh/transports/HttpsClient.h. This header is the port between them and a
// network stack. An adapter is any implementation of it: SCE's reference
// adapters on the platform's HTTP stack (CurlHttpsTransport, the system's
// libcurl; WinHttpHttpsTransport, WinHTTP), or the embedding's own, which then
// brings its own proxy and trust configuration and no second TLS library. The
// dependency points from the adapter to this port, never from SCE to a network
// library: this header names the standard library alone.
//
// What every adapter owes the binding (held by the conformance kit,
// tests/mesh/https_kit/):
//   * the platform's trust store, so a network that inspects TLS with an
//     authority the machine trusts is honoured;
//   * the platform's proxy configuration, tunnelling to an explicit proxy
//     with CONNECT;
//   * a response is a Response whatever its status: only an exchange that
//     produced no response is a Failure;
//   * the request's `timeout` bounds the whole exchange, and an exchange past
//     it ends as Failure::Timeout;
//   * concurrent calls to exchange() from different threads, since the client
//     holds one exchange open for the server's sends while another carries its
//     own;
//   * a cancellation the caller requests ends a waiting exchange promptly.

#pragma once

#include <atomic>
#include <chrono>
#include <condition_variable>
#include <cstdint>
#include <functional>
#include <map>
#include <mutex>
#include <string>
#include <utility>
#include <vector>

namespace SCE::Mesh {

/// A header field as it travels: its name and its value. Names compare
/// without regard to ASCII case (RFC 9110 section 5.1).
using HttpsHeader = std::pair<std::string, std::string>;

/// A request the caller may stop waiting for. The caller requests the
/// cancellation; an adapter that waits on a network stack subscribes the step
/// that ends the wait (closing a request handle, for one), or reads
/// requested() from a progress callback. The structure is the stop token and
/// stop callback of C++20, written in C++17 for the public interface
/// (docs/adr/0059).
class HttpsCancellation {
public:
    /// Holds a subscription; destroying it ends the subscription, and waits
    /// for the callback when another thread is running it at that moment, so
    /// nothing the callback reads is destroyed under it.
    class Subscription {
    public:
        Subscription() = default;
        Subscription(const Subscription &) = delete;
        Subscription &operator=(const Subscription &) = delete;

        Subscription(Subscription &&other) noexcept : owner_(other.owner_), id_(other.id_) {
            other.owner_ = nullptr;
        }

        Subscription &operator=(Subscription &&other) noexcept {
            if (this != &other) {
                reset();
                owner_ = other.owner_;
                id_ = other.id_;
                other.owner_ = nullptr;
            }
            return *this;
        }

        ~Subscription() {
            reset();
        }

    private:
        friend class HttpsCancellation;

        Subscription(HttpsCancellation *owner, std::uint64_t id) : owner_(owner), id_(id) {}

        void reset() {
            if (owner_ != nullptr) {
                owner_->unsubscribe(id_);
                owner_ = nullptr;
            }
        }

        HttpsCancellation *owner_ = nullptr;
        std::uint64_t id_ = 0;
    };

    HttpsCancellation() = default;
    HttpsCancellation(const HttpsCancellation &) = delete;
    HttpsCancellation &operator=(const HttpsCancellation &) = delete;

    /// Whether the caller has asked every exchange to end.
    bool requested() const noexcept {
        return requested_.load(std::memory_order_acquire);
    }

    /// Ask every exchange that reads this cancellation to end, running each
    /// subscribed callback once, on this thread. Later calls do nothing.
    void request() {
        std::unique_lock<std::mutex> lock(mutex_);
        if (requested_.exchange(true, std::memory_order_acq_rel)) {
            return;
        }
        while (!callbacks_.empty()) {
            auto it = callbacks_.begin();
            const std::uint64_t id = it->first;
            std::function<void()> callback = std::move(it->second);
            callbacks_.erase(it);
            running_ = id;
            lock.unlock();
            callback();
            lock.lock();
            running_ = 0;
            finished_.notify_all();
        }
    }

    /// Run `onCancel` when the cancellation is requested, or at once on this
    /// thread when it already was. `onCancel` must not throw.
    Subscription subscribe(std::function<void()> onCancel) {
        std::unique_lock<std::mutex> lock(mutex_);
        if (requested_.load(std::memory_order_acquire)) {
            lock.unlock();
            onCancel();
            return Subscription();
        }
        const std::uint64_t id = ++next_id_;
        callbacks_.emplace(id, std::move(onCancel));
        return Subscription(this, id);
    }

private:
    void unsubscribe(std::uint64_t id) {
        std::unique_lock<std::mutex> lock(mutex_);
        callbacks_.erase(id);
        finished_.wait(lock, [&] { return running_ != id; });
    }

    std::mutex mutex_;
    std::condition_variable finished_;
    std::atomic<bool> requested_{false};
    std::map<std::uint64_t, std::function<void()>> callbacks_;
    std::uint64_t next_id_ = 0;
    std::uint64_t running_ = 0;
};

/// One POST the binding makes.
struct HttpsRequest {
    /// The absolute URL, `https://` (or `http://` to a loopback host).
    std::string url;
    /// The request's header fields; an adapter adds what its stack needs
    /// (Host, Content-Length) and nothing that changes their meaning.
    std::vector<HttpsHeader> headers;
    /// The request's content.
    std::vector<std::uint8_t> body;
    /// The bound on the whole exchange, from the first byte sent to the last
    /// byte of the response received.
    std::chrono::milliseconds timeout{0};
    /// When set, the caller may request it to end the exchange early; the
    /// adapter then answers Failure::Other. It outlives the exchange.
    HttpsCancellation *cancellation = nullptr;
};

/// What an adapter answers an exchange with.
struct HttpsExchange {
    enum class Kind {
        /// A response came, whatever its status.
        Response,
        /// No response came.
        Failure,
    };
    /// Why no response came.
    enum class Failure {
        /// The server could not be reached: resolution, connection refused
        /// or reset before a response.
        Connect,
        /// The TLS handshake failed or the server's certificate was not
        /// trusted.
        Tls,
        /// The proxy refused the tunnel or could not be reached.
        Proxy,
        /// The exchange ran past its timeout.
        Timeout,
        /// Anything else, a cancellation among them.
        Other,
    };

    Kind kind = Kind::Failure;
    /// The response's status code (Response only).
    std::uint16_t status = 0;
    /// The response's header fields (Response only).
    std::vector<HttpsHeader> headers;
    /// The response's content (Response only).
    std::vector<std::uint8_t> body;
    /// Why no response came (Failure only).
    Failure failure = Failure::Other;
    /// The network stack's own words, for a diagnostic.
    std::string detail;

    /// A response.
    static HttpsExchange response(std::uint16_t status, std::vector<HttpsHeader> headers,
                                  std::vector<std::uint8_t> body) {
        HttpsExchange e;
        e.kind = Kind::Response;
        e.status = status;
        e.headers = std::move(headers);
        e.body = std::move(body);
        return e;
    }

    /// No response, for the reason given.
    static HttpsExchange failed(Failure failure, std::string detail) {
        HttpsExchange e;
        e.kind = Kind::Failure;
        e.failure = failure;
        e.detail = std::move(detail);
        return e;
    }
};

/// The name of a failure kind, for a diagnostic.
inline const char *failureName(HttpsExchange::Failure failure) {
    switch (failure) {
    case HttpsExchange::Failure::Connect:
        return "connect";
    case HttpsExchange::Failure::Tls:
        return "tls";
    case HttpsExchange::Failure::Proxy:
        return "proxy";
    case HttpsExchange::Failure::Timeout:
        return "timeout";
    case HttpsExchange::Failure::Other:
        return "other";
    }
    return "other";
}

/// Carries one exchange: the port of SCE_MESH.md §mesh-22.6, which every
/// adapter implements, answering a response whatever its status or a failure
/// of a named kind. The host owns the network policy, so it builds the
/// adapter and hands it to the generated router at construction.
class HttpsTransport {
public:
    virtual ~HttpsTransport() = default;
    /// Make one exchange. Called from more than one thread at once.
    virtual HttpsExchange exchange(const HttpsRequest &request) = 0;
};

}  // namespace SCE::Mesh
