// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#include "Mailbox.h"

#include "mesh/MeshEnvelopeCodec.h"
#include "mesh/transports/HttpsClient.h"

#include <algorithm>

namespace SCE::Mesh::HttpsKit {

namespace {

/// How often a held exchange looks at its cancellation.
constexpr std::chrono::milliseconds kCancellationPoll{50};

}  // namespace

Mailbox::Mailbox(MailboxOptions options) : options_(std::move(options)) {}

void Mailbox::post(const MeshEnvelope &env) {
    std::lock_guard<std::mutex> lock(mutex_);
    mailbox_.push_back(Numbered{next_number_++, env});
    posted_.notify_all();
}

std::vector<MeshEnvelope> Mailbox::received() const {
    std::lock_guard<std::mutex> lock(mutex_);
    return received_;
}

std::size_t Mailbox::exchanges() const {
    std::lock_guard<std::mutex> lock(mutex_);
    return exchanges_;
}

std::vector<std::uint64_t> Mailbox::acks() const {
    std::lock_guard<std::mutex> lock(mutex_);
    return acks_;
}

void Mailbox::dropAfterActing(std::size_t count) {
    std::lock_guard<std::mutex> lock(mutex_);
    drop_ = count;
}

void Mailbox::holdPastBound(std::chrono::milliseconds extra) {
    std::lock_guard<std::mutex> lock(mutex_);
    hold_past_ = extra;
}

void Mailbox::shed(std::size_t count, std::uint16_t status, std::chrono::seconds retryAfter) {
    std::lock_guard<std::mutex> lock(mutex_);
    shed_ = count;
    shed_status_ = status;
    shed_after_ = retryAfter;
}

void Mailbox::wakeAll() {
    posted_.notify_all();
}

ServerAnswer Mailbox::status(std::uint16_t code) const {
    ServerAnswer answer;
    answer.status = code;
    return answer;
}

ServerAnswer Mailbox::answer(const ServerRequest &request, const std::function<bool()> &cancelled) {
    using namespace SCE::Mesh::Https;
    std::unique_lock<std::mutex> lock(mutex_);
    ++exchanges_;
    if (request.method != "POST") {
        return status(405);
    }

    // `<base>/sce-mesh/<version>/<client>`: an unknown version or name is 404.
    const std::string prefix = "/sce-mesh/";
    const std::size_t at = request.target.find(prefix);
    if (at == std::string::npos) {
        return status(404);
    }
    const std::string rest = request.target.substr(at + prefix.size());
    const std::size_t slash = rest.find('/');
    if (slash == std::string::npos || rest.substr(0, slash) != kBindingVersion ||
        rest.substr(slash + 1) != options_.client) {
        return status(404);
    }

    const auto authorization = headerValue(request.headers, "Authorization");
    if (!options_.bearer_token.empty()) {
        if (!authorization || *authorization != "Bearer " + options_.bearer_token) {
            return status(401);
        }
    } else {
        // Trust on first use: the first secret is pinned to the name.
        const std::string scheme = "SCE-Install ";
        if (!authorization || authorization->rfind(scheme, 0) != 0) {
            return status(401);
        }
        const std::string secret = authorization->substr(scheme.size());
        if (!pinned_secret_) {
            pinned_secret_ = secret;
        } else if (*pinned_secret_ != secret) {
            return status(401);
        }
    }

    if (shed_ > 0) {
        --shed_;
        ServerAnswer answer = status(shed_status_);
        answer.headers.emplace_back("Retry-After", std::to_string(shed_after_.count()));
        return answer;
    }
    if (request.body.size() > options_.max_body_bytes) {
        return status(413);
    }

    // Take the request's envelopes in: all of them, or none when one does
    // not decode.
    std::vector<std::pair<std::size_t, std::size_t>> items;
    std::size_t broken = 0;
    if (!splitCborSequence(request.body.data(), request.body.size(), items, broken)) {
        return status(400);
    }
    std::vector<MeshEnvelope> taken;
    for (const auto &item : items) {
        MeshEnvelope env;
        if (!decodeEnvelope(request.body.data() + item.first, item.second, env)) {
            return status(400);
        }
        taken.push_back(std::move(env));
    }
    received_.insert(received_.end(), taken.begin(), taken.end());

    // What the client acknowledges leaves the mailbox.
    std::uint64_t ack = 0;
    if (const auto value = headerValue(request.headers, kAckHeader)) {
        ack = parseUnsigned(*value).value_or(0);
    }
    acks_.push_back(ack);
    while (!mailbox_.empty() && mailbox_.front().number <= ack) {
        mailbox_.pop_front();
    }

    // Hold an exchange with nothing to answer, at most the server's bound.
    std::chrono::milliseconds hold{0};
    if (const auto value = headerValue(request.headers, kHoldHeader)) {
        hold = std::chrono::milliseconds(static_cast<long long>(parseUnsigned(*value).value_or(0)));
    }
    hold = std::min(hold, options_.max_hold);
    const auto until = std::chrono::steady_clock::now() + hold;
    while (mailbox_.empty() && !cancelled() && std::chrono::steady_clock::now() < until) {
        posted_.wait_until(lock, std::min(until, std::chrono::steady_clock::now() + kCancellationPoll));
    }

    if (drop_ > 0) {
        --drop_;
        ServerAnswer answer;
        answer.kind = ServerAnswer::Kind::Drop;
        return answer;
    }
    if (hold_past_.count() > 0) {
        const auto past = std::chrono::steady_clock::now() + hold_past_;
        hold_past_ = std::chrono::milliseconds(0);
        while (!cancelled() && std::chrono::steady_clock::now() < past) {
            posted_.wait_until(lock, std::min(past, std::chrono::steady_clock::now() + kCancellationPoll));
        }
    }

    ServerAnswer answer;
    answer.status = 200;
    answer.headers.emplace_back("Content-Type", kContentType);
    if (!mailbox_.empty()) {
        answer.headers.emplace_back(kFirstHeader, std::to_string(mailbox_.front().number));
        std::vector<MeshEnvelope> carried;
        for (const auto &numbered : mailbox_) {
            carried.push_back(numbered.env);
        }
        answer.body = encodeBody(carried);
    }
    return answer;
}

HttpsExchange LoopbackTransport::exchange(const HttpsRequest &request) {
    ServerRequest serverRequest;
    serverRequest.method = "POST";
    // The path after the authority: `https://host[:port]<path>`.
    const std::size_t scheme = request.url.find("://");
    const std::size_t path = scheme == std::string::npos ? std::string::npos : request.url.find('/', scheme + 3);
    serverRequest.target = path == std::string::npos ? "/" : request.url.substr(path);
    serverRequest.headers = request.headers;
    serverRequest.body = request.body;

    const auto started = std::chrono::steady_clock::now();
    const ServerAnswer answer = mailbox_.answer(
        serverRequest, [&request] { return request.cancellation != nullptr && request.cancellation->requested(); });
    if (request.cancellation != nullptr && request.cancellation->requested()) {
        return HttpsExchange::failed(HttpsExchange::Failure::Other, "cancelled");
    }
    if (request.timeout.count() > 0 && std::chrono::steady_clock::now() - started > request.timeout) {
        return HttpsExchange::failed(HttpsExchange::Failure::Timeout, "the exchange ran past its bound");
    }
    if (answer.kind == ServerAnswer::Kind::Drop) {
        return HttpsExchange::failed(HttpsExchange::Failure::Connect, "the connection closed before a response");
    }
    return HttpsExchange::response(answer.status, answer.headers, answer.body);
}

}  // namespace SCE::Mesh::HttpsKit
