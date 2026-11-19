// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The client of the Mesh binding over HTTP request and response
// (docs/adr/0107, D2): the binding's semantics on the C++ side, over any
// adapter of the port in HttpsTransport.h. One client per bound peer; the
// generated TransportRouter builds it from the adapter and the credentials
// its host hands it, and every generated router shares this one
// implementation (sce/src/mesh/HttpsClient.cpp).
//
// The wire, which the Rust host core's server and client speak too:
//   * The client POSTs to `<base>/sce-mesh/1/<client>`, `<client>` its
//     machine name. Content-Type `application/cbor-seq`; the body is zero or
//     more envelopes (their CBOR bytes, MeshEnvelopeCodec.h) one after
//     another (RFC 8742).
//   * Request headers: `Sce-Mesh-Ack` the highest mailbox number received
//     (0 for none), `Sce-Mesh-Epoch` the mailbox incarnation that number
//     counts in (absent before the first `200`), `Sce-Mesh-Hold-Ms` how long
//     the server may hold an exchange that has nothing to answer yet, and the
//     authenticator's: `Authorization: Bearer <token>`, `Authorization:
//     SCE-Install <base64url of a 32-byte secret>`, or none with a TLS client
//     certificate.
//   * Response `200`, `application/cbor-seq`, `Sce-Mesh-Epoch` the mailbox's
//     incarnation (32 lowercase hexadecimal digits) and `Sce-Mesh-First` the
//     number of the first envelope in the body (absent when the body is
//     empty); the next request acknowledges the highest number received, and
//     the server drops what was acknowledged in its own incarnation. What a
//     lost response carried is carried again, and the client delivers each
//     number once. A server that starts again with its mailbox empty numbers
//     it from 1 in a new incarnation, and the client's acknowledgement starts
//     over at 0 in it.
//   * `404` an unknown version or name, `401` an authenticator refused, `413`
//     a body too big, `429` or `503` with `Retry-After` (the client waits at
//     least that long).
//
// Two exchanges at most are open at once: the poll, which carries no
// envelope and asks to be held so a server's send reaches the client while it
// has nothing to say, and the send, which carries what the machine sent and
// asks for no hold. Either one's response may carry mailbox envelopes.
//
// What reaches the router (each through ClientEvents, on a client thread):
//   * `receive`: each mailbox envelope, once, in mailbox order;
//   * `ready`: the first exchange answered `200` after the client was not
//     ready (SCE_MESH.md section 10.10);
//   * `notReady`: an exchange that failed at the transport, or a poll that was
//     refused, while the client was ready or had not yet been either: section
//     16.7 row 1, once per edge. It loses nothing: the server keeps the
//     mailbox, a reply to a request the router waits on among it, so the
//     router keeps every request waiting (SCE_MESH.md section 22.4);
//   * `restarted`: a response of a new incarnation that replaces the one the
//     client knew, before that response's envelopes: the mailbox the client
//     knew, and any reply it held, is gone, so the router sends each request
//     still waiting on the peer again (section 22.4, WaitingRequests below).
//     Not the first incarnation the client learns, and not a late response
//     in the incarnation its own request was sent in;
//   * `fault`: section 16.7 row 2 for an envelope a send could not carry, row
//     3 for one its retry policy gave up on, row 4 for a mailbox envelope that
//     does not decode, row 9 for an envelope the outbox has no room for, and
//     row 10 for a name the server's authenticator refused (once per run of
//     refusals).

#pragma once

#include "mesh/CommunicationError.h"
#include "mesh/MeshEnvelope.h"
#include "mesh/transports/HttpsTransport.h"

#include <algorithm>
#include <array>
#include <atomic>
#include <chrono>
#include <condition_variable>
#include <cstddef>
#include <cstdint>
#include <deque>
#include <functional>
#include <memory>
#include <mutex>
#include <optional>
#include <string>
#include <string_view>
#include <thread>
#include <utility>
#include <vector>

