// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The message signatures of the https binding on the C++ client (docs/adr/0109,
// D9; sce/include/mesh/transports/HttpsSignature.h), on libsodium through the
// host's crypto (host/SodiumCoseCrypto.h):
//   * RFC 9421 Appendix B.2.6, the Ed25519 example, signed byte for byte;
//   * RFC 9530's Content-Digest example;
//   * the request of tests/mesh/https_signature_cross_language.json signed
//     into exactly the fields the Rust server verifies;
//   * a client under `verify:` that takes nothing of a response whose
//     Sce-Mesh-First or body was altered or that was replayed, acknowledges
//     nothing and says so once (SCE_MESH.md section 16.7 row 19), and takes
//     the next response that verifies;
//   * a client of the `key` authenticator that signs every request.

#include "mesh/transports/HttpsSignature.h"
#include "host/SodiumCoseCrypto.h"
#include "mesh/MeshEnvelopeCodec.h"
#include "mesh/transports/HttpsClient.h"

#include <gtest/gtest.h>
#include <nlohmann/json.hpp>

#include <chrono>
#include <condition_variable>
#include <cstdint>
#include <fstream>
#include <functional>
#include <mutex>
#include <optional>
#include <sstream>
#include <string>
#include <thread>
#include <vector>

#ifndef SCE_HTTPS_SIGNATURE_VECTOR_PATH
#error "SCE_HTTPS_SIGNATURE_VECTOR_PATH must be injected by CMake (target_compile_definitions)"
#endif

