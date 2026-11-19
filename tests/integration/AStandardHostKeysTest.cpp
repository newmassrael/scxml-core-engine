// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Random bytes, keyed digests and keys of the reference host (sce/include/host/CoseHost.h;
// docs/SCE_HOST_FEATURES.md, sections 11 to 13; docs/adr/0109, D5 to D7) on libsodium: each operation's
// answers, failures and refusals, a key's one use, a generation answered once under its identity, a retired
// key that makes nothing new, and the vectors of tests/host_features/keys_cross_language.json, which the Rust
// host is held to by backends/rust/host-features/tests/keys_cross_language.rs.
//
// Compiled into standard_host_scenario_test only in a build that found libsodium, whose target
// `sce_host_cose` it links (cmake/SCEHostCose.cmake).

#include "common/Latin1Bytes.h"
#include "host/Cose.h"
#include "host/CoseHost.h"
#include "host/SodiumCoseCrypto.h"
#include "host/StandardHost.h"
#include "integration/StandardHostScenarioReplay.h"

#include <cstddef>
#include <cstdint>
#include <filesystem>
#include <fstream>
#include <gtest/gtest.h>
#include <map>
#include <nlohmann/json.hpp>
#include <optional>
#include <string>
#include <vector>

namespace SCE::Tests {

namespace {

using json = nlohmann::json;
namespace host = SCE::StandardHost;
namespace cose = SCE::StandardHost::Cose;
using cose::Bytes;

Bytes hex(const std::string &text) {
    const auto bytes = HostScenario::bytesOfHex(text);
    EXPECT_TRUE(bytes.has_value()) << text << " is not hex";
    return bytes.value_or(Bytes{});
}

/// The text a byte string crosses a request or a result as.
std::string latin1(const Bytes &bytes) {
    return Latin1Bytes::textOf(bytes);
}

Bytes ascii(const std::string &text) {
    return Bytes(text.begin(), text.end());
}

/// RFC 8032, section 7.1, TEST 1: a seed, and the COSE_Key of its public key under the key id `11`.
const char *const kSeed11 = "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60";
const char *const kCoseKey11 =
    "a40101024231312006215820d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";

/// The keys of the RFC 4231 test case 2: the key `Jefe`, its message and its tag.
const char *const kJefeMessage = "what do ya want for nothing?";
const char *const kJefeTag = "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843";

/// A random source that gives nothing.
class DryCrypto : public host::SodiumCoseCrypto {
public:
    std::optional<Bytes> random(size_t) override {
        return std::nullopt;
    }
};

/// A reference host with the feature over the primitives it is given: the signing key `11`, the sealing key
/// `box` and the MAC key `jefe`; its random source gives what a test queues first.
struct KeysRig {
    explicit KeysRig(host::CoseCrypto &crypto) : feature(crypto) {
        EXPECT_TRUE(reference.addFeature(feature));
        EXPECT_TRUE(feature.setSigningKeys({{"11", hex(kSeed11)}}));
        EXPECT_TRUE(feature.setSealingKeys({{"box", Bytes(32, uint8_t{0x40})}}));
        EXPECT_TRUE(feature.setMacKeys({{"jefe", ascii("Jefe")}}));
    }

    host::Outcome start(const std::string &operation, const host::Params &params, uint64_t startNumber = 1) {
        const auto outcome = reference.start(operation, host::Invocation{"session", "op", startNumber}, params);
        EXPECT_TRUE(outcome.has_value()) << operation << " is not served";
        return outcome.value_or(host::Outcome{});
    }

    /// The result record of a completion of `operation`.
    json done(const std::string &operation, const host::Params &params, uint64_t startNumber = 1) {
        const host::Outcome outcome = start(operation, params, startNumber);
        EXPECT_EQ(outcome.kind, host::Outcome::Kind::Done) << operation << ": " << outcome.data;
        return outcome.kind == host::Outcome::Kind::Done ? json::parse(outcome.data) : json::object();
    }

    /// The id a generation of `use` answers as the `startNumber`-th start.
    std::string generate(const std::string &use, uint64_t startNumber) {
        const json fields = done(host::KEY_GENERATE, {{"key_use", use}, {"label", "a label"}}, startNumber);
        EXPECT_EQ(fields.at("error"), "") << fields.dump();
        return fields.value("key_id", std::string());
    }

