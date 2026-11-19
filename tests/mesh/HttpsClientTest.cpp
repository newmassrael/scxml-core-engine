// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The client of the https binding (docs/adr/0107, D2;
// sce/include/mesh/transports/HttpsClient.h), held to the binding's semantics
// with no network: the wire's helpers on their own, and the client against a
// mailbox in this process (https_kit/Mailbox.h) or a transport whose answers a
// test scripts. The adapters are held to the wire over real sockets by the
// conformance kit (https_kit/HttpsKit.h).

#include "https_kit/Mailbox.h"

#include "mesh/MeshEnvelopeCodec.h"
#include "mesh/transports/HttpsClient.h"

#include <gtest/gtest.h>

#include <algorithm>
#include <array>
#include <atomic>
#include <chrono>
#include <condition_variable>
#include <cstdint>
#include <functional>
#include <mutex>
#include <optional>
#include <string>
#include <thread>
#include <vector>

namespace {

using namespace std::chrono_literals;
using SCE::Mesh::CommunicationError;
using SCE::Mesh::HttpsExchange;
using SCE::Mesh::HttpsRequest;
using SCE::Mesh::HttpsTransport;
using SCE::Mesh::MeshEnvelope;
using SCE::Mesh::ReasonCode;
namespace Https = SCE::Mesh::Https;
namespace Kit = SCE::Mesh::HttpsKit;

MeshEnvelope envelope(const std::string &type, std::uint8_t tag) {
    MeshEnvelope env;
    env.id[0] = tag;
    env.id[15] = 0x7a;
    env.source = "cloud";
    env.type = type;
    return env;
}

/// Bearer tokens and an installation secret kept in memory.
class MemoryCredentials : public Https::Credentials {
public:
    explicit MemoryCredentials(std::string token) : token_(std::move(token)) {}

    std::optional<std::string> bearerToken(const std::string &) override {
        return token_;
    }

    std::optional<Https::InstallSecret> loadInstallSecret(const std::string &) override {
        std::lock_guard<std::mutex> lock(mutex_);
        return secret_;
    }

    bool storeInstallSecret(const std::string &, const Https::InstallSecret &secret) override {
        std::lock_guard<std::mutex> lock(mutex_);
        secret_ = secret;
        ++stores_;
        return true;
    }

    int stores() const {
        std::lock_guard<std::mutex> lock(mutex_);
        return stores_;
    }

private:
    std::string token_;
    mutable std::mutex mutex_;
    std::optional<Https::InstallSecret> secret_;
    int stores_ = 0;
};

/// How Observed::order_ names a restart.
constexpr const char *kRestarted = "(restarted)";

/// What the client told the router, observed by a test.
class Observed {
public:
    Https::ClientEvents events() {
        Https::ClientEvents e;
        e.receive = [this](MeshEnvelope env) {
            std::lock_guard<std::mutex> lock(mutex_);
            order_.push_back(env.type);
            received_.push_back(std::move(env));
            changed_.notify_all();
        };
        e.restarted = [this] {
            std::lock_guard<std::mutex> lock(mutex_);
            order_.push_back(kRestarted);
            ++restarted_;
            changed_.notify_all();
        };
        e.ready = [this] {
            std::lock_guard<std::mutex> lock(mutex_);
            ++ready_;
            changed_.notify_all();
        };
        e.notReady = [this](CommunicationError err) {
            std::lock_guard<std::mutex> lock(mutex_);
            not_ready_.push_back(std::move(err));
            changed_.notify_all();
        };
        e.fault = [this](CommunicationError err) {
            std::lock_guard<std::mutex> lock(mutex_);
            faults_.push_back(std::move(err));
            changed_.notify_all();
        };
        return e;
    }

    /// Wait until `done` holds, at most `bound`. Whether it held. Asked again
    /// at every event and every 20 ms, since a predicate may read the mailbox,
    /// which tells this object nothing.
    bool waitFor(const std::function<bool()> &done, std::chrono::milliseconds bound = 5000ms) {
        const auto until = std::chrono::steady_clock::now() + bound;
        std::unique_lock<std::mutex> lock(mutex_);
        while (!done()) {
            if (std::chrono::steady_clock::now() >= until) {
                return false;
            }
            changed_.wait_for(lock, 20ms);
        }
        return true;
    }

    // Read under the lock by the predicates above, and by a test after it
    // stopped the client.
    std::vector<MeshEnvelope> received_;
    /// What reached the router in order: each envelope's type, and
    /// kRestarted for each restart.
    std::vector<std::string> order_;
    int restarted_ = 0;
    int ready_ = 0;
    std::vector<CommunicationError> not_ready_;
    std::vector<CommunicationError> faults_;