namespace {

using namespace std::chrono_literals;
using SCE::Mesh::CommunicationError;
using SCE::Mesh::HttpsExchange;
using SCE::Mesh::HttpsHeader;
using SCE::Mesh::HttpsRequest;
using SCE::Mesh::HttpsTransport;
using SCE::Mesh::MeshEnvelope;
using SCE::Mesh::ReasonCode;
namespace Https = SCE::Mesh::Https;
namespace Sig = SCE::Mesh::Https::Signatures;
using Bytes = Sig::Bytes;

Bytes fromHex(const std::string &hex) {
    Bytes out;
    for (std::size_t i = 0; i + 1 < hex.size(); i += 2) {
        out.push_back(static_cast<std::uint8_t>(std::stoul(hex.substr(i, 2), nullptr, 16)));
    }
    return out;
}

Bytes bytesOf(const std::string &text) {
    return Bytes(text.begin(), text.end());
}

/// RFC 9421 Appendix B.2.6: the request of B.2 signed with B.1.4's
/// `test-key-ed25519`. Ed25519 is deterministic, so the base this module
/// writes, signed with the key, is the signature the RFC prints; and the RFC's
/// Signature-Input reads back as the list it writes. Red if a line of the base,
/// an identifier's serialization or the parameters move.
TEST(HttpsSignatureTest, Rfc9421AppendixB26IsSignedByteForByte) {
    SCE::StandardHost::SodiumCoseCrypto crypto;
    ASSERT_TRUE(crypto.ready());
    const Bytes seed = fromHex("9f8362f87a484a954e6e740c5b4c0e84229139a20aa8ab56ff66586f6a7d29c5");
    const auto publicKey = crypto.ed25519PublicKey(seed);
    ASSERT_TRUE(publicKey.has_value());
    ASSERT_EQ(*publicKey, fromHex("26b40b8f93fff3d897112f7ebc582b232dbd72517d082fe83cfb30ddce43d1bb"));

    const std::vector<std::pair<Sig::Component, std::string>> covered = {
        {{"date", false}, "Tue, 20 Apr 2021 02:07:55 GMT"},
        {{"@method", false}, "POST"},
        {{"@path", false}, "/foo"},
        {{"@authority", false}, "example.com"},
        {{"content-type", false}, "application/json"},
        {{"content-length", false}, "18"},
    };
    std::vector<Sig::Item> items;
    for (const auto &entry : covered) {
        items.push_back(Sig::identifier(entry.first));
    }
    const Sig::Parameters parameters = {{"created", Sig::BareItem::ofInteger(1618884473)},
                                        {"keyid", Sig::BareItem::ofString("test-key-ed25519")}};
    const std::string params = Sig::serializeInnerList(items, parameters);
    EXPECT_EQ(params, "(\"date\" \"@method\" \"@path\" \"@authority\" \"content-type\" \"content-length\")"
                      ";created=1618884473;keyid=\"test-key-ed25519\"");
    const std::string base = Sig::signatureBase(covered, params);
    EXPECT_EQ(base, "\"date\": Tue, 20 Apr 2021 02:07:55 GMT\n"
                    "\"@method\": POST\n"
                    "\"@path\": /foo\n"
                    "\"@authority\": example.com\n"
                    "\"content-type\": application/json\n"
                    "\"content-length\": 18\n"
                    "\"@signature-params\": (\"date\" \"@method\" \"@path\" \"@authority\" \"content-type\" "
                    "\"content-length\");created=1618884473;keyid=\"test-key-ed25519\"");
    const auto signature = crypto.ed25519Sign(seed, bytesOf(base));
    ASSERT_TRUE(signature.has_value());
    EXPECT_EQ(Sig::base64(*signature),
              "wqcAqbmYJ2ji2glfAMaRy4gruYYnx2nEFN2HN6jrnDnQCK1u02Gb04v9EDgwUPiu4A0w6vuQv5lIp5WPpBKRCw==");

    const auto read = Sig::parseDictionary("sig-b26=" + params);
    ASSERT_TRUE(read.has_value());
    ASSERT_EQ(read->size(), 1u);
    EXPECT_EQ(read->front().first, "sig-b26");
    ASSERT_TRUE(read->front().second.innerList);
    EXPECT_EQ(Sig::serializeInnerList(read->front().second.items, read->front().second.parameters), params);
    const auto signatures = Sig::parseDictionary("sig-b26=:" + Sig::base64(*signature) + ":");
    ASSERT_TRUE(signatures.has_value());
    EXPECT_EQ(signatures->front().second.item.bare.bytes, *signature);
}

/// RFC 9530 section 2's example: the SHA-256 of `{"hello": "world"}`.
TEST(HttpsSignatureTest, AContentDigestIsTheSha256OfTheBody) {
    SCE::StandardHost::SodiumCoseCrypto crypto;
    const Bytes body = bytesOf("{\"hello\": \"world\"}");
    const auto digest = Sig::contentDigest(crypto, body);
    ASSERT_TRUE(digest.has_value());
    EXPECT_EQ(*digest, "sha-256=:X48E9qOokqqrvdts8nOJRJN3OWDUoyWxBf7kbu9DBPE=:");
    EXPECT_TRUE(Sig::digestMatches(crypto, *digest, body));
    EXPECT_FALSE(Sig::digestMatches(crypto, *digest, bytesOf("{}")));
    EXPECT_FALSE(Sig::digestMatches(crypto, "sha-512=:AAAA:", body));
}

TEST(HttpsSignatureTest, ATargetUriIsReadOneWayAtBothEnds) {
    EXPECT_EQ(Sig::normalizeTargetUri("https://Cal.Example:443/api/sce-mesh/1/phone").value_or(""),
              "https://cal.example/api/sce-mesh/1/phone");
    EXPECT_EQ(Sig::normalizeTargetUri("http://127.0.0.1:80").value_or(""), "http://127.0.0.1/");
    EXPECT_EQ(Sig::normalizeTargetUri("http://[::1]:9443/x?y").value_or(""), "http://[::1]:9443/x?y");
    EXPECT_FALSE(Sig::normalizeTargetUri("wss://cal.example/x").has_value());
    EXPECT_FALSE(Sig::normalizeTargetUri("https://h:x/").has_value());
}

nlohmann::json crossLanguageVector() {
    std::ifstream in(SCE_HTTPS_SIGNATURE_VECTOR_PATH);
    EXPECT_TRUE(in.is_open()) << SCE_HTTPS_SIGNATURE_VECTOR_PATH;
    std::stringstream text;
    text << in.rdbuf();
    return nlohmann::json::parse(text.str());
}

std::vector<HttpsHeader> headersOf(const nlohmann::json &list) {
    std::vector<HttpsHeader> out;
    for (const auto &pair : list) {
        out.emplace_back(pair.at(0).get<std::string>(), pair.at(1).get<std::string>());
    }
    return out;
}

/// tests/mesh/https_signature_cross_language.json: the C++ client's signer,
/// given the vector's seed, target, fields and moment, adds exactly the
/// vector's fields, which the Rust server's verification takes
/// (backends/rust/mesh/tests/https_signature_cross_language.rs). Red if the
/// C++ request signature moves a byte away from the Rust one.
TEST(HttpsSignatureTest, TheCrossLanguageRequestIsSignedByteForByte) {
    SCE::StandardHost::SodiumCoseCrypto crypto;
    const auto v = crossLanguageVector();
    EXPECT_EQ(Sig::normalizeTargetUri(v.at("url").get<std::string>()).value_or(""),
              v.at("target_uri").get<std::string>());
    auto signer = Sig::RequestSigner::make(crypto, fromHex(v.at("seed").get<std::string>()));
    ASSERT_TRUE(signer.has_value());
    EXPECT_EQ(signer->publicKey(), fromHex(v.at("public_key").get<std::string>()));
    const auto headers = headersOf(v.at("headers"));
    EXPECT_TRUE(Sig::digestMatches(crypto, *Sig::fieldValue(headers, Sig::kContentDigestHeader),
                                   fromHex(v.at("body").get<std::string>())));
    const auto signedFields =
        signer->sign(v.at("target_uri").get<std::string>(), headers, v.at("created").get<std::int64_t>());
    ASSERT_TRUE(signedFields.has_value());
    EXPECT_EQ(*signedFields, headersOf(v.at("signed")));
}

// ── The client ──

constexpr const char *kEpoch = "0123456789abcdef0123456789abcdef";
constexpr const char *kServerKeyId = "server-2026";

/// The server's signing seed and the trust set a client verifies it under.
Bytes serverSeed() {
    return Bytes(32, 3);
}

/// A key the client's host generated, a trust set it holds, and no token.
class SigningCredentials : public Https::Credentials {
public:
    SigningCredentials(Bytes seed, std::vector<Sig::TrustedKey> trusted)
        : seed_(std::move(seed)), trusted_(std::move(trusted)) {}

