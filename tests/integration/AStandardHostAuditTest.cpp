// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The audit log of the reference host (sce/include/host/AuditHost.h; docs/SCE_HOST_FEATURES.md, section 15;
// docs/adr/0109, D14) on libsodium: the bytes of tests/host_features/audit_cross_language.json, which the Rust
// host is held to as well (backends/rust/crypto/tests/audit.rs, backends/rust/host-features/tests/audit.rs);
// an append redacted by both marks, chained and verified; a changed entry and a truncation told; retention that
// keeps the rest verifiable; and an append repeated under one identity that appends nothing.
//
// Compiled into standard_host_scenario_test only in a build that found libsodium, whose target `sce_host_cose`
// it links (cmake/SCEHostCose.cmake).

#include "host/AuditHost.h"
#include "host/Cose.h"
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
using Bytes = std::vector<uint8_t>;

json vectors() {
    std::ifstream in(std::filesystem::path(SCE_PROJECT_ROOT) / "tests" / "host_features" / "audit_cross_language.json");
    EXPECT_TRUE(in.good()) << "the vector file is not readable";
    return json::parse(in);
}

Bytes hex(const json &value) {
    const auto bytes = HostScenario::bytesOfHex(value.get<std::string>());
    EXPECT_TRUE(bytes.has_value()) << value.dump();
    return bytes.value_or(Bytes{});
}

/// The primitives on libsodium, whose random source gives the vectors' pseudonymisation key and then their
/// audit seed, in the order the host draws them.
class VectorCrypto : public host::SodiumCoseCrypto {
public:
    VectorCrypto() {
        const json v = vectors();
        queued_ = hex(v.at("pseudonym_key"));
        const Bytes seed = hex(v.at("seed"));
        queued_.insert(queued_.end(), seed.begin(), seed.end());
    }

    std::optional<Bytes> random(size_t size) override {
        if (queued_.size() < size) {
            return host::SodiumCoseCrypto::random(size);
        }
        Bytes out(queued_.begin(), queued_.begin() + static_cast<std::ptrdiff_t>(size));
        queued_.erase(queued_.begin(), queued_.begin() + static_cast<std::ptrdiff_t>(size));
        return out;
    }

private:
    Bytes queued_;
};

/// A reference host with the audit log, keeping `sign-ins`, checkpointed every two rows.
struct AuditRig {
    explicit AuditRig(host::AuditPersonal personal = host::AuditPersonal::Pseudonymised) {
        EXPECT_TRUE(reference.addFeature(audit));
        audit.setStreams({{"sign-ins", host::AuditStreamPolicy{2, 30, personal}}});
    }

    /// The result record of an append of `received` as start `number`, at `atMs`.
    json append(const std::string &received, uint64_t number, int64_t atMs) {
        clock.reading = host::Reading{atMs, 0};
        const auto outcome = reference.start(
            host::AUDIT_APPEND, host::Invocation{"session-1", "audit", number},
            {{"stream", "sign-ins"}, {"entry", received}, {"sensitivity", vectors().at("marks").get<std::string>()}});
        EXPECT_TRUE(outcome.has_value());
        EXPECT_EQ(outcome->kind, host::Outcome::Kind::Done) << outcome->data;
        return json::parse(outcome->data);
    }

    VectorCrypto crypto;
    host::ScriptedClock clock;
    HostScenario::ScriptTransport transport;
    host::AuditFeature audit{crypto, clock};
    host::ReferenceHost reference{clock, transport};
};

/// The three rows of the vectors appended, and the stop's checkpoint of the head.
void appendTheVectors(AuditRig &rig) {
    const json v = vectors();
    uint64_t number = 1;
    for (const auto &row : v.at("rows")) {
        const json answered = rig.append(row.at("received").get<std::string>(), number, row.at("at_ms").get<int64_t>());
        EXPECT_EQ(answered.at("error"), "");
        EXPECT_EQ(answered.at("sequence"), std::to_string(number));
        ++number;
    }
}

}  // namespace

