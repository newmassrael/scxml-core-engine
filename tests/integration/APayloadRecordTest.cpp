// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A record signed and sealed by name (sce:payload, docs/adr/0109, D9), which the C++ and the Rust lanes agree
// on byte for byte: tests/host_features/payload_record_cross_language.json, which the Rust lane is held to by
// backends/rust/host-features/tests/payload_record_cross_language.rs. The record codec (common/RecordCbor.h)
// writes the record's deterministic CBOR and reads it back; the reference host signs and seals those bytes to
// the file's objects; and a generated machine that holds nothing but the signer's public key verifies an object
// it reads from a file and receives the record, with no server and no network in the path.

#include "common/Latin1Bytes.h"
#include "common/RecordCbor.h"
#include "common/SceClock.h"
#include "host/CoseHost.h"
#include "host/SodiumCoseCrypto.h"
#include "host/StandardHost.h"
#include "integration/StandardHostScenarioReplay.h"
#include "static_host_cose_sm.h"

#include <cstdint>
#include <filesystem>
#include <fstream>
#include <gtest/gtest.h>
#include <iterator>
#include <map>
#include <memory>
#include <nlohmann/json.hpp>
#include <optional>
#include <string>
#include <vector>

namespace SCE::Tests {

namespace {

using json = nlohmann::json;
namespace host = SCE::StandardHost;
using Bytes = std::vector<uint8_t>;
using CoseMachine = SCE::Generated::static_host_cose::static_host_cose;

/// The completion of a verification the build writes for the failure `not_a_record`.
const char *const kVerifyFailure = R"({"error":"not_a_record","key_id":"","payload":null,"valid":false})";

/// The same, of an opening.
const char *const kOpenFailure = R"({"error":"not_a_record","opened":false,"plaintext":null})";

json vectors() {
    std::ifstream in(std::filesystem::path(SCE_PROJECT_ROOT) / "tests" / "host_features" /
                     "payload_record_cross_language.json");
    EXPECT_TRUE(in.good()) << "the vector file is not readable";
    return json::parse(in);
}

Bytes hex(const std::string &text) {
    const auto bytes = HostScenario::bytesOfHex(text);
    EXPECT_TRUE(bytes.has_value()) << text << " is not hex";
    return bytes.value_or(Bytes{});
}

/// The text a byte string crosses a request or a result as.
std::string latin1(const Bytes &bytes) {
    return Latin1Bytes::textOf(bytes);
}

/// Run @p machine until what it was handed is taken, as the scenario replay does.
void settle(CoseMachine &machine) {
    if constexpr (::SCE::Core::NeedsEventSchedulerTrait<CoseMachine::PolicyType>::value) {
        for (int round = 0; round < 5; ++round) {
            machine.tick();
        }
    } else {
        machine.step();
    }
}

/// The result record of a completion of @p operation by @p reference.
std::string done(host::ReferenceHost &reference, const std::string &operation, const host::Params &params) {
    const auto outcome = reference.start(operation, host::Invocation{"session", "note", 1}, params);
    if (!outcome.has_value() || outcome->kind != host::Outcome::Kind::Done) {
        ADD_FAILURE() << operation << " did not complete";
        return "{}";
    }
    return outcome->data;
}

}  // namespace

/// The record codec writes the file's bytes from the record as a machine writes it, and reads them back to it;
/// bytes that are no note are refused.
TEST(APayloadRecord, TheRecordCodecWritesTheFilesBytesAndReadsThemBack) {
    const json file = vectors();
    const std::string description = file.at("description").get<std::string>();
    std::string refusal;
    const auto written = Common::RecordCbor::encode(file.at("record").dump(), description, refusal);
    ASSERT_TRUE(written.has_value()) << refusal;
    EXPECT_EQ(*written, hex(file.at("record_cbor").get<std::string>()));
    const auto read = Common::RecordCbor::decode(*written, description, refusal);
    ASSERT_TRUE(read.has_value()) << refusal;
    EXPECT_EQ(json::parse(*read), file.at("record"));
    EXPECT_FALSE(
        Common::RecordCbor::decode(hex(file.at("not_a_record").at("bytes").get<std::string>()), description, refusal)
            .has_value())
        << "bytes that are no note are refused";
}

/// The reference host signs and seals the record's bytes to the file's objects, which the Rust host writes too.
TEST(APayloadRecord, TheHostSignsAndSealsTheRecordsBytesByteForByte) {
    const json file = vectors();
    HostScenario::ScriptedRandomCrypto crypto;
    host::ScriptedClock clock;
    HostScenario::ScriptTransport transport;
    host::ReferenceHost reference{clock, transport};
    host::CoseFeature feature(crypto);
    ASSERT_TRUE(reference.addFeature(feature));
    const std::string signer = file.at("signing_key").at("id").get<std::string>();
    const std::string vault = file.at("sealing_key").at("id").get<std::string>();
    ASSERT_TRUE(feature.setSigningKeys({{signer, hex(file.at("signing_key").at("seed").get<std::string>())}}));
    ASSERT_TRUE(feature.setSealingKeys({{vault, hex(file.at("sealing_key").at("key").get<std::string>())}}));
    const std::string record = latin1(hex(file.at("record_cbor").get<std::string>()));

    EXPECT_EQ(
        json::parse(done(reference, host::COSE_SIGN1, {{"key_id", signer}, {"payload", record}, {"external_aad", ""}})),
        (json{{"error", ""}, {"cose", latin1(hex(file.at("signed").get<std::string>()))}}));
    crypto.queue(hex(file.at("nonce").get<std::string>()));
    EXPECT_EQ(json::parse(done(reference, host::COSE_ENCRYPT0,
                               {{"key_id", vault}, {"plaintext", record}, {"external_aad", ""}})),
              (json{{"error", ""}, {"cose", latin1(hex(file.at("sealed").get<std::string>()))}}));
}

/// A completion of a verification or an opening is read as the record, as no record where the object did not
/// verify, and as the failure `not_a_record` where it verified or opened to bytes that are no note.
TEST(APayloadRecord, ACompletionIsReadAsTheRecordOrAsTheFailure) {
    const json file = vectors();
    HostScenario::ScriptedRandomCrypto crypto;
    host::ScriptedClock clock;
    HostScenario::ScriptTransport transport;
    host::ReferenceHost reference{clock, transport};
    host::CoseFeature feature(crypto);
    ASSERT_TRUE(reference.addFeature(feature));
    const std::string signer = file.at("signing_key").at("id").get<std::string>();
    const std::string vault = file.at("sealing_key").at("id").get<std::string>();
    ASSERT_TRUE(feature.setTrustedKeys({{signer, hex(file.at("signing_key").at("public_key").get<std::string>())}}));
    feature.setTrustSets({{"devices", {signer}}});
    ASSERT_TRUE(feature.setSealingKeys({{vault, hex(file.at("sealing_key").at("key").get<std::string>())}}));
    const std::string description = file.at("description").get<std::string>();
    const auto verified = [&](const std::string &object) {
        return json::parse(Common::RecordCbor::openCompletion(
            done(reference, host::COSE_VERIFY1,
                 {{"trust_set", "devices"}, {"cose", latin1(hex(object))}, {"external_aad", ""}}),
            "payload", "valid", description, kVerifyFailure));
    };
    const auto opened = [&](const std::string &object) {
        return json::parse(Common::RecordCbor::openCompletion(
            done(reference, host::COSE_DECRYPT0,
                 {{"key_id", vault}, {"cose", latin1(hex(object))}, {"external_aad", ""}}),
            "plaintext", "opened", description, kOpenFailure));
    };

    const json signedNote = verified(file.at("signed").get<std::string>());
    EXPECT_EQ(signedNote.at("valid"), true);
    EXPECT_EQ(signedNote.at("key_id"), signer);
    EXPECT_EQ(signedNote.at("payload"), file.at("record")) << "the machine receives the note";
    const json tampered = verified(file.at("tampered").get<std::string>());
    EXPECT_EQ(tampered.at("error"), "") << "a signature that does not verify is an answer";
    EXPECT_EQ(tampered.at("valid"), false);
    EXPECT_TRUE(tampered.at("payload").is_null());
    EXPECT_EQ(verified(file.at("not_a_record").at("signed").get<std::string>()), json::parse(kVerifyFailure));

    const json sealedNote = opened(file.at("sealed").get<std::string>());
    EXPECT_EQ(sealedNote.at("opened"), true);
    EXPECT_EQ(sealedNote.at("plaintext"), file.at("record"));
    EXPECT_EQ(opened(file.at("not_a_record").at("sealed").get<std::string>()), json::parse(kOpenFailure));
}

/// A machine built from the second revision of the note's schema reads the note the first revision wrote, each field
/// the second revision added at what such a note reads in its place (docs/adr/0044): the record codec reads the
/// file's bytes so, and a host that holds nothing but the signer's public key verifies the file's object, which a host
/// of the first revision signed, to that record. A note that lacks a field of the first revision is no note of either
/// revision: its bytes are refused, and an object of it verifies and is the failure `not_a_record`.
TEST(APayloadRecord, ALaterRevisionReadsANoteTheFirstRevisionSigned) {
    const json file = vectors();
    const json &revised = file.at("revised");
    const std::string description = revised.at("description").get<std::string>();
    const Bytes lacking = hex(revised.at("lacking_serial").get<std::string>());
    std::string refusal;
    const auto read = Common::RecordCbor::decode(hex(file.at("record_cbor").get<std::string>()), description, refusal);
    ASSERT_TRUE(read.has_value()) << refusal;
    EXPECT_EQ(json::parse(*read), revised.at("record")) << "a note of the first revision is a note of the second";
    EXPECT_FALSE(Common::RecordCbor::decode(lacking, description, refusal).has_value())
        << "a note that lacks a field of the first revision is refused";
    EXPECT_NE(refusal.find("/serial"), std::string::npos) << refusal;

    // A host of the first revision signs the note that lacks `serial`.
    HostScenario::ScriptedRandomCrypto signingCrypto;
    host::ScriptedClock signingClock;
    HostScenario::ScriptTransport signingTransport;
    host::ReferenceHost signingHost{signingClock, signingTransport};
    host::CoseFeature signing(signingCrypto);
    ASSERT_TRUE(signingHost.addFeature(signing));
    const std::string signer = file.at("signing_key").at("id").get<std::string>();
    ASSERT_TRUE(signing.setSigningKeys({{signer, hex(file.at("signing_key").at("seed").get<std::string>())}}));
    const json signedLacking = json::parse(
        done(signingHost, host::COSE_SIGN1, {{"key_id", signer}, {"payload", latin1(lacking)}, {"external_aad", ""}}));
    ASSERT_EQ(signedLacking.at("error"), "") << "the bytes are signed whatever they hold";

    // A host of the second revision verifies with the signer's public key alone.
    HostScenario::ScriptedRandomCrypto crypto;
    host::ScriptedClock clock;
    HostScenario::ScriptTransport transport;
    host::ReferenceHost reference{clock, transport};
    host::CoseFeature feature(crypto);
    ASSERT_TRUE(reference.addFeature(feature));
    ASSERT_TRUE(feature.setTrustedKeys({{signer, hex(file.at("signing_key").at("public_key").get<std::string>())}}));
    feature.setTrustSets({{"devices", {signer}}});
    const auto verified = [&](const std::string &cose) {
        return json::parse(Common::RecordCbor::openCompletion(
            done(reference, host::COSE_VERIFY1, {{"trust_set", "devices"}, {"cose", cose}, {"external_aad", ""}}),
            "payload", "valid", description, kVerifyFailure));
    };

    const json signedNote = verified(latin1(hex(file.at("signed").get<std::string>())));
    EXPECT_EQ(signedNote.at("error"), "");
    EXPECT_EQ(signedNote.at("valid"), true);
    EXPECT_EQ(signedNote.at("payload"), revised.at("record"))
        << "the machine receives the note with the later fields' defaults";
    EXPECT_EQ(verified(signedLacking.at("cose").get<std::string>()), json::parse(kVerifyFailure))
        << "a note that lacks `serial` is no note";
}

/// A client verifies what its server signed with nothing but the server's public key: the generated machine
/// reads a COSE_Sign1 object from a file, asks its host to verify it under a trust set of that key, and receives
/// the record. The host serves the COSE operations on libsodium alone, with no transport that reaches anything,
/// so no server is in the path.
TEST(APayloadRecord, AMachineVerifiesASignedRecordFromAFileWithNoServer) {
    const json file = vectors();
    const std::filesystem::path objectPath =
        std::filesystem::temp_directory_path() / "sce_payload_record_signed_note.cose";
    {
        const Bytes object = hex(file.at("signed").get<std::string>());
        std::ofstream out(objectPath, std::ios::binary);
        out.write(reinterpret_cast<const char *>(object.data()), static_cast<std::streamsize>(object.size()));
    }
    Bytes fromFile;
    {
        std::ifstream in(objectPath, std::ios::binary);
        ASSERT_TRUE(in.good()) << "the object's file is not readable";
        fromFile.assign(std::istreambuf_iterator<char>(in), std::istreambuf_iterator<char>());
    }
    std::filesystem::remove(objectPath);

    host::SodiumCoseCrypto crypto;
    host::ScriptedClock clock;
    HostScenario::ScriptTransport transport;
    host::ReferenceHost reference{clock, transport};
    host::CoseFeature feature(crypto);
    ASSERT_TRUE(reference.addFeature(feature));
    const std::string signer = file.at("signing_key").at("id").get<std::string>();
    ASSERT_TRUE(feature.setTrustedKeys({{signer, hex(file.at("signing_key").at("public_key").get<std::string>())}}));
    feature.setTrustSets({{"devices", {signer}}});

    CoseMachine machine;
    host::serve(machine, reference);
    machine.setClock(std::make_shared<SCE::ManualClock>(0));
    EXPECT_EQ(machine.initialize(), std::nullopt);
    settle(machine);
    machine.raiseExternal("cose.note.verify", json{{"trust_set", "devices"}, {"cose", latin1(fromFile)}}.dump());
    settle(machine);
    settle(machine);

    const json variables = json::parse(machine.saveAt(0).toJson()).at("variables");
    EXPECT_EQ(variables.at("outcome"), "done");
    EXPECT_EQ(variables.at("note_valid"), true);
    EXPECT_EQ(variables.at("note_signer"), signer);
    EXPECT_EQ(variables.at("verified_note"), file.at("record")) << "the machine receives the record, not its bytes";
    EXPECT_TRUE(transport.log.empty()) << "nothing crossed a network";
}

}  // namespace SCE::Tests