    std::optional<std::string> bearerToken(const std::string &) override {
        return std::nullopt;
    }

    std::optional<Https::InstallSecret> loadInstallSecret(const std::string &) override {
        return std::nullopt;
    }

    bool storeInstallSecret(const std::string &, const Https::InstallSecret &) override {
        return false;
    }

    std::optional<Bytes> requestKeySeed(const std::string &) override {
        return seed_;
    }

    std::optional<std::vector<Sig::TrustedKey>> trustSet(const std::string &name) override {
        if (name != "servers") {
            return std::nullopt;
        }
        return trusted_;
    }

private:
    Bytes seed_;
    std::vector<Sig::TrustedKey> trusted_;
};

/// How the scripted server's answers are altered on the way, as by an
/// intermediary that reads the exchange.
enum class Alteration { None, First, Body, Replay };

/// A server under `sign:` in this process: its first answer is an empty
/// mailbox; from then on it answers every exchange with mailbox number 1 (one
/// `pong`) in kEpoch until a request acknowledges it, then nothing, each
/// response signed for the request it answers. While `altering` holds, a
/// response that carries the envelope is altered as `alteration` says; a
/// replay answers with the first response it sent, which answered another
/// request.
class SigningServer : public HttpsTransport {
public:
    SigningServer(SCE::StandardHost::CoseCrypto &crypto, Alteration alteration)
        : crypto_(crypto), alteration_(alteration) {}