    std::size_t faultsOf(ReasonCode reason) const {
        std::size_t n = 0;
        for (const auto &f : faults_) {
            n += f.reason == reason ? 1 : 0;
        }
        return n;
    }

private:
    std::mutex mutex_;
    std::condition_variable changed_;
};

Https::ClientConfig config(Https::Authenticator authenticator = Https::Authenticator::Bearer) {
    Https::ClientConfig c;
    c.base_url = "https://cloud.example/api/";
    c.client = "phone";
    c.peer = "cloud";
    c.target = "#cloud";
    c.authenticator = authenticator;
    c.hold = 200ms;
    return c;
}

/// A transport whose answers a test writes: a poll (an empty body) and a
/// send each have a script. A poll that has no script is answered `200`
/// after a short wait, as a server whose hold ended with nothing to say.
class Scripted : public HttpsTransport {
public:
    std::function<HttpsExchange(const HttpsRequest &)> onPoll;
    std::function<HttpsExchange(const HttpsRequest &, int)> onSend;

    HttpsExchange exchange(const HttpsRequest &request) override {
        if (request.body.empty()) {
            if (onPoll) {
                return onPoll(request);
            }
            for (int i = 0; i < 4; ++i) {
                if (request.cancellation != nullptr && request.cancellation->requested()) {
                    return HttpsExchange::failed(HttpsExchange::Failure::Other, "cancelled");
                }
                std::this_thread::sleep_for(5ms);
            }
            return HttpsExchange::response(200, {}, {});
        }
        int attempt = 0;
        {
            std::lock_guard<std::mutex> lock(mutex_);
            attempt = ++sends_;
            bodies_.push_back(request.body);
        }
        return onSend(request, attempt);
    }