namespace SCE::Mesh::Https {

/// The binding's version, the path segment after `sce-mesh`.
inline constexpr const char *kBindingVersion = "1";
/// The media type of a request and a response body (RFC 8742).
inline constexpr const char *kContentType = "application/cbor-seq";
/// The highest mailbox number the client received.
inline constexpr const char *kAckHeader = "Sce-Mesh-Ack";
/// How long the server may hold an exchange with nothing to answer yet.
inline constexpr const char *kHoldHeader = "Sce-Mesh-Hold-Ms";
/// The mailbox number of the first envelope of a response body.
inline constexpr const char *kFirstHeader = "Sce-Mesh-First";
/// The mailbox incarnation a response's numbers, or a request's
/// acknowledgement, count in.
inline constexpr const char *kEpochHeader = "Sce-Mesh-Epoch";
/// How many random bytes name an incarnation; it is written as twice as many
/// lowercase hexadecimal digits.
inline constexpr std::size_t kEpochBytes = 16;
/// The transport's name in a CommunicationError.
inline constexpr const char *kTransportName = "https";

/// How long an exchange may take beyond the hold it asked for before the
/// client counts it a transport failure: the server answers at the end of the
/// hold at the latest, so what is left is the network's.
inline constexpr std::chrono::milliseconds kExchangeGrace{10000};
/// How many envelopes the client keeps for a send before it refuses one
/// (section 16.7 row 9). A machine that declares `outbound_buffer:` holds its
/// envelopes there while the client is not ready; this bounds what the client
/// holds once they were released to it.
inline constexpr std::size_t kOutboxCapacity = 4096;
/// The most envelopes one send carries.
inline constexpr std::size_t kMaxBatch = 64;
/// The first wait before a poll after a failed one, when the binding declares
/// no retry policy; it doubles up to kMaxReconnectBackoff.
inline constexpr std::chrono::milliseconds kInitialReconnectBackoff{500};
/// The longest wait between polls after failed ones.
inline constexpr std::chrono::milliseconds kMaxReconnectBackoff{30000};

/// Who may claim a name (deploy.yaml `authenticator:`).
enum class Authenticator {
    /// `Authorization: Bearer <token>`, the token from the host.
    Bearer,
    /// `Authorization: SCE-Install <secret>`, a secret the client draws once.
    Install,
    /// A TLS client certificate the host configured on its adapter.
    Mtls,
};

/// An installation secret: 256 bits from the operating system's random
/// source.
using InstallSecret = std::array<std::uint8_t, 32>;

/// What the host gives the client besides the network. The host owns where a
/// token comes from and where a secret is kept.
class Credentials {
public:
    virtual ~Credentials() = default;
    /// The token a bearer client presents to `peer`, read before each
    /// exchange so a token the host renews is used at once. Empty when the
    /// host has none; the exchange then goes without one and the server
    /// refuses it.
    virtual std::optional<std::string> bearerToken(const std::string &peer) = 0;
    /// The installation secret kept for `peer`, if one was kept.
    virtual std::optional<InstallSecret> loadInstallSecret(const std::string &peer) = 0;
    /// Keep a secret the client drew for `peer`. The client uses it only when
    /// it was kept, since a secret lost at a restart would lock the name out
    /// of the server that pinned it.
    virtual bool storeInstallSecret(const std::string &peer, const InstallSecret &secret) = 0;
};

/// The binding's retry policy (deploy.yaml `retry:`, section 16.7 row 3), in
/// the shape of RetryingDispatcher::Policy without the router's scheduler: the
/// client is where an exchange's answer is seen, so it retries there.
struct RetryPolicy {
    std::uint32_t max_retries = 0;
    std::chrono::milliseconds initial_backoff{100};
    double backoff_multiplier = 2.0;
    std::chrono::milliseconds max_backoff{5000};
    std::uint32_t jitter_pct = 10;
};

/// One client's configuration, from the binding.
struct ClientConfig {
    /// The binding's `url`: `https://<host>[:<port>][<base>]`.
    std::string base_url;
    /// This machine's name: the last segment of the exchange path.
    std::string client;
    /// The peer's machine name, as the host keys its credentials.
    std::string peer;
    /// The binding's target (`#peer`), as a CommunicationError names it.
    std::string target;
    Authenticator authenticator = Authenticator::Bearer;
    /// The binding's `hold_ms`.
    std::chrono::milliseconds hold{25000};
    /// The binding's `retry:`, if it declared one.
    std::optional<RetryPolicy> retry;
};

/// What the client tells the router. Each runs on a client thread; none of
/// them may call back into the client's stop().
struct ClientEvents {
    std::function<void(MeshEnvelope)> receive;
    std::function<void()> ready;
    std::function<void(CommunicationError)> notReady;
    std::function<void(CommunicationError)> fault;
    /// Runs while the client holds the response it is taking in, so the
    /// router's requests are sent again before that response's envelopes
    /// reach it; it may queue envelopes on this client (send, offer).
    std::function<void()> restarted;
};

/// The requests a router sent over one `https` binding, each as it was handed
/// on to the client (its ids and its sequence stamp as they left), kept so
/// that a server that started again without the reply is asked again
/// (SCE_MESH.md section 22.4, "A new incarnation asks again"). Whether a
/// request still waits is the router's to say, from the table that correlates
/// its reply; one that no longer waits is let go whenever a request is
/// recorded or the waiting ones are read, so the table holds about what
/// waits. Thread-safe; `waits` is called with no lock of this table held.
class WaitingRequests {
public:
    using Waits = std::function<bool(const MeshEnvelope &)>;