    HttpsExchange exchange(const HttpsRequest &request) override {
        bool firstExchange = false;
        {
            std::lock_guard<std::mutex> lock(mutex_);
            firstExchange = requests_.empty();
            requests_.push_back(request);
        }
        const auto ack = Sig::fieldValue(request.headers, Https::kAckHeader).value_or("0");
        const bool acknowledged = ack != "0";
        if (acknowledged || firstExchange) {
            std::this_thread::sleep_for(20ms);
        }
        MeshEnvelope pong;
        pong.id[0] = 0x01;
        pong.id[15] = 0x7a;
        pong.source = "cloud";
        pong.type = "pong";
        Bytes body = (acknowledged || firstExchange) ? Bytes() : SCE::Mesh::encodeEnvelope(pong);
        std::vector<HttpsHeader> headers = {{Https::kEpochHeader, kEpoch}};
        if (!body.empty()) {
            headers.emplace_back(Https::kFirstHeader, "1");
        }
        sign(request, body, headers);
        HttpsExchange answer = HttpsExchange::response(200, headers, body);

        std::lock_guard<std::mutex> lock(mutex_);
        if (!first_) {
            first_ = answer;
        }
        if (altering_ && !body.empty()) {
            ++altered_;
            switch (alteration_) {
            case Alteration::None:
                break;
            case Alteration::First:
                for (auto &header : answer.headers) {
                    if (header.first == Https::kFirstHeader) {
                        header.second = "2";
                    }
                }
                break;
            case Alteration::Body:
                answer.body.back() ^= 0x01;
                break;
            case Alteration::Replay:
                answer = *first_;
                break;
            }
        }
        return answer;
    }

    void stopAltering() {
        std::lock_guard<std::mutex> lock(mutex_);
        altering_ = false;
    }

    int altered() {
        std::lock_guard<std::mutex> lock(mutex_);
        return altered_;
    }

    std::vector<HttpsRequest> requests() {
        std::lock_guard<std::mutex> lock(mutex_);
        return requests_;
    }

private:
    /// Add the response signature of SCE_MESH.md section 22.9 to `headers`.
    void sign(const HttpsRequest &request, const Bytes &body, std::vector<HttpsHeader> &headers) {
        const std::string digest = Sig::contentDigest(crypto_, body).value_or("");
        headers.emplace_back(Sig::kContentDigestHeader, digest);
        std::vector<std::pair<Sig::Component, std::string>> covered = {
            {{"@status", false}, "200"},
            {{"content-digest", false}, digest},
            {{"sce-mesh-epoch", false}, kEpoch},
        };
        if (!body.empty()) {
            covered.push_back({{"sce-mesh-first", false}, "1"});
        }
        covered.push_back({{"@method", true}, "POST"});
        covered.push_back({{"@target-uri", true}, Sig::normalizeTargetUri(request.url).value_or("")});
        covered.push_back({{"sce-mesh-ack", true}, Sig::fieldValue(request.headers, Https::kAckHeader).value_or("")});
        covered.push_back({{"sce-mesh-nonce", true}, Sig::fieldValue(request.headers, Sig::kNonceHeader).value_or("")});
        std::vector<Sig::Item> items;
        for (const auto &entry : covered) {
            items.push_back(Sig::identifier(entry.first));
        }
        const Sig::Parameters parameters = {{"keyid", Sig::BareItem::ofString(kServerKeyId)},
                                            {"alg", Sig::BareItem::ofString(Sig::kAlgorithm)},
                                            {"tag", Sig::BareItem::ofString(Sig::kTag)}};
        const std::string params = Sig::serializeInnerList(items, parameters);
        const auto signature = crypto_.ed25519Sign(serverSeed(), bytesOf(Sig::signatureBase(covered, params)));
        headers.emplace_back(Sig::kSignatureInputHeader, std::string(Sig::kLabel) + "=" + params);
        headers.emplace_back(Sig::kSignatureHeader,
                             std::string(Sig::kLabel) + "=:" + Sig::base64(signature.value_or(Bytes())) + ":");
    }

    SCE::StandardHost::CoseCrypto &crypto_;
    Alteration alteration_;
    std::mutex mutex_;
    bool altering_ = true;
    int altered_ = 0;
    std::optional<HttpsExchange> first_;
    std::vector<HttpsRequest> requests_;
};

/// What the client told the router.
class Seen {
public:
    Https::ClientEvents events() {
        Https::ClientEvents e;
        e.receive = [this](MeshEnvelope env) {
            std::lock_guard<std::mutex> lock(mutex_);
            received_.push_back(env.type);
            changed_.notify_all();
        };
        e.fault = [this](CommunicationError err) {
            std::lock_guard<std::mutex> lock(mutex_);
            faults_.push_back(err.reason);
            changed_.notify_all();
        };
        return e;
    }

