// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The server side of the https binding's wire (docs/adr/0107, D2) as the
// conformance kit needs it: one client's mailbox and the answer to one
// exchange, with no network. The kit's TLS server (HttpsKit.h) puts it behind
// a socket to hold an adapter to the wire; LoopbackTransport puts it behind
// the port directly to hold the client (mesh/transports/HttpsClient.h) to the
// binding's semantics without one.
//
// It is the wire's other half for tests, not a server of the binding: the
// Rust host core's server is that (sce-rust-mesh, feature `https`). What it
// adds is the faults a test asks for: a response lost after the server acted,
// an exchange held past the client's bound, load shed with Retry-After.

#pragma once

#include "mesh/MeshEnvelope.h"
#include "mesh/transports/HttpsTransport.h"

#include <chrono>
#include <condition_variable>
#include <cstddef>
#include <cstdint>
#include <deque>
#include <functional>
#include <mutex>
#include <optional>
#include <string>
#include <utility>
#include <vector>

namespace SCE::Mesh::HttpsKit {

/// One request as the server reads it.
struct ServerRequest {
    std::string method;
    /// The path and query, as the request line carries them.
    std::string target;
    std::vector<HttpsHeader> headers;
    std::vector<std::uint8_t> body;
};

/// What the server does with one exchange.
struct ServerAnswer {
    enum class Kind {
        /// Answer with this response.
        Respond,
        /// Close the connection without a response: the server acted on the
        /// request and the response is lost.
        Drop,
    };
    Kind kind = Kind::Respond;
    std::uint16_t status = 200;
    std::vector<HttpsHeader> headers;
    std::vector<std::uint8_t> body;
};

/// How the mailbox authenticates the one client it serves.
struct MailboxOptions {
    /// The client's machine name, the last segment of the exchange path.
    std::string client = "phone";
    /// The token a `bearer` client must present; empty for an `install`
    /// client, whose first secret is pinned.
    std::string bearer_token;
    /// The longest hold the server grants.
    std::chrono::milliseconds max_hold{2000};
    /// The largest request body taken in before `413`.
    std::size_t max_body_bytes = 1u << 20;
};

class Mailbox {
public:
    /// An empty mailbox, numbering from 1 in an incarnation drawn from the
    /// operating system's random source. Throws std::runtime_error when the
    /// source could not be read.
    explicit Mailbox(MailboxOptions options);

    /// A send of the server to the client: it waits in the mailbox until an
    /// acknowledged exchange carried it.
    void post(const MeshEnvelope &env);

    /// The server started again with its mailbox empty: what waited is gone,
    /// and the mailbox numbers from 1 in a new incarnation.
    void restart();

    /// The incarnation the mailbox numbers in (`Sce-Mesh-Epoch`).
    std::string epoch() const;

    /// Every envelope the client's request bodies carried, in arrival order,
    /// each copy a resend carried again included.
    std::vector<MeshEnvelope> received() const;

    /// How many exchanges the server answered or dropped.
    std::size_t exchanges() const;

    /// The `Sce-Mesh-Ack` of each exchange, in order.
    std::vector<std::uint64_t> acks() const;

    /// The next `count` exchanges are taken in and then dropped.
    void dropAfterActing(std::size_t count);

    /// The next exchange is held this long past its hold before it is
    /// answered, so a client bound shorter sees a timeout.
    void holdPastBound(std::chrono::milliseconds extra);

    /// The next `count` exchanges are answered `status` with `Retry-After`,
    /// and nothing of them is taken in.
    void shed(std::size_t count, std::uint16_t status, std::chrono::seconds retryAfter);

    /// Answer one exchange. `cancelled` is asked while it is held, so a
    /// server shutting down does not wait the hold out.
    ServerAnswer answer(const ServerRequest &request, const std::function<bool()> &cancelled);

    /// Wake every held exchange (a server shutting down).
    void wakeAll();

private:
    struct Numbered {
        std::uint64_t number;
        MeshEnvelope env;
    };

    ServerAnswer status(std::uint16_t code) const;

    MailboxOptions options_;
    mutable std::mutex mutex_;
    std::condition_variable posted_;
    std::deque<Numbered> mailbox_;
    std::uint64_t next_number_ = 1;
    std::string epoch_;
    std::vector<MeshEnvelope> received_;
    std::vector<std::uint64_t> acks_;
    std::size_t exchanges_ = 0;
    std::size_t drop_ = 0;
    std::chrono::milliseconds hold_past_{0};
    std::size_t shed_ = 0;
    std::uint16_t shed_status_ = 503;
    std::chrono::seconds shed_after_{0};
    std::optional<std::string> pinned_secret_;
};

/// The port, answered by a Mailbox in this process: what the client sees of
/// a server, with no socket. A dropped answer is Failure::Connect, as a
/// connection closed before a response is.
class LoopbackTransport final : public HttpsTransport {
public:
    explicit LoopbackTransport(Mailbox &mailbox) : mailbox_(mailbox) {}

    HttpsExchange exchange(const HttpsRequest &request) override;

private:
    Mailbox &mailbox_;
};

}  // namespace SCE::Mesh::HttpsKit