    /// Keep `request`, which is leaving now, unless a request of the same
    /// envelope id is kept already (one sent again is that request).
    void record(const MeshEnvelope &request, const Waits &waits) {
        (void)stillWaiting(waits);
        std::lock_guard<std::mutex> lock(mutex_);
        for (const auto &kept : requests_) {
            if (kept->id == request.id) {
                return;
            }
        }
        requests_.push_back(std::make_shared<const MeshEnvelope>(request));
    }

    /// The requests that still wait, in the order they left; the others are
    /// let go.
    std::vector<MeshEnvelope> stillWaiting(const Waits &waits) {
        std::vector<std::shared_ptr<const MeshEnvelope>> snapshot;
        {
            std::lock_guard<std::mutex> lock(mutex_);
            snapshot = requests_;
        }
        std::vector<MeshEnvelope> waiting;
        std::vector<std::shared_ptr<const MeshEnvelope>> answered;
        for (const auto &kept : snapshot) {
            if (waits(*kept)) {
                waiting.push_back(*kept);
            } else {
                answered.push_back(kept);
            }
        }
        if (!answered.empty()) {
            std::lock_guard<std::mutex> lock(mutex_);
            requests_.erase(std::remove_if(requests_.begin(), requests_.end(),
                                           [&answered](const std::shared_ptr<const MeshEnvelope> &kept) {
                                               return std::find(answered.begin(), answered.end(), kept) !=
                                                      answered.end();
                                           }),
                            requests_.end());
        }
        return waiting;
    }

    /// How many requests are kept, waiting or not yet let go.
    std::size_t size() const {
        std::lock_guard<std::mutex> lock(mutex_);
        return requests_.size();
    }

private:
    mutable std::mutex mutex_;
    std::vector<std::shared_ptr<const MeshEnvelope>> requests_;
};

/// The host's part of a router with `https` bindings: the adapter it chose
/// and, for `bearer` and `install`, its credentials.
struct ClientHost {
    HttpsTransport *transport = nullptr;
    Credentials *credentials = nullptr;
};

// ── The wire, as functions the client and the conformance kit share ──

/// `<base>/sce-mesh/1/<client>`, one `/` between the base and the path.
std::string exchangeUrl(const std::string &base, const std::string &client);

/// A request or response body: each envelope's CBOR bytes, one after another.
std::vector<std::uint8_t> encodeBody(const std::vector<MeshEnvelope> &envelopes);

/// Where each item of a CBOR sequence starts and how long it is. False at the
/// first byte that starts no well-formed item, with `position` its offset;
/// the items before it are in `items`.
bool splitCborSequence(const std::uint8_t *data, std::size_t size,
                       std::vector<std::pair<std::size_t, std::size_t>> &items, std::size_t &position);

/// The value of the first header field named `name`, compared without regard
/// to ASCII case.
std::optional<std::string> headerValue(const std::vector<HttpsHeader> &headers, std::string_view name);

/// A number of decimal digits that fits 64 bits, with no sign and no space.
std::optional<std::uint64_t> parseUnsigned(std::string_view text);

/// How long a `Retry-After` value asks to wait from `now`: delay-seconds, or
/// an HTTP-date (RFC 9110 section 10.2.3; IMF-fixdate). Empty for a value of
/// neither form.
std::optional<std::chrono::milliseconds> retryAfter(std::string_view value, std::chrono::system_clock::time_point now);

/// Base64url without padding (RFC 4648 section 5), the form of an
/// installation secret in the Authorization header.
std::string base64url(const std::uint8_t *data, std::size_t size);

/// Draw an installation secret from the operating system's random source
/// (getrandom on Linux, arc4random_buf on the BSDs and Apple systems,
/// BCryptGenRandom on Windows). False when the source could not be read.
bool drawInstallSecret(InstallSecret &out);

/// A fresh mailbox incarnation, drawn from the same source as an installation
/// secret: kEpochBytes bytes as lowercase hexadecimal digits. Empty when the
/// source could not be read.
std::optional<std::string> drawEpoch();

/// Whether `text` names a mailbox incarnation: exactly 2 * kEpochBytes
/// lowercase hexadecimal digits.
bool isEpoch(std::string_view text);

/// The client of one `https` binding.
class Client {
public:
    Client(ClientConfig config, ClientHost host, ClientEvents events);
    Client(const Client &) = delete;
    Client &operator=(const Client &) = delete;
    ~Client();