    bool waitFor(const std::function<bool()> &done, std::chrono::milliseconds bound = 10000ms) {
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

    std::size_t faultsOf(ReasonCode reason) {
        std::size_t n = 0;
        for (const auto f : faults_) {
            n += f == reason ? 1 : 0;
        }
        return n;
    }

    std::vector<std::string> received_;
    std::vector<ReasonCode> faults_;

private:
    std::mutex mutex_;
    std::condition_variable changed_;
};

Https::ClientConfig verifyingConfig(Https::Authenticator authenticator) {
    Https::ClientConfig c;
    c.base_url = "https://cloud.example:9443/api";
    c.client = "phone";
    c.peer = "cloud";
    c.target = "#cloud";
    c.authenticator = authenticator;
    c.hold = 200ms;
    c.verify = "servers";
    return c;
}

/// SCE_MESH.md section 22.9: under `verify:` a response an intermediary
/// altered, in its Sce-Mesh-First, in its body, or by replaying an earlier
/// one, is taken nothing of: no envelope reaches the router, nothing is
/// acknowledged (every request meanwhile acknowledges 0), and the router is
/// told once (row 19). Once responses verify again, the envelope arrives once
/// and is acknowledged. Red if the client takes an unverified `200`.
void runAltered(Alteration alteration) {
    SCE::StandardHost::SodiumCoseCrypto crypto;
    ASSERT_TRUE(crypto.ready());
    const auto serverPublic = crypto.ed25519PublicKey(serverSeed());
    ASSERT_TRUE(serverPublic.has_value());
    SigningCredentials credentials(Bytes(32, 9), {{kServerKeyId, *serverPublic}});
    SigningServer server(crypto, alteration);
    Seen seen;
    Https::Client client(verifyingConfig(Https::Authenticator::Bearer),
                         Https::ClientHost{&server, &credentials, &crypto}, seen.events());
    ASSERT_TRUE(client.start());

    ASSERT_TRUE(seen.waitFor([&] { return server.altered() >= 2; })) << "two altered responses";
    EXPECT_TRUE(seen.received_.empty()) << "nothing of an altered response reaches the router";
    EXPECT_EQ(client.acknowledged(), 0u);
    EXPECT_EQ(seen.faultsOf(ReasonCode::ResponseUnauthenticated), 1u) << "told once per run of them";

    server.stopAltering();
    ASSERT_TRUE(seen.waitFor([&] { return !seen.received_.empty(); }));
    ASSERT_TRUE(seen.waitFor([&] { return client.acknowledged() == 1u; }));
    std::this_thread::sleep_for(100ms);
    client.stop();
    EXPECT_EQ(seen.received_, std::vector<std::string>{"pong"}) << "the envelope arrives once";
    EXPECT_EQ(seen.faultsOf(ReasonCode::ResponseUnauthenticated), 1u);

    const auto requests = server.requests();
    std::size_t acknowledging = 0;
    for (const auto &request : requests) {
        EXPECT_TRUE(Sig::fieldValue(request.headers, Sig::kNonceHeader).has_value());
        acknowledging += Sig::fieldValue(request.headers, Https::kAckHeader) == std::optional<std::string>("1");
    }
    EXPECT_GE(acknowledging, 1u) << "the envelope is acknowledged once it verified";
}

TEST(HttpsSignatureTest, AnAlteredFirstNumberIsTakenNothingOf) {
    runAltered(Alteration::First);
}

TEST(HttpsSignatureTest, AnAlteredBodyIsTakenNothingOf) {
    runAltered(Alteration::Body);
}

TEST(HttpsSignatureTest, AReplayedResponseIsTakenNothingOf) {
    runAltered(Alteration::Replay);
}

/// A binding that names `verify:` is never run without its trust set: a host
/// that holds no set of that name, or gave no crypto, is refused at start.
TEST(HttpsSignatureTest, AVerifyingClientWithoutItsTrustSetDoesNotStart) {
    SCE::StandardHost::SodiumCoseCrypto crypto;
    SigningCredentials credentials(Bytes(32, 9), {});
    SigningServer server(crypto, Alteration::None);
    Seen seen;
    auto unknown = verifyingConfig(Https::Authenticator::Bearer);
    unknown.verify = "elsewhere";
    Https::Client noSet(unknown, Https::ClientHost{&server, &credentials, &crypto}, seen.events());
    EXPECT_FALSE(noSet.start());
    Https::Client noCrypto(verifyingConfig(Https::Authenticator::Bearer),
                           Https::ClientHost{&server, &credentials, nullptr}, seen.events());
    EXPECT_FALSE(noCrypto.start());
}

/// SCE_MESH.md section 22.9: a client of the `key` authenticator signs every
/// request over its body's digest with the key its host keeps, offers the
/// key, and sends no Authorization; each signature verifies under that key
/// over the base the request's own fields make. Red if a request leaves
/// unsigned or a covered field leaves the base.
TEST(HttpsSignatureTest, AKeyClientSignsEveryRequest) {
    SCE::StandardHost::SodiumCoseCrypto crypto;
    const Bytes clientSeed(32, 6);
    const auto clientPublic = crypto.ed25519PublicKey(clientSeed);
    const auto serverPublic = crypto.ed25519PublicKey(serverSeed());
    ASSERT_TRUE(clientPublic.has_value() && serverPublic.has_value());
    SigningCredentials credentials(clientSeed, {{kServerKeyId, *serverPublic}});
    SigningServer server(crypto, Alteration::None);
    Seen seen;
    Https::Client client(verifyingConfig(Https::Authenticator::Key), Https::ClientHost{&server, &credentials, &crypto},
                         seen.events());
    ASSERT_TRUE(client.start());
    ASSERT_TRUE(seen.waitFor([&] { return !seen.received_.empty(); }));
    MeshEnvelope ping;
    ping.id[0] = 0x02;
    ping.source = "phone";
    ping.type = "ping";
    ASSERT_TRUE(client.send(ping));
    ASSERT_TRUE(seen.waitFor([&] {
        for (const auto &request : server.requests()) {
            if (!request.body.empty()) {
                return true;
            }
        }
        return false;
    }));
    client.stop();
    EXPECT_EQ(seen.faultsOf(ReasonCode::Unauthorized), 0u);

    for (const auto &request : server.requests()) {
        EXPECT_FALSE(Sig::fieldValue(request.headers, "Authorization").has_value());
        const auto digest = Sig::fieldValue(request.headers, Sig::kContentDigestHeader);
        ASSERT_TRUE(digest.has_value());
        EXPECT_TRUE(Sig::digestMatches(crypto, *digest, request.body));
        const auto input = Sig::parseDictionary(Sig::fieldValue(request.headers, Sig::kSignatureInputHeader).value());
        const auto signature = Sig::parseDictionary(Sig::fieldValue(request.headers, Sig::kSignatureHeader).value());
        ASSERT_TRUE(input.has_value() && signature.has_value());
        const auto &list = input->front().second;
        std::vector<std::pair<Sig::Component, std::string>> covered;
        for (const auto &item : list.items) {
            const std::string name = item.bare.text;
            std::string value;
            if (name == "@method") {
                value = "POST";
            } else if (name == "@target-uri") {
                value = Sig::normalizeTargetUri(request.url).value();
            } else {
                value = Sig::fieldValue(request.headers, name).value();
            }
            covered.push_back({{name, false}, value});
        }
        const std::string base = Sig::signatureBase(covered, Sig::serializeInnerList(list.items, list.parameters));
        EXPECT_TRUE(crypto.ed25519Verify(*clientPublic, bytesOf(base), signature->front().second.item.bare.bytes));
        EXPECT_EQ(list.items.size(), Sig::fieldValue(request.headers, Https::kEpochHeader) ? 7u : 6u);
    }
}

}  // namespace
