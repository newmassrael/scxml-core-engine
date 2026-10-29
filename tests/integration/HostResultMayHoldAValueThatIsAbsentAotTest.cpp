// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Accepted Subset §2.12 and docs/adr/0055, step 10 — a host-run `<invoke>`
// whose `sce:result` record holds values that may be absent. C++ AOT
// compile+run gate; the Rust, Go, Kotlin and Python twins are
// `a_host_result_may_hold_a_value_that_is_absent.rs`, `host_invoker_optional_test.go`,
// `AHostResultMayHoldAValueThatIsAbsentTest` and
// `test_host_result_may_hold_a_value_that_is_absent.py`.
//
// The machine is built with NO script engine: a `datamodel="sce-static"` machine
// holds its variables as fields.
//
// The host answers through the generated typed interface, so the record it hands
// in holds each field that may be absent as a `std::optional`; the engine
// delivers the record as JSON, an absent value as `null` with its key written all
// the same; and the machine reads each field where a test proves it present. What
// these hold apart is the three states a field has — absent, present and zero or
// empty, and present — which a sentinel could not: `""` is a label, `false` is a
// flag that was answered, and `0` is a day.
//
// Fixture: sce-build/tests/fixtures/host_processor/statechart_host_invoker_optional.scxml
// (shared with the other channels; `tests/CMakeLists.txt` compiles it here with the
// same invoker declaration `scripts/regen_host_processor.sh` passes there).

#include "statechart_host_invoker_optional_sm.h"

#include <cstdint>
#include <gtest/gtest.h>
#include <limits>
#include <memory>
#include <optional>
#include <string>
#include <vector>

#include "core/HostProcessor.h"

namespace SCE::Tests {

namespace {

using Machine = SCE::Generated::statechart_host_invoker_optional::statechart_host_invoker_optional;
using LookupResult = SCE::Generated::statechart_host_invoker_optional::LookupResult;

/// The type the fixture was compiled for. `tests/CMakeLists.txt` passes this same
/// string to the invoker declaration.
constexpr const char *DECLARED_TYPE = "x-sce-host";

/// 2^53 + 1, the least positive integer a double cannot hold.
constexpr int64_t PAST_DOUBLES_RANGE = 9007199254740993;
constexpr int64_t INT64_LEAST = std::numeric_limits<int64_t>::min();
constexpr int64_t INT64_GREATEST = std::numeric_limits<int64_t>::max();
constexpr uint64_t TOTAL_MAX = std::numeric_limits<uint64_t>::max();

/// A host implementing the generated interface; its work outlives the call, so
/// each test completes the start it was handed.
struct LookupHost : SCE::Generated::statechart_host_invoker_optional::XSceHostInvoker {
    std::vector<uint64_t> tokens;

    std::optional<LookupResult> startLookup(uint64_t token) override {
        tokens.push_back(token);
        return std::nullopt;
    }

    void cancelLookup(uint64_t) override {}
};

/// What the machine published after a completion: `answered` counts the
/// completions it read, and the rest is what each field of the record held.
struct Answer {
    int answered = 0;
    int rowsSeen = 0;
    uint64_t totalSeen = 0;
    bool atPresent = false;
    int64_t atValue = 0;
    bool labelPresent = false;
    std::string labelValue;
    bool hitKnown = false;
    bool hitTrue = false;
};

class HostResultMayHoldAValueThatIsAbsentAotTest : public ::testing::Test {
protected:
    /// A machine standing at `asking`, with the token the host was started with.
    void SetUp() override {
        host = std::make_shared<LookupHost>();
        sm.registerXSceHostInvoker(host);
        sm.initialize();
        sm.step();
        sm.raiseExternal("ask");
        sm.step();
        ASSERT_EQ(host->tokens.size(), 1U) << "the lookup was started once";
        token = host->tokens[0];
    }