TEST(AStandardHostAudit, EveryRowAndTheCheckpointOfTheVectorsAreTheirBytes) {
    host::SodiumCoseCrypto crypto;
    ASSERT_TRUE(crypto.ready());
    const json v = vectors();
    const std::string stream = v.at("stream").get<std::string>();
    EXPECT_EQ(crypto.ed25519PublicKey(hex(v.at("seed"))), hex(v.at("public_key")));
    Bytes chain(32, 0);
    for (const auto &row : v.at("rows")) {
        const uint64_t sequence = row.at("sequence").get<uint64_t>();
        const uint64_t atMs = row.at("at_ms").get<uint64_t>();
        const std::string entry = row.at("entry").get<std::string>();
        EXPECT_EQ(host::audit::entryItem(stream, sequence, atMs, entry), hex(row.at("item_hex"))) << sequence;
        chain = host::audit::chained(crypto, chain, stream, sequence, atMs, entry).value_or(Bytes{});
        EXPECT_EQ(chain, hex(row.at("chain_hex"))) << sequence;
    }
    const json &checkpoint = v.at("checkpoint");
    EXPECT_EQ(host::audit::checkpointPayload(stream, 3, chain), hex(checkpoint.at("payload_hex")));
    const auto signature = host::audit::signCheckpoint(crypto, hex(v.at("seed")), stream, 3, chain);
    ASSERT_TRUE(signature.has_value());
    EXPECT_EQ(*signature, hex(checkpoint.at("cose_sign1_hex"))) << "an Ed25519 signature is deterministic";
    EXPECT_TRUE(host::audit::checkpointVerifies(crypto, hex(v.at("public_key")), stream,
                                                host::AuditCheckpoint{3, chain, *signature}));
    EXPECT_FALSE(host::audit::checkpointVerifies(crypto, hex(v.at("public_key")), stream,
                                                 host::AuditCheckpoint{2, chain, *signature}))
        << "a signature moved to another sequence does not verify";
}

TEST(AStandardHostAudit, AnEntryIsRedactedByBothMarksToTheBytesOfTheVectors) {
    host::SodiumCoseCrypto crypto;
    ASSERT_TRUE(crypto.ready());
    const json v = vectors();
    const Bytes key = hex(v.at("pseudonym_key"));
    const host::audit::Pseudonym pseudonym = [&crypto, &key](const std::string &text) -> std::optional<std::string> {
        const auto digest = crypto.hmacSha256(key, Bytes(text.begin(), text.end()));
        if (!digest) {
            return std::nullopt;
        }
        return host::lowerHex(*digest, digest->size());
    };
    for (const auto &row : v.at("rows")) {
        std::string written;
        EXPECT_EQ(host::audit::redact(row.at("received").get<std::string>(), v.at("marks").get<std::string>(),
                                      host::AuditPersonal::Pseudonymised, pseudonym, written),
                  host::audit::Unredacted::None);
        EXPECT_EQ(written, row.at("entry").get<std::string>());
    }
    std::string clear;
    EXPECT_EQ(host::audit::redact(R"({"password":"p","account":"a@b"})", "/account=personal;/password=secret",
                                  host::AuditPersonal::Clear, pseudonym, clear),
              host::audit::Unredacted::None);
    EXPECT_EQ(clear, R"({"account":"a@b","password":"sce-secret"})")
        << "a stream in the clear keeps the personal value and still never the secret";
    for (const auto &[entry, marks] : std::vector<std::pair<std::string, std::string>>{
             {"[1]", ""}, {R"({"a":1})", "/b=secret"}, {R"({"a":1})", "/a=personal"}, {R"({"a":"x"})", "a=secret"}}) {
        std::string written;
        EXPECT_EQ(host::audit::redact(entry, marks, host::AuditPersonal::Pseudonymised, pseudonym, written),
                  host::audit::Unredacted::Refused)
            << entry << " under " << marks;
    }
}

