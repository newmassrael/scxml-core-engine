// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The client of the Mesh binding over HTTP request and response
// (docs/adr/0107, D2). The contract is stated in
// mesh/transports/HttpsClient.h; this file is its one implementation, shared
// by every generated router and by the conformance kit.

#include "mesh/transports/HttpsClient.h"

#include "mesh/MeshEnvelopeCodec.h"

#include <algorithm>
#include <cstdio>
#include <iterator>
#include <limits>
#include <random>

#if defined(_WIN32)
// Kept to this file: no public header names a Windows type, so no consumer
// receives the macros windows.h defines.
#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#ifndef NOMINMAX
#define NOMINMAX
#endif
#include <windows.h>

#include <bcrypt.h>
#elif defined(__linux__)
#include <cerrno>
#include <sys/random.h>
#elif defined(__APPLE__) || defined(__FreeBSD__) || defined(__OpenBSD__) || defined(__NetBSD__)
#include <stdlib.h>
#endif

namespace SCE::Mesh::Https {

namespace {

// ── CBOR sequence (RFC 8742): where each item ends ──

/// The deepest nesting an item may have before the scan refuses it, so a
/// hostile body cannot exhaust the stack.
constexpr int kMaxCborDepth = 64;

/// Read the head of the item at `pos` (RFC 8949 section 3): its major type,
/// its additional information and its argument. False when the bytes end
/// first or the additional information is reserved.
bool readHead(const std::uint8_t *data, std::size_t size, std::size_t &pos, std::uint8_t &major, std::uint8_t &info,
              std::uint64_t &arg) {
    if (pos >= size) {
        return false;
    }
    const std::uint8_t initial = data[pos++];
    major = static_cast<std::uint8_t>(initial >> 5);
    info = static_cast<std::uint8_t>(initial & 0x1f);
    if (info < 24) {
        arg = info;
        return true;
    }
    std::size_t length = 0;
    switch (info) {
    case 24:
        length = 1;
        break;
    case 25:
        length = 2;
        break;
    case 26:
        length = 4;
        break;
    case 27:
        length = 8;
        break;
    case 31:
        arg = 0;
        return true;
    default:
        return false;
    }
    if (size - pos < length) {
        return false;
    }
    arg = 0;
    for (std::size_t i = 0; i < length; ++i) {
        arg = (arg << 8) | data[pos++];
    }
    return true;
}

/// Move `pos` past one well-formed item. False when it is not one.
bool skipItem(const std::uint8_t *data, std::size_t size, std::size_t &pos, int depth) {
    if (depth > kMaxCborDepth) {
        return false;
    }
    std::uint8_t major = 0;
    std::uint8_t info = 0;
    std::uint64_t arg = 0;
    if (!readHead(data, size, pos, major, info, arg)) {
        return false;
    }
    const bool indefinite = info == 31;
    switch (major) {
    case 0:
    case 1:
        return !indefinite;
    case 2:
    case 3:
        if (!indefinite) {
            if (arg > size - pos) {
                return false;
            }
            pos += static_cast<std::size_t>(arg);
            return true;
        }
        // Chunks of the same major type, each definite, until the break.
        for (;;) {
            if (pos >= size) {
                return false;
            }
            if (data[pos] == 0xff) {
                ++pos;
                return true;
            }
            std::uint8_t chunkMajor = 0;
            std::uint8_t chunkInfo = 0;
            std::uint64_t chunkLength = 0;
            if (!readHead(data, size, pos, chunkMajor, chunkInfo, chunkLength) || chunkMajor != major ||
                chunkInfo == 31 || chunkLength > size - pos) {
                return false;
            }
            pos += static_cast<std::size_t>(chunkLength);
        }
    case 4:
    case 5: {
        if (!indefinite) {
            std::uint64_t count = arg;
            if (major == 5) {
                if (count > std::numeric_limits<std::uint64_t>::max() / 2) {
                    return false;
                }
                count *= 2;
            }
            // Every item is at least one byte, so a count past what is left
            // cannot be met.
            if (count > size - pos) {
                return false;
            }
            for (std::uint64_t i = 0; i < count; ++i) {
                if (!skipItem(data, size, pos, depth + 1)) {
                    return false;
                }
            }
            return true;
        }
        for (;;) {
            if (pos >= size) {
                return false;
            }
            if (data[pos] == 0xff) {
                ++pos;
                return true;
            }
            if (!skipItem(data, size, pos, depth + 1)) {
                return false;
            }
            if (major == 5) {
                if (pos < size && data[pos] == 0xff) {
                    return false;
                }
                if (!skipItem(data, size, pos, depth + 1)) {
                    return false;
                }
            }
        }
    }
    case 6:
        return !indefinite && skipItem(data, size, pos, depth + 1);
    case 7:
        // A simple value or a float, whose bytes readHead took; a break is
        // well-formed only inside an indefinite-length item.
        return !indefinite;
    default:
        return false;
    }
}

// ── Text ──

char lowerAscii(char c) {
    return (c >= 'A' && c <= 'Z') ? static_cast<char>(c - 'A' + 'a') : c;
}

bool equalsIgnoringCase(std::string_view a, std::string_view b) {
    if (a.size() != b.size()) {
        return false;
    }
    for (std::size_t i = 0; i < a.size(); ++i) {
        if (lowerAscii(a[i]) != lowerAscii(b[i])) {
            return false;
        }
    }
    return true;
}

/// Days from 1970-01-01 to the civil date given (the proleptic Gregorian
/// calendar), as in Howard Hinnant's `days_from_civil`.
std::int64_t daysFromCivil(std::int64_t year, unsigned month, unsigned day) {
    year -= month <= 2 ? 1 : 0;
    const std::int64_t era = (year >= 0 ? year : year - 399) / 400;
    const auto yoe = static_cast<unsigned>(year - era * 400);
    const unsigned shifted = month > 2 ? month - 3 : month + 9;
    const unsigned doy = (153 * shifted + 2) / 5 + day - 1;
    const unsigned doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    return era * 146097 + static_cast<std::int64_t>(doe) - 719468;
}

/// Seconds since 1970 of an IMF-fixdate (`Sun, 06 Nov 1994 08:49:37 GMT`).
std::optional<std::int64_t> parseImfFixdate(std::string_view text) {
    static constexpr std::array<const char *, 12> kMonths = {"Jan", "Feb", "Mar", "Apr", "May", "Jun",
                                                             "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"};
    if (text.size() != 29 || text[3] != ',' || text[4] != ' ' || text[7] != ' ' || text[11] != ' ' || text[16] != ' ' ||
        text[19] != ':' || text[22] != ':' || text[25] != ' ' || text.substr(26) != "GMT") {
        return std::nullopt;
    }
    const auto number = [&](std::size_t at, std::size_t length) -> std::optional<unsigned> {
        const auto value = parseUnsigned(text.substr(at, length));
        if (!value || *value > 9999) {
            return std::nullopt;
        }
        return static_cast<unsigned>(*value);
    };
    const auto day = number(5, 2);
    const auto year = number(12, 4);
    const auto hour = number(17, 2);
    const auto minute = number(20, 2);
    const auto second = number(23, 2);
    unsigned month = 0;
    for (unsigned i = 0; i < kMonths.size(); ++i) {
        if (text.substr(8, 3) == kMonths[i]) {
            month = i + 1;
        }
    }
    if (!day || !year || !hour || !minute || !second || month == 0 || *day == 0 || *day > 31 || *hour > 23 ||
        *minute > 59 || *second > 60) {
        return std::nullopt;
    }
    return daysFromCivil(*year, month, *day) * 86400 + static_cast<std::int64_t>(*hour) * 3600 +
           static_cast<std::int64_t>(*minute) * 60 + static_cast<std::int64_t>(*second);
}

// ── Pacing ──

/// `base`, moved by up to `pct` percent either way.
std::chrono::milliseconds withJitter(std::chrono::milliseconds base, std::uint32_t pct) {
    if (pct == 0 || base.count() <= 0) {
        return base;
    }
    thread_local std::mt19937 rng{std::random_device{}()};
    const long long b = base.count();
    const long long delta = b * static_cast<long long>(std::min<std::uint32_t>(pct, 100)) / 100;
    std::uniform_int_distribution<long long> spread(-delta, delta);
    return std::chrono::milliseconds(std::max<long long>(1, b + spread(rng)));
}

/// The `step`-th wait of a schedule that starts at `initial` and grows by
/// `multiplier` up to `cap` (step 1 is `initial`).
std::chrono::milliseconds grow(std::chrono::milliseconds initial, double multiplier, std::chrono::milliseconds cap,
                               std::uint32_t step) {
    double wait = static_cast<double>(initial.count());
    const auto ceiling = static_cast<double>(cap.count());
    for (std::uint32_t i = 1; i < step && wait < ceiling; ++i) {
        wait *= multiplier;
    }
    return std::chrono::milliseconds(static_cast<long long>(std::min(wait, ceiling)));
}

CommunicationError communicationError(ReasonCode reason, const ClientConfig &config) {
    CommunicationError err;
    err.reason = reason;
    err.transport = kTransportName;
    err.target = config.target;
    return err;
}

}  // namespace

// ── The wire ──

std::string exchangeUrl(const std::string &base, const std::string &client) {
    std::string url = base;
    while (!url.empty() && url.back() == '/') {
        url.pop_back();
    }
    url += "/sce-mesh/";
    url += kBindingVersion;
    url += '/';
    url += client;
    return url;
}

std::vector<std::uint8_t> encodeBody(const std::vector<MeshEnvelope> &envelopes) {
    std::vector<std::uint8_t> body;
    for (const auto &env : envelopes) {
        const auto bytes = encodeEnvelope(env);
        body.insert(body.end(), bytes.begin(), bytes.end());
    }
    return body;
}

bool splitCborSequence(const std::uint8_t *data, std::size_t size,
                       std::vector<std::pair<std::size_t, std::size_t>> &items, std::size_t &position) {
    std::size_t pos = 0;
    while (pos < size) {
        const std::size_t start = pos;
        if (!skipItem(data, size, pos, 0)) {
            position = start;
            return false;
        }
        items.emplace_back(start, pos - start);
    }
    position = size;
    return true;
}

std::optional<std::string> headerValue(const std::vector<HttpsHeader> &headers, std::string_view name) {
    for (const auto &header : headers) {
        if (equalsIgnoringCase(header.first, name)) {
            return header.second;
        }
    }
    return std::nullopt;
}

std::optional<std::uint64_t> parseUnsigned(std::string_view text) {
    if (text.empty()) {
        return std::nullopt;
    }
    std::uint64_t value = 0;
    for (const char c : text) {
        if (c < '0' || c > '9') {
            return std::nullopt;
        }
        const auto digit = static_cast<std::uint64_t>(c - '0');
        if (value > (std::numeric_limits<std::uint64_t>::max() - digit) / 10) {
            return std::nullopt;
        }
        value = value * 10 + digit;
    }
    return value;
}

std::optional<std::chrono::milliseconds> retryAfter(std::string_view value, std::chrono::system_clock::time_point now) {
    while (!value.empty() && (value.front() == ' ' || value.front() == '\t')) {
        value.remove_prefix(1);
    }
    while (!value.empty() && (value.back() == ' ' || value.back() == '\t')) {
        value.remove_suffix(1);
    }
    // A delay past a day is held to a day: a server that asks for longer is
    // asking for something no waiting client will keep.
    constexpr std::uint64_t kLongestSeconds = 86400;
    if (const auto seconds = parseUnsigned(value)) {
        return std::chrono::milliseconds(static_cast<long long>(std::min(*seconds, kLongestSeconds) * 1000));
    }
    const auto at = parseImfFixdate(value);
    if (!at) {
        return std::nullopt;
    }
    const auto nowSeconds = std::chrono::duration_cast<std::chrono::seconds>(now.time_since_epoch()).count();
    const long long ahead = *at - static_cast<long long>(nowSeconds);
    const long long clamped =
        std::max<long long>(0, std::min<long long>(ahead, static_cast<long long>(kLongestSeconds)));
    return std::chrono::milliseconds(clamped * 1000);
}

std::string base64url(const std::uint8_t *data, std::size_t size) {
    static constexpr char kAlphabet[] = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    std::string out;
    out.reserve((size * 4 + 2) / 3);
    std::size_t i = 0;
    for (; i + 3 <= size; i += 3) {
        const std::uint32_t triple =
            (static_cast<std::uint32_t>(data[i]) << 16) | (static_cast<std::uint32_t>(data[i + 1]) << 8) | data[i + 2];
        out += kAlphabet[(triple >> 18) & 0x3f];
        out += kAlphabet[(triple >> 12) & 0x3f];
        out += kAlphabet[(triple >> 6) & 0x3f];
        out += kAlphabet[triple & 0x3f];
    }
    const std::size_t rest = size - i;
    if (rest == 1) {
        const std::uint32_t one = static_cast<std::uint32_t>(data[i]) << 16;
        out += kAlphabet[(one >> 18) & 0x3f];
        out += kAlphabet[(one >> 12) & 0x3f];
    } else if (rest == 2) {
        const std::uint32_t two =
            (static_cast<std::uint32_t>(data[i]) << 16) | (static_cast<std::uint32_t>(data[i + 1]) << 8);
        out += kAlphabet[(two >> 18) & 0x3f];
        out += kAlphabet[(two >> 12) & 0x3f];
        out += kAlphabet[(two >> 6) & 0x3f];
    }
    return out;
}

namespace {

/// Fill `size` bytes at `data` from the operating system's random source.
/// False when the source could not be read.
bool fillFromSystem(std::uint8_t *data, std::size_t size) {
#if defined(_WIN32)
    const NTSTATUS status = BCryptGenRandom(nullptr, data, static_cast<ULONG>(size), BCRYPT_USE_SYSTEM_PREFERRED_RNG);
    return BCRYPT_SUCCESS(status);
#elif defined(__linux__)
    std::size_t filled = 0;
    while (filled < size) {
        const ssize_t got = getrandom(data + filled, size - filled, 0);
        if (got < 0) {
            if (errno == EINTR) {
                continue;
            }
            return false;
        }
        filled += static_cast<std::size_t>(got);
    }
    return true;
#elif defined(__APPLE__) || defined(__FreeBSD__) || defined(__OpenBSD__) || defined(__NetBSD__)
    arc4random_buf(data, size);
    return true;
#else
    // Any other POSIX system: the kernel's random device.
    std::FILE *source = std::fopen("/dev/urandom", "rb");
    if (source == nullptr) {
        return false;
    }
    const std::size_t got = std::fread(data, 1, size, source);
    std::fclose(source);
    return got == size;
#endif
}

}  // namespace

bool drawInstallSecret(InstallSecret &out) {
    return fillFromSystem(out.data(), out.size());
}

std::optional<std::string> drawEpoch() {
    static constexpr char kDigits[] = "0123456789abcdef";
    std::array<std::uint8_t, kEpochBytes> bytes{};
    if (!fillFromSystem(bytes.data(), bytes.size())) {
        return std::nullopt;
    }
    std::string epoch;
    epoch.reserve(2 * bytes.size());
    for (const std::uint8_t byte : bytes) {
        epoch += kDigits[byte >> 4];
        epoch += kDigits[byte & 0x0f];
    }
    return epoch;
}

bool isEpoch(std::string_view text) {
    return text.size() == 2 * kEpochBytes && std::all_of(text.begin(), text.end(), [](char c) {
               return (c >= '0' && c <= '9') || (c >= 'a' && c <= 'f');
           });
}

// ── The client ──

Client::Client(ClientConfig config, ClientHost host, ClientEvents events)
    : config_(std::move(config)), host_(host), events_(std::move(events)) {}

Client::~Client() {
    stop();
}

bool Client::start() {
    {
        std::lock_guard<std::mutex> lock(mutex_);
        if (running_ || stopping_ || host_.transport == nullptr) {
            return false;
        }
        running_ = true;
    }
    if (config_.authenticator == Authenticator::Install) {
        // Drawn once and kept where the host says; a secret the host could
        // not keep is not used, since the server pins the name to the first
        // secret it sees and a lost one would lock the name out.
        std::optional<InstallSecret> kept;
        if (host_.credentials != nullptr) {
            kept = host_.credentials->loadInstallSecret(config_.peer);
            if (!kept) {
                InstallSecret drawn{};
                if (drawInstallSecret(drawn) && host_.credentials->storeInstallSecret(config_.peer, drawn)) {
                    kept = drawn;
                }
            }
        }
        if (kept) {
            install_secret_ = kept;
        } else {
            auto err = communicationError(ReasonCode::Unauthorized, config_);
            err.detail = "no installation secret: the host keeps none and could not keep a new one";
            raise(std::move(err));
        }
    }
    poller_ = std::thread(&Client::pollLoop, this);
    sender_ = std::thread(&Client::sendLoop, this);
    return true;
}

void Client::stop() {
    {
        std::lock_guard<std::mutex> lock(mutex_);
        if (!running_ || stopping_) {
            return;
        }
        stopping_ = true;
    }
    cancellation_.request();
    wake_.notify_all();
    if (poller_.joinable()) {
        poller_.join();
    }
    if (sender_.joinable()) {
        sender_.join();
    }
    std::lock_guard<std::mutex> lock(mutex_);
    outbox_.clear();
    running_ = false;
}

Client::Offer Client::offer(const MeshEnvelope &env) {
    std::lock_guard<std::mutex> lock(mutex_);
    if (!running_ || stopping_) {
        return Offer::Stopped;
    }
    if (outbox_.size() >= kOutboxCapacity) {
        return Offer::Full;
    }
    outbox_.push_back(Pending{env, 0});
    wake_.notify_all();
    return Offer::Queued;
}

bool Client::send(const MeshEnvelope &env) {
    switch (offer(env)) {
    case Offer::Queued:
        return true;
    case Offer::Stopped:
        return false;
    case Offer::Full:
        break;
    }
    auto err = communicationError(ReasonCode::BackpressureDrop, config_);
    err.queue_depth = static_cast<std::int64_t>(kOutboxCapacity);
    err.envelope_id = env.id;
    raise(std::move(err));
    return false;
}

std::optional<std::string> Client::authorization() {
    switch (config_.authenticator) {
    case Authenticator::Bearer:
        if (host_.credentials != nullptr) {
            if (auto token = host_.credentials->bearerToken(config_.peer)) {
                return "Bearer " + *token;
            }
        }
        return std::nullopt;
    case Authenticator::Install:
        if (install_secret_) {
            return "SCE-Install " + base64url(install_secret_->data(), install_secret_->size());
        }
        return std::nullopt;
    case Authenticator::Mtls:
        return std::nullopt;
    }
    return std::nullopt;
}

HttpsRequest Client::buildRequest(std::vector<std::uint8_t> body, std::chrono::milliseconds hold, Mark &sent) {
    {
        std::lock_guard<std::mutex> deliver(deliver_mutex_);
        sent.ack = acknowledged_.load(std::memory_order_acquire);
        sent.epoch = epoch_;
    }
    HttpsRequest request;
    request.url = exchangeUrl(config_.base_url, config_.client);
    request.headers.emplace_back("Content-Type", kContentType);
    request.headers.emplace_back("Accept", kContentType);
    request.headers.emplace_back(kAckHeader, std::to_string(sent.ack));
    if (!sent.epoch.empty()) {
        request.headers.emplace_back(kEpochHeader, sent.epoch);
    }
    request.headers.emplace_back(kHoldHeader, std::to_string(hold.count()));
    if (auto value = authorization()) {
        request.headers.emplace_back("Authorization", std::move(*value));
    }
    request.body = std::move(body);
    request.timeout = hold + kExchangeGrace;
    request.cancellation = &cancellation_;
    return request;
}

Client::Verdict Client::judge(const HttpsExchange &answer) {
    Verdict verdict;
    if (answer.kind == HttpsExchange::Kind::Failure) {
        verdict.kind = Verdict::Kind::Failed;
        verdict.error = std::string("no response (") + failureName(answer.failure) + ")";
        if (!answer.detail.empty()) {
            verdict.error += ": " + answer.detail;
        }
        return verdict;
    }
    verdict.error = "HTTP " + std::to_string(answer.status);
    const std::uint16_t status = answer.status;
    if (status == 200) {
        verdict.kind = Verdict::Kind::Taken;
    } else if (status == 429 || status == 503) {
        verdict.kind = Verdict::Kind::Shed;
        if (const auto value = headerValue(answer.headers, "Retry-After")) {
            if (const auto wait = retryAfter(*value, std::chrono::system_clock::now())) {
                verdict.wait = *wait;
            }
        }
    } else if (status == 413) {
        verdict.kind = Verdict::Kind::TooBig;
    } else if (status == 408 || (status >= 500 && status <= 599)) {
        verdict.kind = Verdict::Kind::Transient;
    } else {
        verdict.kind = Verdict::Kind::Refused;
    }
    return verdict;
}

void Client::takeIn(const HttpsExchange &answer, const Mark &sent) {
    // The incarnation the response's numbers count in (section 22.3).
    const auto epochText = headerValue(answer.headers, kEpochHeader);
    if (!epochText || !isEpoch(*epochText)) {
        if (!answer.body.empty()) {
            // Without an incarnation the numbers say nothing the client can
            // acknowledge, so the body is reported once here and not taken.
            auto err = communicationError(ReasonCode::EnvelopeCorrupt, config_);
            err.codec = "cbor";
            err.detail = "a response body without a valid Sce-Mesh-Epoch";
            raise(std::move(err));
        }
        return;
    }

    std::lock_guard<std::mutex> deliver(deliver_mutex_);
    // The numbering the client knows; one it has left since the request went,
    // because the other exchange's response named a newer one, whose numbers
    // above what the request acknowledged are handed on and the section 10.5
    // window drops what arrived before; or a mailbox that numbers anew (a
    // server that started again with it empty), in which the acknowledgement
    // starts over at 0.
    bool current = true;
    if (*epochText != epoch_) {
        if (!sent.epoch.empty() && *epochText == sent.epoch) {
            current = false;
        } else {
            epoch_ = *epochText;
            acknowledged_.store(0, std::memory_order_release);
        }
    }
    if (answer.body.empty()) {
        return;
    }
    const auto firstText = headerValue(answer.headers, kFirstHeader);
    const auto first = firstText ? parseUnsigned(*firstText) : std::nullopt;
    if (!first || *first == 0) {
        // Without a number nothing in the body can be acknowledged, so it is
        // reported once here and carried again by the server.
        auto err = communicationError(ReasonCode::EnvelopeCorrupt, config_);
        err.codec = "cbor";
        err.detail = "a response body without a valid Sce-Mesh-First";
        raise(std::move(err));
        return;
    }
    std::vector<std::pair<std::size_t, std::size_t>> items;
    std::size_t brokenAt = 0;
    const bool whole = splitCborSequence(answer.body.data(), answer.body.size(), items, brokenAt);

    const std::uint64_t taken = current ? acknowledged_.load(std::memory_order_acquire) : sent.ack;
    for (std::size_t i = 0; i < items.size(); ++i) {
        const std::uint64_t number = *first + i;
        if (number <= taken) {
            // Carried again because a response was lost; taken in before.
            continue;
        }
        MeshEnvelope env;
        if (decodeEnvelope(answer.body.data() + items[i].first, items[i].second, env)) {
            if (events_.receive) {
                events_.receive(std::move(env));
            }
        } else {
            auto err = communicationError(ReasonCode::EnvelopeCorrupt, config_);
            err.codec = "cbor";
            err.position = static_cast<std::int64_t>(items[i].first);
            raise(std::move(err));
        }
        if (current) {
            acknowledged_.store(number, std::memory_order_release);
        }
    }
    if (!whole) {
        // The item that does not parse is acknowledged with the ones before
        // it, so the server drops it and the next response carries the rest.
        auto err = communicationError(ReasonCode::EnvelopeCorrupt, config_);
        err.codec = "cbor";
        err.position = static_cast<std::int64_t>(brokenAt);
        raise(std::move(err));
        const std::uint64_t broken = *first + items.size();
        if (current && broken > acknowledged_.load(std::memory_order_acquire)) {
            acknowledged_.store(broken, std::memory_order_release);
        }
    }
}

void Client::becomeReady() {
    refused_.store(false, std::memory_order_release);
    if (readiness_.exchange(Readiness::Ready, std::memory_order_acq_rel) != Readiness::Ready && events_.ready) {
        events_.ready();
    }
}

void Client::becomeNotReady(const std::string &why) {
    if (readiness_.exchange(Readiness::NotReady, std::memory_order_acq_rel) == Readiness::NotReady) {
        return;
    }
    if (events_.notReady) {
        auto err = communicationError(ReasonCode::TransportUnavailable, config_);
        err.detail = why;
        events_.notReady(std::move(err));
    }
}

void Client::refusedName(const HttpsExchange &answer) {
    if (refused_.exchange(true, std::memory_order_acq_rel)) {
        return;
    }
    auto err = communicationError(ReasonCode::Unauthorized, config_);
    err.transport_status = std::to_string(answer.status);
    raise(std::move(err));
}

void Client::failEnvelopes(std::vector<Pending> batch, const std::string &error, bool retryable) {
    if (!config_.retry) {
        for (const auto &pending : batch) {
            auto err = communicationError(ReasonCode::SendFailed, config_);
            err.transport_error = error;
            err.envelope_id = pending.env.id;
            raise(std::move(err));
        }
        return;
    }
    std::vector<Pending> again;
    std::uint32_t fewest = std::numeric_limits<std::uint32_t>::max();
    for (auto &pending : batch) {
        pending.attempts += 1;
        if (!retryable || pending.attempts > config_.retry->max_retries) {
            auto err = communicationError(ReasonCode::DeliveryExhausted, config_);
            err.transport_error = error;
            err.attempts = static_cast<std::int64_t>(pending.attempts);
            err.envelope_id = pending.env.id;
            raise(std::move(err));
        } else {
            fewest = std::min(fewest, pending.attempts);
            again.push_back(std::move(pending));
        }
    }
    if (again.empty()) {
        return;
    }
    {
        std::lock_guard<std::mutex> lock(mutex_);
        outbox_.insert(outbox_.begin(), std::make_move_iterator(again.begin()), std::make_move_iterator(again.end()));
    }
    (void)waitStopping(backoffFor(fewest));
}

std::chrono::milliseconds Client::backoffFor(std::uint32_t attempts) {
    const RetryPolicy &policy = *config_.retry;
    return withJitter(grow(policy.initial_backoff, policy.backoff_multiplier, policy.max_backoff, attempts),
                      policy.jitter_pct);
}

std::chrono::milliseconds Client::reconnectBackoff(std::uint32_t failures) {
    const std::uint32_t step = std::max<std::uint32_t>(1, failures);
    if (config_.retry) {
        return backoffFor(step);
    }
    return grow(kInitialReconnectBackoff, 2.0, kMaxReconnectBackoff, step);
}

bool Client::waitStopping(std::chrono::milliseconds wait) {
    std::unique_lock<std::mutex> lock(mutex_);
    return wake_.wait_for(lock, wait, [&] { return stopping_; });
}

void Client::raise(CommunicationError err) {
    if (events_.fault) {
        events_.fault(std::move(err));
    }
}

void Client::pollLoop() {
    std::uint32_t failures = 0;
    for (;;) {
        {
            std::lock_guard<std::mutex> lock(mutex_);
            if (stopping_) {
                return;
            }
        }
        Mark sent;
        const HttpsExchange answer = host_.transport->exchange(buildRequest({}, config_.hold, sent));
        if (cancellation_.requested()) {
            return;
        }
        const Verdict verdict = judge(answer);
        switch (verdict.kind) {
        case Verdict::Kind::Taken:
            failures = 0;
            becomeReady();
            takeIn(answer, sent);
            break;
        case Verdict::Kind::Shed:
            // A server shedding load is alive: readiness stands, and the
            // client waits at least as long as it was asked to.
            if (waitStopping(verdict.wait.count() > 0 ? verdict.wait : kInitialReconnectBackoff)) {
                return;
            }
            break;
        case Verdict::Kind::TooBig:
        case Verdict::Kind::Refused:
        case Verdict::Kind::Transient:
        case Verdict::Kind::Failed:
            if (answer.kind == HttpsExchange::Kind::Response && answer.status == 401) {
                refusedName(answer);
            }
            ++failures;
            becomeNotReady(verdict.error);
            if (waitStopping(reconnectBackoff(failures))) {
                return;
            }
            break;
        }
    }
}

void Client::sendLoop() {
    for (;;) {
        std::vector<Pending> batch;
        {
            std::unique_lock<std::mutex> lock(mutex_);
            wake_.wait(lock, [&] { return stopping_ || !outbox_.empty(); });
            if (stopping_) {
                return;
            }
            const std::size_t count = std::min(batch_limit_, outbox_.size());
            for (std::size_t i = 0; i < count; ++i) {
                batch.push_back(std::move(outbox_.front()));
                outbox_.pop_front();
            }
        }
        std::vector<MeshEnvelope> envelopes;
        envelopes.reserve(batch.size());
        for (const auto &pending : batch) {
            envelopes.push_back(pending.env);
        }
        Mark sent;
        const HttpsExchange answer =
            host_.transport->exchange(buildRequest(encodeBody(envelopes), std::chrono::milliseconds(0), sent));
        if (cancellation_.requested()) {
            return;
        }
        const Verdict verdict = judge(answer);
        switch (verdict.kind) {
        case Verdict::Kind::Taken: {
            becomeReady();
            {
                std::lock_guard<std::mutex> lock(mutex_);
                batch_limit_ = kMaxBatch;
            }
            takeIn(answer, sent);
            break;
        }
        case Verdict::Kind::Shed: {
            {
                std::lock_guard<std::mutex> lock(mutex_);
                outbox_.insert(outbox_.begin(), std::make_move_iterator(batch.begin()),
                               std::make_move_iterator(batch.end()));
            }
            if (waitStopping(verdict.wait.count() > 0 ? verdict.wait : kInitialReconnectBackoff)) {
                return;
            }
            break;
        }
        case Verdict::Kind::TooBig:
            if (batch.size() > 1) {
                // The body was too big for the server, not each envelope:
                // carry them again in smaller sends.
                std::lock_guard<std::mutex> lock(mutex_);
                batch_limit_ = std::max<std::size_t>(1, batch.size() / 2);
                outbox_.insert(outbox_.begin(), std::make_move_iterator(batch.begin()),
                               std::make_move_iterator(batch.end()));
            } else {
                failEnvelopes(std::move(batch), verdict.error, false);
            }
            break;
        case Verdict::Kind::Refused:
            if (answer.status == 401) {
                refusedName(answer);
            }
            failEnvelopes(std::move(batch), verdict.error, false);
            break;
        case Verdict::Kind::Transient:
            failEnvelopes(std::move(batch), verdict.error, true);
            break;
        case Verdict::Kind::Failed:
            becomeNotReady(verdict.error);
            failEnvelopes(std::move(batch), verdict.error, true);
            break;
        }
    }
}

}  // namespace SCE::Mesh::Https