    /// What the machine published.
    Answer read() const {
        Answer answer;
        answer.answered = static_cast<int>(sm.answered());
        answer.rowsSeen = static_cast<int>(sm.rows_seen());
        answer.totalSeen = static_cast<uint64_t>(sm.total_seen());
        answer.atPresent = sm.at_present();
        answer.atValue = sm.at_value();
        answer.labelPresent = sm.label_present();
        answer.labelValue = std::string(sm.label_value());
        answer.hitKnown = sm.hit_known();
        answer.hitTrue = sm.hit_true();
        return answer;
    }

    /// Complete the lookup with `result` through the typed interface, and settle
    /// the machine.
    Answer complete(const LookupResult &result) {
        EXPECT_TRUE(sm.completeLookup(token, result)) << "a running invocation's typed completion was refused";
        sm.step();
        return read();
    }

    static LookupResult record(uint8_t rows, uint64_t total, std::optional<int64_t> at,
                               std::optional<std::string> label, std::optional<bool> hit) {
        LookupResult result;
        result.rows = rows;
        result.total = total;
        result.at = at;
        result.label = label;
        result.hit = hit;
        return result;
    }

    Machine sm;
    std::shared_ptr<LookupHost> host;
    uint64_t token = 0;
};

}  // namespace

// Every field that may be absent is: the machine reads three absent values and the
// count every answer carries.
TEST_F(HostResultMayHoldAValueThatIsAbsentAotTest, AnAbsentFieldIsReadAsAbsent) {
    const Answer got = complete(record(3, 5, std::nullopt, std::nullopt, std::nullopt));
    EXPECT_EQ(got.answered, 1);
    EXPECT_EQ(got.rowsSeen, 3);
    EXPECT_EQ(got.totalSeen, 5U);
    EXPECT_FALSE(got.atPresent);
    EXPECT_EQ(got.atValue, 0);
    EXPECT_FALSE(got.labelPresent);
    EXPECT_EQ(got.labelValue, "");
    EXPECT_FALSE(got.hitKnown);
    EXPECT_FALSE(got.hitTrue);
}

// Every field is present: the machine reads each value.
TEST_F(HostResultMayHoldAValueThatIsAbsentAotTest, APresentFieldIsReadAsItsValue) {
    const Answer got = complete(record(4, 6, 41, std::string("ab"), true));
    EXPECT_EQ(got.answered, 1);
    EXPECT_EQ(got.rowsSeen, 4);
    EXPECT_EQ(got.totalSeen, 6U);
    EXPECT_TRUE(got.atPresent);
    EXPECT_EQ(got.atValue, 41);
    EXPECT_TRUE(got.labelPresent);
    EXPECT_EQ(got.labelValue, "ab");
    EXPECT_TRUE(got.hitKnown);
    EXPECT_TRUE(got.hitTrue);
}

// A zero, an empty text and a false flag are values, not absences: each is present,
// and is read as itself.
TEST_F(HostResultMayHoldAValueThatIsAbsentAotTest, AZeroAnEmptyTextAndAFalseFlagAreValuesAndNotAbsences) {
    const Answer got = complete(record(1, 0, 0, std::string(), false));
    EXPECT_EQ(got.answered, 1);
    EXPECT_EQ(got.totalSeen, 0U);
    EXPECT_TRUE(got.atPresent);
    EXPECT_EQ(got.atValue, 0);
    EXPECT_TRUE(got.labelPresent);
    EXPECT_EQ(got.labelValue, "");
    EXPECT_TRUE(got.hitKnown);
    EXPECT_FALSE(got.hitTrue);
}

// Each field is independent: a day with no label and a flag.
TEST_F(HostResultMayHoldAValueThatIsAbsentAotTest, EachFieldIsAbsentOrPresentOnItsOwn) {
    const Answer got = complete(record(2, 8, 7, std::nullopt, true));
    EXPECT_EQ(got.answered, 1);
    EXPECT_EQ(got.rowsSeen, 2);
    EXPECT_EQ(got.totalSeen, 8U);
    EXPECT_TRUE(got.atPresent);
    EXPECT_EQ(got.atValue, 7);
    EXPECT_FALSE(got.labelPresent);
    EXPECT_TRUE(got.hitKnown);
    EXPECT_TRUE(got.hitTrue);
}

// The record's JSON writes the key of every field, `null` where the value is absent,
// in the order the schema declares them. The day is absent in both, so that what is
// held is the key, the `null` and the other values' own spelling; the total, a 64-bit
// integer, is the text of its digits.
TEST_F(HostResultMayHoldAValueThatIsAbsentAotTest, TheRecordIsWrittenWithNullForAnAbsentValue) {
    EXPECT_EQ(record(3, 5, std::nullopt, std::nullopt, std::nullopt).wire(),
              R"({"rows":3,"total":"5","at":null,"label":null,"hit":null})");
    EXPECT_EQ(record(3, 5, std::nullopt, std::string("ab"), false).wire(),
              R"({"rows":3,"total":"5","at":null,"label":"ab","hit":false})");
}

// A 64-bit integer is written as the text of its digits, not as a number: a bare
// number past 2^53 - 1 is refused by the engine's reader, and a quoted one reaches
// the machine exactly. The count is a small integer and stays bare.
TEST_F(HostResultMayHoldAValueThatIsAbsentAotTest, ASixtyFourBitFieldIsWrittenAsTheTextOfItsDigits) {
    EXPECT_EQ(record(1, TOTAL_MAX, PAST_DOUBLES_RANGE, std::nullopt, std::nullopt).wire(),
              R"({"rows":1,"total":"18446744073709551615","at":"9007199254740993","label":null,"hit":null})");
}

// A 64-bit value past 2^53 - 1 crosses the typed interface exactly: the machine
// publishes the value the host wrote, where a number read as a double would have
// rounded the day to 9007199254740992.
TEST_F(HostResultMayHoldAValueThatIsAbsentAotTest, ASixtyFourBitValuePastTheDoublesRangeArrivesExact) {
    const Answer got = complete(record(1, TOTAL_MAX, PAST_DOUBLES_RANGE, std::nullopt, std::nullopt));
    EXPECT_EQ(got.answered, 1);
    EXPECT_TRUE(got.atPresent);
    EXPECT_EQ(got.atValue, PAST_DOUBLES_RANGE);
    EXPECT_EQ(got.totalSeen, TOTAL_MAX);
}

// The least day crosses exactly, as written.
TEST_F(HostResultMayHoldAValueThatIsAbsentAotTest, TheLeastInt64ArrivesExact) {
    const Answer got = complete(record(1, 1, INT64_LEAST, std::nullopt, std::nullopt));
    EXPECT_EQ(got.answered, 1);
    EXPECT_TRUE(got.atPresent);
    EXPECT_EQ(got.atValue, INT64_LEAST);
}

// The greatest day crosses exactly, as written.
TEST_F(HostResultMayHoldAValueThatIsAbsentAotTest, TheGreatestInt64ArrivesExact) {
    const Answer got = complete(record(1, 1, INT64_GREATEST, std::nullopt, std::nullopt));
    EXPECT_EQ(got.answered, 1);
    EXPECT_TRUE(got.atPresent);
    EXPECT_EQ(got.atValue, INT64_GREATEST);
}

// A key left out is not an absent value: every field of the first revision of a
// schema is carried by every delivery, and `null` is the one spelling of absence, so
// a completion that lacks `at` is refused as any typed payload the data does not fit
// is, and the machine reads no answer.
TEST_F(HostResultMayHoldAValueThatIsAbsentAotTest, ACompletionThatLeavesAKeyOutIsRefused) {
    EXPECT_TRUE(
        sm.completeHostInvoke(DECLARED_TYPE, "lookup", token, R"({"rows":1,"total":"1","label":null,"hit":null})"))
        << "a running invocation's completion was refused";
    sm.step();
    EXPECT_EQ(read().answered, 0) << "a record without `at` was read as an answer";
}

}  // namespace SCE::Tests