    host::ScriptedClock clock;
    HostScenario::ScriptTransport transport;
    host::ReferenceHost reference{clock, transport};
    host::CoseFeature feature;
};

json vectors() {
    std::ifstream in(std::filesystem::path(SCE_PROJECT_ROOT) / "tests" / "host_features" / "keys_cross_language.json");
    EXPECT_TRUE(in.good()) << "the vector file is not readable";
    return json::parse(in);
}

}  // namespace

/// The one comparison of a secret tells equal bytes from bytes that differ anywhere, and strings of two
/// lengths apart (docs/adr/0109, D3 and D6).
TEST(AStandardHostKeys, TheComparisonOfASecretTellsEveryDifference) {
    host::SodiumCoseCrypto crypto;
    const Bytes tag = hex(kJefeTag);
    EXPECT_TRUE(crypto.sameSecret(tag, Bytes(tag)));
    for (const size_t at : {size_t{0}, size_t{15}, size_t{31}}) {
        Bytes other = tag;
        other[at] ^= 0x01;
        EXPECT_FALSE(crypto.sameSecret(tag, other)) << "a difference at byte " << at;
    }
    EXPECT_FALSE(crypto.sameSecret(tag, Bytes(tag.begin(), tag.begin() + 31))) << "a shorter string";
    EXPECT_TRUE(crypto.sameSecret(Bytes{}, Bytes{}));
    Bytes key = tag;
    crypto.wipe(key);
    EXPECT_EQ(key, Bytes(32, uint8_t{0})) << "a wiped key is zeros";
}

/// Random bytes are drawn from the host's source, as many as asked; a length of 0 or past the bound is
/// refused, and a source that gives nothing fails `unavailable`. Two draws from the system differ.
TEST(AStandardHostKeys, RandomBytesAreDrawnFromTheHostsSource) {
    HostScenario::ScriptedRandomCrypto crypto;
    KeysRig rig(crypto);
    crypto.queue({0x00, 0x7F, 0x80, 0xFF});
    EXPECT_EQ(rig.done(host::RANDOM_BYTES, {{"length", "4"}}),
              (json{{"error", ""}, {"bytes", latin1({0x00, 0x7F, 0x80, 0xFF})}}));
    const auto draw = [&rig]() { return rig.done(host::RANDOM_BYTES, {{"length", "32"}}).at("bytes"); };
    EXPECT_NE(draw(), draw());
    EXPECT_EQ(rig.done(host::RANDOM_BYTES, {{"length", "256"}}).at("error"), "") << "the bound of the result";
    for (const char *length : {"0", "257", "", "x", "-1"}) {
        EXPECT_EQ(rig.start(host::RANDOM_BYTES, {{"length", length}}).kind, host::Outcome::Kind::Refuse)
            << "length " << length;
    }

    DryCrypto dry;
    KeysRig dryRig(dry);
    EXPECT_EQ(dryRig.done(host::RANDOM_BYTES, {{"length", "8"}}), (json{{"error", "unavailable"}, {"bytes", ""}}));
    EXPECT_EQ(dryRig.done(host::KEY_GENERATE, {{"key_use", "seal"}, {"label", ""}}),
              (json{{"error", "unavailable"}, {"key_id", ""}}));
}

/// A tag is HMAC-SHA-256 under a MAC key and verifies whole; a tag that does not verify is an answer; a key
/// that does not tag fails `unknown_key`, and a request past its record is refused.
TEST(AStandardHostKeys, ATagVerifiesWholeUnderItsKeyAndNothingElseDoes) {
    host::SodiumCoseCrypto crypto;
    KeysRig rig(crypto);
    const std::string tag = latin1(hex(kJefeTag));
    EXPECT_EQ(rig.done(host::MAC, {{"key_id", "jefe"}, {"message", kJefeMessage}}),
              (json{{"error", ""}, {"tag", tag}}));
    const auto verify = [&rig](const std::string &message, const std::string &candidate) {
        return rig.done(host::MAC_VERIFY, {{"key_id", "jefe"}, {"message", message}, {"tag", candidate}});
    };
    EXPECT_EQ(verify(kJefeMessage, tag), (json{{"error", ""}, {"valid", true}}));
    Bytes flipped = hex(kJefeTag);
    flipped[0] ^= 0x01;
    const json invalid = {{"error", ""}, {"valid", false}};
    EXPECT_EQ(verify(kJefeMessage, latin1(flipped)), invalid) << "a changed tag";
    EXPECT_EQ(verify("what do ya want?", tag), invalid) << "another message";
    EXPECT_EQ(verify(kJefeMessage, tag.substr(0, 16)), invalid) << "a truncated tag";
    for (const char *keyId : {"nobody", "11", "box"}) {
        EXPECT_EQ(rig.done(host::MAC, {{"key_id", keyId}, {"message", "m"}}),
                  (json{{"error", "unknown_key"}, {"tag", ""}}))
            << keyId << " is no MAC key";
    }
    EXPECT_EQ(rig.start(host::MAC, {{"key_id", ""}, {"message", "m"}}).kind, host::Outcome::Kind::Refuse);
    EXPECT_EQ(rig.start(host::MAC, {{"key_id", "jefe"}, {"message", std::string(4097, 'm')}}).kind,
              host::Outcome::Kind::Refuse);
    EXPECT_EQ(rig.start(host::MAC_VERIFY, {{"key_id", "jefe"}, {"message", "m"}, {"tag", std::string(33, 't')}}).kind,
              host::Outcome::Kind::Refuse);
}

/// A generated key is named by the bytes it drew, the id's 16 and then the key's 32; the same start,
/// repeated by a restore, answers its first key and draws nothing; a use that is not one of the four is
/// refused.
TEST(AStandardHostKeys, AGeneratedKeyIsAnsweredOnceUnderItsIdentity) {
    HostScenario::ScriptedRandomCrypto crypto;
    KeysRig rig(crypto);
    Bytes drawn;
    for (int k = 0; k < 48; ++k) {
        drawn.push_back(static_cast<uint8_t>(k));
    }
    crypto.queue(drawn);
    const std::string first = rig.generate("sign", 1);
    EXPECT_EQ(first, "generated:000102030405060708090a0b0c0d0e0f");
    crypto.queue(Bytes(48, uint8_t{0xEE}));
    EXPECT_EQ(rig.generate("sign", 1), first) << "the same start answers its first key";
    EXPECT_EQ(rig.generate("mac", 2), "generated:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee") << "the next start draws";
    for (const auto &[use, label] : std::vector<std::pair<std::string, std::string>>{
             {"derived", ""}, {"verify", ""}, {"", ""}, {"sign", std::string(65, 'l')}}) {
        EXPECT_EQ(rig.start(host::KEY_GENERATE, {{"key_use", use}, {"label", label}}, 3).kind,
                  host::Outcome::Kind::Refuse)
            << "use " << use << ", a label of " << label.size() << " bytes";
    }
    EXPECT_EQ(rig.feature.keyRingIds(),
              (std::vector<std::string>{first, "generated:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"}));
}

/// Each generated key serves its one use: a signing key signs and exports, a sealing key seals, a MAC key
/// tags, a key to derive from derives, and none serves another's.
TEST(AStandardHostKeys, EachGeneratedKeyServesItsOneUse) {
    host::SodiumCoseCrypto crypto;
    KeysRig rig(crypto);
    const std::map<std::string, std::string> keys = {
        {"sign", rig.generate("sign", 1)},
        {"seal", rig.generate("seal", 2)},
        {"mac", rig.generate("mac", 3)},
        {"derive", rig.generate("derive", 4)},
    };
    for (const auto &[use, keyId] : keys) {
        const auto expect = [&use](const char *serves) { return use == serves ? "" : "unknown_key"; };
        EXPECT_EQ(rig.done(host::COSE_SIGN1, {{"key_id", keyId}, {"payload", "p"}, {"external_aad", ""}}).at("error"),
                  expect("sign"))
            << use;
        EXPECT_EQ(
            rig.done(host::COSE_ENCRYPT0, {{"key_id", keyId}, {"plaintext", "p"}, {"external_aad", ""}}).at("error"),
            expect("seal"))
            << use;
        EXPECT_EQ(rig.done(host::MAC, {{"key_id", keyId}, {"message", "m"}}).at("error"), expect("mac")) << use;
        EXPECT_EQ(rig.done(host::KEY_DERIVE, {{"secret", ""}, {"input_key_id", keyId}, {"salt", ""}, {"info", ""}})
                      .at("error"),
                  expect("derive"))
            << use;
        EXPECT_EQ(rig.done(host::KEY_PUBLIC, {{"key_id", keyId}}).at("error"), expect("sign")) << use;
    }
    EXPECT_EQ(rig.done(host::KEY_PUBLIC, {{"key_id", "11"}}),
              (json{{"error", ""}, {"cose_key", latin1(hex(kCoseKey11))}}))
        << "a configured signing key exports its COSE_Key";
}

/// A retired key signs and tags nothing more and answers `key_retired`, still exports and verifies, and
/// retiring it again answers false; a configured key is not the key ring's to retire.
TEST(AStandardHostKeys, ARetiredKeyMakesNothingNewAndStillVerifies) {
    host::SodiumCoseCrypto crypto;
    KeysRig rig(crypto);
    const std::string sign = rig.generate("sign", 1);
    const std::string mac = rig.generate("mac", 2);
    const json tagged = rig.done(host::MAC, {{"key_id", mac}, {"message", "m"}});
    for (const std::string &keyId : {sign, mac}) {
        EXPECT_EQ(rig.done(host::KEY_RETIRE, {{"key_id", keyId}}), (json{{"error", ""}, {"retired", true}}));
        EXPECT_EQ(rig.done(host::KEY_RETIRE, {{"key_id", keyId}}), (json{{"error", ""}, {"retired", false}}));
    }
    EXPECT_EQ(rig.done(host::COSE_SIGN1, {{"key_id", sign}, {"payload", "p"}, {"external_aad", ""}}),
              (json{{"error", "key_retired"}, {"cose", ""}}));
    EXPECT_EQ(rig.done(host::KEY_PUBLIC, {{"key_id", sign}}).at("error"), "");
    EXPECT_EQ(rig.done(host::MAC, {{"key_id", mac}, {"message", "m"}}), (json{{"error", "key_retired"}, {"tag", ""}}));
    EXPECT_EQ(
        rig.done(host::MAC_VERIFY, {{"key_id", mac}, {"message", "m"}, {"tag", tagged.at("tag").get<std::string>()}}),
        (json{{"error", ""}, {"valid", true}}));
    for (const char *keyId : {"11", "generated:00000000000000000000000000000000"}) {
        EXPECT_EQ(rig.done(host::KEY_RETIRE, {{"key_id", keyId}}), (json{{"error", "unknown_key"}, {"retired", false}}))
            << keyId;
    }
}

/// The values of tests/host_features/keys_cross_language.json, which the Rust host gives as well: every tag of
/// RFC 4231, every derived key's id and key under the file's naming key, every generated key's id, COSE_Key,
/// signature and tag, and every configured signing key's COSE_Key.
TEST(AStandardHostKeys, KeysAndTagsCrossBetweenTheHosts) {
    const json file = vectors();

    {
        host::SodiumCoseCrypto crypto;
        KeysRig rig(crypto);
        std::map<std::string, Bytes> macKeys;
        const json &mac = file.at("mac");
        ASSERT_EQ(mac.size(), 6u) << "RFC 4231's test cases 1 to 4, 6 and 7";
        for (size_t n = 0; n < mac.size(); ++n) {
            macKeys["rfc4231-" + std::to_string(n)] = hex(mac[n].at("key").get<std::string>());
        }
        ASSERT_TRUE(rig.feature.setMacKeys(macKeys));
        for (size_t n = 0; n < mac.size(); ++n) {
            const std::string keyId = "rfc4231-" + std::to_string(n);
            const std::string message = latin1(hex(mac[n].at("message").get<std::string>()));
            const std::string tag = latin1(hex(mac[n].at("tag").get<std::string>()));
            EXPECT_EQ(rig.done(host::MAC, {{"key_id", keyId}, {"message", message}}),
                      (json{{"error", ""}, {"tag", tag}}))
                << mac[n].at("name");
            EXPECT_EQ(rig.done(host::MAC_VERIFY, {{"key_id", keyId}, {"message", message}, {"tag", tag}}),
                      (json{{"error", ""}, {"valid", true}}))
                << mac[n].at("name");
        }
    }

    {
        HostScenario::ScriptedRandomCrypto crypto;
        KeysRig rig(crypto);
        std::map<std::string, Bytes> sealing;
        for (const json &key : file.at("sealing_keys")) {
            sealing[key.at("id").get<std::string>()] = hex(key.at("key").get<std::string>());
        }
        ASSERT_TRUE(rig.feature.setSealingKeys(sealing));
        crypto.queue(hex(file.at("naming_key").get<std::string>()));
        ASSERT_FALSE(file.at("derived").empty());
        for (const json &item : file.at("derived")) {
            const std::string name = item.at("name").get<std::string>();
            const std::string keyId = item.at("key_id").get<std::string>();
            EXPECT_EQ(rig.done(host::KEY_DERIVE, {{"secret", latin1(hex(item.at("secret").get<std::string>()))},
                                                  {"input_key_id", item.at("input_key_id").get<std::string>()},
                                                  {"salt", latin1(hex(item.at("salt").get<std::string>()))},
                                                  {"info", latin1(hex(item.at("info").get<std::string>()))}}),
                      (json{{"error", ""}, {"key_id", keyId}}))
                << name;
            // The key kept is HKDF-SHA-256's: an object sealed under it here opens under the file's key with
            // the primitives alone.
            const json sealed = rig.done(
                host::COSE_ENCRYPT0, {{"key_id", keyId}, {"plaintext", "under a derived key"}, {"external_aad", ""}});
            Bytes object;
            ASSERT_EQ(Latin1Bytes::bytesOf(sealed.at("cose").get<std::string>(), object), "");
            const auto read = cose::readEncrypt0(object);
            ASSERT_TRUE(read.has_value()) << name;
            EXPECT_EQ(crypto.aeadOpen(hex(item.at("key").get<std::string>()),
                                      read->unprotectedHeaders.iv.value_or(Bytes{}), read->ciphertext,
                                      cose::encStructure(read->protectedHeader, {})),
                      std::optional<Bytes>(ascii("under a derived key")))
                << name << ": the derived key is the file's";
        }
    }

    for (const json &item : file.at("generated")) {
        const std::string name = item.at("name").get<std::string>();
        const std::string keyId = item.at("key_id").get<std::string>();
        const std::string use = item.at("key_use").get<std::string>();
        HostScenario::ScriptedRandomCrypto crypto;
        KeysRig rig(crypto);
        crypto.queue(hex(item.at("random").get<std::string>()));
        EXPECT_EQ(rig.done(host::KEY_GENERATE, {{"key_use", use}, {"label", name}}),
                  (json{{"error", ""}, {"key_id", keyId}}))
            << name;
        if (use == "sign") {
            EXPECT_EQ(rig.done(host::KEY_PUBLIC, {{"key_id", keyId}}),
                      (json{{"error", ""}, {"cose_key", latin1(hex(item.at("cose_key").get<std::string>()))}}))
                << name;
            EXPECT_EQ(rig.done(host::COSE_SIGN1, {{"key_id", keyId},
                                                  {"payload", latin1(hex(item.at("payload").get<std::string>()))},
                                                  {"external_aad", ""}}),
                      (json{{"error", ""}, {"cose", latin1(hex(item.at("cose").get<std::string>()))}}))
                << name << ": the object the other host verifies under the exported key";
        } else if (use == "mac") {
            EXPECT_EQ(rig.done(host::MAC,
                               {{"key_id", keyId}, {"message", latin1(hex(item.at("message").get<std::string>()))}}),
                      (json{{"error", ""}, {"tag", latin1(hex(item.at("tag").get<std::string>()))}}))
                << name;
        } else {
            ADD_FAILURE() << name << ": no expectation is written for a key of use " << use;
        }
    }

    {
        host::SodiumCoseCrypto crypto;
        KeysRig rig(crypto);
        std::map<std::string, Bytes> seeds;
        for (const json &item : file.at("cose_keys")) {
            seeds[item.at("key_id").get<std::string>()] = hex(item.at("seed").get<std::string>());
        }
        ASSERT_TRUE(rig.feature.setSigningKeys(seeds));
        for (const json &item : file.at("cose_keys")) {
            const std::string keyId = item.at("key_id").get<std::string>();
            EXPECT_EQ(rig.done(host::KEY_PUBLIC, {{"key_id", keyId}}),
                      (json{{"error", ""}, {"cose_key", latin1(hex(item.at("cose_key").get<std::string>()))}}))
                << keyId;
        }
    }
}

}  // namespace SCE::Tests