    std::vector<std::vector<std::uint8_t>> bodies() const {
        std::lock_guard<std::mutex> lock(mutex_);
        return bodies_;
    }

private:
    mutable std::mutex mutex_;
    int sends_ = 0;
    std::vector<std::vector<std::uint8_t>> bodies_;
};

std::size_t envelopesIn(const std::vector<std::uint8_t> &body) {
    std::vector<std::pair<std::size_t, std::size_t>> items;
    std::size_t at = 0;
    EXPECT_TRUE(Https::splitCborSequence(body.data(), body.size(), items, at));
    return items.size();
}

// ── The wire ──

TEST(HttpsWire, ExchangePathIsTheBaseThenVersionThenClient) {
    EXPECT_EQ(Https::exchangeUrl("https://h.example/api/", "phone"), "https://h.example/api/sce-mesh/1/phone");
    EXPECT_EQ(Https::exchangeUrl("https://h.example", "phone"), "https://h.example/sce-mesh/1/phone");
}

TEST(HttpsWire, Base64UrlHasNoPaddingAndTheUrlAlphabet) {
    const std::uint8_t f[] = {'f', 'o', 'o', 'b'};
    EXPECT_EQ(Https::base64url(f, 1), "Zg");
    EXPECT_EQ(Https::base64url(f, 2), "Zm8");
    EXPECT_EQ(Https::base64url(f, 3), "Zm9v");
    EXPECT_EQ(Https::base64url(f, 4), "Zm9vYg");
    const std::uint8_t high[] = {0xfb, 0xff};
    EXPECT_EQ(Https::base64url(high, 2), "-_8");
    const Https::InstallSecret secret{};
    EXPECT_EQ(Https::base64url(secret.data(), secret.size()).size(), 43u);
}

TEST(HttpsWire, ACborSequenceSplitsIntoItsEnvelopes) {
    const auto body = Https::encodeBody({envelope("a", 1), envelope("b", 2), envelope("c", 3)});
    std::vector<std::pair<std::size_t, std::size_t>> items;
    std::size_t at = 0;
    ASSERT_TRUE(Https::splitCborSequence(body.data(), body.size(), items, at));
    ASSERT_EQ(items.size(), 3u);
    MeshEnvelope second;
    ASSERT_TRUE(SCE::Mesh::decodeEnvelope(body.data() + items[1].first, items[1].second, second));
    EXPECT_EQ(second.type, "b");
    EXPECT_EQ(items[2].first + items[2].second, body.size());
}

TEST(HttpsWire, ASequenceThatBreaksNamesWhereAndKeepsWhatCameBefore) {
    auto body = Https::encodeBody({envelope("a", 1)});
    const std::size_t whole = body.size();
    body.push_back(0xa2);  // a map of two pairs, with nothing after its head
    std::vector<std::pair<std::size_t, std::size_t>> items;
    std::size_t at = 0;
    EXPECT_FALSE(Https::splitCborSequence(body.data(), body.size(), items, at));
    EXPECT_EQ(items.size(), 1u);
    EXPECT_EQ(at, whole);
    // A break outside an indefinite item, and a reserved head.
    const std::uint8_t stray[] = {0xff};
    items.clear();
    EXPECT_FALSE(Https::splitCborSequence(stray, 1, items, at));
    const std::uint8_t reserved[] = {0x1c};
    EXPECT_FALSE(Https::splitCborSequence(reserved, 1, items, at));
    // Indefinite text and a nested array are items.
    const std::uint8_t indefinite[] = {0x7f, 0x61, 'a', 0x61, 'b', 0xff, 0x82, 0x01, 0x81, 0x02};
    items.clear();
    ASSERT_TRUE(Https::splitCborSequence(indefinite, sizeof(indefinite), items, at));
    EXPECT_EQ(items.size(), 2u);
}

TEST(HttpsWire, RetryAfterReadsSecondsAndAnHttpDate) {
    const auto now = std::chrono::system_clock::time_point(std::chrono::seconds(784111777));  // 1994-11-06 08:49:37
    EXPECT_EQ(Https::retryAfter("120", now), std::chrono::milliseconds(120000));
    EXPECT_EQ(Https::retryAfter(" 3 ", now), std::chrono::milliseconds(3000));
    EXPECT_EQ(Https::retryAfter("Sun, 06 Nov 1994 08:50:37 GMT", now), std::chrono::milliseconds(60000));
    EXPECT_EQ(Https::retryAfter("Sun, 06 Nov 1994 08:40:00 GMT", now), std::chrono::milliseconds(0));
    EXPECT_FALSE(Https::retryAfter("soon", now).has_value());
    EXPECT_FALSE(Https::retryAfter("-1", now).has_value());
}

TEST(HttpsWire, NumbersAndHeaderNames) {
    EXPECT_EQ(Https::parseUnsigned("18446744073709551615"), std::optional<std::uint64_t>(UINT64_MAX));
    EXPECT_FALSE(Https::parseUnsigned("18446744073709551616").has_value());
    EXPECT_FALSE(Https::parseUnsigned("").has_value());
    EXPECT_FALSE(Https::parseUnsigned("1a").has_value());
    const std::vector<SCE::Mesh::HttpsHeader> headers = {{"sce-mesh-first", "7"}};
    EXPECT_EQ(Https::headerValue(headers, "Sce-Mesh-First"), std::optional<std::string>("7"));
}

TEST(HttpsWire, TheOperatingSystemDrawsDifferentSecrets) {
    Https::InstallSecret a{};
    Https::InstallSecret b{};
    ASSERT_TRUE(Https::drawInstallSecret(a));
    ASSERT_TRUE(Https::drawInstallSecret(b));
    EXPECT_NE(a, b);
}

TEST(HttpsWire, AnIncarnationIsThirtyTwoLowercaseHexDigitsDrawnAfresh) {
    const auto a = Https::drawEpoch();
    const auto b = Https::drawEpoch();
    ASSERT_TRUE(a.has_value());
    ASSERT_TRUE(b.has_value());
    EXPECT_NE(*a, *b);
    EXPECT_TRUE(Https::isEpoch(*a)) << *a;
    EXPECT_TRUE(Https::isEpoch("0123456789abcdef0123456789abcdef"));
    EXPECT_FALSE(Https::isEpoch("0123456789ABCDEF0123456789abcdef"));
    EXPECT_FALSE(Https::isEpoch("0123456789abcdef0123456789abcde"));
    EXPECT_FALSE(Https::isEpoch("0123456789abcdef0123456789abcdef0"));
    EXPECT_FALSE(Https::isEpoch("0123456789abcdef0123456789abcdeg"));
}

// ── The client against a mailbox ──

TEST(HttpsClient, TheFirstAnsweredExchangeIsReadyAndTheMailboxArrivesOnceInOrder) {
    Kit::Mailbox mailbox(Kit::MailboxOptions{"phone", "token", 2000ms, 1u << 20});
    Kit::LoopbackTransport transport(mailbox);
    MemoryCredentials credentials("token");
    Observed seen;
    Https::Client client(config(), Https::ClientHost{&transport, &credentials}, seen.events());
    ASSERT_TRUE(client.start());
    ASSERT_TRUE(seen.waitFor([&] { return seen.ready_ == 1; }));
    EXPECT_TRUE(client.ready());

    mailbox.post(envelope("service.response.a", 1));
    mailbox.post(envelope("service.response.b", 2));
    ASSERT_TRUE(seen.waitFor([&] { return seen.received_.size() == 2; }));
    EXPECT_EQ(seen.received_[0].type, "service.response.a");
    EXPECT_EQ(seen.received_[1].type, "service.response.b");
    // The next exchange acknowledges both, and the server drops them.
    ASSERT_TRUE(seen.waitFor([&] {
        const auto acks = mailbox.acks();
        return !acks.empty() && acks.back() == 2;
    }));
    client.stop();
    EXPECT_EQ(client.acknowledged(), 2u);
    EXPECT_EQ(seen.received_.size(), 2u);
    EXPECT_EQ(seen.ready_, 1);
    EXPECT_TRUE(seen.not_ready_.empty());
    EXPECT_TRUE(seen.faults_.empty());
}

TEST(HttpsClient, AResponseLostAfterTheServerActedIsCarriedAgainAndDeliveredOnce) {
    Kit::Mailbox mailbox(Kit::MailboxOptions{"phone", "token", 2000ms, 1u << 20});
    Kit::LoopbackTransport transport(mailbox);
    MemoryCredentials credentials("token");
    Observed seen;
    Https::Client client(config(), Https::ClientHost{&transport, &credentials}, seen.events());
    ASSERT_TRUE(client.start());
    ASSERT_TRUE(seen.waitFor([&] { return seen.ready_ == 1; }));

    mailbox.dropAfterActing(1);
    mailbox.post(envelope("service.response.once", 1));
    ASSERT_TRUE(seen.waitFor([&] { return seen.received_.size() == 1 && seen.ready_ == 2; }));
    // A later exchange carries nothing more.
    mailbox.post(envelope("service.response.next", 2));
    ASSERT_TRUE(seen.waitFor([&] { return seen.received_.size() == 2; }));
    client.stop();
    ASSERT_EQ(seen.received_.size(), 2u);
    EXPECT_EQ(seen.received_[0].type, "service.response.once");
    EXPECT_EQ(seen.received_[1].type, "service.response.next");
    // The lost response is section 16.7 row 1, once, for its one edge, and
    // the same mailbox carried what it held: no restart.
    ASSERT_EQ(seen.not_ready_.size(), 1u);
    EXPECT_EQ(seen.not_ready_[0].reason, ReasonCode::TransportUnavailable);
    EXPECT_EQ(seen.not_ready_[0].target, std::optional<std::string>("#cloud"));
    EXPECT_EQ(seen.restarted_, 0);
}

TEST(HttpsClient, ANewIncarnationStartsTheAcknowledgementOverAndDeliversOnce) {
    Kit::Mailbox mailbox(Kit::MailboxOptions{"phone", "token", 2000ms, 1u << 20});
    Kit::LoopbackTransport transport(mailbox);
    MemoryCredentials credentials("token");
    Observed seen;
    Https::Client client(config(), Https::ClientHost{&transport, &credentials}, seen.events());
    ASSERT_TRUE(client.start());
    mailbox.post(envelope("service.response.before", 1));
    ASSERT_TRUE(seen.waitFor([&] {
        const auto acks = mailbox.acks();
        return seen.received_.size() == 1 && !acks.empty() && acks.back() == 1;
    }));
    const std::string before = mailbox.epoch();
    EXPECT_EQ(client.epoch(), before);

    // The server starts again with its mailbox empty: its next envelope is
    // number 1 again, a number the client acknowledged in the earlier
    // incarnation, and still reaches the router once.
    mailbox.restart();
    ASSERT_NE(mailbox.epoch(), before);
    mailbox.post(envelope("service.response.after", 2));
    ASSERT_TRUE(seen.waitFor([&] { return seen.received_.size() == 2; }));
    mailbox.post(envelope("service.response.next", 3));
    ASSERT_TRUE(seen.waitFor([&] {
        const auto acks = mailbox.acks();
        return seen.received_.size() == 3 && !acks.empty() && acks.back() == 2;
    }));
    client.stop();
    ASSERT_EQ(seen.received_.size(), 3u);
    EXPECT_EQ(seen.received_[0].type, "service.response.before");
    EXPECT_EQ(seen.received_[1].type, "service.response.after");
    EXPECT_EQ(seen.received_[2].type, "service.response.next");
    EXPECT_EQ(client.epoch(), mailbox.epoch());
    EXPECT_EQ(client.acknowledged(), 2u);
    EXPECT_TRUE(seen.faults_.empty());
}

// SCE_MESH.md section 22.4: the router is told the server restarted once,
// when the new incarnation replaces the one the client knew, and before what
// the new mailbox sends, so it asks again before a reply could find nothing
// waiting. The first incarnation the client learns replaces none. Red if
// takeIn tells no restart, or tells it for the first incarnation.
TEST(HttpsClient, ANewIncarnationThatReplacesAKnownOneIsToldOnceBeforeItsEnvelopes) {
    Kit::Mailbox mailbox(Kit::MailboxOptions{"phone", "token", 2000ms, 1u << 20});
    Kit::LoopbackTransport transport(mailbox);
    MemoryCredentials credentials("token");
    Observed seen;
    Https::Client client(config(), Https::ClientHost{&transport, &credentials}, seen.events());
    ASSERT_TRUE(client.start());
    mailbox.post(envelope("service.response.before", 1));
    ASSERT_TRUE(seen.waitFor([&] { return seen.received_.size() == 1; }));

    mailbox.restart();
    mailbox.post(envelope("service.response.after", 2));
    ASSERT_TRUE(seen.waitFor([&] { return seen.received_.size() == 2; }));
    mailbox.post(envelope("service.response.next", 3));
    ASSERT_TRUE(seen.waitFor([&] { return seen.received_.size() == 3; }));
    client.stop();
    EXPECT_EQ(seen.restarted_, 1);
    const std::vector<std::string> order = {"service.response.before", kRestarted, "service.response.after",
                                            "service.response.next"};
    EXPECT_EQ(seen.order_, order);
}

// A response of the incarnation the request was sent in, taken in after the
// other exchange's response named a newer one, is a late answer and not a
// second restart. Red if every incarnation other than the client's is told
// as a restart.
TEST(HttpsClient, ALateResponseInTheRequestsOwnIncarnationIsNoRestart) {
    const std::string earlier = "0123456789abcdef0123456789abcdef";
    const std::string later = "fedcba9876543210fedcba9876543210";
    Scripted transport;
    Https::Client *running = nullptr;
    std::atomic<int> polls{0};
    // The first poll learns `earlier`; the second names `later`, once the
    // send below went out acknowledging in `earlier`.
    transport.onPoll = [&](const HttpsRequest &) {
        const int n = polls.fetch_add(1);
        if (n == 0) {
            return HttpsExchange::response(200, {{"Sce-Mesh-Epoch", earlier}}, {});
        }
        if (n == 1) {
            for (int i = 0; i < 400 && transport.bodies().empty(); ++i) {
                std::this_thread::sleep_for(5ms);
            }
        } else {
            std::this_thread::sleep_for(10ms);
        }
        return HttpsExchange::response(200, {{"Sce-Mesh-Epoch", later}}, {});
    };
    // The send is answered in `earlier`, after the client took `later` in.
    transport.onSend = [&](const HttpsRequest &, int) {
        for (int i = 0; i < 400 && (running == nullptr || running->epoch() != later); ++i) {
            std::this_thread::sleep_for(5ms);
        }
        return HttpsExchange::response(200, {{"Sce-Mesh-Epoch", earlier}}, {});
    };
    MemoryCredentials credentials("token");
    Observed seen;
    Https::Client client(config(), Https::ClientHost{&transport, &credentials}, seen.events());
    running = &client;
    ASSERT_TRUE(client.start());
    // Read without the observer's lock: the client holds its own while it
    // tells the observer what it took in.
    for (int i = 0; i < 400 && client.epoch() != earlier; ++i) {
        std::this_thread::sleep_for(5ms);
    }
    ASSERT_EQ(client.epoch(), earlier);
    ASSERT_TRUE(client.send(envelope("service.request.a", 8)));
    ASSERT_TRUE(seen.waitFor([&] { return seen.restarted_ == 1; }));
    // Long enough for the send's late answer to be taken in.
    std::this_thread::sleep_for(300ms);
    client.stop();
    EXPECT_EQ(seen.restarted_, 1);
    EXPECT_EQ(client.epoch(), later);
}

// ── What a router keeps to ask again ──

MeshEnvelope requestTagged(std::uint8_t tag) {
    return envelope("service.request.a", tag);
}

// The requests a router keeps are those that still wait, in the order they
// left, each once; one that no longer waits is let go when another is kept
// or the waiting ones are read. Red if a request is kept twice when it is
// sent again, or if one that was answered is sent again.
TEST(HttpsWaitingRequests, KeepsWhatStillWaitsInOrderAndOnce) {
    Https::WaitingRequests kept;
    std::vector<std::uint8_t> answered;
    const Https::WaitingRequests::Waits waits = [&answered](const MeshEnvelope &env) {
        return std::find(answered.begin(), answered.end(), env.id[0]) == answered.end();
    };
    kept.record(requestTagged(1), waits);
    kept.record(requestTagged(2), waits);
    kept.record(requestTagged(1), waits);  // the first, sent again
    kept.record(requestTagged(3), waits);
    EXPECT_EQ(kept.size(), 3u);

    answered.push_back(2);
    const auto waiting = kept.stillWaiting(waits);
    ASSERT_EQ(waiting.size(), 2u);
    EXPECT_EQ(waiting[0].id[0], 1);
    EXPECT_EQ(waiting[1].id[0], 3);
    EXPECT_EQ(kept.size(), 2u);

    answered.push_back(1);
    kept.record(requestTagged(4), waits);
    EXPECT_EQ(kept.size(), 2u);
    const auto left = kept.stillWaiting(waits);
    ASSERT_EQ(left.size(), 2u);
    EXPECT_EQ(left[0].id[0], 3);
    EXPECT_EQ(left[1].id[0], 4);
}

TEST(HttpsClient, ASendIsCarriedInARequestBodyAndTakenIn) {
    Kit::Mailbox mailbox(Kit::MailboxOptions{"phone", "token", 2000ms, 1u << 20});
    Kit::LoopbackTransport transport(mailbox);
    MemoryCredentials credentials("token");
    Observed seen;
    Https::Client client(config(), Https::ClientHost{&transport, &credentials}, seen.events());
    ASSERT_TRUE(client.start());
    EXPECT_TRUE(client.send(envelope("service.request.a", 9)));
    ASSERT_TRUE(seen.waitFor(
        [&] {
            const auto in = mailbox.received();
            return in.size() == 1 && in[0].type == "service.request.a";
        },
        5000ms));
    client.stop();
    EXPECT_TRUE(seen.faults_.empty());
    EXPECT_FALSE(client.send(envelope("after.stop", 10)));
}

TEST(HttpsClient, ARefusedNameIsUnauthorizedOncePerRunOfRefusals) {
    Kit::Mailbox mailbox(Kit::MailboxOptions{"phone", "right", 2000ms, 1u << 20});
    Kit::LoopbackTransport transport(mailbox);
    MemoryCredentials credentials("wrong");
    Observed seen;
    Https::Client client(config(), Https::ClientHost{&transport, &credentials}, seen.events());
    ASSERT_TRUE(client.start());
    // Two refused polls: the first and the one after a 500 ms backoff.
    ASSERT_TRUE(seen.waitFor([&] { return mailbox.exchanges() >= 2; }, 3000ms));
    client.stop();
    EXPECT_EQ(seen.faultsOf(ReasonCode::Unauthorized), 1u);
    EXPECT_EQ(seen.not_ready_.size(), 1u);
    EXPECT_EQ(seen.ready_, 0);
    ASSERT_FALSE(seen.faults_.empty());
    EXPECT_EQ(seen.faults_[0].transport_status, std::optional<std::string>("401"));
}

TEST(HttpsClient, AnInstallationSecretIsDrawnOnceKeptAndPinned) {
    Kit::Mailbox mailbox(Kit::MailboxOptions{"phone", "", 2000ms, 1u << 20});
    Kit::LoopbackTransport transport(mailbox);
    MemoryCredentials credentials("");
    Observed seen;
    {
        Https::Client client(config(Https::Authenticator::Install), Https::ClientHost{&transport, &credentials},
                             seen.events());
        ASSERT_TRUE(client.start());
        ASSERT_TRUE(seen.waitFor([&] { return seen.ready_ == 1; }));
        client.stop();
    }
    EXPECT_EQ(credentials.stores(), 1);
    // A client started again uses the kept secret, which the server pinned.
    Observed again;
    Https::Client second(config(Https::Authenticator::Install), Https::ClientHost{&transport, &credentials},
                         again.events());
    ASSERT_TRUE(second.start());
    ASSERT_TRUE(again.waitFor([&] { return again.ready_ == 1; }));
    second.stop();
    EXPECT_EQ(credentials.stores(), 1);
    EXPECT_EQ(again.faultsOf(ReasonCode::Unauthorized), 0u);
}

TEST(HttpsClient, ShedLoadIsWaitedOutWithoutAFault) {
    Kit::Mailbox mailbox(Kit::MailboxOptions{"phone", "token", 2000ms, 1u << 20});
    Kit::LoopbackTransport transport(mailbox);
    MemoryCredentials credentials("token");
    mailbox.shed(1, 503, std::chrono::seconds(1));
    Observed seen;
    Https::Client client(config(), Https::ClientHost{&transport, &credentials}, seen.events());
    const auto started = std::chrono::steady_clock::now();
    ASSERT_TRUE(client.start());
    ASSERT_TRUE(seen.waitFor([&] { return seen.ready_ == 1; }));
    const auto waited = std::chrono::steady_clock::now() - started;
    client.stop();
    EXPECT_GE(waited, 1000ms);
    EXPECT_TRUE(seen.faults_.empty());
    EXPECT_TRUE(seen.not_ready_.empty());
}

// ── The client against scripted answers ──

TEST(HttpsClient, ASendNoAnswerConfirmedIsSendFailedWithoutAPolicy) {
    Scripted transport;
    transport.onSend = [](const HttpsRequest &, int) {
        return HttpsExchange::failed(HttpsExchange::Failure::Connect, "refused");
    };
    MemoryCredentials credentials("token");
    Observed seen;
    Https::Client client(config(), Https::ClientHost{&transport, &credentials}, seen.events());
    ASSERT_TRUE(client.start());
    const auto env = envelope("service.request.a", 4);
    ASSERT_TRUE(client.send(env));
    ASSERT_TRUE(seen.waitFor([&] { return seen.faultsOf(ReasonCode::SendFailed) == 1; }));
    client.stop();
    EXPECT_EQ(seen.faultsOf(ReasonCode::SendFailed), 1u);
    for (const auto &f : seen.faults_) {
        if (f.reason == ReasonCode::SendFailed) {
            EXPECT_EQ(f.envelope_id, (std::optional<std::array<std::uint8_t, 16>>(env.id)));
            EXPECT_EQ(f.transport, std::optional<std::string>("https"));
            ASSERT_TRUE(f.transport_error.has_value());
            EXPECT_NE(f.transport_error->find("connect"), std::string::npos);
        }
    }
}

TEST(HttpsClient, ARetryPolicyCarriesASendAgainAndThenGivesUpAsDeliveryExhausted) {
    // Two failures, then an answer: three retries are enough.
    {
        Scripted transport;
        transport.onSend = [](const HttpsRequest &, int attempt) {
            return attempt <= 2 ? HttpsExchange::failed(HttpsExchange::Failure::Timeout, "slow")
                                : HttpsExchange::response(200, {}, {});
        };
        MemoryCredentials credentials("token");
        Observed seen;
        auto c = config();
        c.retry = Https::RetryPolicy{3, 10ms, 2.0, 100ms, 0};
        Https::Client client(c, Https::ClientHost{&transport, &credentials}, seen.events());
        ASSERT_TRUE(client.start());
        ASSERT_TRUE(client.send(envelope("service.request.a", 5)));
        ASSERT_TRUE(seen.waitFor([&] { return transport.bodies().size() == 3; }));
        client.stop();
        EXPECT_EQ(seen.faultsOf(ReasonCode::SendFailed), 0u);
        EXPECT_EQ(seen.faultsOf(ReasonCode::DeliveryExhausted), 0u);
    }
    // Always failing, one retry: two attempts, then row 3.
    {
        Scripted transport;
        transport.onSend = [](const HttpsRequest &, int) {
            return HttpsExchange::failed(HttpsExchange::Failure::Connect, "refused");
        };
        MemoryCredentials credentials("token");
        Observed seen;
        auto c = config();
        c.retry = Https::RetryPolicy{1, 10ms, 2.0, 100ms, 0};
        Https::Client client(c, Https::ClientHost{&transport, &credentials}, seen.events());
        ASSERT_TRUE(client.start());
        ASSERT_TRUE(client.send(envelope("service.request.a", 6)));
        ASSERT_TRUE(seen.waitFor([&] { return seen.faultsOf(ReasonCode::DeliveryExhausted) == 1; }));
        client.stop();
        EXPECT_EQ(transport.bodies().size(), 2u);
        for (const auto &f : seen.faults_) {
            if (f.reason == ReasonCode::DeliveryExhausted) {
                EXPECT_EQ(f.attempts, std::optional<std::int64_t>(2));
            }
        }
    }
    // A refusal is not worth a retry: one attempt, then row 3.
    {
        Scripted transport;
        transport.onSend = [](const HttpsRequest &, int) { return HttpsExchange::response(404, {}, {}); };
        MemoryCredentials credentials("token");
        Observed seen;
        auto c = config();
        c.retry = Https::RetryPolicy{5, 10ms, 2.0, 100ms, 0};
        Https::Client client(c, Https::ClientHost{&transport, &credentials}, seen.events());
        ASSERT_TRUE(client.start());
        ASSERT_TRUE(client.send(envelope("service.request.a", 7)));
        ASSERT_TRUE(seen.waitFor([&] { return seen.faultsOf(ReasonCode::DeliveryExhausted) == 1; }));
        client.stop();
        EXPECT_EQ(transport.bodies().size(), 1u);
    }
}

TEST(HttpsClient, ABodyTooBigIsCarriedAgainInSmallerSends) {
    Scripted transport;
    transport.onSend = [](const HttpsRequest &request, int) {
        return envelopesIn(request.body) > 1 ? HttpsExchange::response(413, {}, {})
                                             : HttpsExchange::response(200, {}, {});
    };
    MemoryCredentials credentials("token");
    Observed seen;
    Https::Client client(config(), Https::ClientHost{&transport, &credentials}, seen.events());
    ASSERT_TRUE(client.start());
    for (std::uint8_t i = 0; i < 4; ++i) {
        ASSERT_TRUE(client.send(envelope("service.request.a", i)));
    }
    ASSERT_TRUE(seen.waitFor([&] {
        std::size_t taken = 0;
        for (const auto &body : transport.bodies()) {
            taken += envelopesIn(body) == 1 ? 1 : 0;
        }
        return taken == 4;
    }));
    client.stop();
    EXPECT_TRUE(seen.faults_.empty());
}

TEST(HttpsClient, AMailboxEnvelopeThatDoesNotDecodeIsEnvelopeCorruptAndAcknowledged) {
    Scripted transport;
    std::atomic<int> polls{0};
    transport.onPoll = [&polls](const HttpsRequest &) {
        if (polls.fetch_add(1) == 0) {
            // A well-formed CBOR item that is no envelope.
            return HttpsExchange::response(
                200, {{"Sce-Mesh-Epoch", "0123456789abcdef0123456789abcdef"}, {"Sce-Mesh-First", "1"}}, {0x01});
        }
        std::this_thread::sleep_for(10ms);
        return HttpsExchange::response(200, {}, {});
    };
    MemoryCredentials credentials("token");
    Observed seen;
    Https::Client client(config(), Https::ClientHost{&transport, &credentials}, seen.events());
    ASSERT_TRUE(client.start());
    ASSERT_TRUE(seen.waitFor([&] { return seen.faultsOf(ReasonCode::EnvelopeCorrupt) == 1; }));
    client.stop();
    EXPECT_EQ(client.acknowledged(), 1u);
    EXPECT_TRUE(seen.received_.empty());
    for (const auto &f : seen.faults_) {
        if (f.reason == ReasonCode::EnvelopeCorrupt) {
            EXPECT_EQ(f.codec, std::optional<std::string>("cbor"));
            EXPECT_EQ(f.position, std::optional<std::int64_t>(0));
        }
    }
}

TEST(HttpsClient, StopEndsAHeldExchangeAtOnce) {
    Kit::Mailbox mailbox(Kit::MailboxOptions{"phone", "token", 30000ms, 1u << 20});
    Kit::LoopbackTransport transport(mailbox);
    MemoryCredentials credentials("token");
    Observed seen;
    auto c = config();
    c.hold = 30000ms;
    Https::Client client(c, Https::ClientHost{&transport, &credentials}, seen.events());
    ASSERT_TRUE(client.start());
    // The first poll is held: the mailbox is empty and the hold is 30 s.
    ASSERT_TRUE(seen.waitFor([&] { return mailbox.exchanges() >= 1; }));
    std::this_thread::sleep_for(100ms);
    const auto started = std::chrono::steady_clock::now();
    client.stop();
    EXPECT_LT(std::chrono::steady_clock::now() - started, 2000ms);
}

TEST(HttpsClient, NoAdapterIsNoStart) {
    Observed seen;
    Https::Client client(config(), Https::ClientHost{}, seen.events());
    EXPECT_FALSE(client.start());
    EXPECT_FALSE(client.send(envelope("a", 1)));
}

}  // namespace
