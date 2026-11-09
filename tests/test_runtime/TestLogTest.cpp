// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The test log of the test profile's C++ runtime (docs/SCE_TEST_PROFILE.md
// §12.5; docs/adr/0091, C3): the canonical form of a message's value
// (test/Canonical.h), a message's value with its type and writer
// (test/Port.h), and the records of a run (test/TestLog.h), each written in
// the JSON Canonicalization Scheme (CanonicalJsonTest.cpp). Each case is the
// twin of the Rust runtime's of the same subject, in
// backends/rust/runtime/src/test/canonical.rs, src/test/port.rs and
// src/test/test_log.rs, with the same values and the same bytes.

#include <gtest/gtest.h>

#include <any>
#include <cstddef>
#include <cstdint>
#include <fstream>
#include <limits>
#include <optional>
#include <sstream>
#include <stdexcept>
#include <string>
#include <utility>
#include <vector>

#include "saved/SavedJson.h"
#include "test/Canonical.h"
#include "test/Port.h"
#include "test/TestCaseRun.h"
#include "test/TestLog.h"

#ifndef SCE_PROJECT_ROOT
#define SCE_PROJECT_ROOT "."
#endif

namespace {

using SCE::Saved::JsonValue;
using SCE::Saved::writeCanonical;
using SCE::Test::TestCaseResult;
using SCE::Test::TestLogRecord;
using SCE::Test::Verdict;

namespace Canonical = SCE::Test::Canonical;
namespace TestLog = SCE::Test::TestLog;

using namespace std::string_literals;

// ── The canonical form of a value (test/Canonical.h) ─────────────────

/// An integer is the text of its digits at every width, the ends of the
/// 64-bit ones among them, which a double does not hold exactly.
TEST(CanonicalValue, AnIntegerIsTheTextOfItsDecimalDigits) {
    EXPECT_EQ(writeCanonical(Canonical::integer(std::uint8_t{0})), "\"0\"");
    EXPECT_EQ(writeCanonical(Canonical::integer(std::int8_t{-128})), "\"-128\"");
    EXPECT_EQ(writeCanonical(Canonical::integer(std::numeric_limits<std::uint64_t>::max())),
              "\"18446744073709551615\"");
    EXPECT_EQ(writeCanonical(Canonical::integer(std::numeric_limits<std::int64_t>::min())), "\"-9223372036854775808\"");
}

TEST(CanonicalValue, ABoolIsATruthValueAndATextAText) {
    EXPECT_EQ(writeCanonical(Canonical::boolean(true)), "true");
    EXPECT_EQ(writeCanonical(Canonical::boolean(false)), "false");
    EXPECT_EQ(writeCanonical(Canonical::text("a\"b")), "\"a\\\"b\"");
}

/// Two hex digits an octet, upper case, the leading zero kept.
TEST(CanonicalValue, AByteStringIsItsOctetsInUpperCaseHex) {
    EXPECT_EQ(writeCanonical(Canonical::bytes(std::vector<std::uint8_t>{0x00, 0xab, 0x0f, 0xff})), "\"00AB0FFF\"");
    EXPECT_EQ(writeCanonical(Canonical::bytes(std::vector<std::uint8_t>{})), "\"\"");
}

/// A closed enum by its declared name; an open one by its name, or by its
/// number where no variant declares it. A value of a closed enum no variant
/// names is no value of its type, and is refused.
TEST(CanonicalValue, AnEnumIsItsDeclaredName) {
    EXPECT_EQ(writeCanonical(Canonical::enumerated("red")), "\"red\"");
    EXPECT_EQ(writeCanonical(Canonical::openEnumerated("red", std::uint8_t{0})), "\"red\"");
    EXPECT_EQ(writeCanonical(Canonical::openEnumerated(nullptr, std::uint8_t{7})), "\"7\"");
    EXPECT_THROW(Canonical::enumerated(nullptr), std::invalid_argument);
}

/// An absent field is omitted, and the fields are sorted when written,
/// whatever order they were given in.
TEST(CanonicalValue, ARecordOmitsAnAbsentField) {
    const JsonValue value = Canonical::record({{"quantity", Canonical::integer(std::uint16_t{2})},
                                               {"note", std::nullopt},
                                               {"color", Canonical::enumerated("red")}});
    EXPECT_EQ(writeCanonical(value), "{\"color\":\"red\",\"quantity\":\"2\"}");
    EXPECT_EQ(writeCanonical(Canonical::record({{"note", std::nullopt}})), "{}");
}

TEST(CanonicalValue, AListIsItsElementsInOrder) {
    const auto number = [](const std::uint8_t &n) { return Canonical::integer(n); };
    EXPECT_EQ(writeCanonical(Canonical::list(std::vector<std::uint8_t>{3, 1, 2}, number)), "[\"3\",\"1\",\"2\"]");
    EXPECT_EQ(writeCanonical(Canonical::list(std::vector<std::uint8_t>{}, number)), "[]");
}

/// A field that may be absent is the form of what it holds, or none.
TEST(CanonicalValue, AnOptionalFieldIsWhatItHoldsOrNone) {
    const auto text = [](const std::string &held) { return Canonical::text(held); };
    const std::optional<JsonValue> present = Canonical::optional(std::optional<std::string>("late"), text);
    ASSERT_TRUE(present.has_value());
    EXPECT_EQ(writeCanonical(*present), "\"late\"");
    EXPECT_FALSE(Canonical::optional(std::optional<std::string>(), text).has_value());
}

// ── A message's value (test/Port.h) ──────────────────────────────────

/// A message's value is read back by its type, and by no other, and its
/// canonical form and type are the ones its writer gives, whichever copy
/// reads them.
TEST(AMessageValue, IsReadByItsTypeAndWrittenByItsWriter) {
    const SCE::Test::Message message{
        "number", SCE::Test::messageValue(std::uint32_t{7}, "number",
                                          [](const std::uint32_t &n) { return Canonical::integer(n); })};
    ASSERT_NE(message.value.as<std::uint32_t>(), nullptr);
    EXPECT_EQ(*message.value.as<std::uint32_t>(), 7u);
    EXPECT_EQ(message.value.as<std::int32_t>(), nullptr);
    EXPECT_EQ(message.value.get<std::uint32_t>(), 7u);
    EXPECT_THROW(message.value.get<std::int32_t>(), std::bad_any_cast);
    const SCE::Test::Message copy = message;
    EXPECT_EQ(writeCanonical(copy.value.canonical()), "\"7\"");
    EXPECT_EQ(copy.value.typeName(), "number");
}

// ── The records (test/TestLog.h) ─────────────────────────────────────

std::string readFile(const std::string &path) {
    std::ifstream in(path);
    EXPECT_TRUE(in.is_open()) << "not readable: " << path;
    std::ostringstream buffer;
    buffer << in.rdbuf();
    return buffer.str();
}

std::string schemaText() {
    return readFile(std::string(SCE_PROJECT_ROOT) + "/schemas/sce-test-log.v1.schema.json");
}

/// The status this runtime declares is the schema file's own, and the schema
/// pins the name of the shape it writes. SCE_WIRE_CONTRACTS.md requires one
/// commit to move both.
TEST(TheTestLog, TheSchemaFileDeclaresTheStatusThisRuntimeDoes) {
    const JsonValue schema = SCE::Saved::parseJson(schemaText());
    const JsonValue *status = schema.asObject().get("x-sce-schema-status");
    ASSERT_NE(status, nullptr);
    EXPECT_EQ(status->asString(), SCE::Test::TEST_LOG_SCHEMA_STATUS)
        << "schemas/sce-test-log.v1.schema.json x-sce-schema-status disagrees with TEST_LOG_SCHEMA_STATUS";
    const JsonValue *properties = schema.asObject().get("properties");
    ASSERT_NE(properties, nullptr);
    const JsonValue *shape = properties->asObject().get("schema");
    ASSERT_NE(shape, nullptr);
    const JsonValue *named = shape->asObject().get("const");
    ASSERT_NE(named, nullptr);
    EXPECT_EQ(named->asString(), SCE::Test::TEST_LOG_SCHEMA);
}

/// Every kind the runtime writes is one the schema names, each once.
TEST(TheTestLog, EveryKindIsOneTheSchemaNames) {
    const std::string schema = schemaText();
    const std::vector<SCE::Test::TestLogEvent> events = {
        TestLog::TestCaseStarted{},
        TestLog::TestCaseEnded{Verdict::None},
        TestLog::ComponentCreated{"", ""},
        TestLog::ComponentDone{"", Verdict::None},
        TestLog::MessageSent{"", "", "", JsonValue()},
        TestLog::MessageReceived{"", "", "", JsonValue()},
        TestLog::TimerStarted{"", "", 0},
        TestLog::TimerStopped{"", ""},
        TestLog::TimerTimedOut{"", ""},
        TestLog::VerdictSet{"", Verdict::None, std::nullopt},
        TestLog::Log{"", ""},
    };
    for (const auto &event : events) {
        const std::string named = "\"const\": \""s + SCE::Test::testLogKind(event) + "\"";
        std::size_t count = 0;
        for (std::size_t at = schema.find(named); at != std::string::npos; at = schema.find(named, at + 1)) {
            ++count;
        }
        EXPECT_EQ(count, 1u) << "the schema names the kind " << SCE::Test::testLogKind(event) << " once";
    }
}

std::string written(SCE::Test::TestLogEvent event) {
    return writeCanonical(TestLogRecord{250, std::move(event)}.toValue());
}

/// A record of each shape: `at` and every integer as the text of its digits,
/// an absent reason omitted, the members sorted when written.
TEST(TheTestLog, ARecordIsWrittenWithTheMembersOfItsKind) {
    EXPECT_EQ(written(TestLog::TimerStarted{"mtc", "reply", 1000000}),
              "{\"at\":\"250\",\"component\":\"mtc\",\"duration_us\":\"1000000\","
              "\"kind\":\"timer-started\",\"timer\":\"reply\"}");
    EXPECT_EQ(written(TestLog::VerdictSet{"echo", Verdict::Inconclusive, std::nullopt}),
              "{\"at\":\"250\",\"component\":\"echo\",\"kind\":\"verdict-set\",\"verdict\":\"inconc\"}");
    EXPECT_EQ(written(TestLog::VerdictSet{"echo", Verdict::Fail, std::string("late")}),
              "{\"at\":\"250\",\"component\":\"echo\",\"kind\":\"verdict-set\",\"reason\":\"late\","
              "\"verdict\":\"fail\"}");
    EXPECT_EQ(written(TestLog::MessageReceived{"mtc", "sut", "job_reply",
                                               Canonical::record({{"job", Canonical::integer(std::uint32_t{7})}})}),
              "{\"at\":\"250\",\"component\":\"mtc\",\"kind\":\"message-received\","
              "\"port\":\"sut\",\"type\":\"job_reply\",\"value\":{\"job\":\"7\"}}");
}

/// The log of a run: its shape, suite, test case and verdict before its
/// records, all sorted, with no whitespace.
TEST(TheTestLog, ALogIsItsShapeSuiteCaseVerdictAndRecords) {
    TestCaseResult result;
    result.verdict = Verdict::Error;
    result.records.push_back(TestLogRecord{0, TestLog::TestCaseStarted{}});
    result.records.push_back(TestLogRecord{0, TestLog::TestCaseEnded{Verdict::Error}});
    EXPECT_EQ(result.canonicalLog("a_suite", "a_case"),
              "{\"records\":[{\"at\":\"0\",\"kind\":\"test-case-started\"},"
              "{\"at\":\"0\",\"kind\":\"test-case-ended\",\"verdict\":\"error\"}],"
              "\"schema\":\"sce-test-log.v1\",\"suite\":\"a_suite\",\"test_case\":\"a_case\","
              "\"verdict\":\"error\"}");
}

}  // namespace
