// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The sealed record of a server's store (sce/include/host/SealedRecord.h; docs/SCE_SEALED_RECORD.md;
// docs/adr/0109, D8) on libsodium: the records and entries of tests/host_features/sealed_cross_language.json,
// which the Rust sealing store is held to by backends/rust/crypto/tests/sealed.rs, so the two agree byte for
// byte; and a record opened for another row or store, changed by a byte, or under another key, does not open.
//
// Compiled into standard_host_scenario_test only in a build that found libsodium, whose target `sce_host_cose`
// it links (cmake/SCEHostCose.cmake).

#include "host/SealedRecord.h"
#include "integration/StandardHostScenarioReplay.h"

#include <cstddef>
#include <cstdint>
#include <filesystem>
#include <fstream>
#include <gtest/gtest.h>
#include <nlohmann/json.hpp>
#include <optional>
#include <string>
#include <vector>

namespace SCE::Tests {

namespace {

using json = nlohmann::json;
namespace sealed = SCE::StandardHost::SealedRecord;
using Bytes = std::vector<uint8_t>;

Bytes hex(const json &value) {
    const auto bytes = HostScenario::bytesOfHex(value.get<std::string>());
    EXPECT_TRUE(bytes.has_value()) << value << " is not hex";
    return bytes.value_or(Bytes{});
}

json vectors() {
    std::ifstream in(std::filesystem::path(SCE_PROJECT_ROOT) / "tests" / "host_features" /
                     "sealed_cross_language.json");
    EXPECT_TRUE(in.good()) << "the vector file is not readable";
    return json::parse(in);
}

}  // namespace

/// Every record of the vectors is sealed into its bytes and opened back into its plaintext, and names the key
/// that sealed it.
TEST(ASealedRecord, EveryRecordOfTheVectorsIsSealedAndOpenedToItsBytes) {
    const json records = vectors().at("records");
    ASSERT_EQ(records.size(), 4u);
    for (const json &vector : records) {
        const std::string about = vector.at("about").get<std::string>();
        const std::string keyId = vector.at("key_id").get<std::string>();
        const std::string store = vector.at("store").get<std::string>();
        const Bytes key = hex(vector.at("key"));
        const Bytes indexed = hex(vector.at("indexed"));
        const auto record =
            sealed::seal(keyId, key, hex(vector.at("nonce")), store, indexed, hex(vector.at("plaintext")));
        ASSERT_TRUE(record.has_value()) << about;
        EXPECT_EQ(*record, hex(vector.at("record"))) << about;
        EXPECT_EQ(sealed::keyIdOf(*record), keyId) << about;
        EXPECT_EQ(sealed::open(key, store, indexed, *record), hex(vector.at("plaintext"))) << about;
    }
}

/// A record opened for another row or another store, with any byte of it changed, or under another key, does
/// not open; bytes that are no record name no key; a key with no id seals nothing.
TEST(ASealedRecord, ARecordOpenedForAnotherRowChangedOrUnderAnotherKeyDoesNotOpen) {
    const json vector = vectors().at("records").at(0);
    const Bytes key = hex(vector.at("key"));
    const Bytes indexed = hex(vector.at("indexed"));
    const Bytes record = hex(vector.at("record"));
    const std::string store = vector.at("store").get<std::string>();
    ASSERT_TRUE(sealed::open(key, store, indexed, record).has_value());

    Bytes otherEntry = indexed;
    otherEntry[0] ^= 0x01;
    EXPECT_FALSE(sealed::open(key, store, otherEntry, record).has_value()) << "the record of another row";
    EXPECT_FALSE(sealed::open(key, "credential_pins", indexed, record).has_value()) << "another store";
    for (const size_t at : {sealed::MAGIC_BYTES + 1, record.size() - 30, record.size() - 1}) {
        Bytes changed = record;
        changed[at] ^= 0x01;
        EXPECT_FALSE(sealed::open(key, store, indexed, changed).has_value()) << "a byte changed at " << at;
    }
    Bytes otherKey = key;
    otherKey[31] ^= 0x01;
    EXPECT_FALSE(sealed::open(otherKey, store, indexed, record).has_value()) << "another key";
    const Bytes tooShort(record.begin(), record.begin() + static_cast<std::ptrdiff_t>(sealed::MAGIC_BYTES + 20));
    EXPECT_FALSE(sealed::keyIdOf(tooShort).has_value()) << "too short for a nonce and a tag";
    EXPECT_FALSE(sealed::keyIdOf(Bytes{'n', 'o', 't'}).has_value());
    EXPECT_FALSE(sealed::seal("", key, Bytes(sealed::NONCE_BYTES), store, indexed, Bytes{1}).has_value())
        << "a key with no id";
}

/// Every key of the vectors is kept under its entry, and a token's digest under its keyed digest; a fresh
/// sealing draws its own nonce and opens.
TEST(ASealedRecord, EveryKeyIsKeptUnderItsKeyedDigest) {
    const json all = vectors();
    const Bytes indexKey = hex(all.at("index_key"));
    const json digests = all.at("digests");
    ASSERT_EQ(digests.size(), 4u);
    for (const json &digest : digests) {
        EXPECT_EQ(sealed::digestOf(indexKey, digest.at("key").get<std::string>()),
                  digest.at("digest").get<std::string>())
            << digest.at("key");
    }
    EXPECT_EQ(sealed::keyedDigest(indexKey, hex(all.at("token_digest"))), hex(all.at("token_entry")));

    const json vector = all.at("records").at(0);
    const Bytes key = hex(vector.at("key"));
    const Bytes indexed = hex(vector.at("indexed"));
    const Bytes plaintext = hex(vector.at("plaintext"));
    const auto first = sealed::sealFresh("sealing:fresh", key, "sessions", indexed, plaintext);
    const auto second = sealed::sealFresh("sealing:fresh", key, "sessions", indexed, plaintext);
    ASSERT_TRUE(first.has_value() && second.has_value());
    EXPECT_NE(*first, *second) << "each sealing draws its own nonce";
    EXPECT_EQ(sealed::open(key, "sessions", indexed, *first), plaintext);
}

}  // namespace SCE::Tests