TEST(AStandardHostAudit, AnAppendIsChainedCheckpointedAndVerified) {
    AuditRig rig;
    appendTheVectors(rig);
    const json v = vectors();
    const auto &rows = rig.audit.rows("sign-ins");
    ASSERT_EQ(rows.size(), 3u);
    for (size_t k = 0; k < rows.size(); ++k) {
        EXPECT_EQ(rows[k].entry, v.at("rows").at(k).at("entry").get<std::string>());
        EXPECT_EQ(rows[k].chain, hex(v.at("rows").at(k).at("chain_hex")));
    }
    ASSERT_EQ(rig.audit.checkpoints("sign-ins").size(), 1u) << "a checkpoint every two rows";
    const auto publicKey = rig.audit.publicKey();
    ASSERT_TRUE(publicKey.has_value());
    EXPECT_EQ(*publicKey, hex(v.at("public_key"))) << "the audit key is the second 32 bytes the host drew";
    const auto signed_ = rig.audit.stop();
    ASSERT_TRUE(signed_.has_value());
    ASSERT_EQ(signed_->size(), 1u);
    EXPECT_EQ(signed_->front().second, 3u) << "the stop signs the head";
    EXPECT_EQ(rig.audit.checkpoints("sign-ins").back().signature, hex(v.at("checkpoint").at("cose_sign1_hex")));
    const host::AuditVerdict verdict = rig.audit.verify("sign-ins", *publicKey);
    EXPECT_EQ(verdict.kind, host::AuditVerdict::Kind::Verified);
    EXPECT_EQ(verdict.from, 1u);
    EXPECT_EQ(verdict.through, 3u);
}

TEST(AStandardHostAudit, AChangedEntryIsToldAtItsRow) {
    AuditRig rig;
    appendTheVectors(rig);
    ASSERT_TRUE(rig.audit.stop().has_value());
    const Bytes publicKey = rig.audit.publicKey().value_or(Bytes{});
    rig.audit.rows("sign-ins")[1].entry =
        R"({"account":"bob","attempt":1,"password":"sce-secret","result":"accepted"})";
    const host::AuditVerdict verdict = rig.audit.verify("sign-ins", publicKey);
    EXPECT_EQ(verdict.kind, host::AuditVerdict::Kind::Changed);
    EXPECT_EQ(verdict.sequence, 2u);
}

TEST(AStandardHostAudit, ATruncationIsToldByTheCheckpointPastTheLastRow) {
    AuditRig rig;
    appendTheVectors(rig);
    ASSERT_TRUE(rig.audit.stop().has_value());
    const Bytes publicKey = rig.audit.publicKey().value_or(Bytes{});
    rig.audit.rows("sign-ins").pop_back();
    const host::AuditVerdict verdict = rig.audit.verify("sign-ins", publicKey);
    EXPECT_EQ(verdict.kind, host::AuditVerdict::Kind::Truncated);
    EXPECT_EQ(verdict.sequence, 3u);
    EXPECT_EQ(verdict.through, 2u);
    const host::AuditVerdict forged = rig.audit.verify("sign-ins", Bytes(32, 9));
    EXPECT_EQ(forged.kind, host::AuditVerdict::Kind::Forged) << "a checkpoint under another key";
}

TEST(AStandardHostAudit, RetentionDeletesWholeSegmentsAndKeepsTheRestVerifiable) {
    AuditRig rig;
    // Rows at 1 000, 2 000, ... 7 000 ms; checkpoints at 2, 4 and 6.
    for (uint64_t n = 1; n <= 7; ++n) {
        rig.append(R"({"account":"a","password":"p"})", n, static_cast<int64_t>(n * 1000));
    }
    const Bytes publicKey = rig.audit.publicKey().value_or(Bytes{});
    // 30 days and 5 500 ms after the epoch: rows 1 to 5 are older, and the latest checkpoint before row 6 is
    // at 4.
    const uint64_t now = 30ull * 86400000ull + 5500;
    EXPECT_EQ(rig.audit.retain(now), 4u);
    const host::AuditVerdict verdict = rig.audit.verify("sign-ins", publicKey);
    EXPECT_EQ(verdict.kind, host::AuditVerdict::Kind::Verified);
    EXPECT_EQ(verdict.from, 5u) << "the checkpoint at the cut anchors what remains";
    EXPECT_EQ(verdict.through, 7u);
    EXPECT_EQ(rig.audit.checkpoints("sign-ins").front().sequence, 4u);
}