    /// Start polling and sending. False when the host gave no adapter, or the
    /// client was started before.
    bool start();

    /// End every exchange and join the client's threads. What the outbox
    /// still holds is discarded. Idempotent.
    void stop();

    /// What became of an envelope offered for a send.
    enum class Offer {
        /// Queued for the next send.
        Queued,
        /// The client is not running.
        Stopped,
        /// The outbox holds kOutboxCapacity envelopes.
        Full,
    };

    /// Queue an envelope for the next send, raising nothing. For a caller
    /// that reports a refusal itself, as the section 10.10 OutboundBuffer's
    /// dispatcher does.
    Offer offer(const MeshEnvelope &env);

    /// Queue an envelope for the next send. False when the client is not
    /// running, or the outbox is full, which raises section 16.7 row 9.
    bool send(const MeshEnvelope &env);

    /// Whether the last exchange was answered `200`.
    bool ready() const noexcept {
        return readiness_.load(std::memory_order_acquire) == Readiness::Ready;
    }

    /// The highest mailbox number delivered so far, which the next request
    /// acknowledges.
    std::uint64_t acknowledged() const noexcept {
        return acknowledged_.load(std::memory_order_acquire);
    }

    /// The mailbox incarnation the acknowledgement counts in: the last one a
    /// `200` named, empty before the first.
    std::string epoch() const {
        std::lock_guard<std::mutex> deliver(deliver_mutex_);
        return epoch_;
    }

private:
    enum class Readiness { Unknown, Ready, NotReady };

    /// Where a request's acknowledgement stood when it was made.
    struct Mark {
        std::uint64_t ack = 0;
        std::string epoch;
    };

    /// One envelope waiting for a send, with the attempts made for it.
    struct Pending {
        MeshEnvelope env;
        std::uint32_t attempts = 0;
    };

    /// What one exchange's answer means for the client.
    struct Verdict {
        enum class Kind {
            /// `200`: the body was taken in.
            Taken,
            /// `429` or `503`: wait, then make the same exchange again.
            Shed,
            /// `413`: the body was too big.
            TooBig,
            /// A status the client cannot take back (a `4xx` but `408`).
            Refused,
            /// A status worth trying again (`408`, a `5xx` but `503`).
            Transient,
            /// No response.
            Failed,
        };
        Kind kind = Kind::Failed;
        std::chrono::milliseconds wait{0};
        std::string error;
    };

    void pollLoop();
    void sendLoop();
    HttpsRequest buildRequest(std::vector<std::uint8_t> body, std::chrono::milliseconds hold, Mark &sent);
    Verdict judge(const HttpsExchange &answer);
    void takeIn(const HttpsExchange &answer, const Mark &sent);
    void becomeReady();
    void becomeNotReady(const std::string &why);
    void refusedName(const HttpsExchange &answer);
    void failEnvelopes(std::vector<Pending> batch, const std::string &error, bool retryable);
    std::chrono::milliseconds backoffFor(std::uint32_t attempts);
    std::chrono::milliseconds reconnectBackoff(std::uint32_t failures);
    bool waitStopping(std::chrono::milliseconds wait);
    std::optional<std::string> authorization();
    void raise(CommunicationError err);

    ClientConfig config_;
    ClientHost host_;
    ClientEvents events_;
    HttpsCancellation cancellation_;

    std::mutex mutex_;
    std::condition_variable wake_;
    std::deque<Pending> outbox_;
    bool running_ = false;
    bool stopping_ = false;
    std::size_t batch_limit_ = kMaxBatch;

    /// Held while a response is taken in, and while a request reads where the
    /// acknowledgement stands, so the two read and move it together.
    mutable std::mutex deliver_mutex_;
    std::atomic<std::uint64_t> acknowledged_{0};
    /// The incarnation acknowledged_ counts in; under deliver_mutex_.
    std::string epoch_;
    std::atomic<Readiness> readiness_{Readiness::Unknown};
    std::atomic<bool> refused_{false};
    std::optional<InstallSecret> install_secret_;

    std::thread poller_;
    std::thread sender_;
};

}  // namespace SCE::Mesh::Https