TEST(AStandardHostAudit, AnAppendRepeatedUnderItsIdentityAppendsNothing) {
    AuditRig rig;
    const std::string entry = R"({"account":"a","password":"p"})";
    const json first = rig.append(entry, 1, 1000);
    const json again = rig.append(entry, 1, 2000);
    EXPECT_EQ(first, again) << "the first sequence";
    EXPECT_EQ(rig.audit.rows("sign-ins").size(), 1u) << "nothing appended twice";
    EXPECT_EQ(rig.append(entry, 2, 3000).at("sequence"), "2") << "another start is another entry";
}

TEST(AStandardHostAudit, AStreamTheHostKeepsNoPolicyForAndARefusedRequestAppendNothing) {
    AuditRig rig;
    rig.clock.reading = host::Reading{1000, 0};
    const auto unknown = rig.reference.start(host::AUDIT_APPEND, host::Invocation{"s", "audit", 1},
                                             {{"stream", "other"}, {"entry", "{}"}, {"sensitivity", ""}});
    ASSERT_TRUE(unknown.has_value());
    EXPECT_EQ(json::parse(unknown->data).at("error"), "unknown_stream");
    EXPECT_EQ(json::parse(unknown->data).at("sequence"), "0");
    for (const auto &params : std::vector<host::Params>{
             {{"stream", ""}, {"entry", "{}"}, {"sensitivity", ""}},
             {{"stream", std::string(65, 's')}, {"entry", "{}"}, {"sensitivity", ""}},
             {{"stream", "sign-ins"}, {"entry", "[]"}, {"sensitivity", ""}},
             {{"stream", "sign-ins"}, {"entry", R"({"account":"a"})"}, {"sensitivity", "/password=secret"}}}) {
        const auto refused = rig.reference.start(host::AUDIT_APPEND, host::Invocation{"s", "audit", 2}, params);
        ASSERT_TRUE(refused.has_value());
        EXPECT_EQ(refused->kind, host::Outcome::Kind::Refuse);
    }
    EXPECT_TRUE(rig.audit.rows("sign-ins").empty());
}

// docs/adr/0109, D16: each entry the feature appends is reported and a repeat under one identity, which
// appends nothing, is not; through the host, an append to a stream it keeps no policy for is a failure of its
// own code, and a refused request one of invalid_argument. Red if AuditFeature::run reports the repeat's
// answer as an append, or the host reads a feature's failure code from anywhere but its answer.
TEST(AStandardHostAudit, EachEntryAppendedIsReportedAndARepeatIsNot) {
    AuditRig rig;
    SCE::Metrics::Recording recording;
    rig.audit.setMetrics(recording);
    rig.reference.setMetrics(recording);
    const std::string entry = R"({"account":"a","password":"p"})";
    rig.append(entry, 1, 1000);
    rig.append(entry, 1, 2000);
    EXPECT_EQ(recording.counted(SCE::Metrics::Counter::auditAppends()), 1u);
    rig.append(entry, 2, 3000);
    EXPECT_EQ(recording.counted(SCE::Metrics::Counter::auditAppends()), 2u);

    rig.reference.start(host::AUDIT_APPEND, host::Invocation{"s", "audit", 3},
                        {{"stream", "other"}, {"entry", "{}"}, {"sensitivity", ""}});
    rig.reference.start(host::AUDIT_APPEND, host::Invocation{"s", "audit", 4},
                        {{"stream", ""}, {"entry", "{}"}, {"sensitivity", ""}});
    EXPECT_EQ(recording.counted(SCE::Metrics::Counter::auditAppends()), 2u);
    EXPECT_EQ(recording.counted(SCE::Metrics::Counter::hostOperationFailures(SCE::Metrics::HostOperation::AuditAppend,
                                                                             SCE::Metrics::HostFailure::UnknownStream)),
              1u);
    EXPECT_EQ(recording.counted(SCE::Metrics::Counter::hostOperationFailures(
                  SCE::Metrics::HostOperation::AuditAppend, SCE::Metrics::HostFailure::InvalidArgument)),
              1u);
    EXPECT_EQ(
        recording.observed(SCE::Metrics::Histogram::hostOperationDuration(SCE::Metrics::HostOperation::AuditAppend))
            .size(),
        5u)
        << "each start the host answered is measured once";
}

}  // namespace SCE::Tests
